pub mod browse_panel;
pub mod connection_tree;
pub mod data_table;
pub mod history_tab;
pub mod log_panel;
pub mod toolbar;
pub mod value_panel;

pub fn quality_color(q: &str) -> egui::Color32 {
    use opcuaegui_shared::theme;
    if q.is_empty() {
        theme::STATUS_IDLE()
    } else if q.starts_with("Good") {
        theme::STATUS_OK()
    } else if q.starts_with("Bad") || q.contains("Error") {
        theme::STATUS_BAD()
    } else if q.starts_with("Uncertain") {
        theme::STATUS_WARN()
    } else {
        theme::TEXT_MUTED()
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
