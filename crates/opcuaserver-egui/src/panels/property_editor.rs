use opcuasim_core::server::models::{LinearMode, SimulationMode};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.heading("节点属性");
    ui.separator();

    let Some(node) = model.selected_node().cloned() else {
        ui.label("从左侧树或节点表选择一个变量");
        return;
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(egui::RichText::new("NODE INFO").small().strong());
        info(ui, "NodeId", &node.node_id);
        info(ui, "Name", &node.display_name);
        info(ui, "Parent", &node.parent_id);
        info(ui, "DataType", &node.data_type.to_string());
        ui.horizontal(|ui| {
            ui.label("Writable:");
            let mut w = node.writable;
            if ui.checkbox(&mut w, "").changed() {
                backend.send(UiCommand::UpdateNode {
                    node_id: node.node_id.clone(),
                    display_name: None,
                    data_type: None,
                    writable: Some(w),
                    simulation: None,
                });
            }
        });
        ui.separator();

        ui.label(egui::RichText::new("CURRENT VALUE").small().strong());
        let current = model
            .current_values
            .get(&node.node_id)
            .cloned()
            .or_else(|| node.current_value.clone())
            .unwrap_or_else(|| "—".to_string());
        ui.label(egui::RichText::new(current).size(20.0).monospace());
        ui.separator();

        ui.label(egui::RichText::new("SIMULATION").small().strong());
        let mut sim = node.simulation.clone();
        let changed = edit_simulation(ui, &mut sim);
        if changed {
            backend.send(UiCommand::UpdateNode {
                node_id: node.node_id.clone(),
                display_name: None,
                data_type: None,
                writable: None,
                simulation: Some(sim),
            });
        }
    });
}

fn info(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{label}:")).color(egui::Color32::GRAY));
        ui.label(value);
    });
}

fn edit_simulation(ui: &mut egui::Ui, sim: &mut SimulationMode) -> bool {
    let mut changed = false;
    match sim {
        SimulationMode::Static { value } => {
            ui.label("Static");
            if ui.text_edit_singleline(value).lost_focus() {
                changed = true;
            }
        }
        SimulationMode::Random {
            min,
            max,
            interval_ms,
        } => {
            ui.label("Random");
            egui::Grid::new("random_grid").num_columns(2).show(ui, |ui| {
                ui.label("Min");
                if ui.add(egui::DragValue::new(min)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Max");
                if ui.add(egui::DragValue::new(max)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Interval (ms)");
                if ui
                    .add(egui::DragValue::new(interval_ms).range(50..=3_600_000))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
        }
        SimulationMode::Sine {
            amplitude,
            offset,
            period_ms,
            interval_ms,
        } => {
            ui.label("Sine");
            egui::Grid::new("sine_grid").num_columns(2).show(ui, |ui| {
                ui.label("Amplitude");
                if ui.add(egui::DragValue::new(amplitude)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Offset");
                if ui.add(egui::DragValue::new(offset)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Period (ms)");
                if ui
                    .add(egui::DragValue::new(period_ms).range(50..=86_400_000))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
                ui.label("Interval (ms)");
                if ui
                    .add(egui::DragValue::new(interval_ms).range(50..=3_600_000))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
        }
        SimulationMode::Linear {
            start,
            step,
            min,
            max,
            mode,
            interval_ms,
        } => {
            ui.label("Linear");
            egui::Grid::new("linear_grid").num_columns(2).show(ui, |ui| {
                ui.label("Start");
                if ui.add(egui::DragValue::new(start)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Step");
                if ui.add(egui::DragValue::new(step)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Min");
                if ui.add(egui::DragValue::new(min)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Max");
                if ui.add(egui::DragValue::new(max)).changed() {
                    changed = true;
                }
                ui.end_row();
                ui.label("Mode");
                let mut bounce = matches!(mode, LinearMode::Bounce);
                if ui.checkbox(&mut bounce, "Bounce (else Repeat)").changed() {
                    *mode = if bounce {
                        LinearMode::Bounce
                    } else {
                        LinearMode::Repeat
                    };
                    changed = true;
                }
                ui.end_row();
                ui.label("Interval (ms)");
                if ui
                    .add(egui::DragValue::new(interval_ms).range(50..=3_600_000))
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
        }
        SimulationMode::Script {
            expression,
            interval_ms,
        } => {
            ui.label("Script (evalexpr)");
            ui.add(
                egui::TextEdit::multiline(expression)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            ui.horizontal(|ui| {
                ui.label("Interval (ms)");
                if ui
                    .add(egui::DragValue::new(interval_ms).range(50..=3_600_000))
                    .changed()
                {
                    changed = true;
                }
            });
            if ui.button("应用").clicked() {
                changed = true;
            }
        }
    }
    changed
}
