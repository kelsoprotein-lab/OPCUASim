use tokio::sync::mpsc::UnboundedReceiver;

use crate::events::{BackendEvent, UiCommand};
use crate::model::{AppModel, Modal};
use crate::panels::{browse_panel, connection_tree, data_table, log_panel, toolbar, value_panel};
use crate::runtime::BackendHandle;
use crate::widgets::connection_dialog;

pub struct MasterApp {
    backend: BackendHandle,
    event_rx: UnboundedReceiver<BackendEvent>,
    model: AppModel,
    last_size: (f32, f32),
}

impl MasterApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        opcuaegui_shared::fonts::install_cjk_fonts(&cc.egui_ctx);
        let (backend, event_rx) = BackendHandle::new(
            cc.egui_ctx.clone(),
            "opcua-master-backend",
            crate::backend::dispatcher::run,
        );
        backend.send(UiCommand::ListConnections);
        backend.send(UiCommand::ListGroups);
        Self {
            backend,
            event_rx,
            model: AppModel::default(),
            last_size: (0.0, 0.0),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.model.modal.is_some() {
            return;
        }
        let (cmd_n, cmd_s, cmd_o, del) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::N),
                i.modifiers.command && i.key_pressed(egui::Key::S),
                i.modifiers.command && i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
            )
        });
        if cmd_n {
            self.model.modal = Some(Modal::NewConnection(
                crate::widgets::connection_dialog::ConnDialogState::default(),
            ));
        }
        if cmd_s {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("project.opcuaproj")
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .save_file()
            {
                self.backend.send(UiCommand::SaveProject(path));
            }
        }
        if cmd_o {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .pick_file()
            {
                self.backend.send(UiCommand::LoadProject(path));
            }
        }
        if del && !self.model.monitor.selected_rows.is_empty() {
            if let Some(conn_id) = self.model.selected_conn.clone() {
                let ids: Vec<String> =
                    self.model.monitor.selected_rows.iter().cloned().collect();
                self.backend.send(UiCommand::RemoveMonitoredNodes {
                    conn_id: conn_id.clone(),
                    node_ids: ids.clone(),
                });
                if let Some(per) = self.model.monitor.per_conn.get_mut(&conn_id) {
                    for id in &ids {
                        per.rows.shift_remove(id);
                    }
                }
                self.model.monitor.selected_rows.clear();
                self.model.monitor.filter_dirty = true;
            }
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.event_rx.try_recv() {
            self.apply_event(ev);
        }
    }

    fn apply_event(&mut self, ev: BackendEvent) {
        match ev {
            BackendEvent::Connections(list) => {
                if let Some(sel) = &self.model.selected_conn {
                    if !list.iter().any(|c| &c.id == sel) {
                        self.model.selected_conn = None;
                    }
                }
                self.model.connections = list;
            }
            BackendEvent::ConnectionStateChanged { id, state } => {
                if let Some(c) = self.model.connections.iter_mut().find(|c| c.id == id) {
                    c.state = state;
                }
            }
            BackendEvent::BrowseResult {
                req_id,
                parent,
                items,
            } => {
                browse_panel::apply_browse_result(&mut self.model, req_id, parent, items);
            }
            BackendEvent::MonitoredSnapshot {
                conn_id,
                seq,
                full,
                nodes,
            } => {
                self.model.apply_monitored_snapshot(&conn_id, seq, full, nodes);
            }
            BackendEvent::NodeAttrs { req_id, attrs } => {
                if self.model.value_panel.pending_read_req == Some(req_id) {
                    self.model.value_panel.pending_read_req = None;
                }
                self.model.value_panel.attrs = Some(attrs);
            }
            BackendEvent::WriteOk { req_id, node_id } => {
                if self.model.value_panel.pending_write_req == Some(req_id) {
                    self.model.value_panel.pending_write_req = None;
                }
                self.model.value_panel.last_result = Some(format!("✔ 写入成功 ({node_id})"));
                self.model
                    .push_toast(crate::events::ToastLevel::Info, "写入成功");
            }
            BackendEvent::CommLogEntries { conn_id, entries } => {
                self.model
                    .logs
                    .per_conn
                    .entry(conn_id)
                    .or_default()
                    .append(entries);
            }
            BackendEvent::LogsCleared { conn_id } => {
                if let Some(per) = self.model.logs.per_conn.get_mut(&conn_id) {
                    per.entries.clear();
                }
            }
            BackendEvent::Groups(list) => {
                self.model.groups = list;
            }
            BackendEvent::EndpointsDiscovered { req_id, endpoints } => {
                if let Some(Modal::NewConnection(state)) = self.model.modal.as_mut() {
                    if state.discovery_req_id == Some(req_id) {
                        state.discovered = endpoints;
                        state.discovery_in_flight = false;
                        state.discovery_req_id = None;
                    }
                }
            }
            BackendEvent::MethodArgs {
                req_id,
                inputs,
                outputs,
            } => {
                if let Some(Modal::MethodCall(state)) = self.model.modal.as_mut() {
                    if state.pending_args_req == Some(req_id) {
                        state.pending_args_req = None;
                        state.input_values = inputs.iter().map(default_input_for).collect();
                        state.inputs_meta = inputs;
                        state.outputs_meta = outputs;
                    }
                }
            }
            BackendEvent::MethodCallResult {
                req_id,
                status,
                outputs,
            } => {
                if let Some(Modal::MethodCall(state)) = self.model.modal.as_mut() {
                    if state.pending_call_req == Some(req_id) {
                        state.pending_call_req = None;
                        state.last_result_status = Some(status);
                        state.last_result_outputs = outputs;
                    }
                }
            }
            BackendEvent::HistoryResult {
                req_id,
                node_id,
                points,
                error,
            } => {
                if let Some(tab) = self
                    .model
                    .history_tabs
                    .iter_mut()
                    .find(|t| t.pending_req == Some(req_id) && t.node_id == node_id)
                {
                    tab.pending_req = None;
                    tab.points = points;
                    tab.error = error;
                    tab.last_loaded = Some(std::time::Instant::now());
                }
            }
            BackendEvent::CertificateList { req_id, role, certs } => {
                if let Some(Modal::CertManager(state)) = self.model.modal.as_mut() {
                    match role {
                        crate::events::CertRoleDto::Trusted => {
                            if state.pending_trusted_req == Some(req_id) {
                                state.trusted = certs;
                                state.pending_trusted_req = None;
                            }
                        }
                        crate::events::CertRoleDto::Rejected => {
                            if state.pending_rejected_req == Some(req_id) {
                                state.rejected = certs;
                                state.pending_rejected_req = None;
                            }
                        }
                    }
                }
            }
            BackendEvent::Toast { level, message } => {
                self.model.push_toast(level, message);
            }
        }
    }

    fn render_modal(&mut self, ctx: &egui::Context) {
        let Some(mut modal) = self.model.modal.take() else {
            return;
        };
        match &mut modal {
            Modal::NewConnection(state) => {
                let mut close = false;
                let actions = connection_dialog::show(ctx, state, &mut close);
                if let Some(req) = actions.submit {
                    self.backend.send(UiCommand::CreateConnection(req));
                }
                if let Some((url, timeout_ms)) = actions.discover {
                    if !url.is_empty() {
                        let req_id = self.model.alloc_req_id();
                        state.discovery_in_flight = true;
                        state.discovery_req_id = Some(req_id);
                        state.discovered.clear();
                        state.error = None;
                        self.backend.send(UiCommand::DiscoverEndpoints {
                            url,
                            timeout_ms,
                            req_id,
                        });
                    }
                }
                if !close {
                    self.model.modal = Some(modal);
                }
            }
            Modal::MethodCall(state) => {
                let actions = crate::widgets::method_call_dialog::show(ctx, state);
                if let Some(inputs) = actions.call {
                    let req_id = self.model.alloc_req_id();
                    state.pending_call_req = Some(req_id);
                    state.last_result_status = None;
                    state.last_result_outputs.clear();
                    self.backend.send(UiCommand::CallMethod {
                        conn_id: state.conn_id.clone(),
                        object_id: state.object_id.clone(),
                        method_id: state.method_id.clone(),
                        inputs,
                        req_id,
                    });
                }
                if !actions.close {
                    self.model.modal = Some(modal);
                }
            }
            Modal::CertManager(state) => {
                let actions = crate::widgets::cert_manager_dialog::show(ctx, state);
                let mut needs_refresh = actions.refresh;
                if let Some((path, to_role)) = actions.move_to {
                    self.backend
                        .send(UiCommand::MoveCertificate { path, to_role });
                    needs_refresh = true;
                }
                if let Some(path) = actions.delete {
                    self.backend.send(UiCommand::DeleteCertificate { path });
                    needs_refresh = true;
                }
                if needs_refresh {
                    let trusted_req = self.model.alloc_req_id();
                    let rejected_req = self.model.alloc_req_id();
                    state.pending_trusted_req = Some(trusted_req);
                    state.pending_rejected_req = Some(rejected_req);
                    state.selected_path = None;
                    self.backend.send(UiCommand::ListCertificates {
                        role: crate::events::CertRoleDto::Trusted,
                        req_id: trusted_req,
                    });
                    self.backend.send(UiCommand::ListCertificates {
                        role: crate::events::CertRoleDto::Rejected,
                        req_id: rejected_req,
                    });
                }
                if !actions.close {
                    self.model.modal = Some(modal);
                }
            }
        }
    }

    fn render_toasts(&mut self, ctx: &egui::Context) {
        if self.model.toasts.is_empty() {
            return;
        }
        let now = std::time::Instant::now();
        self.model
            .toasts
            .retain(|t| now.duration_since(t.created_at).as_secs() < 4);
        if self.model.toasts.is_empty() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        egui::Area::new("toasts".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for t in &self.model.toasts {
                        let color = match t.level {
                            crate::events::ToastLevel::Info => egui::Color32::LIGHT_BLUE,
                            crate::events::ToastLevel::Warn => egui::Color32::YELLOW,
                            crate::events::ToastLevel::Error => egui::Color32::LIGHT_RED,
                        };
                        egui::Frame::popup(ui.style())
                            .fill(egui::Color32::from_black_alpha(230))
                            .show(ui, |ui| {
                                ui.colored_label(color, &t.message);
                            });
                    }
                });
            });
    }
}

