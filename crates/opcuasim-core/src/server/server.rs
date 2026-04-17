use std::sync::Arc;

use log::info;
use tokio::sync::RwLock;

use opcua_server::node_manager::memory::{
    simple_node_manager, SimpleNodeManager,
};
use opcua_server::diagnostics::NamespaceMetadata;
use opcua_server::{
    Server, ServerBuilder, ServerHandle, ServerUserToken, SubscriptionCache,
    ANONYMOUS_USER_TOKEN_ID,
};
use opcua_types::MessageSecurityMode;
use opcua_crypto::SecurityPolicy;

use super::address_space::populate_address_space;
use super::models::{ServerConfig, ServerFolder, ServerNode, ServerState};
use super::simulation::SimulationEngine;
use crate::error::OpcUaSimError;

const NAMESPACE_URI: &str = "urn:opcuasim:server";

/// The OPC UA simulation server.
pub struct OpcUaServer {
    state: Arc<RwLock<ServerState>>,
    handle: Arc<RwLock<Option<ServerHandle>>>,
    node_manager: Arc<RwLock<Option<Arc<SimpleNodeManager>>>>,
    simulation_engine: Arc<RwLock<Option<Arc<SimulationEngine>>>>,
    namespace_index: Arc<RwLock<u16>>,
}

/// Result of building the server (all sync, no async).
struct BuildResult {
    server: Server,
    handle: ServerHandle,
    node_manager: Arc<SimpleNodeManager>,
    namespace_index: u16,
    subscriptions: Arc<SubscriptionCache>,
}

/// Build the OPC UA server synchronously (ServerBuilder is not Send).
fn build_server(
    config: &ServerConfig,
    folders: &[ServerFolder],
    nodes: &[ServerNode],
) -> Result<BuildResult, OpcUaSimError> {
    // Build user tokens
    let mut user_token_ids: Vec<String> = Vec::new();
    let mut builder = ServerBuilder::new()
        .application_name(&config.name)
        .application_uri("urn:opcuasim:server")
        .product_uri("urn:opcuasim")
        .create_sample_keypair(true)
        .pki_dir("./pki-server")
        .host("0.0.0.0")
        .port(config.port)
        .trust_client_certs(true)
        .with_node_manager(simple_node_manager(
            NamespaceMetadata {
                namespace_uri: NAMESPACE_URI.to_string(),
                ..Default::default()
            },
            "SimNodeManager",
        ));

    if config.anonymous_enabled {
        user_token_ids.push(ANONYMOUS_USER_TOKEN_ID.to_string());
    }

    for user in &config.users {
        let token_id = format!("user_{}", user.username);
        builder = builder.add_user_token(
            &token_id,
            ServerUserToken {
                user: user.username.clone(),
                pass: Some(user.password.clone()),
                ..Default::default()
            },
        );
        user_token_ids.push(token_id);
    }

    let endpoint_path = "/";
    let token_ids_ref: Vec<&str> = user_token_ids.iter().map(|s| s.as_str()).collect();

    builder = builder.add_endpoint(
        "none",
        (
            endpoint_path,
            SecurityPolicy::None,
            MessageSecurityMode::None,
            &token_ids_ref as &[&str],
        ),
    );

    for policy in &config.security_policies {
        for mode in &config.security_modes {
            if policy == "None" && mode == "None" {
                continue;
            }
            let sec_policy = match policy.as_str() {
                "Basic128Rsa15" => SecurityPolicy::Basic128Rsa15,
                "Basic256" => SecurityPolicy::Basic256,
                "Basic256Sha256" => SecurityPolicy::Basic256Sha256,
                "Aes128Sha256RsaOaep" => SecurityPolicy::Aes128Sha256RsaOaep,
                "Aes256Sha256RsaPss" => SecurityPolicy::Aes256Sha256RsaPss,
                _ => continue,
            };
            let sec_mode = match mode.as_str() {
                "Sign" => MessageSecurityMode::Sign,
                "SignAndEncrypt" => MessageSecurityMode::SignAndEncrypt,
                _ => continue,
            };
            let id = format!("{}_{}", policy.to_lowercase(), mode.to_lowercase());
            builder = builder.add_endpoint(
                &id,
                (endpoint_path, sec_policy, sec_mode, &token_ids_ref as &[&str]),
            );
        }
    }

    builder = builder.discovery_urls(vec![endpoint_path.to_string()]);

    let (server, handle) = builder.build().map_err(|e| {
        OpcUaSimError::ServerError(format!("Server build failed: {}", e))
    })?;

    let node_managers = handle.node_managers();
    let sim_nm = node_managers
        .get_of_type::<SimpleNodeManager>()
        .ok_or_else(|| OpcUaSimError::ServerError("SimpleNodeManager not found".into()))?;

    let ns_index = {
        let ns = sim_nm.namespaces();
        ns.keys().find(|&&k| k > 1).copied().unwrap_or(2)
    };

    // Populate address space (sync)
    {
        let mut address_space = sim_nm.address_space().write();
        populate_address_space(&mut address_space, ns_index, folders, nodes);
    }
    info!("Address space populated: {} folders, {} nodes", folders.len(), nodes.len());

    let subscriptions = server.subscriptions();

    Ok(BuildResult {
        server,
        handle,
        node_manager: sim_nm,
        namespace_index: ns_index,
        subscriptions,
    })
}

