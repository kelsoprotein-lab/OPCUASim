use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::{empty_state, info_row, section_label};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    section_label(ui, "节点详情");
    ui.separator();

    let Some(conn_id) = model.selected_conn.clone() else {
        empty_state(ui, "🔌", "未选择连接", Some("从左侧选择一个连接"));
        return;
    };

    let selected_count = model.monitor.selected_rows.len();
    if selected_count > 1 {
        empty_state(
            ui,
            "🗂",
            &format!("已选 {selected_count} 个节点"),
            Some("多选时仅支持批量操作，详情请先单选"),
        );
        return;
    }

    let Some(node_id) = model
        .monitor
        .focused_row
        .clone()
        .or_else(|| model.monitor.selected_rows.iter().next().cloned())
    else {
        empty_state(
            ui,
            "👈",
            "未选择节点",
            Some("从中央表格选择一行查看详情"),
        );
        return;
    };

    let row = model
        .monitor
        .per_conn
        .get(&conn_id)
        .and_then(|p| p.rows.get(&node_id))
        .cloned();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(row) = &row {
            section_label(ui, "Node Info");
            info_row(ui, "NodeId", &row.node_id);
            info_row(ui, "Name", &row.display_name);
            info_row(ui, "DataType", &row.data_type);
            info_row(ui, "Access", &access_str(row.user_access_level));
            info_row(
                ui,
                "Mode",
                &format!("{} · {:.0} ms", row.access_mode, row.interval_ms),
            );
            ui.add_space(6.0);

            section_label(ui, "Current Value");
            let value = row.value.as_deref().unwrap_or("—");
            ui.label(
                egui::RichText::new(value)
                    .size(22.0)
                    .monospace()
                    .color(theme::TEXT_PRIMARY),
            );
            if let Some(q) = &row.quality {
                ui.colored_label(super::quality_color(q), q);
            }
            ui.add_space(4.0);
            if let Some(ts) = &row.source_timestamp {
                info_row(ui, "Source", ts);
            }
            if let Some(ts) = &row.server_timestamp {
                info_row(ui, "Server", ts);
            }
            ui.add_space(6.0);
        }

        section_label(ui, "Actions");
        ui.horizontal(|ui| {
            if ui.button("⟳ 读取").clicked() {
                let req_id = model.alloc_req_id();
                model.value_panel.pending_read_req = Some(req_id);
                model.value_panel.last_result = None;
                backend.send(UiCommand::ReadAttrs {
                    conn_id: conn_id.clone(),
                    node_id: node_id.clone(),
                    req_id,
                });
            }
            if model.value_panel.pending_read_req.is_some() {
                ui.spinner();
            }
        });

        if let Some(attrs) = &model.value_panel.attrs {
            if attrs.node_id == node_id {
                ui.add_space(6.0);
                section_label(ui, "Read Result");
                info_row(ui, "DataType", &attrs.data_type);
                info_row(ui, "AccessLevel", &attrs.access_level);
                if let Some(v) = &attrs.value {
                    info_row(ui, "Value", v);
                }
                if let Some(q) = &attrs.quality {
                    info_row(ui, "Quality", q);
                }
                if !attrs.description.is_empty() {
                    info_row(ui, "Desc", &attrs.description);
                }
            }
        }

        let is_writable = row
            .as_ref()
            .map(|r| r.user_access_level & 0x02 != 0)
            .unwrap_or(false);
        if is_writable {
            ui.add_space(6.0);
            section_label(ui, "Write Value");
            let data_type = row
                .as_ref()
                .map(|r| r.data_type.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&data_type)
                        .small()
                        .color(theme::TEXT_MUTED),
                );
                ui.text_edit_singleline(&mut model.value_panel.write_value);
                let enabled = !model.value_panel.write_value.trim().is_empty()
                    && model.value_panel.pending_write_req.is_none();
                ui.add_enabled_ui(enabled, |ui| {
                    if ui.button("写入").clicked() {
                        let req_id = model.alloc_req_id();
                        model.value_panel.pending_write_req = Some(req_id);
                        backend.send(UiCommand::WriteValue {
                            conn_id: conn_id.clone(),
                            node_id: node_id.clone(),
                            value: model.value_panel.write_value.clone(),
                            data_type: data_type.clone(),
                            req_id,
                        });
                    }
                });
                if model.value_panel.pending_write_req.is_some() {
                    ui.spinner();
                }
            });
        }
    });
}

fn access_str(level: u8) -> String {
    let mut parts = Vec::new();
    if level & 0x01 != 0 {
        parts.push("R");
    }
    if level & 0x02 != 0 {
        parts.push("W");
    }
    if parts.is_empty() {
        format!("0x{level:02x}")
    } else {
        parts.join(" · ")
    }
}
