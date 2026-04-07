use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::info;

use crate::error::OpcUaSimError;
use crate::node::MonitoredNode;
use crate::output::DataChangeItem;

pub struct SubscriptionManager {
    monitored_items: Arc<RwLock<HashMap<String, MonitoredNode>>>,
    update_seq: Arc<RwLock<u64>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            monitored_items: Arc::new(RwLock::new(HashMap::new())),
            update_seq: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn add_nodes(&self, nodes: Vec<MonitoredNode>) -> Result<(), OpcUaSimError> {
        let mut items = self.monitored_items.write().await;
        for node in nodes {
            info!("Adding subscription for node: {}", node.node_id);
            items.insert(node.node_id.clone(), node);
        }
        // TODO: Task 8 will create actual OPC UA monitored items.
        Ok(())
    }

    pub async fn remove_nodes(&self, node_ids: &[String]) -> Result<(), OpcUaSimError> {
        let mut items = self.monitored_items.write().await;
        for id in node_ids {
            items.remove(id);
        }
        // TODO: Task 8 will remove actual OPC UA monitored items.
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