impl OpcUaServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::Stopped)),
            handle: Arc::new(RwLock::new(None)),
            node_manager: Arc::new(RwLock::new(None)),
            simulation_engine: Arc::new(RwLock::new(None)),
            namespace_index: Arc::new(RwLock::new(2)),
        }
    }

    /// Start the OPC UA server with the given configuration.
    pub async fn start(
        &self,
        config: &ServerConfig,
        folders: &[ServerFolder],
        nodes: &[ServerNode],
    ) -> Result<(), OpcUaSimError> {
        {
            let state = self.state.read().await;
            if *state != ServerState::Stopped {
                return Err(OpcUaSimError::ServerError("Server is not stopped".into()));
            }
        }
        *self.state.write().await = ServerState::Starting;

        info!("Starting OPC UA server on port {}", config.port);

        // Build server synchronously (ServerBuilder is not Send)
        let config_clone = config.clone();
        let folders_clone = folders.to_vec();
        let nodes_clone = nodes.to_vec();

        let build_result = tokio::task::spawn_blocking(move || {
            build_server(&config_clone, &folders_clone, &nodes_clone)
        })
        .await
        .map_err(|e| OpcUaSimError::ServerError(format!("Build task failed: {}", e)))??;

        let BuildResult {
            server,
            handle,
            node_manager: sim_nm,
            namespace_index: ns_index,
            subscriptions,
        } = build_result;

        *self.namespace_index.write().await = ns_index;
        *self.handle.write().await = Some(handle);
        *self.node_manager.write().await = Some(sim_nm.clone());

        // Start simulation engine
        let sim_engine = Arc::new(SimulationEngine::new());
        sim_engine.register_nodes(nodes, ns_index).await;
        sim_engine.start(sim_nm, subscriptions);
        *self.simulation_engine.write().await = Some(sim_engine.clone());

        // Run server in background task
        let state = self.state.clone();
        let sim_engine_bg = sim_engine.clone();

        tokio::spawn(async move {
            *state.write().await = ServerState::Running;
            info!("OPC UA server is running");

            let result = server.run().await;

            sim_engine_bg.stop();
            *state.write().await = ServerState::Stopped;

            match result {
                Ok(_) => info!("OPC UA server stopped normally"),
                Err(e) => info!("OPC UA server stopped with error: {}", e),
            }
        });

        // Wait briefly for server to start
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        Ok(())
    }

    /// Stop the server.
    pub async fn stop(&self) -> Result<(), OpcUaSimError> {
        let handle = self.handle.read().await;
        if let Some(ref h) = *handle {
            *self.state.write().await = ServerState::Stopping;
            info!("Stopping OPC UA server");
            h.cancel();
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Ok(())
        } else {
            Err(OpcUaSimError::ServerError("Server is not running".into()))
        }
    }

    /// Get the current server state.
    pub async fn state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// Get the current namespace index.
    pub async fn namespace_index(&self) -> u16 {
        *self.namespace_index.read().await
    }

    /// Get a reference to the node manager (if server is running).
    pub async fn node_manager(&self) -> Option<Arc<SimpleNodeManager>> {
        self.node_manager.read().await.clone()
    }

    /// Get a reference to the simulation engine (if server is running).
    pub async fn simulation_engine(&self) -> Option<Arc<SimulationEngine>> {
        self.simulation_engine.read().await.clone()
    }
}

impl Default for OpcUaServer {
    fn default() -> Self {
        Self::new()
    }
}
