use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use opcuasim_core::cert_manager::{
    self, delete_certificate, list_certificates, move_certificate, CertRole,
};
use opcuasim_core::client::{ConnectionState, OpcUaConnection};
use opcuasim_core::discovery::discover_endpoints;
use opcuasim_core::history::history_read_raw;
use opcuasim_core::method::{call_method, read_method_arguments};
use opcuasim_core::config::{AuthConfig, ConnectionConfig, ConnectionProjectEntry, ProjectFile};
use opcuasim_core::node::{
    AccessMode, DataChangeFilterCfg, DataChangeTriggerKind, DeadbandKind, MonitoredNode, NodeGroup,
};
use opcuasim_core::polling::PollingManager;
use opcuasim_core::subscription::SubscriptionManager;

use crate::backend::state::{BackendState, ConnectionEntry};
use crate::events::{
    AuthKindReq, BackendEvent, BrowseItem, CertRoleDto, CertSummaryDto, ConnectionInfo,
    CreateConnectionReq, DataChangeFilterReq, DataChangeTriggerKindReq, DeadbandKindReq,
    DiscoveredEndpointDto, HistoryPointDto, LogRow, MethodArgInfo, MethodArgValue,
    MonitoredNodeReq, MonitoredRow, NodeAttrsDto, NodeGroupDto, ToastLevel, UiCommand,
};

pub async fn run(
    mut cmd_rx: UnboundedReceiver<UiCommand>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let state = BackendState::new_shared();
    log::info!("backend dispatcher started");

    // Spawn 250ms monitor snapshot timer
    tokio::spawn(monitor_timer(
        state.clone(),
        event_tx.clone(),
        cancel.clone(),
        egui_ctx.clone(),
    ));
    // Spawn 1.5s communication log timer
    tokio::spawn(log_timer(
        state.clone(),
        event_tx.clone(),
        cancel.clone(),
        egui_ctx.clone(),
    ));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                log::info!("backend dispatcher cancelled");
                break;
            }
            maybe = cmd_rx.recv() => {
                let Some(cmd) = maybe else {
                    log::info!("cmd channel closed");
                    break;
                };
                tokio::spawn(handle_cmd(
                    cmd,
                    state.clone(),
                    event_tx.clone(),
                    egui_ctx.clone(),
                ));
            }
        }
    }
}

async fn monitor_timer(
    state: Arc<BackendState>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let last_seq: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let sub_mgrs: Vec<(String, SubscriptionManager)> = match state.connections.read() {
                    Ok(conns) => conns.iter().map(|(id, e)| (id.clone(), e.subscription_mgr.clone())).collect(),
                    Err(_) => continue,
                };
                let mut any_sent = false;
                for (conn_id, sub_mgr) in sub_mgrs {
                    let current_seq = sub_mgr.get_update_seq().await;
                    let last = {
                        let map = last_seq.lock().await;
                        map.get(&conn_id).copied().unwrap_or(0)
                    };
                    if current_seq == last {
                        continue;
                    }
                    let (nodes, full) = if last == 0 {
                        (sub_mgr.get_monitored_nodes().await, true)
                    } else {
                        (sub_mgr.get_monitored_nodes_since(last).await, false)
                    };
                    if nodes.is_empty() && !full {
                        let mut map = last_seq.lock().await;
                        map.insert(conn_id.clone(), current_seq);
                        continue;
                    }
                    let rows = nodes.into_iter().map(monitored_node_to_row).collect();
                    let _ = event_tx.send(BackendEvent::MonitoredSnapshot {
                        conn_id: conn_id.clone(),
                        seq: current_seq,
                        full,
                        nodes: rows,
                    });
                    {
                        let mut map = last_seq.lock().await;
                        map.insert(conn_id, current_seq);
                    }
                    any_sent = true;
                }
                if any_sent {
                    egui_ctx.request_repaint();
                }
            }
        }
    }
}

