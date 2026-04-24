use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::node::NodeGroup;
use opcuasim_core::polling::PollingManager;
use opcuasim_core::subscription::SubscriptionManager;

pub struct ConnectionEntry {
    pub connection: OpcUaConnection,
    pub subscription_mgr: SubscriptionManager,
    #[allow(dead_code)]
    pub polling_mgr: PollingManager,
}

#[derive(Default)]
pub struct BackendState {
    pub connections: RwLock<HashMap<String, ConnectionEntry>>,
    pub groups: RwLock<Vec<NodeGroup>>,
}

impl BackendState {
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }
}
