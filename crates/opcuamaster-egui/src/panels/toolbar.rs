use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::status_chip;

use crate::events::UiCommand;
use crate::model::{AppModel, Modal};
use crate::runtime::BackendHandle;
use crate::widgets::connection_dialog::ConnDialogState;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("OPCUAMaster")
                .strong()
                .size(15.0)
                .color(theme::ACCENT),
        );

        let sel_info = model
            .selected_conn
            .as_ref()
            .and_then(|id| model.connections.iter().find(|c| &c.id == id));
        if let Some(info) = sel_info {
            ui.label(
                egui::RichText::new(format!("· {}", info.name))
                    .small()
                    .color(theme::TEXT_MUTED),
            );
            let (icon, color, label) = match info.state.as_str() {
                "Connected" => ("●", theme::STATUS_OK, "在线"),
                "Connecting" => ("◐", theme::STATUS_WARN, "连接中"),
                "Disconnected" => ("○", theme::STATUS_BAD, "离线"),
                other => ("·", theme::STATUS_IDLE, other),
            };
            status_chip(ui, color, icon, label);
        }

        ui.separator();

        // ─── Group: Connection ─────────────────────────────────────────
        if ui
            .button("➕ 新建连接")
            .on_hover_text("Cmd/Ctrl+N")
            .clicked()
        {
            model.modal = Some(Modal::NewConnection(ConnDialogState::default()));
        }

        let has_sel = model.selected_conn.is_some();
        let is_connected = sel_info.map(|c| c.state == "Connected").unwrap_or(false);
        let is_connecting = sel_info.map(|c| c.state == "Connecting").unwrap_or(false);

        ui.add_enabled_ui(has_sel && !is_connected && !is_connecting, |ui| {
            if ui.button("🔌 连接").clicked() {
                if let Some(id) = model.selected_conn.clone() {
                    backend.send(UiCommand::Connect(id));
                }
            }
        });

        ui.add_enabled_ui(is_connected, |ui| {
            if ui.button("✂ 断开").clicked() {
                if let Some(id) = model.selected_conn.clone() {
                    backend.send(UiCommand::Disconnect(id));
                }
            }
        });

        ui.add_enabled_ui(has_sel, |ui| {
            if ui
                .button("🗑 删除")
                .on_hover_text("从项目中删除该连接")
                .clicked()
            {
                if let Some(id) = model.selected_conn.clone() {
                    backend.send(UiCommand::DeleteConnection(id.clone()));
                    if model.selected_conn.as_deref() == Some(&id) {
                        model.selected_conn = None;
                    }
                }
            }
        });

        ui.separator();

        // ─── Group: Data ───────────────────────────────────────────────
        ui.add_enabled_ui(is_connected, |ui| {
            if ui
                .button("🌲 浏览节点")
                .on_hover_text("打开地址空间浏览器")
                .clicked()
            {
                if let Some(id) = model.selected_conn.clone() {
                    let req_id = model.alloc_req_id();
                    model.browse.open = true;
                    model.browse.conn_id = Some(id.clone());
                    model.browse.root_loaded = false;
                    model.browse.nodes.clear();
                    model.browse.roots.clear();
                    model.browse.pending.clear();
                    model.browse.pending.insert(req_id);
                    backend.send(UiCommand::BrowseRoot {
                        conn_id: id,
                        req_id,
                    });
                }
            }
        });

        ui.separator();

        // ─── Group: Project ────────────────────────────────────────────
        if ui
            .button("💾 保存")
            .on_hover_text("Cmd/Ctrl+S — 保存项目")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("project.opcuaproj")
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .save_file()
            {
                backend.send(UiCommand::SaveProject(path));
            }
        }
        if ui
            .button("📂 打开")
            .on_hover_text("Cmd/Ctrl+O — 加载项目")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .pick_file()
            {
                backend.send(UiCommand::LoadProject(path));
            }
        }

        ui.separator();

        // ─── Group: System ─────────────────────────────────────────────
        if ui
            .button("🔐 证书")
            .on_hover_text("管理 PKI 信任 / 拒绝列表")
            .clicked()
        {
            let trusted_req = model.alloc_req_id();
            let rejected_req = model.alloc_req_id();
            model.modal = Some(Modal::CertManager(crate::model::CertManagerState {
                pending_trusted_req: Some(trusted_req),
                pending_rejected_req: Some(rejected_req),
                ..Default::default()
            }));
            backend.send(UiCommand::ListCertificates {
                role: crate::events::CertRoleDto::Trusted,
                req_id: trusted_req,
            });
            backend.send(UiCommand::ListCertificates {
                role: crate::events::CertRoleDto::Rejected,
                req_id: rejected_req,
            });
        }
    });
}