async fn handle_cmd(
    cmd: UiCommand,
    state: Arc<BackendState>,
    event_tx: UnboundedSender<BackendEvent>,
    egui_ctx: egui::Context,
) {
    let result: Result<(), String> = match cmd {
        UiCommand::CreateConnection(req) => create_connection(req, &state, &event_tx).await,
        UiCommand::DiscoverEndpoints {
            url,
            timeout_ms,
            req_id,
        } => do_discover_endpoints(url, timeout_ms, req_id, &event_tx).await,
        UiCommand::Connect(id) => connect(id, &state, &event_tx).await,
        UiCommand::Disconnect(id) => disconnect(id, &state, &event_tx).await,
        UiCommand::DeleteConnection(id) => delete_connection(id, &state, &event_tx).await,
        UiCommand::ListConnections => list_connections(&state, &event_tx).await,
        UiCommand::BrowseRoot { conn_id, req_id } => {
            browse_root(conn_id, req_id, &state, &event_tx).await
        }
        UiCommand::BrowseNode {
            conn_id,
            node_id,
            req_id,
        } => browse_node(conn_id, node_id, req_id, &state, &event_tx).await,
        UiCommand::AddMonitoredNodes { conn_id, nodes } => {
            add_monitored_nodes(conn_id, nodes, &state, &event_tx).await
        }
        UiCommand::AddVariablesUnderNode {
            conn_id,
            node_id,
            access_mode,
            interval_ms,
            max_depth,
            filter,
        } => {
            add_variables_under_node(
                conn_id,
                node_id,
                access_mode,
                interval_ms,
                max_depth,
                filter,
                &state,
                &event_tx,
            )
            .await
        }
        UiCommand::RemoveMonitoredNodes { conn_id, node_ids } => {
            remove_monitored_nodes(conn_id, node_ids, &state).await
        }
        UiCommand::ReadAttrs {
            conn_id,
            node_id,
            req_id,
        } => read_attrs(conn_id, node_id, req_id, &state, &event_tx).await,
        UiCommand::WriteValue {
            conn_id,
            node_id,
            value,
            data_type,
            req_id,
        } => write_value(conn_id, node_id, value, data_type, req_id, &state, &event_tx).await,
        UiCommand::ClearCommLogs(conn_id) => clear_logs(conn_id, &state, &event_tx),
        UiCommand::ExportCommLogs { conn_id, path } => export_logs(conn_id, path, &state, &event_tx),
        UiCommand::CreateGroup(name) => create_group(name, &state, &event_tx),
        UiCommand::DeleteGroup(id) => delete_group(id, &state, &event_tx),
        UiCommand::AddNodesToGroup { group_id, node_ids } => {
            add_nodes_to_group(group_id, node_ids, &state, &event_tx)
        }
        UiCommand::ListGroups => list_groups(&state, &event_tx),
        UiCommand::SaveProject(path) => save_project(path, &state, &event_tx).await,
        UiCommand::LoadProject(path) => load_project(path, &state, &event_tx).await,
        UiCommand::ListCertificates { role, req_id } => {
            do_list_certs(role, req_id, &event_tx).await
        }
        UiCommand::MoveCertificate { path, to_role } => {
            do_move_cert(path, to_role, &event_tx).await
        }
        UiCommand::DeleteCertificate { path } => do_delete_cert(path, &event_tx).await,
        UiCommand::ReadMethodArgs {
            conn_id,
            method_id,
            req_id,
        } => do_read_method_args(conn_id, method_id, req_id, &state, &event_tx).await,
        UiCommand::CallMethod {
            conn_id,
            object_id,
            method_id,
            inputs,
            req_id,
        } => {
            do_call_method(
                conn_id, object_id, method_id, inputs, req_id, &state, &event_tx,
            )
            .await
        }
        UiCommand::ReadHistory {
            conn_id,
            node_id,
            start_iso,
            end_iso,
            max_values,
            req_id,
        } => {
            do_read_history(
                conn_id, node_id, start_iso, end_iso, max_values, req_id, &state, &event_tx,
            )
            .await
        }
    };

    if let Err(e) = result {
        log::warn!("command failed: {e}");
        let _ = event_tx.send(BackendEvent::Toast {
            level: ToastLevel::Error,
            message: e,
        });
    }
    egui_ctx.request_repaint();
}

fn auth_from_req(auth: AuthKindReq) -> AuthConfig {
    match auth {
        AuthKindReq::Anonymous => AuthConfig::Anonymous,
        AuthKindReq::UserPassword { username, password } => {
            AuthConfig::UserPassword { username, password }
        }
        AuthKindReq::Certificate {
            cert_path,
            key_path,
        } => AuthConfig::Certificate {
            cert_path,
            key_path,
        },
    }
}

