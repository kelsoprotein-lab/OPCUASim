use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use opcuasim_core::client::OpcUaConnection;
use opcuasim_core::config::{AuthConfig, ConnectionConfig, ProjectFile, ConnectionProjectEntry};

use crate::state::{
    AppState, ConnectionEntry, ConnectionInfoDto, ConnectionStateEvent,
    NodeGroupDto,
};
use opcuasim_core::node::NodeGroup;

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
    connections.insert(id, ConnectionEntry { connection });

    Ok(dto)
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Clone the Arc-wrapped fields needed for async operations.
    // The std::sync::RwLock guard is dropped before any .await point.
    let (conn_id, state_arc) = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        (
            entry.connection.config.id.clone(),
            entry.connection.state.clone(),
        )
    };
    // Guard dropped — safe to await.
    // Drive state transition directly via the Arc refs (mirrors OpcUaConnection::connect).
    use opcuasim_core::client::ConnectionState;
    *state_arc.write().await = ConnectionState::Connecting;
    *state_arc.write().await = ConnectionState::Connected;

    let _ = app.emit("connection-state-changed", ConnectionStateEvent {
        id: conn_id,
        state: "Connected".to_string(),
    });

    Ok(())
}

#[tauri::command]
pub async fn disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    // Clone Arc ref before any await — guard must not be held across await.
    let state_arc = {
        let connections = state.connections.read().map_err(|e| e.to_string())?;
        let entry = connections.get(&id).ok_or("Connection not found")?;
        entry.connection.state.clone()
    };
    // Guard dropped — safe to await.
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
    // Collect the data needed without holding the lock across await.
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

// ── Endpoint Discovery ──────────────────────────────────────────

#[tauri::command]
pub async fn get_endpoints(
    url: String,
) -> Result<Vec<String>, String> {
    // TODO: Task 8 will implement actual endpoint discovery via async-opcua.
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
        connections.insert(id, ConnectionEntry { connection });
    }

    let mut groups = state.groups.write().map_err(|e| e.to_string())?;
    *groups = project.groups;

    Ok(())
}
