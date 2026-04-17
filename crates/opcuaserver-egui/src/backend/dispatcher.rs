use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

use opcuasim_core::server::models::{ServerFolder, ServerNode, ServerProjectFile};

use crate::backend::state::BackendState;
use crate::events::{
    AddressSpaceDto, BackendEvent, FolderRow, NodeRow, ServerStatus, ToastLevel, UiCommand,
};

pub async fn run(
    mut cmd_rx: UnboundedReceiver<UiCommand>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let state = BackendState::new_shared();
    log::info!("server backend dispatcher started");

    tokio::spawn(status_timer(
        state.clone(),
        event_tx.clone(),
        cancel.clone(),
        egui_ctx.clone(),
    ));
    tokio::spawn(sim_timer(
        state.clone(),
        event_tx.clone(),
        cancel.clone(),
        egui_ctx.clone(),
    ));

    // Initial snapshots
    let _ = event_tx.send(build_address_space_event(&state));
    let _ = event_tx.send(BackendEvent::Config(state.config.read().unwrap().clone()));

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            maybe = cmd_rx.recv() => {
                let Some(cmd) = maybe else { break };
                if matches!(cmd, UiCommand::Shutdown) { break; }
                let s = state.clone();
                let tx = event_tx.clone();
                let ctx = egui_ctx.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_cmd(cmd, &s, &tx).await {
                        let _ = tx.send(BackendEvent::Toast {
                            level: ToastLevel::Error,
                            message: e,
                        });
                    }
                    ctx.request_repaint();
                });
            }
        }
    }
    log::info!("server backend dispatcher exiting");
    let _ = state.server.stop().await;
}

async fn handle_cmd(
    cmd: UiCommand,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    match cmd {
        UiCommand::StartServer => {
            let (config, folders, nodes) = {
                (
                    state.config.read().unwrap().clone(),
                    state.folders.read().unwrap().clone(),
                    state.nodes.read().unwrap().clone(),
                )
            };
            state
                .server
                .start(&config, &folders, &nodes)
                .await
                .map_err(|e| e.to_string())?;
            let _ = event_tx.send(BackendEvent::Toast {
                level: ToastLevel::Info,
                message: format!("服务器已启动 · {}", config.endpoint_url),
            });
        }
        UiCommand::StopServer => {
            state.server.stop().await.map_err(|e| e.to_string())?;
            let _ = event_tx.send(BackendEvent::Toast {
                level: ToastLevel::Info,
                message: "服务器已停止".into(),
            });
        }
        UiCommand::RefreshStatus => {
            send_status(state, event_tx).await;
        }
        UiCommand::RefreshAddressSpace => {
            let _ = event_tx.send(build_address_space_event(state));
        }
        UiCommand::AddFolder {
            node_id,
            display_name,
            parent_id,
        } => {
            let folder = ServerFolder {
                node_id,
                display_name,
                parent_id,
            };
            if let Some(nm) = state.server.node_manager().await {
                let ns = state.server.namespace_index().await;
                let mut addr = nm.address_space().write();
                opcuasim_core::server::address_space::populate_address_space(
                    &mut addr,
                    ns,
                    &[folder.clone()],
                    &[],
                );
            }
            state.folders.write().unwrap().push(folder);
            let _ = event_tx.send(build_address_space_event(state));
        }
        UiCommand::AddNode(req) => {
            let node = ServerNode {
                node_id: req.node_id,
                display_name: req.display_name,
                parent_id: req.parent_id,
                data_type: req.data_type,
                writable: req.writable,
                simulation: req.simulation,
                update_seq: 0,
                current_value: None,
            };
            if let Some(nm) = state.server.node_manager().await {
                let ns = state.server.namespace_index().await;
                let mut addr = nm.address_space().write();
                opcuasim_core::server::address_space::add_variable_node(&mut addr, ns, &node);
            }
            state.nodes.write().unwrap().push(node);
            let _ = event_tx.send(build_address_space_event(state));
        }
        UiCommand::RemoveNode(node_id) => {
            if let Some(nm) = state.server.node_manager().await {
                let ns = state.server.namespace_index().await;
                let mut addr = nm.address_space().write();
                opcuasim_core::server::address_space::remove_node(&mut addr, ns, &node_id);
            }
            {
                let mut folders = state.folders.write().unwrap();
                folders.retain(|f| f.node_id != node_id && f.parent_id != node_id);
            }
            {
                let mut nodes = state.nodes.write().unwrap();
                nodes.retain(|n| n.node_id != node_id && n.parent_id != node_id);
            }
            let _ = event_tx.send(build_address_space_event(state));
        }
        UiCommand::UpdateNode {
            node_id,
            display_name,
            data_type,
            writable,
            simulation,
        } => {
            {
                let mut nodes = state.nodes.write().unwrap();
                let Some(n) = nodes.iter_mut().find(|n| n.node_id == node_id) else {
                    return Err(format!("节点 {node_id} 未找到"));
                };
                if let Some(dn) = display_name {
                    n.display_name = dn;
                }
                if let Some(dt) = data_type {
                    n.data_type = dt;
                }
                if let Some(w) = writable {
                    n.writable = w;
                }
                if let Some(s) = simulation {
                    n.simulation = s;
                }
            }
            let _ = event_tx.send(build_address_space_event(state));
        }
        UiCommand::SaveProject(path) => {
            let project = ServerProjectFile {
                project_type: "OpcUaServer".into(),
                version: "0.1.0".into(),
                server_config: state.config.read().unwrap().clone(),
                folders: state.folders.read().unwrap().clone(),
                nodes: state.nodes.read().unwrap().clone(),
            };
            let json =
                serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
            std::fs::write(&path, json).map_err(|e| e.to_string())?;
            let _ = event_tx.send(BackendEvent::Toast {
                level: ToastLevel::Info,
                message: format!("项目已保存到 {}", path.display()),
            });
        }
        UiCommand::LoadProject(path) => {
            let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let project: ServerProjectFile =
                serde_json::from_str(&json).map_err(|e| e.to_string())?;
            *state.config.write().unwrap() = project.server_config.clone();
            *state.folders.write().unwrap() = project.folders;
            *state.nodes.write().unwrap() = project.nodes;
            let _ = event_tx.send(BackendEvent::Config(project.server_config));
            let _ = event_tx.send(build_address_space_event(state));
            let _ = event_tx.send(BackendEvent::Toast {
                level: ToastLevel::Info,
                message: format!("项目已加载 ({})", path.display()),
            });
        }
        UiCommand::Shutdown => {}
    }
    Ok(())
}