fn filter_req_to_core(req: &DataChangeFilterReq) -> DataChangeFilterCfg {
    DataChangeFilterCfg {
        trigger: match req.trigger {
            DataChangeTriggerKindReq::Status => DataChangeTriggerKind::Status,
            DataChangeTriggerKindReq::StatusValue => DataChangeTriggerKind::StatusValue,
            DataChangeTriggerKindReq::StatusValueTimestamp => {
                DataChangeTriggerKind::StatusValueTimestamp
            }
        },
        deadband_kind: match req.deadband_kind {
            DeadbandKindReq::None => DeadbandKind::None,
            DeadbandKindReq::Absolute => DeadbandKind::Absolute,
            DeadbandKindReq::Percent => DeadbandKind::Percent,
        },
        deadband_value: req.deadband_value,
    }
}

fn auth_label(a: &AuthConfig) -> &'static str {
    match a {
        AuthConfig::Anonymous => "Anonymous",
        AuthConfig::UserPassword { .. } => "UserPassword",
        AuthConfig::Certificate { .. } => "Certificate",
    }
}

fn monitored_node_to_row(n: MonitoredNode) -> MonitoredRow {
    let (access_mode, interval_ms) = match &n.access_mode {
        AccessMode::Subscription { interval_ms } => ("Subscription".to_string(), *interval_ms),
        AccessMode::Polling { interval_ms } => ("Polling".to_string(), *interval_ms as f64),
    };
    MonitoredRow {
        node_id: n.node_id,
        display_name: n.display_name,
        data_type: n.data_type,
        value: n.value,
        quality: n.quality,
        source_timestamp: n.timestamp,
        server_timestamp: n.server_timestamp,
        access_mode,
        interval_ms,
        update_seq: n.update_seq,
        user_access_level: n.user_access_level,
    }
}

async fn create_connection(
    req: CreateConnectionReq,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let config = ConnectionConfig {
        id: id.clone(),
        name: req.name,
        endpoint_url: req.endpoint_url,
        security_policy: req.security_policy,
        security_mode: req.security_mode,
        auth: auth_from_req(req.auth),
        timeout_ms: req.timeout_ms,
    };
    let connection = OpcUaConnection::new(config);
    {
        let mut conns = state.connections.write().map_err(|e| e.to_string())?;
        conns.insert(
            id,
            ConnectionEntry {
                connection,
                subscription_mgr: SubscriptionManager::new(),
                polling_mgr: PollingManager::new(),
            },
        );
    }
    list_connections(state, event_tx).await
}

async fn connect(
    id: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let (state_arc, config_clone) = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&id).ok_or("Connection not found")?;
        (entry.connection.state.clone(), entry.connection.config.clone())
    };

    *state_arc.write().await = ConnectionState::Connecting;
    let _ = event_tx.send(BackendEvent::ConnectionStateChanged {
        id: id.clone(),
        state: "Connecting".to_string(),
    });

    let temp_conn = OpcUaConnection::new(config_clone);
    match temp_conn.connect().await {
        Ok(()) => {
            {
                let mut conns = state.connections.write().map_err(|e| e.to_string())?;
                if let Some(entry) = conns.get_mut(&id) {
                    entry.connection = temp_conn;
                    entry.subscription_mgr = SubscriptionManager::new();
                }
            }
            *state_arc.write().await = ConnectionState::Connected;
            let _ = event_tx.send(BackendEvent::ConnectionStateChanged {
                id: id.clone(),
                state: "Connected".to_string(),
            });
            list_connections(state, event_tx).await
        }
        Err(e) => {
            *state_arc.write().await = ConnectionState::Disconnected;
            let _ = event_tx.send(BackendEvent::ConnectionStateChanged {
                id: id.clone(),
                state: "Disconnected".to_string(),
            });
            Err(format!("Connection failed: {e}"))
        }
    }
}

async fn disconnect(
    id: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let (state_arc, config) = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&id).ok_or("Connection not found")?;
        (entry.connection.state.clone(), entry.connection.config.clone())
    };

    {
        let mut conns = state.connections.write().map_err(|e| e.to_string())?;
        if let Some(entry) = conns.get_mut(&id) {
            entry.connection = OpcUaConnection::new(config);
        }
    }
    *state_arc.write().await = ConnectionState::Disconnected;
    let _ = event_tx.send(BackendEvent::ConnectionStateChanged {
        id: id.clone(),
        state: "Disconnected".to_string(),
    });
    list_connections(state, event_tx).await
}