impl eframe::App for MasterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();

        let ctx = ui.ctx().clone();
        self.handle_shortcuts(&ctx);

        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
            self.last_size = (rect.width(), rect.height());
        }

        ui.add_enabled_ui(self.model.modal.is_none(), |ui| {
            egui::Panel::top("toolbar").show_inside(ui, |ui| {
                toolbar::show(ui, &mut self.model, &self.backend);
            });

            egui::Panel::bottom("log_panel")
                .resizable(true)
                .default_size(if self.model.logs.expanded { 240.0 } else { 44.0 })
                .min_size(36.0)
                .show_inside(ui, |ui| {
                    log_panel::show(ui, &mut self.model, &self.backend);
                });

            egui::Panel::left("connection_tree")
                .resizable(true)
                .default_size(260.0)
                .min_size(180.0)
                .show_inside(ui, |ui| {
                    connection_tree::show(ui, &mut self.model, &self.backend);
                });

            egui::Panel::right("value_panel")
                .resizable(true)
                .default_size(300.0)
                .min_size(220.0)
                .show_inside(ui, |ui| {
                    value_panel::show(ui, &mut self.model, &self.backend);
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            matches!(
                                self.model.central_tab,
                                crate::model::CentralPanelTab::DataTable
                            ),
                            "📊 监控表",
                        )
                        .clicked()
                    {
                        self.model.central_tab = crate::model::CentralPanelTab::DataTable;
                    }
                    let mut clicked_tab: Option<usize> = None;
                    for (i, tab) in self.model.history_tabs.iter().enumerate() {
                        let label = format!("📈 {}", tab.display_name);
                        let selected = matches!(
                            self.model.central_tab,
                            crate::model::CentralPanelTab::History(j) if j == i
                        );
                        if ui.selectable_label(selected, &label).clicked() {
                            clicked_tab = Some(i);
                        }
                    }
                    if let Some(i) = clicked_tab {
                        self.model.central_tab = crate::model::CentralPanelTab::History(i);
                    }
                });
                ui.separator();

                match self.model.central_tab.clone() {
                    crate::model::CentralPanelTab::DataTable => {
                        data_table::show(ui, &mut self.model, &self.backend);
                    }
                    crate::model::CentralPanelTab::History(idx) => {
                        let mut close_idx: Option<usize> = None;
                        if let Some(state) = self.model.history_tabs.get_mut(idx) {
                            if state.pending_req.is_none() && state.last_loaded.is_none() {
                                crate::panels::history_tab::dispatch_refresh(
                                    state,
                                    &self.backend,
                                    &mut self.model.next_req_id,
                                );
                            }
                            let actions = crate::panels::history_tab::show(ui, state);
                            if actions.refresh {
                                crate::panels::history_tab::dispatch_refresh(
                                    state,
                                    &self.backend,
                                    &mut self.model.next_req_id,
                                );
                            }
                            if actions.close {
                                close_idx = Some(idx);
                            }
                        } else {
                            self.model.central_tab = crate::model::CentralPanelTab::DataTable;
                        }
                        if let Some(i) = close_idx {
                            self.model.history_tabs.remove(i);
                            if self.model.history_tabs.is_empty() {
                                self.model.central_tab =
                                    crate::model::CentralPanelTab::DataTable;
                            } else {
                                let new_idx = i.min(self.model.history_tabs.len() - 1);
                                self.model.central_tab =
                                    crate::model::CentralPanelTab::History(new_idx);
                            }
                        }
                    }
                }
            });
        });

        browse_panel::show(&ctx, &mut self.model, &self.backend);
        self.render_modal(&ctx);
        self.render_toasts(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.last_size.0 > 0.0 && self.last_size.1 > 0.0 {
            opcuaegui_shared::settings::save(
                crate::APP_ID,
                &opcuaegui_shared::settings::WindowSettings {
                    width: self.last_size.0,
                    height: self.last_size.1,
                },
            );
        }
    }
}

fn default_input_for(arg: &crate::events::MethodArgInfo) -> String {
    match arg.data_type.as_str() {
        "Boolean" => "false".into(),
        "String" => "".into(),
        "Float" | "Double" => "0.0".into(),
        _ => "0".into(),
    }
}
