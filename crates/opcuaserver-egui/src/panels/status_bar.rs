use crate::model::AppModel;

pub fn show(ui: &mut egui::Ui, model: &AppModel) {
    ui.horizontal(|ui| {
        let (icon, color) = match model.status.state.as_str() {
            "Running" => ("●", egui::Color32::from_rgb(80, 200, 120)),
            "Starting" => ("◐", egui::Color32::from_rgb(240, 200, 80)),
            "Stopping" => ("◑", egui::Color32::from_rgb(240, 160, 80)),
            "Stopped" => ("○", egui::Color32::from_rgb(180, 80, 80)),
            _ => ("·", egui::Color32::GRAY),
        };
        ui.colored_label(color, format!("{icon} {}", model.status.state));
        ui.separator();
        ui.label(format!(
            "文件夹 {} · 节点 {}",
            model.status.folder_count, model.status.node_count
        ));
        ui.separator();
        ui.label(format!("Endpoint: {}", model.status.endpoint_url));
    });
}
