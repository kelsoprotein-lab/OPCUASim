use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::config::{AuthConfig, ConnectionConfig, ProjectFile, ConnectionProjectEntry};
use opcuasim_core::subscription::SubscriptionManager;
use opcuasim_core::polling::PollingManager;
use opcuasim_core::node::{AccessMode, MonitoredNode, NodeGroup};

use crate::state::{
    AppState, ConnectionEntry, ConnectionInfoDto, ConnectionStateEvent,
    NodeGroupDto, BrowseResultDto, NodeAttributesDto, MonitoredNodeDto,
    MonitoredDataDto,
};

// ── Connection Management ──────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: Option<String>,
    pub security_mode: Option<String>,
    pub auth_type: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[tauri::command]
pub fn create_connection(
    state: State<'_, AppState>,
    request: CreateConnectionRequest,
) -> Result<ConnectionInfoDto, String> {
    let id = Uuid::new_v4().to_string();

    let auth = match request.auth_type.as_deref() {
        Some("UserPassword") => AuthConfig::UserPassword {
            username: request.username.unwrap_or_default(),
            password: request.password.unwrap_or_default(),
        },
        Some("Certificate") => AuthConfig::Certificate {
            cert_path: request.cert_path.unwrap_or_default(),
            key_path: request.key_path.unwrap_or_default(),
        },
        _ => AuthConfig::Anonymous,
    };

    let config = ConnectionConfig {
        id: id.clone(),
        name: request.name.clone(),
        endpoint_url: request.endpoint_url.clone(),
        security_policy: request.security_policy.unwrap_or_else(|| "None".to_string()),
        security_mode: request.security_mode.unwrap_or_else(|| "None".to_string()),
        auth,
        timeout_ms: request.timeout_ms.unwrap_or(5000),
    };

    let auth_type = match &config.auth {
        AuthConfig::Anonymous => "Anonymous",
        AuthConfig::UserPassword { .. } => "UserPassword",
        AuthConfig::Certificate { .. } => "Certificate",
    };

    let dto = ConnectionInfoDto {
        id: id.clone(),
        name: config.name.clone(),
        endpoint_url: config.endpoint_url.clone(),
        security_policy: config.security_policy.clone(),
        security_mode: config.security_mode.clone(),
        auth_type: auth_type.to_string(),
        state: "Disconnected".to_string(),
    };

    let connection = OpcUaConnection::new(config);
    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.insert(id, ConnectionEntry {
        connection,
        subscription_mgr: SubscriptionManager::new(),
        polling_mgr: PollingManager::new(),
    });

    Ok(dto)
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let (conn_id, state_arc, config_clone) = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        (
            entry.connection.config.id.clone(),
            entry.connection.state.clone(),
            entry.connection.config.clone(),
        )
    };
    // Guard dropped — safe to await.

    use opcuasim_core::client::ConnectionState;
    *state_arc.write().await = ConnectionState::Connecting;

    let _ = app.emit("connection-state-changed", ConnectionStateEvent {
        id: conn_id.clone(),
        state: "Connecting".to_string(),
    });

    // Create a new OpcUaConnection for the actual connection attempt
    let temp_conn = OpcUaConnection::new(config_clone);
    match temp_conn.connect().await {
        Ok(()) => {
            // Replace the connection entry with the connected one
            {
                let mut connections = state.connections.write().map_err(|e| e.to_string())?;
                if let Some(entry) = connections.get_mut(&id) {
                    entry.connection = temp_conn;
                    // Reset subscription manager for the new session
                    entry.subscription_mgr = SubscriptionManager::new();
                }
            }

            *state_arc.write().await = ConnectionState::Connected;

            let _ = app.emit("connection-state-changed", ConnectionStateEvent {
                id: conn_id,
                state: "Connected".to_string(),
            });
            Ok(())
        }
        Err(e) => {
            *state_arc.write().await = ConnectionState::Disconnected;
            let _ = app.emit("connection-state-changed", ConnectionStateEvent {
                id: conn_id,
                state: "Disconnected".to_string(),
            });
            Err(format!("Connection failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Clone the state arc and get session before any await
    let state_arc = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        entry.connection.state.clone()
    };
    // Guard dropped.

    // Extract the config. Can't hold std::sync::RwLock across await.
    let config = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        entry.connection.config.clone()
    };

    // Replace connection with a fresh disconnected one (drops old session)
    {
        let mut connections = state.connections.write().map_err(|e| e.to_string())?;
        if let Some(entry) = connections.get_mut(&id) {
            // Replace with a fresh disconnected connection (drops old one, closing session)
            entry.connection = OpcUaConnection::new(config);
        }
    }

    use opcuasim_core::client::ConnectionState;
    *state_arc.write().await = ConnectionState::Disconnected;

    let _ = app.emit("connection-state-changed", ConnectionStateEvent {
        id: id.clone(),
        state: "Disconnected".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub fn delete_connection(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.remove(&id).ok_or("Connection not found")?;
    Ok(())
}

#[tauri::command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionInfoDto>, String> {
    let entries: Vec<(String, String, String, String, String, AuthConfig, std::sync::Arc<tokio::sync::RwLock<opcuasim_core::client::ConnectionState>>)> = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        connections.iter().map(|(id, entry)| {
            (
                id.clone(),
                entry.connection.config.name.clone(),
                entry.connection.config.endpoint_url.clone(),
                entry.connection.config.security_policy.clone(),
                entry.connection.config.security_mode.clone(),
                entry.connection.config.auth.clone(),
                entry.connection.state.clone(),
            )
        }).collect()
    };

    let mut result = Vec::new();
    for (id, name, endpoint_url, security_policy, security_mode, auth, state_arc) in entries {
        let conn_state = state_arc.read().await.clone();
        let auth_type = match &auth {
            AuthConfig::Anonymous => "Anonymous",
            AuthConfig::UserPassword { .. } => "UserPassword",
            AuthConfig::Certificate { .. } => "Certificate",
        };
        result.push(ConnectionInfoDto {
            id,
            name,
            endpoint_url,
            security_policy,
            security_mode,
            auth_type: auth_type.to_string(),
            state: conn_state.to_string(),
        });
    }

    Ok(result)
}

// ── Browse Commands ──────────────────────────────────────────

#[tauri::command]
pub async fn browse_root(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<Vec<BrowseResultDto>, String> {
    let session = get_session_from_state(&state, &conn_id).await?;

    let items = opcuasim_core::browse::browse_node(&session, None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(items.into_iter().map(|item| BrowseResultDto {
        node_id: item.node_id,
        display_name: item.display_name,
        node_class: item.node_class,
        data_type: item.data_type,
        has_children: item.has_children,
    }).collect())
}

#[tauri::command]
pub async fn browse_node(
    state: State<'_, AppState>,
    conn_id: String,
    node_id: String,
) -> Result<Vec<BrowseResultDto>, String> {
    let session = get_session_from_state(&state, &conn_id).await?;

    let items = opcuasim_core::browse::browse_node(&session, Some(&node_id))
        .await
        .map_err(|e| e.to_string())?;

    Ok(items.into_iter().map(|item| BrowseResultDto {
        node_id: item.node_id,
        display_name: item.display_name,
        node_class: item.node_class,
        data_type: item.data_type,
        has_children: item.has_children,
    }).collect())
}

/// Extract the session Arc from state without holding std::sync::RwLock across await.
/// Clones the session holder Arc synchronously, drops the std lock, then awaits the tokio lock.
async fn get_session_from_state(
    state: &State<'_, AppState>,
    conn_id: &str,
) -> Result<std::sync::Arc<opcuasim_core::OpcUaSession>, String> {
    let session_holder = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(conn_id).ok_or("Connection not found")?;
        entry.connection.get_session_holder()
    };
    // std lock dropped — safe to await.
    let guard = session_holder.read().await;
    guard.clone().ok_or_else(|| "Not connected — no active session".to_string())
}

#[tauri::command]
pub async fn read_node_attributes(
    state: State<'_, AppState>,
    conn_id: String,
    node_id: String,
) -> Result<NodeAttributesDto, String> {
    let session = get_session_from_state(&state, &conn_id).await?;

    let attrs = opcuasim_core::browse::read_node_attributes(&session, &node_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(NodeAttributesDto {
        node_id: attrs.node_id,
        display_name: attrs.display_name,
        description: attrs.description,
        data_type: attrs.data_type,
        access_level: attrs.access_level,
        value: attrs.value,
        quality: attrs.quality,
        timestamp: attrs.timestamp,
    })
}

// ── Monitor Commands ──────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct AddMonitoredNodesRequest {
    pub conn_id: String,
    pub nodes: Vec<MonitoredNodeRequest>,
}

#[derive(serde::Deserialize)]
pub struct MonitoredNodeRequest {
    pub node_id: String,
    pub display_name: String,
    pub browse_path: Option<String>,
    pub data_type: Option<String>,
    pub access_mode: Option<String>,
    pub interval_ms: Option<f64>,
    pub group_id: Option<String>,
}

#[tauri::command]
pub async fn add_monitored_nodes(
    state: State<'_, AppState>,
    request: AddMonitoredNodesRequest,
) -> Result<(), String> {
    let nodes: Vec<MonitoredNode> = request.nodes.into_iter().map(|n| {
        let access_mode = match n.access_mode.as_deref() {
            Some("Polling") => AccessMode::Polling {
                interval_ms: n.interval_ms.unwrap_or(1000.0) as u64,
            },
            _ => AccessMode::Subscription {
                interval_ms: n.interval_ms.unwrap_or(1000.0),
            },
        };
        MonitoredNode {
            node_id: n.node_id,
            display_name: n.display_name,
            browse_path: n.browse_path.unwrap_or_default(),
            data_type: n.data_type.unwrap_or_else(|| "Unknown".to_string()),
            value: None,
            quality: None,
            timestamp: None,
            access_mode,
            group_id: n.group_id,
            update_seq: 0,
        }
    }).collect();

    // Clone SubscriptionManager (cheap Arc clones), drop std lock, then await.
    let (sub_mgr, session_holder) = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&request.conn_id).ok_or("Connection not found")?;
        (entry.subscription_mgr.clone(), entry.connection.get_session_holder())
    };
    // std lock dropped — safe to await.

    let session_guard = session_holder.read().await;
    let session = session_guard.as_ref();

    sub_mgr.add_nodes(nodes, session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn remove_monitored_nodes(
    state: State<'_, AppState>,
    conn_id: String,
    node_ids: Vec<String>,
) -> Result<(), String> {
    let sub_mgr = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&conn_id).ok_or("Connection not found")?;
        entry.subscription_mgr.clone()
    };
    sub_mgr.remove_nodes(&node_ids)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_monitored_data(
    state: State<'_, AppState>,
    conn_id: String,
    since_seq: u64,
) -> Result<MonitoredDataDto, String> {
    let sub_mgr = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&conn_id).ok_or("Connection not found")?;
        entry.subscription_mgr.clone()
    };
    // std lock dropped — safe to await.

    let nodes = if since_seq == 0 {
        sub_mgr.get_monitored_nodes().await
    } else {
        sub_mgr.get_monitored_nodes_since(since_seq).await
    };
    let seq = sub_mgr.get_update_seq().await;

    let node_dtos: Vec<MonitoredNodeDto> = nodes.into_iter().map(|n| {
        let (access_mode_str, interval_ms) = match &n.access_mode {
            AccessMode::Subscription { interval_ms } => ("Subscription".to_string(), *interval_ms),
            AccessMode::Polling { interval_ms } => ("Polling".to_string(), *interval_ms as f64),
        };
        MonitoredNodeDto {
            node_id: n.node_id,
            display_name: n.display_name,
            browse_path: n.browse_path,
            data_type: n.data_type,
            value: n.value,
            quality: n.quality,
            timestamp: n.timestamp,
            access_mode: access_mode_str,
            interval_ms,
            group_id: n.group_id,
        }
    }).collect();

    Ok(MonitoredDataDto {
        nodes: node_dtos,
        seq,
    })
}