async fn delete_connection(
    id: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    {
        let mut conns = state.connections.write().map_err(|e| e.to_string())?;
        conns.remove(&id).ok_or("Connection not found")?;
    }
    list_connections(state, event_tx).await
}

async fn list_connections(
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let snapshot: Vec<(String, String, String, String, String, AuthConfig, _)> = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        conns
            .iter()
            .map(|(id, entry)| {
                (
                    id.clone(),
                    entry.connection.config.name.clone(),
                    entry.connection.config.endpoint_url.clone(),
                    entry.connection.config.security_policy.clone(),
                    entry.connection.config.security_mode.clone(),
                    entry.connection.config.auth.clone(),
                    entry.connection.state.clone(),
                )
            })
            .collect()
    };

    let mut infos = Vec::with_capacity(snapshot.len());
    for (id, name, endpoint_url, security_policy, security_mode, auth, state_arc) in snapshot {
        let st = state_arc.read().await.clone();
        infos.push(ConnectionInfo {
            id,
            name,
            endpoint_url,
            security_policy,
            security_mode,
            auth_type: auth_label(&auth).to_string(),
            state: format!("{st:?}"),
        });
    }
    infos.sort_by(|a, b| a.name.cmp(&b.name));

    let _ = event_tx.send(BackendEvent::Connections(infos));
    Ok(())
}

async fn browse_root(
    conn_id: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let items = opcuasim_core::browse::browse_node(&session, None)
        .await
        .map_err(|e| e.to_string())?;
    let out = items.into_iter().map(browse_item_to_dto).collect();
    let _ = event_tx.send(BackendEvent::BrowseResult {
        req_id,
        parent: None,
        items: out,
    });
    Ok(())
}

async fn browse_node(
    conn_id: String,
    node_id: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let items = opcuasim_core::browse::browse_node(&session, Some(&node_id))
        .await
        .map_err(|e| e.to_string())?;
    let out = items.into_iter().map(browse_item_to_dto).collect();
    let _ = event_tx.send(BackendEvent::BrowseResult {
        req_id,
        parent: Some(node_id),
        items: out,
    });
    Ok(())
}

async fn add_monitored_nodes(
    conn_id: String,
    nodes: Vec<MonitoredNodeReq>,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let monitored: Vec<MonitoredNode> = nodes
        .into_iter()
        .map(|n| {
            let access_mode = match n.access_mode.as_str() {
                "Polling" => AccessMode::Polling {
                    interval_ms: n.interval_ms as u64,
                },
                _ => AccessMode::Subscription {
                    interval_ms: n.interval_ms,
                },
            };
            MonitoredNode {
                node_id: n.node_id,
                display_name: n.display_name,
                browse_path: String::new(),
                data_type: n.data_type.unwrap_or_else(|| "Unknown".to_string()),
                value: None,
                quality: None,
                timestamp: None,
                server_timestamp: None,
                access_mode,
                group_id: None,
                update_seq: 0,
                user_access_level: 0,
                filter: n.filter.as_ref().map(filter_req_to_core),
            }
        })
        .collect();

    let (sub_mgr, session_holder) = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&conn_id).ok_or("Connection not found")?;
        (
            entry.subscription_mgr.clone(),
            entry.connection.get_session_holder(),
        )
    };

    let session_guard = session_holder.read().await;
    let session = session_guard.as_ref();
    sub_mgr
        .add_nodes(monitored, session)
        .await
        .map_err(|e| e.to_string())?;
    drop(session_guard);

    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: "已添加到监控".into(),
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn add_variables_under_node(
    conn_id: String,
    node_id: String,
    access_mode: String,
    interval_ms: f64,
    max_depth: u32,
    filter: Option<DataChangeFilterReq>,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let variables = opcuasim_core::browse::collect_variables(&session, &node_id, max_depth)
        .await
        .map_err(|e| e.to_string())?;
    if variables.is_empty() {
        let _ = event_tx.send(BackendEvent::Toast {
            level: ToastLevel::Warn,
            message: "此节点下未发现变量".into(),
        });
        return Ok(());
    }

    let mode = match access_mode.as_str() {
        "Polling" => AccessMode::Polling {
            interval_ms: interval_ms as u64,
        },
        _ => AccessMode::Subscription { interval_ms },
    };
    let count = variables.len();
    let core_filter = filter.as_ref().map(filter_req_to_core);
    let nodes: Vec<MonitoredNode> = variables
        .into_iter()
        .map(|v| MonitoredNode {
            node_id: v.node_id,
            display_name: v.display_name,
            browse_path: String::new(),
            data_type: v.data_type.unwrap_or_else(|| "Unknown".to_string()),
            value: None,
            quality: None,
            timestamp: None,
            server_timestamp: None,
            access_mode: mode.clone(),
            group_id: None,
            update_seq: 0,
            user_access_level: 0,
            filter: core_filter,
        })
        .collect();

    let (sub_mgr, session_holder) = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&conn_id).ok_or("Connection not found")?;
        (
            entry.subscription_mgr.clone(),
            entry.connection.get_session_holder(),
        )
    };
    let session_guard = session_holder.read().await;
    sub_mgr
        .add_nodes(nodes, session_guard.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("已添加 {count} 个变量"),
    });
    Ok(())
}