fn build_address_space_event(state: &Arc<BackendState>) -> BackendEvent {
    let folders_raw = state.folders.read().unwrap();
    let nodes_raw = state.nodes.read().unwrap();
    let dto = AddressSpaceDto {
        folders: folders_raw
            .iter()
            .map(|f| FolderRow {
                node_id: f.node_id.clone(),
                display_name: f.display_name.clone(),
                parent_id: f.parent_id.clone(),
            })
            .collect(),
        nodes: nodes_raw
            .iter()
            .map(|n| NodeRow {
                node_id: n.node_id.clone(),
                display_name: n.display_name.clone(),
                parent_id: n.parent_id.clone(),
                data_type: n.data_type.clone(),
                writable: n.writable,
                simulation: n.simulation.clone(),
                current_value: n.current_value.clone(),
            })
            .collect(),
    };
    BackendEvent::AddressSpace(dto)
}

async fn compute_status(state: &Arc<BackendState>) -> ServerStatus {
    let st = state.server.state().await;
    let (nodes, folders, endpoint) = {
        let nodes = state.nodes.read().unwrap();
        let folders = state.folders.read().unwrap();
        let config = state.config.read().unwrap();
        (nodes.len(), folders.len(), config.endpoint_url.clone())
    };
    ServerStatus {
        state: format!("{st:?}"),
        node_count: nodes,
        folder_count: folders,
        endpoint_url: endpoint,
    }
}

async fn send_status(state: &Arc<BackendState>, event_tx: &UnboundedSender<BackendEvent>) {
    let _ = event_tx.send(BackendEvent::Status(compute_status(state).await));
}

async fn status_timer(
    state: Arc<BackendState>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last: Option<ServerStatus> = None;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let current = compute_status(&state).await;
                if last.as_ref() != Some(&current) {
                    let _ = event_tx.send(BackendEvent::Status(current.clone()));
                    last = Some(current);
                    egui_ctx.request_repaint();
                }
            }
        }
    }
}

async fn sim_timer(
    state: Arc<BackendState>,
    event_tx: UnboundedSender<BackendEvent>,
    cancel: CancellationToken,
    egui_ctx: egui::Context,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_seq = 0u64;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                let Some(engine) = state.server.simulation_engine().await else {
                    continue;
                };
                let (changed, seq) = engine.get_values_since(last_seq).await;
                if seq == last_seq || changed.is_empty() {
                    continue;
                }
                last_seq = seq;
                let _ = event_tx.send(BackendEvent::SimValues { seq, values: changed });
                egui_ctx.request_repaint();
            }
        }
    }
}

