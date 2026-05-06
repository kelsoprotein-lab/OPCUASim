use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::{empty_state, info_row, section_label};
use opcuasim_core::server::models::{LinearMode, SimulationMode};

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    section_label(ui, "节点属性");
    ui.separator();

    let Some(node) = model.selected_node().cloned() else {
        empty_state(
            ui,
            "👈",
            "未选择节点",
            Some("从左侧地址空间或节点表中选择一个变量"),
        );
        return;
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        section_label(ui, "Node Info");
        info_row(ui, "NodeId", &node.node_id);
        info_row(ui, "Name", &node.display_name);
        info_row(ui, "Parent", &node.parent_id);
        info_row(ui, "DataType", &node.data_type.to_string());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Writable:")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
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
        ui.add_space(6.0);

        section_label(ui, "Current Value");
        let current = model
            .current_values
            .get(&node.node_id)
            .cloned()
            .or_else(|| node.current_value.clone())
            .unwrap_or_else(|| "—".to_string());
        ui.label(
            egui::RichText::new(current)
                .size(22.0)
                .monospace()
                .color(theme::TEXT_PRIMARY()),
        );
        ui.add_space(6.0);

        section_label(ui, "Simulation");
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

/// Returns true when the user finished editing a value (changed AND lost focus,
/// or pressed Apply for Static/Script). This avoids streaming an UpdateNode
/// command on every drag pixel.
fn edit_simulation(ui: &mut egui::Ui, sim: &mut SimulationMode) -> bool {
    let mut commit = false;
    match sim {
        SimulationMode::Static { value } => {
            ui.label(
                egui::RichText::new("Static")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
            let resp = ui.text_edit_singleline(value);
            if resp.lost_focus() && resp.changed() {
                commit = true;
            }
        }
        SimulationMode::Random {
            min,
            max,
            interval_ms,
        } => {
            ui.label(
                egui::RichText::new("Random")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
            egui::Grid::new("random_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Min");
                    commit |= drag_commit(ui, egui::DragValue::new(min));
                    ui.end_row();
                    ui.label("Max");
                    commit |= drag_commit(ui, egui::DragValue::new(max));
                    ui.end_row();
                    ui.label("Interval (ms)");
                    commit |= drag_commit(
                        ui,
                        egui::DragValue::new(interval_ms).range(50..=3_600_000),
                    );
                    ui.end_row();
                });
        }
        SimulationMode::Sine {
            amplitude,
            offset,
            period_ms,
            interval_ms,
        } => {
            ui.label(
                egui::RichText::new("Sine")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
            egui::Grid::new("sine_grid").num_columns(2).show(ui, |ui| {
                ui.label("Amplitude");
                commit |= drag_commit(ui, egui::DragValue::new(amplitude));
                ui.end_row();
                ui.label("Offset");
                commit |= drag_commit(ui, egui::DragValue::new(offset));
                ui.end_row();
                ui.label("Period (ms)");
                commit |= drag_commit(
                    ui,
                    egui::DragValue::new(period_ms).range(50..=86_400_000),
                );
                ui.end_row();
                ui.label("Interval (ms)");
                commit |= drag_commit(
                    ui,
                    egui::DragValue::new(interval_ms).range(50..=3_600_000),
                );
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
            ui.label(
                egui::RichText::new("Linear")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
            egui::Grid::new("linear_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Start");
                    commit |= drag_commit(ui, egui::DragValue::new(start));
                    ui.end_row();
                    ui.label("Step");
                    commit |= drag_commit(ui, egui::DragValue::new(step));
                    ui.end_row();
                    ui.label("Min");
                    commit |= drag_commit(ui, egui::DragValue::new(min));
                    ui.end_row();
                    ui.label("Max");
                    commit |= drag_commit(ui, egui::DragValue::new(max));
                    ui.end_row();
                    ui.label("Mode");
                    let mut bounce = matches!(mode, LinearMode::Bounce);
                    if ui
                        .checkbox(&mut bounce, "Bounce (else Repeat)")
                        .changed()
                    {
                        *mode = if bounce {
                            LinearMode::Bounce
                        } else {
                            LinearMode::Repeat
                        };
                        commit = true;
                    }
                    ui.end_row();
                    ui.label("Interval (ms)");
                    commit |= drag_commit(
                        ui,
                        egui::DragValue::new(interval_ms).range(50..=3_600_000),
                    );
                    ui.end_row();
                });
        }
        SimulationMode::Script {
            expression,
            interval_ms,
        } => {
            ui.label(
                egui::RichText::new("Script (evalexpr)")
                    .small()
                    .color(theme::TEXT_MUTED()),
            );
            ui.add(
                egui::TextEdit::multiline(expression)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            ui.horizontal(|ui| {
                ui.label("Interval (ms)");
                commit |= drag_commit(
                    ui,
                    egui::DragValue::new(interval_ms).range(50..=3_600_000),
                );
            });
            if ui.button("应用").clicked() {
                commit = true;
            }
        }
    }
    commit
}

/// Wraps a DragValue so we only commit on `lost_focus + changed`. This makes
/// dragging a value smooth in the UI without spamming UpdateNode per pixel.
fn drag_commit(ui: &mut egui::Ui, drag: egui::DragValue<'_>) -> bool {
    let resp = ui.add(drag);
    resp.lost_focus() && resp.changed()
}