// ── Endpoint Discovery ──────────────────────────────────────────

#[tauri::command]
pub async fn get_endpoints(
    url: String,
) -> Result<Vec<String>, String> {
    Ok(vec![url])
}

// ── Log Commands ──────────────────────────────────────────

#[tauri::command]
pub fn get_communication_logs(
    state: State<'_, AppState>,
    conn_id: String,
    since_seq: u64,
) -> Result<Vec<opcuasim_core::log_entry::LogEntry>, String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    Ok(entry.connection.log_collector.get_since(since_seq))
}

#[tauri::command]
pub fn clear_communication_logs(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<(), String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    entry.connection.log_collector.clear();
    Ok(())
}

#[tauri::command]
pub fn export_logs_csv(
    state: State<'_, AppState>,
    conn_id: String,
) -> Result<String, String> {
    let connections = state.connections.read().map_err(|e| e.to_string())?;
    let entry = connections.get(&conn_id).ok_or("Connection not found")?;
    Ok(entry.connection.log_collector.export_csv())
}

// ── Group Commands ──────────────────────────────────────────

#[tauri::command]
pub fn create_group(
    state: State<'_, AppState>,
    name: String,
) -> Result<NodeGroupDto, String> {
    let id = Uuid::new_v4().to_string();
    let group = NodeGroup {
        id: id.clone(),
        name: name.clone(),
        node_ids: vec![],
    };
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    groups.push(group);
    Ok(NodeGroupDto { id, name, node_count: 0 })
}

