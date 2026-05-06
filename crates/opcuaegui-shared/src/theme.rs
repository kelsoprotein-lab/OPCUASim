//! Shared visual identity for OPCUASim apps.
//!
//! Provides both an "industrial dark" and a softer "industrial light" palette,
//! with a runtime switch persisted in user settings. Widgets read colours via
//! the helper functions (e.g. [`accent`], [`text_primary`]) so that toggling
//! [`set_mode`] re-themes the whole UI on the next paint.

use std::sync::atomic::{AtomicU8, Ordering};

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

const MODE_DARK: u8 = 0;
const MODE_LIGHT: u8 = 1;

static MODE: AtomicU8 = AtomicU8::new(MODE_DARK);

pub fn set_mode(m: ThemeMode) {
    let v = match m {
        ThemeMode::Dark => MODE_DARK,
        ThemeMode::Light => MODE_LIGHT,
    };
    MODE.store(v, Ordering::Relaxed);
}

pub fn current_mode() -> ThemeMode {
    if MODE.load(Ordering::Relaxed) == MODE_LIGHT {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    }
}

// ─── Palette accessors ──────────────────────────────────────────────────────

#[inline]
fn pick(dark: Color32, light: Color32) -> Color32 {
    match current_mode() {
        ThemeMode::Dark => dark,
        ThemeMode::Light => light,
    }
}

pub fn accent() -> Color32 {
    pick(
        Color32::from_rgb(64, 188, 184),
        Color32::from_rgb(32, 134, 130),
    )
}
pub fn accent_dim() -> Color32 {
    pick(
        Color32::from_rgb(38, 124, 122),
        Color32::from_rgb(150, 210, 208),
    )
}
pub fn accent_soft() -> Color32 {
    pick(
        Color32::from_rgb(28, 70, 70),
        Color32::from_rgb(216, 240, 238),
    )
}

pub fn bg_base() -> Color32 {
    pick(
        Color32::from_rgb(20, 22, 28),
        Color32::from_rgb(244, 246, 250),
    )
}
pub fn bg_panel() -> Color32 {
    pick(
        Color32::from_rgb(28, 31, 38),
        Color32::from_rgb(252, 253, 255),
    )
}
pub fn bg_raised() -> Color32 {
    pick(
        Color32::from_rgb(36, 40, 48),
        Color32::from_rgb(255, 255, 255),
    )
}
pub fn bg_hover() -> Color32 {
    pick(
        Color32::from_rgb(48, 54, 64),
        Color32::from_rgb(228, 232, 240),
    )
}
pub fn border() -> Color32 {
    pick(
        Color32::from_rgb(52, 58, 70),
        Color32::from_rgb(208, 214, 224),
    )
}

pub fn text_primary() -> Color32 {
    pick(
        Color32::from_rgb(228, 232, 240),
        Color32::from_rgb(28, 32, 40),
    )
}
pub fn text_muted() -> Color32 {
    pick(
        Color32::from_rgb(148, 156, 170),
        Color32::from_rgb(96, 104, 118),
    )
}
pub fn text_faint() -> Color32 {
    pick(
        Color32::from_rgb(100, 108, 122),
        Color32::from_rgb(150, 158, 172),
    )
}

pub fn status_ok() -> Color32 {
    pick(
        Color32::from_rgb(120, 200, 130),
        Color32::from_rgb(36, 144, 60),
    )
}
pub fn status_warn() -> Color32 {
    pick(
        Color32::from_rgb(240, 200, 90),
        Color32::from_rgb(168, 124, 24),
    )
}
pub fn status_bad() -> Color32 {
    pick(
        Color32::from_rgb(228, 110, 110),
        Color32::from_rgb(196, 50, 50),
    )
}
pub fn status_info() -> Color32 {
    pick(
        Color32::from_rgb(120, 180, 230),
        Color32::from_rgb(40, 110, 180),
    )
}
pub fn status_idle() -> Color32 {
    pick(
        Color32::from_rgb(140, 148, 160),
        Color32::from_rgb(124, 132, 144),
    )
}

// ─── Backwards-compatible UPPER_CASE shims ──────────────────────────────────
//
// Existing call sites use `theme::ACCENT()` etc. We expose those names as
// re-exported fn pointers via `const fn`-style getters returning `Color32`
// only when the mode is queried. To keep callers simple, we provide *macro*
// constants below that look like consts but are actually functions.

#[allow(non_snake_case)]
pub mod compat {}

