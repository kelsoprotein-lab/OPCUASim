use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use log::info;

use opcua_client::{DataChangeCallback, Session};
use opcua_types::{MonitoredItemCreateRequest, NodeId, ReadValueId, TimestampsToReturn};

use crate::error::OpcUaSimError;
use crate::node::MonitoredNode;
use crate::output::DataChangeItem;

#[derive(Clone)]
pub struct SubscriptionManager {
    monitored_items: Arc<RwLock<HashMap<String, MonitoredNode>>>,
    update_seq: Arc<RwLock<u64>>,
    subscription_id: Arc<RwLock<Option<u32>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            monitored_items: Arc::new(RwLock::new(HashMap::new())),
            update_seq: Arc::new(RwLock::new(0)),
            subscription_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn add_nodes(
        &self,
        nodes: Vec<MonitoredNode>,
        session: Option<&Arc<Session>>,
    ) -> Result<(), OpcUaSimError> {
        // Insert into local tracking
        {
            let mut items = self.monitored_items.write().await;
            for node in &nodes {
                info!("Adding subscription for node: {}", node.node_id);
                items.insert(node.node_id.clone(), node.clone());
            }
        }

        // If we have a session, create actual OPC UA monitored items
        if let Some(session) = session {
            let sub_id = self.ensure_subscription(session).await?;

            let items_to_create: Vec<MonitoredItemCreateRequest> = nodes.iter()
                .filter_map(|n| {
                    n.node_id.parse::<NodeId>().ok().map(|nid| nid.into())
                })
                .collect();

            if !items_to_create.is_empty() {
                match session.create_monitored_items(sub_id, TimestampsToReturn::Both, items_to_create.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        // If subscription ID is invalid (e.g. after reconnect), recreate it
                        let err_str = format!("{}", e);
                        if err_str.contains("BadSubscriptionIdInvalid") {
                            info!("Subscription {} invalid, recreating...", sub_id);
                            self.reset_subscription_id().await;
                            let new_sub_id = self.ensure_subscription(session).await?;
                            session
                                .create_monitored_items(new_sub_id, TimestampsToReturn::Both, items_to_create)
                                .await
                                .map_err(|e2| OpcUaSimError::SubscriptionError(format!("Retry create monitored items failed: {}", e2)))?;
                        } else {
                            return Err(OpcUaSimError::SubscriptionError(format!("Create monitored items failed: {}", e)));
                        }
                    }
                }
            }

            // Do an initial read to populate values immediately (don't wait for data change)
            self.initial_read(session, &nodes).await;
        }

