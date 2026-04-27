use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessMode {
    Subscription { interval_ms: f64 },
    Polling { interval_ms: u64 },
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::Subscription { interval_ms: 1000.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DataChangeTriggerKind {
    Status,
    #[default]
    StatusValue,
    StatusValueTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeadbandKind {
    #[default]
    None,
    Absolute,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct DataChangeFilterCfg {
    #[serde(default)]
    pub trigger: DataChangeTriggerKind,
    #[serde(default)]
    pub deadband_kind: DeadbandKind,
    #[serde(default)]
    pub deadband_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredNode {
    pub node_id: String,
    pub display_name: String,
    pub browse_path: String,
    pub data_type: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
    pub server_timestamp: Option<String>,
    pub access_mode: AccessMode,
    pub group_id: Option<String>,
    pub update_seq: u64,
    /// OPC UA UserAccessLevel bitmask (bit 0=Read, bit 1=Write). 0 = unknown.
    pub user_access_level: u8,
    #[serde(default)]
    pub filter: Option<DataChangeFilterCfg>,
}

impl MonitoredNode {
    pub fn new(node_id: String, display_name: String, browse_path: String, data_type: String) -> Self {
        Self {
            node_id,
            display_name,
            browse_path,
            data_type,
            value: None,
            quality: None,
            timestamp: None,
            server_timestamp: None,
            access_mode: AccessMode::default(),
            group_id: None,
            update_seq: 0,
            user_access_level: 0,
            filter: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroup {
    pub id: String,
    pub name: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResultItem {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub data_type: Option<String>,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttributes {
    pub node_id: String,
    pub display_name: String,
    pub description: String,
    pub data_type: String,
    pub access_level: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
}
