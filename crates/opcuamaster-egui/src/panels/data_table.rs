use egui_extras::{Column, TableBuilder};
use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::{empty_state, status_chip};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    let Some(conn_id) = model.selected_conn.clone() else {
        empty_state(
            ui,
            "📡",
            "未选择连接",
            Some("从左侧连接列表选择一个已连接的实例"),
        );
        return;
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("监控数据")
                .strong()
                .color(theme::TEXT_PRIMARY()),
        );
        let total_rows = model
            .monitor
            .per_conn
            .get(&conn_id)
            .map(|p| p.rows.len())
            .unwrap_or(0);
        ui.label(
            egui::RichText::new(format!("· {total_rows} 个节点"))
                .small()
                .color(theme::TEXT_MUTED()),
        );
        ui.separator();
        ui.label(
            egui::RichText::new("搜索")
                .small()
                .color(theme::TEXT_MUTED()),
        );
        let resp = ui.add(
            egui::TextEdit::singleline(&mut model.monitor.search)
                .desired_width(220.0)
                .hint_text("NodeId / Name / Value"),
        );
        if resp.changed() {
            model.monitor.filter_dirty = true;
        }
        ui.separator();
        let selected_count = model.monitor.selected_rows.len();
        if selected_count > 0 {
            status_chip(
                ui,
                theme::ACCENT(),
                "▣",
                &format!("已选 {selected_count}"),
            );
            if ui
                .button("🗑 移除选中")
                .on_hover_text("Delete / Backspace")
                .clicked()
            {
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

    let rows_empty = model
        .monitor
        .per_conn
        .get(&conn_id)
        .map(|p| p.rows.is_empty())
        .unwrap_or(true);
    if rows_empty {
        empty_state(
            ui,
            "🌲",
            "尚无订阅节点",
            Some("点击工具栏 🌲 浏览节点，勾选变量后添加"),
        );
        return;
    }

    let _ = model.monitor.ensure_filter(&conn_id);
    let total = model.monitor.filtered_cache.len();

    let modifiers = ui.ctx().input(|i| i.modifiers);
    let ctrl_held = modifiers.ctrl || modifiers.command;
    let shift_held = modifiers.shift;

    enum RowAction {
        Click {
            filtered_idx: usize,
            node_id: String,
        },
        History {
            node_id: String,
            display_name: String,
        },
    }
    let mut action: Option<RowAction> = None;

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
            let cache = &model.monitor.filtered_cache;
            let rows = model.monitor.per_conn.get(&conn_id).map(|p| &p.rows);
            body.rows(20.0, total, |mut row| {
                let idx = row.index();
                let Some(node_id) = cache.get(idx) else {
                    return;
                };
                let Some(rows) = rows else { return };
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

                let row_resp = row.response();
                if row_resp.clicked() {
                    action = Some(RowAction::Click {
                        filtered_idx: idx,
                        node_id: node_id.clone(),
                    });
                }
                let nid = data.node_id.clone();
                let dname = data.display_name.clone();
                row_resp.context_menu(|ui| {
                    if ui.button("📈 查看历史").clicked() {
                        action = Some(RowAction::History {
                            node_id: nid.clone(),
                            display_name: dname.clone(),
                        });
                        ui.close();
                    }
                });
            });
        });

    match action {
        Some(RowAction::Click { filtered_idx, node_id }) => {
            if shift_held {
                if let Some(anchor) = model.monitor.last_clicked_filtered_idx {
                    let (lo, hi) = if anchor <= filtered_idx {
                        (anchor, filtered_idx)
                    } else {
                        (filtered_idx, anchor)
                    };
                    if !ctrl_held {
                        model.monitor.selected_rows.clear();
                    }
                    let ids_to_add: Vec<String> = model
                        .monitor
                        .filtered_cache
                        .iter()
                        .skip(lo)
                        .take(hi - lo + 1)
                        .cloned()
                        .collect();
                    for id in ids_to_add {
                        model.monitor.selected_rows.insert(id);
                    }
                } else {
                    model.monitor.selected_rows.insert(node_id.clone());
                }
            } else if ctrl_held {
                if model.monitor.selected_rows.contains(&node_id) {
                    model.monitor.selected_rows.remove(&node_id);
                } else {
                    model.monitor.selected_rows.insert(node_id.clone());
                }
                model.monitor.last_clicked_filtered_idx = Some(filtered_idx);
            } else {
                model.monitor.selected_rows.clear();
                model.monitor.selected_rows.insert(node_id.clone());
                model.monitor.last_clicked_filtered_idx = Some(filtered_idx);
            }
            model.monitor.focused_row = Some(node_id);
            model.value_panel.attrs = None;
            model.value_panel.write_value.clear();
            model.value_panel.last_result = None;
        }
        Some(RowAction::History { node_id, display_name }) => {
            crate::panels::browse_panel::open_history_tab(
                model,
                &conn_id,
                &node_id,
                &display_name,
            );
        }
        None => {}
    }
}