async fn remove_monitored_nodes(
    conn_id: String,
    node_ids: Vec<String>,
    state: &Arc<BackendState>,
) -> Result<(), String> {
    let sub_mgr = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&conn_id).ok_or("Connection not found")?;
        entry.subscription_mgr.clone()
    };
    sub_mgr
        .remove_nodes(&node_ids)
        .await
        .map_err(|e| e.to_string())
}

fn browse_item_to_dto(item: opcuasim_core::node::BrowseResultItem) -> BrowseItem {
    BrowseItem {
        node_id: item.node_id,
        display_name: item.display_name,
        node_class: item.node_class,
        data_type: item.data_type,
        has_children: item.has_children,
    }
}

async fn read_attrs(
    conn_id: String,
    node_id: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let attrs = opcuasim_core::browse::read_node_attributes(&session, &node_id)
        .await
        .map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::NodeAttrs {
        req_id,
        attrs: NodeAttrsDto {
            node_id: attrs.node_id,
            display_name: attrs.display_name,
            description: attrs.description,
            data_type: attrs.data_type,
            access_level: attrs.access_level,
            value: attrs.value,
            quality: attrs.quality,
            timestamp: attrs.timestamp,
        },
    });
    Ok(())
}

async fn write_value(
    conn_id: String,
    node_id: String,
    value: String,
    data_type: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    opcuasim_core::browse::write_node_value(&session, &node_id, &value, &data_type)
        .await
        .map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::WriteOk {
        req_id,
        node_id,
    });
    Ok(())
}

fn clear_logs(
    conn_id: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let conns = state.connections.read().map_err(|e| e.to_string())?;
    let entry = conns.get(&conn_id).ok_or("Connection not found")?;
    entry.connection.log_collector.clear();
    let _ = event_tx.send(BackendEvent::LogsCleared { conn_id });
    Ok(())
}

fn export_logs(
    conn_id: String,
    path: std::path::PathBuf,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let csv = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(&conn_id).ok_or("Connection not found")?;
        entry.connection.log_collector.export_csv()
    };
    std::fs::write(&path, csv).map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("日志已导出到 {}", path.display()),
    });
    Ok(())
}

fn group_to_dto(g: &NodeGroup) -> NodeGroupDto {
    NodeGroupDto {
        id: g.id.clone(),
        name: g.name.clone(),
        node_ids: g.node_ids.clone(),
    }
}

fn create_group(
    name: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let group = NodeGroup {
        id,
        name,
        node_ids: Vec::new(),
    };
    {
        let mut groups = state.groups.write().map_err(|e| e.to_string())?;
        groups.push(group);
    }
    list_groups(state, event_tx)
}

fn delete_group(
    id: String,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    {
        let mut groups = state.groups.write().map_err(|e| e.to_string())?;
        groups.retain(|g| g.id != id);
    }
    list_groups(state, event_tx)
}

fn add_nodes_to_group(
    group_id: String,
    node_ids: Vec<String>,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    {
        let mut groups = state.groups.write().map_err(|e| e.to_string())?;
        let g = groups
            .iter_mut()
            .find(|g| g.id == group_id)
            .ok_or("Group not found")?;
        for nid in node_ids {
            if !g.node_ids.contains(&nid) {
                g.node_ids.push(nid);
            }
        }
    }
    list_groups(state, event_tx)
}