        Ok(())
    }

    /// Ensure a subscription exists, creating one if needed.
    async fn ensure_subscription(&self, session: &Arc<Session>) -> Result<u32, OpcUaSimError> {
        {
            let sub_id = self.subscription_id.read().await;
            if let Some(id) = *sub_id {
                return Ok(id);
            }
        }

        // Create the subscription with a DataChangeCallback that feeds into our apply_data_changes
        let monitored_items = self.monitored_items.clone();
        let update_seq = self.update_seq.clone();

        let callback = DataChangeCallback::new(move |data_value, monitored_item| {
            let raw_node_id = &monitored_item.item_to_monitor().node_id;
            let node_id_str = format!("{}", raw_node_id);
            info!("DataChange callback for node: {}", node_id_str);
            let value_str = data_value.value.as_ref()
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "null".to_string());
            let data_type_str = data_value.value.as_ref().map(|v| {
                match v.type_id() {
                    opcua_types::variant::VariantTypeId::Empty => "Empty".to_string(),
                    opcua_types::variant::VariantTypeId::Scalar(s) => format!("{}", s),
                    opcua_types::variant::VariantTypeId::Array(s, _) => format!("Array<{}>", s),
                }
            });
            let quality_str = data_value.status
                .as_ref()
                .map(|s| format!("{}", s))
                .unwrap_or_else(|| "Good".to_string());
            let source_ts = data_value.source_timestamp
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_default();
            let server_ts = data_value.server_timestamp
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_default();

            let items = monitored_items.clone();
            let seq = update_seq.clone();
            tokio::spawn(async move {
                let mut monitored = items.write().await;
                let mut seq_val = seq.write().await;
                if let Some(node) = monitored.get_mut(&node_id_str) {
                    *seq_val += 1;
                    node.value = Some(value_str);
                    node.quality = Some(quality_str);
                    node.timestamp = Some(source_ts);
                    node.server_timestamp = Some(server_ts);
                    if let Some(dt) = data_type_str {
                        node.data_type = dt;
                    }
                    node.update_seq = *seq_val;
                }
            });
        });

        let sub_id = session
            .create_subscription(
                Duration::from_millis(1000),  // publishing interval
                300,  // lifetime count (must be >= 3 * max_keep_alive_count)
                10,   // max keep alive count
                0,    // max notifications per publish (0 = unlimited)
                0,    // priority
                true, // publishing enabled
                callback,
            )
            .await
            .map_err(|e| OpcUaSimError::SubscriptionError(format!("Create subscription failed: {}", e)))?;

        {
            let mut sid = self.subscription_id.write().await;
            *sid = Some(sub_id);
        }

        info!("Created OPC UA subscription with id: {}", sub_id);
        Ok(sub_id)
    }

    /// Batch read current values for all nodes in one OPC UA request (per batch of 200).
    async fn initial_read(&self, session: &Arc<Session>, nodes: &[MonitoredNode]) {
        const BATCH_SIZE: usize = 200;

        // Build all ReadValueIds upfront: 4 attributes per node
        const ATTRS_PER_NODE: usize = 4;
        let mut valid_nodes: Vec<(usize, NodeId)> = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            if let Ok(nid) = node.node_id.parse::<NodeId>() {
                valid_nodes.push((i, nid));
            }
        }

        let mut items = self.monitored_items.write().await;
        let mut seq = self.update_seq.write().await;

        for batch in valid_nodes.chunks(BATCH_SIZE) {
            let read_ids: Vec<ReadValueId> = batch.iter().flat_map(|(_, nid)| {
                vec![
                    ReadValueId::new(nid.clone(), opcua_types::AttributeId::DataType),
                    ReadValueId::new(nid.clone(), opcua_types::AttributeId::Value),
                    ReadValueId::new(nid.clone(), opcua_types::AttributeId::AccessLevel),
                    ReadValueId::new(nid.clone(), opcua_types::AttributeId::UserAccessLevel),
                ]
            }).collect();

            match session.read(&read_ids, TimestampsToReturn::Both, 0.0).await {
                Ok(values) => {
                    for (batch_idx, (node_idx, _)) in batch.iter().enumerate() {
                        let dt_dv = values.get(batch_idx * ATTRS_PER_NODE);
                        let val_dv = values.get(batch_idx * ATTRS_PER_NODE + 1);
                        let al_dv = values.get(batch_idx * ATTRS_PER_NODE + 2);
                        let ual_dv = values.get(batch_idx * ATTRS_PER_NODE + 3);

                        let data_type = dt_dv
                            .and_then(|dv| dv.value.as_ref())
                            .map(|v| resolve_data_type(&format!("{}", v)))
                            .unwrap_or_else(|| "Unknown".to_string());

                        let value = val_dv.and_then(|dv| dv.value.as_ref()).map(|v| format!("{}", v));
                        let quality = val_dv.and_then(|dv| dv.status.as_ref()).map(|s| format!("{}", s));
                        let source_ts = val_dv.and_then(|dv| dv.source_timestamp.as_ref()).map(|t| t.to_string());
                        let server_ts = val_dv.and_then(|dv| dv.server_timestamp.as_ref()).map(|t| t.to_string());
                        let is_value_ok = quality.as_deref() != Some("BadAttributeIdInvalid");

                        // Extract access level byte from Variant, handling multiple numeric types
                        let extract_byte = |dv: Option<&opcua_types::DataValue>| -> Option<u8> {
                            let v = dv?.value.as_ref()?;
                            match v {
                                opcua_types::Variant::Byte(b) => Some(*b),
                                opcua_types::Variant::UInt16(u) => Some(*u as u8),
                                opcua_types::Variant::Int16(i) => Some(*i as u8),
                                opcua_types::Variant::UInt32(u) => Some(*u as u8),
                                opcua_types::Variant::Int32(i) => Some(*i as u8),
                                _ => None,
                            }
                        };
                        // Prefer UserAccessLevel; fall back to AccessLevel if unavailable
                        let user_access_level = extract_byte(ual_dv)
                            .or_else(|| extract_byte(al_dv))
                            .unwrap_or(0);

                        if let Some(n) = items.get_mut(&nodes[*node_idx].node_id) {
                            *seq += 1;
                            n.data_type = data_type;
                            n.timestamp = source_ts;
                            n.server_timestamp = server_ts;
                            n.user_access_level = user_access_level;
                            if is_value_ok {
                                n.value = value;
                                n.quality = Some(quality.unwrap_or_else(|| "Good".to_string()));
                            } else {
                                n.value = None;
                                n.quality = Some("N/A".to_string());
                            }
                            n.update_seq = *seq;
                        }
                    }
                    info!("Batch read completed: {} nodes", batch.len());
                }
                Err(e) => {
                    // Mark all nodes in this batch as failed
                    for (node_idx, _) in batch {
                        if let Some(n) = items.get_mut(&nodes[*node_idx].node_id) {
                            *seq += 1;
                            n.quality = Some(format!("ReadError: {}", e));
                            n.update_seq = *seq;
                        }
                    }
                    info!("Batch read failed: {}", e);
                }
            }
        }
        info!("Initial read completed for {} nodes ({} batches)", nodes.len(), valid_nodes.len().div_ceil(BATCH_SIZE));
    }

    /// Reset the subscription ID (e.g. after reconnect)
    async fn reset_subscription_id(&self) {
        let mut sid = self.subscription_id.write().await;
        *sid = None;
    }

    pub async fn remove_nodes(&self, node_ids: &[String]) -> Result<(), OpcUaSimError> {
        let mut items = self.monitored_items.write().await;
        for id in node_ids {
            items.remove(id);
        }
        Ok(())
    }

    pub async fn get_monitored_nodes(&self) -> Vec<MonitoredNode> {
        self.monitored_items.read().await.values().cloned().collect()
    }

    pub async fn get_monitored_nodes_since(&self, since_seq: u64) -> Vec<MonitoredNode> {
        self.monitored_items
            .read()
            .await
            .values()
            .filter(|n| n.update_seq > since_seq)
            .cloned()
            .collect()
    }

    pub async fn apply_data_changes(&self, items: &[DataChangeItem]) {
        let mut monitored = self.monitored_items.write().await;
        let mut seq = self.update_seq.write().await;
        for item in items {
            if let Some(node) = monitored.get_mut(&item.node_id) {
                *seq += 1;
                node.value = Some(item.value.clone());
                node.quality = Some(item.quality.clone());
                node.timestamp = Some(item.timestamp.clone());
                node.update_seq = *seq;
            }
        }
    }

    pub async fn get_update_seq(&self) -> u64 {
        *self.update_seq.read().await
    }
}

