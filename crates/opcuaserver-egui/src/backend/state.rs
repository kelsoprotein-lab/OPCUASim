use std::sync::{Arc, RwLock};

use opcuasim_core::server::models::{ServerConfig, ServerFolder, ServerNode};
use opcuasim_core::server::server::OpcUaServer;

pub struct BackendState {
    pub server: Arc<OpcUaServer>,
    pub config: RwLock<ServerConfig>,
    pub folders: RwLock<Vec<ServerFolder>>,
    pub nodes: RwLock<Vec<ServerNode>>,
}

impl BackendState {
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self {
            server: Arc::new(OpcUaServer::new()),
            config: RwLock::new(ServerConfig::default()),
            folders: RwLock::new(Vec::new()),
            nodes: RwLock::new(Vec::new()),
        })
    }
}