fn list_groups(
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let groups = state.groups.read().map_err(|e| e.to_string())?;
    let dtos: Vec<NodeGroupDto> = groups.iter().map(group_to_dto).collect();
    let _ = event_tx.send(BackendEvent::Groups(dtos));
    Ok(())
}

async fn save_project(
    path: std::path::PathBuf,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let (conn_entries, groups_snapshot) = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let groups = state.groups.read().map_err(|e| e.to_string())?;
        let conn_data: Vec<ConnectionProjectEntry> = conns
            .values()
            .map(|entry| {
                let c = &entry.connection.config;
                ConnectionProjectEntry {
                    name: c.name.clone(),
                    endpoint_url: c.endpoint_url.clone(),
                    security_policy: c.security_policy.clone(),
                    security_mode: c.security_mode.clone(),
                    auth: c.auth.clone(),
                    timeout_ms: c.timeout_ms,
                    monitored_nodes: Vec::new(),
                }
            })
            .collect();
        (conn_data, groups.clone())
    };
    let mut project = ProjectFile::new_master();
    project.connections = conn_entries;
    project.groups = groups_snapshot;
    let json = project.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("项目已保存到 {}", path.display()),
    });
    Ok(())
}

async fn load_project(
    path: std::path::PathBuf,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let project = ProjectFile::from_json(&json).map_err(|e| e.to_string())?;

    {
        let mut conns = state.connections.write().map_err(|e| e.to_string())?;
        conns.clear();
        for ce in &project.connections {
            let id = Uuid::new_v4().to_string();
            let config = ConnectionConfig {
                id: id.clone(),
                name: ce.name.clone(),
                endpoint_url: ce.endpoint_url.clone(),
                security_policy: ce.security_policy.clone(),
                security_mode: ce.security_mode.clone(),
                auth: ce.auth.clone(),
                timeout_ms: ce.timeout_ms,
            };
            conns.insert(
                id,
                ConnectionEntry {
                    connection: OpcUaConnection::new(config),
                    subscription_mgr: SubscriptionManager::new(),
                    polling_mgr: PollingManager::new(),
                },
            );
        }
    }
    {
        let mut groups = state.groups.write().map_err(|e| e.to_string())?;
        *groups = project.groups;
    }
    list_connections(state, event_tx).await?;
    list_groups(state, event_tx)?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("项目已加载 ({})", path.display()),
    });
    Ok(())
}

async fn log_timer(
    state: Arc<BackendState>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(1500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_seq: HashMap<String, u64> = HashMap::new();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let collectors: Vec<(String, opcuasim_core::log_collector::LogCollector)> =
                    match state.connections.read() {
                        Ok(conns) => conns
                            .iter()
                            .map(|(id, entry)| (id.clone(), entry.connection.log_collector.clone()))
                            .collect(),
                        Err(_) => continue,
                    };
                let mut any_sent = false;
                for (conn_id, collector) in collectors {
                    let last = last_seq.get(&conn_id).copied().unwrap_or(0);
                    let entries = collector.get_since(last);
                    if entries.is_empty() {
                        continue;
                    }
                    let newest = entries.iter().map(|e| e.seq).max().unwrap_or(last);
                    let rows: Vec<LogRow> = entries
                        .into_iter()
                        .map(|e| LogRow {
                            seq: e.seq,
                            timestamp_ms: e.timestamp.timestamp_millis(),
                            direction: e.direction.to_string(),
                            service: e.service,
                            detail: e.detail,
                            status: e.status,
                        })
                        .collect();
                    let _ = event_tx.send(BackendEvent::CommLogEntries {
                        conn_id: conn_id.clone(),
                        entries: rows,
                    });
                    last_seq.insert(conn_id, newest);
                    any_sent = true;
                }
                if any_sent {
                    egui_ctx.request_repaint();
                }
            }
        }
    }
}

async fn get_session(
    state: &Arc<BackendState>,
    conn_id: &str,
) -> Result<Arc<opcuasim_core::OpcUaSession>, String> {
    let holder = {
        let conns = state.connections.read().map_err(|e| e.to_string())?;
        let entry = conns.get(conn_id).ok_or("Connection not found")?;
        entry.connection.get_session_holder()
    };
    let guard = holder.read().await;
    guard
        .clone()
        .ok_or_else(|| "Not connected — no active session".to_string())
}

