use std::collections::HashMap;
use std::sync::RwLock;
use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::node::NodeGroup;
use serde::Serialize;

pub struct ConnectionEntry {
    pub connection: OpcUaConnection,
}

pub struct AppState {
    pub connections: RwLock<HashMap<String, ConnectionEntry>>,
    pub groups: RwLock<Vec<NodeGroup>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            groups: RwLock::new(Vec::new()),
        }
    }
}

// DTOs for frontend

#[derive(Serialize)]
pub struct ConnectionInfoDto {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth_type: String,
    pub state: String,
}

#[derive(Serialize)]
pub struct BrowseResultDto {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub data_type: Option<String>,
    pub has_children: bool,
}

#[derive(Serialize)]
pub struct MonitoredNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub browse_path: String,
    pub data_type: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
    pub access_mode: String,
    pub interval_ms: f64,
    pub group_id: Option<String>,
}

#[derive(Serialize)]
pub struct NodeGroupDto {
    pub id: String,
    pub name: String,
    pub node_count: usize,
}

#[derive(Clone, Serialize)]
pub struct ConnectionStateEvent {
    pub id: String,
    pub state: String,
}

#[derive(Clone, Serialize)]
pub struct DataChangedEvent {
    pub connection_id: String,
    pub items: Vec<DataChangeItemDto>,
}

#[derive(Clone, Serialize)]
pub struct DataChangeItemDto {
    pub node_id: String,
    pub value: String,
    pub quality: String,
    pub timestamp: String,
}
