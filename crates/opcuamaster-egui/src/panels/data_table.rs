use egui_extras::{Column, TableBuilder};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    let Some(conn_id) = model.selected_conn.clone() else {
        ui.label("左侧选择一个连接以查看监控数据。");
        return;
    };

    ui.horizontal(|ui| {
        ui.heading("监控数据");
        ui.separator();
        ui.label("搜索:");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut model.monitor.search)
                .desired_width(200.0)
                .hint_text("NodeId / Name / Value"),
        );
        if resp.changed() {
            model.monitor.filter_dirty = true;
        }
        ui.separator();
        let selected_count = model.monitor.selected_rows.len();
        if selected_count > 0 {
            ui.label(format!("已选 {selected_count} 行"));
            if ui.button("🗑 移除选中").clicked() {
                let ids: Vec<String> = model.monitor.selected_rows.iter().cloned().collect();
                backend.send(UiCommand::RemoveMonitoredNodes {
                    conn_id: conn_id.clone(),
                    node_ids: ids.clone(),
                });
                if let Some(per) = model.monitor.per_conn.get_mut(&conn_id) {
                    for id in &ids {
                        per.rows.shift_remove(id);
                    }
                }
                model.monitor.selected_rows.clear();
                model.monitor.filter_dirty = true;
            }
            if !model.groups.is_empty() {
                ui.menu_button("➕ 加入分组", |ui| {
                    let groups = model.groups.clone();
                    for g in &groups {
                        if ui.button(format!("{} ({})", g.name, g.node_ids.len())).clicked() {
                            let ids: Vec<String> =
                                model.monitor.selected_rows.iter().cloned().collect();
                            backend.send(UiCommand::AddNodesToGroup {
                                group_id: g.id.clone(),
                                node_ids: ids,
                            });
                            ui.close();
                        }
                    }
                });
            }
        }
    });
    ui.separator();

    let filtered = model.monitor.ensure_filter(&conn_id).to_vec();
    let total = filtered.len();
    let per = model.monitor.per_conn.get(&conn_id);
    let rows_ref = per.map(|p| &p.rows);

    if rows_ref.map(|r| r.is_empty()).unwrap_or(true) {
        ui.label("(尚无订阅节点,点击工具栏 🌲 浏览节点 添加)");
        return;
    }

    let ctx_modifiers = ui.ctx().input(|i| i.modifiers);
    let ctrl_held = ctx_modifiers.ctrl || ctx_modifiers.command;

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(220.0).at_least(100.0).clip(true))
        .column(Column::initial(140.0).at_least(60.0).clip(true))
        .column(Column::initial(90.0).at_least(50.0).clip(true))
        .column(Column::initial(140.0).at_least(60.0).clip(true))
        .column(Column::initial(90.0).at_least(50.0).clip(true))
        .column(Column::initial(100.0).at_least(60.0).clip(true))
        .column(Column::initial(100.0).at_least(60.0).clip(true))
        .column(Column::initial(110.0).at_least(80.0).clip(true));
    table = table.sense(egui::Sense::click());

    table
        .header(22.0, |mut header| {
            for label in ["NodeId", "Name", "Type", "Value", "Quality", "Src TS", "Srv TS", "Mode"] {
                header.col(|ui| {
                    ui.strong(label);
                });
            }
        })
        .body(|body| {
            body.rows(20.0, total, |mut row| {
                let idx = row.index();
                let Some(node_id) = filtered.get(idx) else {
                    return;
                };
                let Some(rows) = rows_ref else { return };
                let Some(data) = rows.get(node_id) else {
                    return;
                };
                let selected = model.monitor.selected_rows.contains(node_id);
                row.set_selected(selected);

                row.col(|ui| {
                    ui.label(&data.node_id);
                });
                row.col(|ui| {
                    ui.label(&data.display_name);
                });
                row.col(|ui| {
                    ui.label(&data.data_type);
                });
                row.col(|ui| {
                    let v = data.value.as_deref().unwrap_or("—");
                    ui.monospace(v);
                });
                row.col(|ui| {
                    let q = data.quality.as_deref().unwrap_or("");
                    let color = super::quality_color(q);
                    ui.colored_label(color, q);
                });
                row.col(|ui| {
                    ui.label(super::format_hms(data.source_timestamp.as_deref()));
                });
                row.col(|ui| {
                    ui.label(super::format_hms(data.server_timestamp.as_deref()));
                });
                row.col(|ui| {
                    ui.label(format!("{} · {:.0}ms", data.access_mode, data.interval_ms));
                });

                if row.response().clicked() {
                    if ctrl_held {
                        if selected {
                            model.monitor.selected_rows.remove(node_id);
                        } else {
                            model.monitor.selected_rows.insert(node_id.clone());
                        }
                    } else {
                        model.monitor.selected_rows.clear();
                        model.monitor.selected_rows.insert(node_id.clone());
                    }
                    model.monitor.focused_row = Some(node_id.clone());
                    model.value_panel.attrs = None;
                    model.value_panel.write_value.clear();
                    model.value_panel.last_result = None;
                }
            });
        });
}

