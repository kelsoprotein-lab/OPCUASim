use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum UiCommand {
    CreateConnection(CreateConnectionReq),
    Connect(String),
    Disconnect(String),
    DeleteConnection(String),
    ListConnections,
    BrowseRoot {
        conn_id: String,
        req_id: u64,
    },
    BrowseNode {
        conn_id: String,
        node_id: String,
        req_id: u64,
    },
    AddMonitoredNodes {
        conn_id: String,
        nodes: Vec<MonitoredNodeReq>,
    },
    AddVariablesUnderNode {
        conn_id: String,
        node_id: String,
        access_mode: String,
        interval_ms: f64,
        max_depth: u32,
    },
    RemoveMonitoredNodes {
        conn_id: String,
        node_ids: Vec<String>,
    },
    ReadAttrs {
        conn_id: String,
        node_id: String,
        req_id: u64,
    },
    WriteValue {
        conn_id: String,
        node_id: String,
        value: String,
        data_type: String,
        req_id: u64,
    },
    ClearCommLogs(String),
    ExportCommLogs {
        conn_id: String,
        path: std::path::PathBuf,
    },
    CreateGroup(String),
    DeleteGroup(String),
    AddNodesToGroup {
        group_id: String,
        node_ids: Vec<String>,
    },
    ListGroups,
    SaveProject(std::path::PathBuf),
    LoadProject(std::path::PathBuf),
}

#[derive(Debug, Clone)]
pub struct CreateConnectionReq {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthKindReq,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub enum AuthKindReq {
    Anonymous,
    UserPassword { username: String, password: String },
    Certificate { cert_path: String, key_path: String },
}

#[derive(Debug, Clone)]
pub struct MonitoredNodeReq {
    pub node_id: String,
    pub display_name: String,
    pub data_type: Option<String>,
    pub access_mode: String, // "Subscription" or "Polling"
    pub interval_ms: f64,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Connections(Vec<ConnectionInfo>),
    ConnectionStateChanged {
        id: String,
        state: String,
    },
    BrowseResult {
        req_id: u64,
        parent: Option<String>,
        items: Vec<BrowseItem>,
    },
    MonitoredSnapshot {
        conn_id: String,
        seq: u64,
        full: bool,
        nodes: Vec<MonitoredRow>,
    },
    NodeAttrs {
        req_id: u64,
        attrs: NodeAttrsDto,
    },
    WriteOk {
        req_id: u64,
        node_id: String,
    },
    CommLogEntries {
        conn_id: String,
        entries: Vec<LogRow>,
    },
    LogsCleared {
        conn_id: String,
    },
    Groups(Vec<NodeGroupDto>),
    Toast {
        level: ToastLevel,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth_type: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseItem {
    pub node_id: String,
    pub display_name: String,
    pub node_class: String,
    pub data_type: Option<String>,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredRow {
    pub node_id: String,
    pub display_name: String,
    pub data_type: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub source_timestamp: Option<String>,
    pub server_timestamp: Option<String>,
    pub access_mode: String,
    pub interval_ms: f64,
    pub update_seq: u64,
    pub user_access_level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAttrsDto {
    pub node_id: String,
    pub display_name: String,
    pub description: String,
    pub data_type: String,
    pub access_level: String,
    pub value: Option<String>,
    pub quality: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRow {
    pub seq: u64,
    pub timestamp_ms: i64, // utc millis
    pub direction: String, // "Request" | "Response"
    pub service: String,
    pub detail: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeGroupDto {
    pub id: String,
    pub name: String,
    pub node_ids: Vec<String>,
}
