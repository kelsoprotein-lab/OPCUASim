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
                session
                    .create_monitored_items(sub_id, TimestampsToReturn::Both, items_to_create)
                    .await
                    .map_err(|e| OpcUaSimError::SubscriptionError(format!("Create monitored items failed: {}", e)))?;
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
            let quality_str = data_value.status
                .as_ref()
                .map(|s| format!("{}", s))
                .unwrap_or_else(|| "Good".to_string());
            let timestamp_str = data_value.source_timestamp
                .as_ref()
                .map(|t| t.to_string())
                .unwrap_or_default();

            // We must use a blocking approach here since callback is FnMut, not async
            let items = monitored_items.clone();
            let seq = update_seq.clone();
            tokio::spawn(async move {
                let mut monitored = items.write().await;
                let mut seq_val = seq.write().await;
                if let Some(node) = monitored.get_mut(&node_id_str) {
                    *seq_val += 1;
                    node.value = Some(value_str);
                    node.quality = Some(quality_str);
                    node.timestamp = Some(timestamp_str);
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

    /// Read current values for nodes immediately after adding them.
    /// Reads each node individually to avoid one bad node failing the entire batch.
    async fn initial_read(&self, session: &Arc<Session>, nodes: &[MonitoredNode]) {
        let mut items = self.monitored_items.write().await;
        let mut seq = self.update_seq.write().await;

        for node in nodes {
            let node_id = match node.node_id.parse::<NodeId>() {
                Ok(nid) => nid,
                Err(_) => continue,
            };

            let read_ids = vec![ReadValueId::from(node_id)];
            match session.read(&read_ids, TimestampsToReturn::Both, 0.0).await {
                Ok(values) => {
                    if let Some(dv) = values.first() {
                        if let Some(n) = items.get_mut(&node.node_id) {
                            *seq += 1;
                            n.value = dv.value.as_ref().map(|v| format!("{}", v));
                            n.quality = Some(
                                dv.status.as_ref()
                                    .map(|s| format!("{}", s))
                                    .unwrap_or_else(|| "Good".to_string())
                            );
                            n.timestamp = dv.source_timestamp.as_ref().map(|t| t.to_string());
                            n.update_seq = *seq;
                            info!("Initial read for {}: value={:?}", node.node_id, n.value);
                        }
                    }
                }
                Err(e) => {
                    // Mark the node so it shows up in DataTable even if read fails
                    if let Some(n) = items.get_mut(&node.node_id) {
                        *seq += 1;
                        n.quality = Some(format!("ReadError: {}", e));
                        n.update_seq = *seq;
                    }
                    info!("Initial read failed for {}: {}", node.node_id, e);
                }
            }
        }
        info!("Initial read completed for {} nodes", nodes.len());
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

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