async fn do_discover_endpoints(
    url: String,
    timeout_ms: u64,
    req_id: u64,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    match discover_endpoints(&url, timeout_ms).await {
        Ok(list) => {
            let endpoints: Vec<DiscoveredEndpointDto> = list
                .into_iter()
                .map(|e| DiscoveredEndpointDto {
                    endpoint_url: e.endpoint_url,
                    security_policy: e.security_policy,
                    security_mode: e.security_mode,
                    security_level: e.security_level,
                    server_cert_thumbprint: e.server_cert_thumbprint,
                    user_token_policy_ids: e
                        .user_token_policies
                        .into_iter()
                        .map(|t| t.policy_id)
                        .collect(),
                })
                .collect();
            let _ = event_tx.send(BackendEvent::EndpointsDiscovered { req_id, endpoints });
            Ok(())
        }
        Err(e) => Err(format!("Discovery failed: {e}")),
    }
}

const PKI_DIR: &str = "./pki";

fn role_to_core(r: CertRoleDto) -> CertRole {
    match r {
        CertRoleDto::Trusted => CertRole::Trusted,
        CertRoleDto::Rejected => CertRole::Rejected,
    }
}

fn role_to_dto(r: CertRole) -> CertRoleDto {
    match r {
        CertRole::Trusted => CertRoleDto::Trusted,
        CertRole::Rejected => CertRoleDto::Rejected,
    }
}

async fn do_list_certs(
    role: CertRoleDto,
    req_id: u64,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let core_role = role_to_core(role);
    let pki = std::path::Path::new(PKI_DIR);
    let list = list_certificates(pki, core_role).map_err(|e| e.to_string())?;
    let certs: Vec<CertSummaryDto> = list
        .into_iter()
        .map(|c| CertSummaryDto {
            path: c.path,
            file_name: c.file_name,
            role: role_to_dto(c.role),
            thumbprint: c.thumbprint,
            subject_cn: c.subject_cn,
            issuer_cn: c.issuer_cn,
            valid_from: c.valid_from,
            valid_to: c.valid_to,
        })
        .collect();
    let _ = event_tx.send(BackendEvent::CertificateList { req_id, role, certs });
    Ok(())
}

async fn do_move_cert(
    path: std::path::PathBuf,
    to_role: CertRoleDto,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let pki = std::path::Path::new(PKI_DIR);
    move_certificate(pki, &path, role_to_core(to_role)).map_err(|e| e.to_string())?;
    let target_name = match to_role {
        CertRoleDto::Trusted => "Trusted",
        CertRoleDto::Rejected => "Rejected",
    };
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: format!("证书已移动到 {target_name}"),
    });
    Ok(())
}

async fn do_delete_cert(
    path: std::path::PathBuf,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    delete_certificate(&path).map_err(|e| e.to_string())?;
    let _ = event_tx.send(BackendEvent::Toast {
        level: ToastLevel::Info,
        message: "证书已删除".into(),
    });
    Ok(())
}

#[allow(dead_code)]
fn _cert_manager_keep() -> &'static str {
    cert_manager::CertRole::Trusted.dir_name()
}