// Provide upper-case wrappers so old call sites compile. These are simple
// nullary `pub fn` aliases — call sites use `theme::ACCENT()` syntax via the
// `pub use` re-export at the bottom.
#[allow(non_snake_case)]
pub fn ACCENT() -> Color32 {
    accent()
}
#[allow(non_snake_case)]
pub fn ACCENT_DIM() -> Color32 {
    accent_dim()
}
#[allow(non_snake_case)]
pub fn ACCENT_SOFT() -> Color32 {
    accent_soft()
}
#[allow(non_snake_case)]
pub fn BG_BASE() -> Color32 {
    bg_base()
}
#[allow(non_snake_case)]
pub fn BG_PANEL() -> Color32 {
    bg_panel()
}
#[allow(non_snake_case)]
pub fn BG_RAISED() -> Color32 {
    bg_raised()
}
#[allow(non_snake_case)]
pub fn BG_HOVER() -> Color32 {
    bg_hover()
}
#[allow(non_snake_case)]
pub fn BORDER() -> Color32 {
    border()
}
#[allow(non_snake_case)]
pub fn TEXT_PRIMARY() -> Color32 {
    text_primary()
}
#[allow(non_snake_case)]
pub fn TEXT_MUTED() -> Color32 {
    text_muted()
}
#[allow(non_snake_case)]
pub fn TEXT_FAINT() -> Color32 {
    text_faint()
}
#[allow(non_snake_case)]
pub fn STATUS_OK() -> Color32 {
    status_ok()
}
#[allow(non_snake_case)]
pub fn STATUS_WARN() -> Color32 {
    status_warn()
}
#[allow(non_snake_case)]
pub fn STATUS_BAD() -> Color32 {
    status_bad()
}
#[allow(non_snake_case)]
pub fn STATUS_INFO() -> Color32 {
    status_info()
}
#[allow(non_snake_case)]
pub fn STATUS_IDLE() -> Color32 {
    status_idle()
}

// ─── Apply ──────────────────────────────────────────────────────────────────

pub fn apply(ctx: &egui::Context) {
    let mut visuals = match current_mode() {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };
    visuals.panel_fill = bg_panel();
    visuals.window_fill = bg_raised();
    visuals.window_stroke = Stroke::new(1.0, border());
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.faint_bg_color = match current_mode() {
        ThemeMode::Dark => Color32::from_rgb(32, 35, 42),
        ThemeMode::Light => Color32::from_rgb(238, 242, 248),
    };
    visuals.extreme_bg_color = bg_base();
    visuals.code_bg_color = bg_base();
    visuals.hyperlink_color = accent();

    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border());
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_primary());
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    let inactive_bg = match current_mode() {
        ThemeMode::Dark => Color32::from_rgb(44, 49, 58),
        ThemeMode::Light => Color32::from_rgb(248, 250, 253),
    };
    let inactive_weak = match current_mode() {
        ThemeMode::Dark => Color32::from_rgb(38, 42, 50),
        ThemeMode::Light => Color32::from_rgb(238, 242, 248),
    };
    let inactive_stroke = match current_mode() {
        ThemeMode::Dark => Color32::from_rgb(60, 66, 78),
        ThemeMode::Light => Color32::from_rgb(196, 204, 216),
    };
    visuals.widgets.inactive.bg_fill = inactive_bg;
    visuals.widgets.inactive.weak_bg_fill = inactive_weak;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, inactive_stroke);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary());
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    let hover_weak = match current_mode() {
        ThemeMode::Dark => Color32::from_rgb(52, 58, 70),
        ThemeMode::Light => Color32::from_rgb(220, 226, 236),
    };
    let hover_text = match current_mode() {
        ThemeMode::Dark => Color32::WHITE,
        ThemeMode::Light => Color32::from_rgb(8, 16, 28),
    };
    visuals.widgets.hovered.bg_fill = bg_hover();
    visuals.widgets.hovered.weak_bg_fill = hover_weak;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent());
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, hover_text);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    let active_text = match current_mode() {
        ThemeMode::Dark => Color32::WHITE,
        ThemeMode::Light => Color32::from_rgb(8, 16, 28),
    };
    visuals.widgets.active.bg_fill = accent_dim();
    visuals.widgets.active.weak_bg_fill = accent_soft();
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent());
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, active_text);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.widgets.open.bg_fill = accent_soft();
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent());
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, text_primary());

    visuals.selection.bg_fill = accent_soft();
    visuals.selection.stroke = Stroke::new(1.0, accent());

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style: &mut Style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.indent = 16.0;
        style.spacing.scroll.floating = true;
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.scroll.floating_allocated_width = 6.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.handle_min_length = 16.0;
    });
}
