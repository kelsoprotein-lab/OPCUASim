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
        crate::fonts::install_cjk_fonts(&cc.egui_ctx);
        let (backend, event_rx) = BackendHandle::new(cc.egui_ctx.clone());
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
            BackendEvent::Toast { level, message } => {
                self.model.push_toast(level, message);
            }
        }
    }

    fn render_modal(&mut self, ctx: &egui::Context) {
        let Some(modal) = &mut self.model.modal else {
            return;
        };
        match modal {
            Modal::NewConnection(state) => {
                let mut close = false;
                let submitted = connection_dialog::show(ctx, state, &mut close);
                if let Some(req) = submitted {
                    self.backend.send(UiCommand::CreateConnection(req));
                }
                if close {
                    self.model.modal = None;
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
                data_table::show(ui, &mut self.model, &self.backend);
            });
        });

        browse_panel::show(&ctx, &mut self.model, &self.backend);
        self.render_modal(&ctx);
        self.render_toasts(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.last_size.0 > 0.0 && self.last_size.1 > 0.0 {
            crate::settings::save(
                crate::APP_ID,
                &crate::settings::WindowSettings {
                    width: self.last_size.0,
                    height: self.last_size.1,
                },
            );
        }
    }
}
