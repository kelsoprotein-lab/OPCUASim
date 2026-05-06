use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::status_chip;

use crate::model::AppModel;

pub fn show(ui: &mut egui::Ui, model: &AppModel) {
    ui.horizontal(|ui| {
        let (icon, color, label) = match model.status.state.as_str() {
            "Running" => ("●", theme::STATUS_OK(), "Running"),
            "Starting" => ("◐", theme::STATUS_WARN(), "Starting"),
            "Stopping" => ("◑", theme::STATUS_WARN(), "Stopping"),
            "Stopped" => ("○", theme::STATUS_BAD(), "Stopped"),
            other => ("·", theme::STATUS_IDLE(), other),
        };
        status_chip(ui, color, icon, label);
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "📁 {} 文件夹 · 📊 {} 节点",
                model.status.folder_count, model.status.node_count
            ))
            .small()
            .color(theme::TEXT_MUTED()),
        );
        ui.separator();
        ui.label(
            egui::RichText::new("Endpoint")
                .small()
                .color(theme::TEXT_FAINT()),
        );
        ui.label(
            egui::RichText::new(&model.status.endpoint_url)
                .small()
                .monospace()
                .color(theme::TEXT_MUTED()),
        );
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new(format!("seq #{}", model.last_sim_seq))
                        .small()
                        .color(theme::TEXT_FAINT()),
                );
            },
        );
    });
}
