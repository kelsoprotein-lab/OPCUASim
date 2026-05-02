//! Reusable presentational widgets shared by the master and server UIs.

use egui::{Color32, CornerRadius, Frame, Margin, Response, RichText, Stroke, Ui};

use crate::theme;

/// Small uppercase muted heading used to group fields inside a panel.
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(theme::TEXT_MUTED),
    );
    ui.add_space(2.0);
}

/// A `label: value` pair laid out horizontally.
pub fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{label}:"))
                .small()
                .color(theme::TEXT_MUTED),
        );
        ui.label(RichText::new(value).color(theme::TEXT_PRIMARY));
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
        ui.label(RichText::new(icon).size(34.0).color(theme::TEXT_FAINT));
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .color(theme::TEXT_MUTED)
                .strong(),
        );
        if let Some(h) = hint {
            ui.add_space(2.0);
            ui.label(RichText::new(h).small().color(theme::TEXT_FAINT));
        }
        ui.add_space(28.0);
    });
}

/// Renders a single toast card. Caller is responsible for placing it in an
/// `egui::Area` and managing lifetimes.
pub fn toast_card(ui: &mut Ui, color: Color32, message: &str) {
    Frame::default()
        .fill(theme::BG_RAISED)
        .stroke(Stroke::new(1.0, color.linear_multiply(0.6)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(color).strong());
                ui.label(RichText::new(message).color(theme::TEXT_PRIMARY));
            });
        });
}