async fn do_read_method_args(
    conn_id: String,
    method_id: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let nid: opcua_types::NodeId = method_id
        .parse()
        .map_err(|_| format!("invalid node id: {method_id}"))?;
    let info = read_method_arguments(&session, &nid)
        .await
        .map_err(|e| e.to_string())?;
    let inputs = info
        .inputs
        .into_iter()
        .map(|a| MethodArgInfo {
            name: a.name,
            data_type: a.data_type,
            description: a.description,
        })
        .collect();
    let outputs = info
        .outputs
        .into_iter()
        .map(|a| MethodArgInfo {
            name: a.name,
            data_type: a.data_type,
            description: a.description,
        })
        .collect();
    let _ = event_tx.send(BackendEvent::MethodArgs {
        req_id,
        inputs,
        outputs,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn do_call_method(
    conn_id: String,
    object_id: String,
    method_id: String,
    inputs: Vec<MethodArgValue>,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let oid: opcua_types::NodeId = object_id
        .parse()
        .map_err(|_| format!("invalid object id: {object_id}"))?;
    let mid: opcua_types::NodeId = method_id
        .parse()
        .map_err(|_| format!("invalid method id: {method_id}"))?;

    let variants: Vec<opcua_types::Variant> = inputs
        .iter()
        .map(|a| string_to_variant(&a.data_type, &a.value))
        .collect::<Result<_, String>>()?;

    let outcome = call_method(&session, &oid, &mid, variants)
        .await
        .map_err(|e| e.to_string())?;

    let outputs: Vec<MethodArgValue> = outcome
        .outputs
        .into_iter()
        .map(|v| MethodArgValue {
            data_type: variant_type_label(&v),
            value: format!("{v}"),
        })
        .collect();

    let _ = event_tx.send(BackendEvent::MethodCallResult {
        req_id,
        status: format!("{}", outcome.status),
        outputs,
    });
    Ok(())
}

fn string_to_variant(data_type: &str, value: &str) -> Result<opcua_types::Variant, String> {
    use opcua_types::Variant;
    match data_type {
        "Boolean" => value
            .parse::<bool>()
            .map(Variant::Boolean)
            .map_err(|e| e.to_string()),
        "SByte" => value.parse::<i8>().map(Variant::SByte).map_err(|e| e.to_string()),
        "Byte" => value.parse::<u8>().map(Variant::Byte).map_err(|e| e.to_string()),
        "Int16" => value.parse::<i16>().map(Variant::Int16).map_err(|e| e.to_string()),
        "UInt16" => value.parse::<u16>().map(Variant::UInt16).map_err(|e| e.to_string()),
        "Int32" => value.parse::<i32>().map(Variant::Int32).map_err(|e| e.to_string()),
        "UInt32" => value.parse::<u32>().map(Variant::UInt32).map_err(|e| e.to_string()),
        "Int64" => value.parse::<i64>().map(Variant::Int64).map_err(|e| e.to_string()),
        "UInt64" => value.parse::<u64>().map(Variant::UInt64).map_err(|e| e.to_string()),
        "Float" => value.parse::<f32>().map(Variant::Float).map_err(|e| e.to_string()),
        "Double" => value.parse::<f64>().map(Variant::Double).map_err(|e| e.to_string()),
        "String" => Ok(Variant::String(value.into())),
        other => Err(format!("unsupported method arg type: {other}")),
    }
}

fn variant_type_label(v: &opcua_types::Variant) -> String {
    use opcua_types::variant::VariantTypeId;
    match v.type_id() {
        VariantTypeId::Empty => "Empty".to_string(),
        VariantTypeId::Scalar(s) => format!("{s}"),
        VariantTypeId::Array(s, _) => format!("Array<{s}>"),
    }
}

#[allow(clippy::too_many_arguments)]
async fn do_read_history(
    conn_id: String,
    node_id: String,
    start_iso: String,
    end_iso: String,
    max_values: u32,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let nid: opcua_types::NodeId = node_id
        .parse()
        .map_err(|_| format!("invalid node id: {node_id}"))?;
    let start = parse_iso_to_datetime(&start_iso)?;
    let end = parse_iso_to_datetime(&end_iso)?;

    match history_read_raw(&session, &nid, start, end, max_values, true).await {
        Ok(pts) => {
            let dtos: Vec<HistoryPointDto> = pts
                .into_iter()
                .map(|p| HistoryPointDto {
                    source_timestamp: p.source_timestamp,
                    server_timestamp: p.server_timestamp,
                    value: p.value,
                    numeric: p.numeric,
                    status: p.status,
                })
                .collect();
            let _ = event_tx.send(BackendEvent::HistoryResult {
                req_id,
                node_id: node_id.clone(),
                points: dtos,
                error: None,
            });
            Ok(())
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = event_tx.send(BackendEvent::HistoryResult {
                req_id,
                node_id: node_id.clone(),
                points: Vec::new(),
                error: Some(msg.clone()),
            });
            Err(format!("HistoryRead failed: {msg}"))
        }
    }
}

fn parse_iso_to_datetime(s: &str) -> Result<opcua_types::DateTime, String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(s.trim())
        .map_err(|e| format!("invalid time '{s}': {e}"))?;
    let utc: chrono::DateTime<chrono::Utc> = parsed.with_timezone(&chrono::Utc);
    Ok(opcua_types::DateTime::from(utc))
}