/// Resolve OPC UA DataType NodeId to human-readable name
fn resolve_data_type(node_id_str: &str) -> String {
    // OPC UA built-in type NodeIds (namespace 0, numeric identifiers)
    match node_id_str {
        "i=1" => "Boolean".to_string(),
        "i=2" => "SByte".to_string(),
        "i=3" => "Byte".to_string(),
        "i=4" => "Int16".to_string(),
        "i=5" => "UInt16".to_string(),
        "i=6" => "Int32".to_string(),
        "i=7" => "UInt32".to_string(),
        "i=8" => "Int64".to_string(),
        "i=9" => "UInt64".to_string(),
        "i=10" => "Float".to_string(),
        "i=11" => "Double".to_string(),
        "i=12" => "String".to_string(),
        "i=13" => "DateTime".to_string(),
        "i=14" => "Guid".to_string(),
        "i=15" => "ByteString".to_string(),
        "i=16" => "XmlElement".to_string(),
        "i=17" => "NodeId".to_string(),
        "i=19" => "StatusCode".to_string(),
        "i=20" => "QualifiedName".to_string(),
        "i=21" => "LocalizedText".to_string(),
        "i=22" => "ExtensionObject".to_string(),
        "i=24" => "BaseDataType".to_string(),
        "i=26" => "Number".to_string(),
        "i=27" => "Integer".to_string(),
        "i=28" => "UInteger".to_string(),
        "i=29" => "Enumeration".to_string(),
        other => other.to_string(),
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
