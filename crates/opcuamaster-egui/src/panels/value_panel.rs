use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.heading("节点详情");
    ui.separator();

    let Some(conn_id) = model.selected_conn.clone() else {
        ui.label("未选择连接");
        return;
    };

    let selected_count = model.monitor.selected_rows.len();
    if selected_count > 1 {
        ui.label(format!("已选 {selected_count} 个节点"));
        ui.label("多选时不可查看详情。");
        return;
    }

    let Some(node_id) = model
        .monitor
        .focused_row
        .clone()
        .or_else(|| model.monitor.selected_rows.iter().next().cloned())
    else {
        ui.label("从表格选择一行查看详情");
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
            ui.label(egui::RichText::new("NODE INFO").small().strong());
            info_row(ui, "NodeId", &row.node_id);
            info_row(ui, "Name", &row.display_name);
            info_row(ui, "DataType", &row.data_type);
            info_row(
                ui,
                "Access",
                &access_str(row.user_access_level),
            );
            info_row(
                ui,
                "Mode",
                &format!("{} · {:.0}ms", row.access_mode, row.interval_ms),
            );
            ui.separator();

            ui.label(egui::RichText::new("CURRENT VALUE").small().strong());
            ui.add_space(4.0);
            let value = row.value.as_deref().unwrap_or("—");
            ui.label(egui::RichText::new(value).size(20.0).monospace());
            if let Some(q) = &row.quality {
                ui.colored_label(super::quality_color(q), q);
            }
            ui.add_space(6.0);
            if let Some(ts) = &row.source_timestamp {
                ui.small(format!("Source: {ts}"));
            }
            if let Some(ts) = &row.server_timestamp {
                ui.small(format!("Server: {ts}"));
            }
            ui.separator();
        }

        ui.label(egui::RichText::new("ACTIONS").small().strong());
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
                ui.separator();
                ui.label(egui::RichText::new("READ RESULT").small().strong());
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

        // Write
        let is_writable = row
            .as_ref()
            .map(|r| r.user_access_level & 0x02 != 0)
            .unwrap_or(false);
        if is_writable {
            ui.separator();
            ui.label(egui::RichText::new("WRITE VALUE").small().strong());
            let data_type = row
                .as_ref()
                .map(|r| r.data_type.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            ui.horizontal(|ui| {
                ui.label(&data_type);
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

        if let Some(msg) = &model.value_panel.last_result {
            ui.add_space(4.0);
            ui.small(msg);
        }
    });
}

fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{label}:")).color(egui::Color32::GRAY));
        ui.label(value);
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
