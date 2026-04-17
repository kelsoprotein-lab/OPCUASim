use std::sync::{Arc, RwLock};

use opcuasim_core::server::models::{ServerConfig, ServerFolder, ServerNode};
use opcuasim_core::server::server::OpcUaServer;
use serde::Serialize;

pub struct AppState {
    pub server: Arc<OpcUaServer>,
    pub config: RwLock<ServerConfig>,
    pub folders: RwLock<Vec<ServerFolder>>,
    pub nodes: RwLock<Vec<ServerNode>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            server: Arc::new(OpcUaServer::new()),
            config: RwLock::new(ServerConfig::default()),
            folders: RwLock::new(Vec::new()),
            nodes: RwLock::new(Vec::new()),
        }
    }
}

// DTOs for frontend

#[derive(Serialize)]
pub struct ServerStatusDto {
    pub state: String,
    pub node_count: usize,
    pub folder_count: usize,
}

#[derive(Serialize)]
pub struct ServerNodeDto {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
    pub data_type: String,
    pub writable: bool,
    pub simulation: serde_json::Value,
    pub current_value: Option<String>,
}

#[derive(Serialize)]
pub struct ServerFolderDto {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
}

#[derive(Serialize)]
pub struct AddressSpaceDto {
    pub folders: Vec<ServerFolderDto>,
    pub nodes: Vec<ServerNodeDto>,
}
