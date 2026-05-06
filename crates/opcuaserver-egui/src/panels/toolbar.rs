use opcuasim_core::server::models::DataType;
use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::{
    pick_open_project_path, pick_save_project_path, status_chip, OBJECTS_ROOT_ID,
};

use crate::events::{AddNodeReq, UiCommand};
use crate::model::{AppModel, SimKind};
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("OPCUAServer")
                .strong()
                .size(15.0)
                .color(theme::ACCENT()),
        );
        let running = model.status.state == "Running";
        let starting = model.status.state == "Starting";
        let (icon, color, label) = match model.status.state.as_str() {
            "Running" => ("●", theme::STATUS_OK(), "运行中"),
            "Starting" => ("◐", theme::STATUS_WARN(), "启动中"),
            "Stopping" => ("◑", theme::STATUS_WARN(), "停止中"),
            "Stopped" => ("○", theme::STATUS_BAD(), "已停止"),
            other => ("·", theme::STATUS_IDLE(), other),
        };
        status_chip(ui, color, icon, label);

        ui.separator();

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
        ui.label(
            egui::RichText::new("Endpoint")
                .small()
                .color(theme::TEXT_MUTED()),
        );
        ui.monospace(&model.status.endpoint_url);

        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                let mode = opcuaegui_shared::theme::current_mode();
                let (label, hint) = match mode {
                    opcuaegui_shared::theme::ThemeMode::Dark => ("🌞", "切换到浅色主题"),
                    opcuaegui_shared::theme::ThemeMode::Light => ("🌙", "切换到暗色主题"),
                };
                if ui.button(label).on_hover_text(hint).clicked() {
                    let next = match mode {
                        opcuaegui_shared::theme::ThemeMode::Dark => {
                            opcuaegui_shared::theme::ThemeMode::Light
                        }
                        opcuaegui_shared::theme::ThemeMode::Light => {
                            opcuaegui_shared::theme::ThemeMode::Dark
                        }
                    };
                    opcuaegui_shared::theme::set_mode(next);
                    opcuaegui_shared::theme::apply(ui.ctx());
                }
                if ui
                    .button("📂 打开")
                    .on_hover_text("Cmd/Ctrl+O")
                    .clicked()
                {
                    if let Some(path) = pick_open_project_path() {
                        backend.send(UiCommand::LoadProject(path));
                    }
                }
                if ui
                    .button("💾 保存")
                    .on_hover_text("Cmd/Ctrl+S")
                    .clicked()
                {
                    if let Some(path) = pick_save_project_path("server.opcuaproj") {
                        backend.send(UiCommand::SaveProject(path));
                    }
                }
            },
        );
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("📁 新建文件夹")
                .small()
                .color(theme::TEXT_MUTED()),
        );
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
                    parent_id: OBJECTS_ROOT_ID.to_string(),
                });
                model.new_folder_name.clear();
            }
        });

        ui.separator();

        ui.label(
            egui::RichText::new("📊 新建节点")
                .small()
                .color(theme::TEXT_MUTED()),
        );
        add_node_form(ui, model, backend);
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
            for k in [
                SimKind::Static,
                SimKind::Random,
                SimKind::Sine,
                SimKind::Linear,
                SimKind::Script,
            ] {
                ui.selectable_value(&mut form.sim_kind, k, sim_label(k));
            }
        });
    ui.checkbox(&mut form.writable, "RW")
        .on_hover_text("是否允许客户端写入");
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
