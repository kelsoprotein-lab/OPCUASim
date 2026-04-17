use crate::events::UiCommand;
use crate::model::{AppModel, Modal};
use crate::runtime::BackendHandle;
use crate::widgets::connection_dialog::ConnDialogState;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.horizontal(|ui| {
        ui.heading("OPCUAMaster");
        ui.separator();

        if ui.button("➕ 新建连接").clicked() {
            model.modal = Some(Modal::NewConnection(ConnDialogState::default()));
        }

        let has_sel = model.selected_conn.is_some();
        let sel_info = model
            .selected_conn
            .as_ref()
            .and_then(|id| model.connections.iter().find(|c| &c.id == id));
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
            if ui.button("🗑 删除").clicked() {
                if let Some(id) = model.selected_conn.clone() {
                    backend.send(UiCommand::DeleteConnection(id.clone()));
                    if model.selected_conn.as_deref() == Some(&id) {
                        model.selected_conn = None;
                    }
                }
            }
        });

        ui.separator();

        ui.add_enabled_ui(is_connected, |ui| {
            if ui.button("🌲 浏览节点").clicked() {
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

        if ui.button("💾 保存项目").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("project.opcuaproj")
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .save_file()
            {
                backend.send(UiCommand::SaveProject(path));
            }
        }
        if ui.button("📂 打开项目").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OPCUA Project", &["opcuaproj", "json"])
                .pick_file()
            {
                backend.send(UiCommand::LoadProject(path));
            }
        }
    });
}