#[tauri::command]
pub fn delete_group(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    groups.retain(|g| g.id != id);
    Ok(())
}

#[tauri::command]
pub fn list_groups(
    state: State<'_, AppState>,
) -> Result<Vec<NodeGroupDto>, String> {
    let groups = state.groups.read().map_err(|e| e.to_string())?;
    Ok(groups.iter().map(|g| NodeGroupDto {
        id: g.id.clone(),
        name: g.name.clone(),
        node_count: g.node_ids.len(),
    }).collect())
}

#[tauri::command]
pub fn add_nodes_to_group(
    state: State<'_, AppState>,
    group_id: String,
    node_ids: Vec<String>,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    let group = groups.iter_mut().find(|g| g.id == group_id).ok_or("Group not found")?;
    for nid in node_ids {
        if !group.node_ids.contains(&nid) {
            group.node_ids.push(nid);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn remove_nodes_from_group(
    state: State<'_, AppState>,
    group_id: String,
    node_ids: Vec<String>,
) -> Result<(), String> {
    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    let group = groups.iter_mut().find(|g| g.id == group_id).ok_or("Group not found")?;
    group.node_ids.retain(|id| !node_ids.contains(id));
    Ok(())
}

// ── Project File Commands ──────────────────────────────────────────

#[tauri::command]
pub async fn save_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let (connections_data, groups_snapshot) = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let groups = state.groups.read().map_err(|e| e.to_string())?;

        let conn_data: Vec<ConnectionProjectEntry> = connections.values().map(|entry| {
            let config = &entry.connection.config;
            ConnectionProjectEntry {
                name: config.name.clone(),
                endpoint_url: config.endpoint_url.clone(),
                security_policy: config.security_policy.clone(),
                security_mode: config.security_mode.clone(),
                auth: config.auth.clone(),
                timeout_ms: config.timeout_ms,
                monitored_nodes: vec![],
            }
        }).collect();

        (conn_data, groups.clone())
    };

    let mut project = ProjectFile::new_master();
    project.groups = groups_snapshot;
    project.connections = connections_data;

    let json = project.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn load_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let project = ProjectFile::from_json(&json).map_err(|e| e.to_string())?;

    let mut connections = state.connections.write().map_err(|e| e.to_string())?;
    connections.clear();

    for conn_entry in &project.connections {
        let id = Uuid::new_v4().to_string();
        let config = ConnectionConfig {
            id: id.clone(),
            name: conn_entry.name.clone(),
            endpoint_url: conn_entry.endpoint_url.clone(),
            security_policy: conn_entry.security_policy.clone(),
            security_mode: conn_entry.security_mode.clone(),
            auth: conn_entry.auth.clone(),
            timeout_ms: conn_entry.timeout_ms,
        };
        let connection = OpcUaConnection::new(config);
        connections.insert(id, ConnectionEntry {
            connection,
            subscription_mgr: SubscriptionManager::new(),
            polling_mgr: PollingManager::new(),
        });
    }

    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    *groups = project.groups;

    Ok(())
}
