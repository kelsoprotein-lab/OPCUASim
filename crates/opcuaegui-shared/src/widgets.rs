//! Reusable presentational widgets shared by the master and server UIs.

use egui::{Align2, Color32, Context, CornerRadius, Frame, Margin, Response, RichText, Stroke, Ui, Vec2};

use crate::theme;

/// Small uppercase muted heading used to group fields inside a panel.
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(theme::TEXT_MUTED()),
    );
    ui.add_space(2.0);
}

/// A `label: value` pair laid out horizontally.
pub fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{label}:"))
                .small()
                .color(theme::TEXT_MUTED()),
        );
        ui.label(RichText::new(value).color(theme::TEXT_PRIMARY()));
    });
}

/// A coloured pill used for connection / server state.
pub fn status_chip(ui: &mut Ui, color: Color32, icon: &str, text: &str) -> Response {
    Frame::default()
        .fill(color.linear_multiply(0.18))
        .stroke(Stroke::new(1.0, color.linear_multiply(0.55)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new(icon).color(color).strong().small());
                ui.label(RichText::new(text).color(color).strong().small());
            });
        })
        .response
}

/// Empty / placeholder block with a centered glyph + title and optional hint.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, hint: Option<&str>) {
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        ui.label(RichText::new(icon).size(34.0).color(theme::TEXT_FAINT()));
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .color(theme::TEXT_MUTED())
                .strong(),
        );
        if let Some(h) = hint {
            ui.add_space(2.0);
            ui.label(RichText::new(h).small().color(theme::TEXT_FAINT()));
        }
        ui.add_space(28.0);
    });
}

/// Renders a single toast card. Caller is responsible for placing it in an
/// `egui::Area` and managing lifetimes.
pub fn toast_card(ui: &mut Ui, color: Color32, message: &str) {
    Frame::default()
        .fill(theme::BG_RAISED())
        .stroke(Stroke::new(1.0, color.linear_multiply(0.6)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(color).strong());
                ui.label(RichText::new(message).color(theme::TEXT_PRIMARY()));
            });
        });
}

/// Renders a list of toasts anchored to bottom-right. Each item is `(color, message)`.
/// Caller decides retention; this helper only paints what's given.
pub fn render_toasts<'a, I, S>(ctx: &Context, items: I, anchor_offset: Vec2)
where
    I: IntoIterator<Item = (Color32, S)>,
    S: AsRef<str> + 'a,
{
    let items: Vec<(Color32, String)> = items
        .into_iter()
        .map(|(c, m)| (c, m.as_ref().to_string()))
        .collect();
    if items.is_empty() {
        return;
    }
    egui::Area::new("toasts".into())
        .anchor(Align2::RIGHT_BOTTOM, anchor_offset)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                for (color, message) in items {
                    toast_card(ui, color, &message);
                    ui.add_space(4.0);
                }
            });
        });
}

/// Common (icon, color, label) mapping for an OPC UA client connection state.
pub fn connection_state_chip(state: &str) -> (&'static str, Color32, &'static str) {
    match state {
        "Connected" => ("●", theme::STATUS_OK(), "在线"),
        "Connecting" => ("◐", theme::STATUS_WARN(), "连接中"),
        "Disconnected" => ("○", theme::STATUS_BAD(), "离线"),
        _ => ("·", theme::STATUS_IDLE(), "未知"),
    }
}

/// Project file dialog helper: prompt the user for a save path with a
/// pre-filled default name and the standard `opcuaproj/json` filter.
pub fn pick_save_project_path(default_name: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("OPCUA Project", &["opcuaproj", "json"])
        .save_file()
}

/// Project file dialog helper: prompt the user for a project to open.
pub fn pick_open_project_path() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("OPCUA Project", &["opcuaproj", "json"])
        .pick_file()
}

/// Hardcoded root id used by the OPC UA server's address space tree.
pub const OBJECTS_ROOT_ID: &str = "Objects";
