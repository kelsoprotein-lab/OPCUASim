//! Shared visual identity for OPCUASim apps.
//!
//! A toned-down "industrial dark" theme: warm-neutral panels, a teal accent,
//! and explicit status colours so badges, toasts and quality cells look
//! coherent across master + server apps.

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};

// ─── Palette ────────────────────────────────────────────────────────────────

pub const ACCENT: Color32 = Color32::from_rgb(64, 188, 184);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(38, 124, 122);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(28, 70, 70);

pub const BG_BASE: Color32 = Color32::from_rgb(20, 22, 28);
pub const BG_PANEL: Color32 = Color32::from_rgb(28, 31, 38);
pub const BG_RAISED: Color32 = Color32::from_rgb(36, 40, 48);
pub const BG_HOVER: Color32 = Color32::from_rgb(48, 54, 64);
pub const BORDER: Color32 = Color32::from_rgb(52, 58, 70);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(228, 232, 240);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 156, 170);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(100, 108, 122);

pub const STATUS_OK: Color32 = Color32::from_rgb(120, 200, 130);
pub const STATUS_WARN: Color32 = Color32::from_rgb(240, 200, 90);
pub const STATUS_BAD: Color32 = Color32::from_rgb(228, 110, 110);
pub const STATUS_INFO: Color32 = Color32::from_rgb(120, 180, 230);
pub const STATUS_IDLE: Color32 = Color32::from_rgb(140, 148, 160);

// ─── Apply ──────────────────────────────────────────────────────────────────

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_RAISED;
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.faint_bg_color = Color32::from_rgb(32, 35, 42);
    visuals.extreme_bg_color = BG_BASE;
    visuals.code_bg_color = BG_BASE;
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(44, 49, 58);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(38, 42, 50);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 66, 78));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 58, 70);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = ACCENT_SOFT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.widgets.open.bg_fill = ACCENT_SOFT;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    visuals.selection.bg_fill = ACCENT_SOFT;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style: &mut Style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.indent = 16.0;
        style.spacing.scroll.bar_width = 8.0;
    });
}
