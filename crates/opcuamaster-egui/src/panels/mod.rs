pub mod browse_panel;
pub mod connection_tree;
pub mod data_table;
pub mod history_tab;
pub mod log_panel;
pub mod toolbar;
pub mod value_panel;

pub fn quality_color(q: &str) -> egui::Color32 {
    if q.is_empty() {
        egui::Color32::GRAY
    } else if q.starts_with("Good") {
        egui::Color32::from_rgb(120, 200, 120)
    } else if q.starts_with("Bad") || q.contains("Error") {
        egui::Color32::from_rgb(220, 100, 100)
    } else if q.starts_with("Uncertain") {
        egui::Color32::from_rgb(220, 200, 100)
    } else {
        egui::Color32::LIGHT_GRAY
    }
}

pub fn format_hms(ts: Option<&str>) -> String {
    let Some(raw) = ts else {
        return String::from("—");
    };
    if raw.is_empty() {
        return String::from("—");
    }
    if raw.len() >= 19 {
        raw[11..19].to_string()
    } else {
        raw.to_string()
    }
}
