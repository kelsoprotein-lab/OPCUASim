use opcuasim_core::server::models::{DataType, ServerConfig, SimulationMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum UiCommand {
    StartServer,
    StopServer,
    RefreshStatus,
    RefreshAddressSpace,
    AddFolder {
        node_id: String,
        display_name: String,
        parent_id: String,
    },
    AddNode(AddNodeReq),
    RemoveNode(String),
    UpdateNode {
        node_id: String,
        display_name: Option<String>,
        data_type: Option<DataType>,
        writable: Option<bool>,
        simulation: Option<SimulationMode>,
    },
    LoadProject(std::path::PathBuf),
    SaveProject(std::path::PathBuf),
}

#[derive(Debug, Clone)]
pub struct AddNodeReq {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
    pub data_type: DataType,
    pub writable: bool,
    pub simulation: SimulationMode,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Status(ServerStatus),
    AddressSpace(AddressSpaceDto),
    SimValues { seq: u64, values: Vec<(String, String)> },
    Config(ServerConfig),
    Toast { level: ToastLevel, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerStatus {
    pub state: String,
    pub node_count: usize,
    pub folder_count: usize,
    pub endpoint_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct AddressSpaceDto {
    pub folders: Vec<FolderRow>,
    pub nodes: Vec<NodeRow>,
}

#[derive(Debug, Clone)]
pub struct FolderRow {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
}

#[derive(Debug, Clone)]
pub struct NodeRow {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
    pub data_type: DataType,
    pub writable: bool,
    pub simulation: SimulationMode,
    pub current_value: Option<String>,
}
