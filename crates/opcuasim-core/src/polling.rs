use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use log::info;

use crate::error::OpcUaSimError;
use crate::node::MonitoredNode;

pub struct PollingManager {
    polling_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    monitored_items: Arc<RwLock<HashMap<String, MonitoredNode>>>,
}

impl PollingManager {
    pub fn new() -> Self {
        Self {
            polling_tasks: Arc::new(RwLock::new(HashMap::new())),
            monitored_items: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_polling_node(&self, node: MonitoredNode, interval_ms: u64) -> Result<(), OpcUaSimError> {
        let node_id = node.node_id.clone();
        info!("Adding polling for node: {} (interval: {}ms)", node_id, interval_ms);

        {
            let mut items = self.monitored_items.write().await;
            items.insert(node_id.clone(), node);
        }

        let items = self.monitored_items.clone();
        let nid = node_id.clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                // TODO: Task 8 will implement actual OPC UA read here
                let _items = items.read().await;
                if !_items.contains_key(&nid) {
                    break;
                }
            }
        });

        let mut tasks = self.polling_tasks.write().await;
        if let Some(old_handle) = tasks.insert(node_id, handle) {
            old_handle.abort();
        }

        Ok(())
    }

    pub async fn remove_polling_node(&self, node_id: &str) {
        let mut tasks = self.polling_tasks.write().await;
        if let Some(handle) = tasks.remove(node_id) {
            handle.abort();
        }
        let mut items = self.monitored_items.write().await;
        items.remove(node_id);
    }

    pub async fn stop_all(&self) {
        let mut tasks = self.polling_tasks.write().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }

    pub async fn get_polling_nodes(&self) -> Vec<MonitoredNode> {
        self.monitored_items.read().await.values().cloned().collect()
    }
}

impl Default for PollingManager {
    fn default() -> Self {
        Self::new()
    }
}
