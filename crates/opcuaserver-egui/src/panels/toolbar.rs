use opcuasim_core::server::models::DataType;

use crate::events::{AddNodeReq, UiCommand};
use crate::model::{AppModel, SimKind};
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.horizontal(|ui| {
        ui.heading("OPCUAServer");
        ui.separator();
        let running = model.status.state == "Running";
        let starting = model.status.state == "Starting";
        ui.add_enabled_ui(!running && !starting, |ui| {
            if ui.button("▶ 启动").clicked() {
                backend.send(UiCommand::StartServer);
            }
        });
        ui.add_enabled_ui(running, |ui| {
            if ui.button("■ 停止").clicked() {
                backend.send(UiCommand::StopServer);
            }
        });
        ui.separator();
        ui.label("Endpoint:");
        ui.monospace(&model.status.endpoint_url);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("➕ 新建文件夹:");
        ui.add(
            egui::TextEdit::singleline(&mut model.new_folder_name)
                .hint_text("display name")
                .desired_width(140.0),
        );
        let enabled = !model.new_folder_name.trim().is_empty();
        ui.add_enabled_ui(enabled, |ui| {
            if ui.button("添加").clicked() {
                let name = model.new_folder_name.trim().to_string();
                let node_id = format!("ns=2;s={}", name.replace(' ', "_"));
                backend.send(UiCommand::AddFolder {
                    node_id,
                    display_name: name,
                    parent_id: "Objects".to_string(),
                });
                model.new_folder_name.clear();
            }
        });
        ui.separator();
        ui.label("📄 新建节点:");
        add_node_form(ui, model, backend);
        ui.separator();
        if ui.button("💾 保存").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("server.opcuaproj")
                .add_filter("OPCUA Server Project", &["opcuaproj", "json"])
                .save_file()
            {
                backend.send(UiCommand::SaveProject(path));
            }
        }
        if ui.button("📂 打开").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OPCUA Server Project", &["opcuaproj", "json"])
                .pick_file()
            {
                backend.send(UiCommand::LoadProject(path));
            }
        }
    });
}

fn add_node_form(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    let form = &mut model.add_node_form;
    ui.add(
        egui::TextEdit::singleline(&mut form.display_name)
            .hint_text("Name")
            .desired_width(120.0),
    );
    egui::ComboBox::from_id_salt("add_dt")
        .selected_text(form.data_type.to_string())
        .width(80.0)
        .show_ui(ui, |ui| {
            for dt in [
                DataType::Boolean,
                DataType::Int32,
                DataType::Double,
                DataType::Float,
                DataType::String,
                DataType::Int16,
                DataType::Int64,
                DataType::UInt16,
                DataType::UInt32,
                DataType::UInt64,
            ] {
                ui.selectable_value(&mut form.data_type, dt.clone(), dt.to_string());
            }
        });
    egui::ComboBox::from_id_salt("add_sim")
        .selected_text(sim_label(form.sim_kind))
        .width(90.0)
        .show_ui(ui, |ui| {
            for k in [SimKind::Static, SimKind::Random, SimKind::Sine, SimKind::Linear, SimKind::Script] {
                ui.selectable_value(&mut form.sim_kind, k, sim_label(k));
            }
        });
    ui.checkbox(&mut form.writable, "W");
    let enabled = !form.display_name.trim().is_empty();
    ui.add_enabled_ui(enabled, |ui| {
        if ui.button("添加").clicked() {
            let name = form.display_name.trim().to_string();
            let node_id = if form.node_id.trim().is_empty() {
                format!("ns=2;s={}", name.replace(' ', "_"))
            } else {
                form.node_id.trim().to_string()
            };
            backend.send(UiCommand::AddNode(AddNodeReq {
                node_id,
                display_name: name,
                parent_id: form.parent_id.clone(),
                data_type: form.data_type.clone(),
                writable: form.writable,
                simulation: form.build_simulation(),
            }));
            form.display_name.clear();
            form.node_id.clear();
        }
    });
}

fn sim_label(k: SimKind) -> &'static str {
    match k {
        SimKind::Static => "Static",
        SimKind::Random => "Random",
        SimKind::Sine => "Sine",
        SimKind::Linear => "Linear",
        SimKind::Script => "Script",
    }
}
