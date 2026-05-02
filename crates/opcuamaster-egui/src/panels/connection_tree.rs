use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::{empty_state, status_chip};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(
            egui::RichText::new("CONNECTIONS")
                .strong()
                .small()
                .color(theme::TEXT_MUTED),
        );
        ui.separator();
        if model.connections.is_empty() {
            empty_state(
                ui,
                "🔌",
                "暂无连接",
                Some("点击工具栏 ➕ 新建连接"),
            );
        } else {
            let conns = model.connections.clone();
            for conn in &conns {
                let selected = model.selected_conn.as_deref() == Some(&conn.id);
                let (icon, color, label) = match conn.state.as_str() {
                    "Connected" => ("●", theme::STATUS_OK, "在线"),
                    "Connecting" => ("◐", theme::STATUS_WARN, "连接中"),
                    "Disconnected" => ("○", theme::STATUS_BAD, "离线"),
                    _ => ("·", theme::STATUS_IDLE, conn.state.as_str()),
                };
                let resp = ui.horizontal(|ui| {
                    let r = ui.selectable_label(
                        selected,
                        egui::RichText::new(&conn.name)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            status_chip(ui, color, icon, label);
                        },
                    );
                    r
                });
                if resp.inner.clicked() {
                    model.selected_conn = Some(conn.id.clone());
                }
                if selected {
                    ui.indent(&conn.id, |ui| {
                        ui.label(
                            egui::RichText::new(&conn.endpoint_url)
                                .small()
                                .color(theme::TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                conn.auth_type, conn.security_policy, conn.security_mode
                            ))
                            .small()
                            .color(theme::TEXT_FAINT),
                        );
                    });
                }
            }
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label(
            egui::RichText::new("GROUPS")
                .strong()
                .small()
                .color(theme::TEXT_MUTED),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut model.group_input)
                    .desired_width(140.0)
                    .hint_text("分组名称"),
            );
            let enabled = !model.group_input.trim().is_empty();
            ui.add_enabled_ui(enabled, |ui| {
                if ui.button("➕").on_hover_text("新建分组").clicked() {
                    backend.send(UiCommand::CreateGroup(
                        model.group_input.trim().to_string(),
                    ));
                    model.group_input.clear();
                }
            });
        });
        if model.groups.is_empty() {
            ui.label(
                egui::RichText::new("(暂无分组)")
                    .small()
                    .color(theme::TEXT_FAINT),
            );
        } else {
            let groups = model.groups.clone();
            for g in &groups {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("· {}", g.name))
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.label(
                        egui::RichText::new(format!("({})", g.node_ids.len()))
                            .small()
                            .color(theme::TEXT_MUTED),
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.small_button("🗑").on_hover_text("删除分组").clicked() {
                                backend.send(UiCommand::DeleteGroup(g.id.clone()));
                            }
                        },
                    );
                });
            }
        }
    });
}
