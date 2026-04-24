use egui_extras::{Column, TableBuilder};

use opcuasim_core::server::models::SimulationMode;

use crate::model::AppModel;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel) {
    ui.heading("节点列表");
    ui.separator();

    let nodes = model.address_space.nodes.clone();
    let total = nodes.len();
    if total == 0 {
        ui.label("(无变量节点。使用顶部表单添加。)");
        return;
    }

    let ctx_modifiers = ui.ctx().input(|i| i.modifiers);
    let _ctrl = ctx_modifiers.ctrl || ctx_modifiers.command;

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
                let Some(n) = nodes.get(row.index()) else {
                    return;
                };
                let selected = model.selected_node_id.as_deref() == Some(&n.node_id);
                row.set_selected(selected);

                row.col(|ui| {
                    ui.label(&n.display_name);
                });
                row.col(|ui| {
                    ui.monospace(&n.node_id);
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
                        .cloned()
                        .or_else(|| n.current_value.clone())
                        .unwrap_or_else(|| "—".to_string());
                    ui.monospace(v);
                });
                row.col(|ui| {
                    ui.label(if n.writable { "RW" } else { "R" });
                });
                if row.response().clicked() {
                    model.selected_node_id = Some(n.node_id.clone());
                }
            });
        });
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
