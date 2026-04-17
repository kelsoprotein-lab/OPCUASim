use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Connections");
        ui.separator();
        if model.connections.is_empty() {
            ui.label("(暂无连接,点击 ➕ 新建连接)");
        } else {
            let conns = model.connections.clone();
            for conn in &conns {
                let selected = model.selected_conn.as_deref() == Some(&conn.id);
                let indicator = match conn.state.as_str() {
                    "Connected" => ("●", egui::Color32::from_rgb(80, 200, 120)),
                    "Connecting" => ("◐", egui::Color32::from_rgb(240, 200, 80)),
                    "Disconnected" => ("○", egui::Color32::from_rgb(180, 80, 80)),
                    _ => ("·", egui::Color32::GRAY),
                };
                let resp = ui.selectable_label(
                    selected,
                    egui::RichText::new(format!("{}  {}", indicator.0, conn.name))
                        .color(indicator.1),
                );
                if resp.clicked() {
                    model.selected_conn = Some(conn.id.clone());
                }
                if selected {
                    ui.indent(&conn.id, |ui| {
                        ui.label(
                            egui::RichText::new(&conn.endpoint_url)
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                conn.auth_type, conn.security_policy, conn.security_mode
                            ))
                            .small()
                            .color(egui::Color32::GRAY),
                        );
                    });
                }
            }
        }

        ui.add_space(10.0);
        ui.separator();
        ui.heading("Groups");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut model.group_input)
                    .desired_width(140.0)
                    .hint_text("分组名称"),
            );
            let enabled = !model.group_input.trim().is_empty();
            ui.add_enabled_ui(enabled, |ui| {
                if ui.button("➕").clicked() {
                    backend.send(UiCommand::CreateGroup(
                        model.group_input.trim().to_string(),
                    ));
                    model.group_input.clear();
                }
            });
        });
        if model.groups.is_empty() {
            ui.small("(暂无分组)");
        } else {
            let groups = model.groups.clone();
            for g in &groups {
                ui.horizontal(|ui| {
                    ui.label(format!("· {} ({})", g.name, g.node_ids.len()));
                    if ui.small_button("🗑").clicked() {
                        backend.send(UiCommand::DeleteGroup(g.id.clone()));
                    }
                });
            }
        }
    });
}
