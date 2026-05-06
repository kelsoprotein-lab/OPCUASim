use egui_extras::{Column, TableBuilder};
use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::empty_state;

use opcuasim_core::server::models::SimulationMode;

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("节点列表")
                .strong()
                .color(theme::TEXT_PRIMARY()),
        );
        let count = model.address_space.nodes.len();
        ui.label(
            egui::RichText::new(format!("· {count} 个变量"))
                .small()
                .color(theme::TEXT_MUTED()),
        );
        let multi = model.selected_node_ids.len();
        if multi > 1 {
            ui.separator();
            ui.label(
                egui::RichText::new(format!("已选 {multi}"))
                    .small()
                    .color(theme::ACCENT()),
            );
            if ui.button("🗑 移除选中").clicked() {
                let ids: Vec<String> = model.selected_node_ids.iter().cloned().collect();
                for id in &ids {
                    backend.send(UiCommand::RemoveNode(id.clone()));
                }
                model.selected_node_ids.clear();
                if let Some(sel) = &model.selected_node_id {
                    if ids.contains(sel) {
                        model.selected_node_id = None;
                    }
                }
            }
        }
    });
    ui.separator();

    let total = model.address_space.nodes.len();
    if total == 0 {
        empty_state(
            ui,
            "📊",
            "尚未定义变量",
            Some("使用顶部 📊 新建节点 添加一个 Variable"),
        );
        return;
    }

    let ctx_modifiers = ui.ctx().input(|i| i.modifiers);
    let ctrl = ctx_modifiers.ctrl || ctx_modifiers.command;
    let shift = ctx_modifiers.shift;

    enum RowAction {
        Click {
            idx: usize,
            node_id: String,
        },
        Delete(String),
    }

    let mut action: Option<RowAction> = None;

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(180.0).at_least(80.0).clip(true))
        .column(Column::initial(160.0).at_least(100.0).clip(true))
        .column(Column::initial(90.0).at_least(60.0).clip(true))
        .column(Column::initial(90.0).at_least(60.0).clip(true))
        .column(Column::initial(180.0).at_least(80.0).clip(true))
        .column(Column::initial(60.0).at_least(40.0));
    table = table.sense(egui::Sense::click());
    table
        .header(22.0, |mut header| {
            for label in ["Name", "NodeId", "DataType", "SimMode", "Value", "RW"] {
                header.col(|ui| {
                    ui.strong(label);
                });
            }
        })
        .body(|body| {
            body.rows(20.0, total, |mut row| {
                let idx = row.index();
                let Some(n) = model.address_space.nodes.get(idx) else {
                    return;
                };
                let multi_selected = model.selected_node_ids.contains(&n.node_id);
                let single_selected =
                    model.selected_node_id.as_deref() == Some(&n.node_id);
                row.set_selected(multi_selected || single_selected);

                row.col(|ui| {
                    ui.label(&n.display_name);
                });
                row.col(|ui| {
                    ui.label(
                        egui::RichText::new(&n.node_id)
                            .monospace()
                            .small()
                            .color(theme::TEXT_MUTED()),
                    );
                });
                row.col(|ui| {
                    ui.label(n.data_type.to_string());
                });
                row.col(|ui| {
                    ui.label(sim_label(&n.simulation));
                });
                row.col(|ui| {
                    let v = model
                        .current_values
                        .get(&n.node_id)
                        .map(|s| s.as_str())
                        .or(n.current_value.as_deref())
                        .unwrap_or("—");
                    ui.monospace(v);
                });
                row.col(|ui| {
                    let (lbl, color) = if n.writable {
                        ("RW", theme::ACCENT())
                    } else {
                        ("R", theme::TEXT_MUTED())
                    };
                    ui.colored_label(color, lbl);
                });

                let resp = row.response();
                if resp.clicked() {
                    action = Some(RowAction::Click {
                        idx,
                        node_id: n.node_id.clone(),
                    });
                }
                let nid_for_menu = n.node_id.clone();
                resp.context_menu(|ui| {
                    if ui.button("🗑 删除节点").clicked() {
                        action = Some(RowAction::Delete(nid_for_menu.clone()));
                        ui.close();
                    }
                });
            });
        });

    match action {
        Some(RowAction::Click { idx, node_id }) => {
            if shift {
                if let Some(anchor) = model.last_clicked_row {
                    let (lo, hi) = if anchor <= idx {
                        (anchor, idx)
                    } else {
                        (idx, anchor)
                    };
                    if !ctrl {
                        model.selected_node_ids.clear();
                    }
                    for i in lo..=hi {
                        if let Some(row) = model.address_space.nodes.get(i) {
                            model.selected_node_ids.insert(row.node_id.clone());
                        }
                    }
                } else {
                    model.selected_node_ids.insert(node_id.clone());
                }
            } else if ctrl {
                if model.selected_node_ids.contains(&node_id) {
                    model.selected_node_ids.remove(&node_id);
                } else {
                    model.selected_node_ids.insert(node_id.clone());
                }
                model.last_clicked_row = Some(idx);
            } else {
                model.selected_node_ids.clear();
                model.last_clicked_row = Some(idx);
            }
            model.selected_node_id = Some(node_id);
        }
        Some(RowAction::Delete(id)) => {
            backend.send(UiCommand::RemoveNode(id.clone()));
            model.selected_node_ids.remove(&id);
            if model.selected_node_id.as_deref() == Some(&id) {
                model.selected_node_id = None;
            }
        }
        None => {}
    }
}

fn sim_label(s: &SimulationMode) -> &'static str {
    match s {
        SimulationMode::Static { .. } => "Static",
        SimulationMode::Random { .. } => "Random",
        SimulationMode::Sine { .. } => "Sine",
        SimulationMode::Linear { .. } => "Linear",
        SimulationMode::Script { .. } => "Script",
    }
}
