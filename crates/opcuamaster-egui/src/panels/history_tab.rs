use chrono::{DateTime, Local, TimeZone};

use crate::events::UiCommand;
use crate::model::HistoryTabState;
use crate::runtime::BackendHandle;

pub struct TabActions {
    pub close: bool,
    pub refresh: bool,
}

const QUICK_RANGES: &[(&str, i64)] = &[
    ("1m", 60),
    ("5m", 300),
    ("30m", 1800),
    ("1h", 3600),
    ("6h", 21600),
    ("24h", 86400),
];

pub fn show(ui: &mut egui::Ui, state: &mut HistoryTabState) -> TabActions {
    let mut actions = TabActions {
        close: false,
        refresh: false,
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("📈 {}", state.display_name))
                .strong()
                .color(opcuaegui_shared::theme::TEXT_PRIMARY),
        );
        ui.label(
            egui::RichText::new(&state.node_id)
                .small()
                .color(opcuaegui_shared::theme::TEXT_MUTED),
        );
    });

    let active_secs = active_quick_range(state);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("快捷")
                .small()
                .color(opcuaegui_shared::theme::TEXT_MUTED),
        );
        for (label, secs) in QUICK_RANGES {
            let is_active = active_secs == Some(*secs);
            let resp = ui.add(egui::Button::selectable(is_active, *label));
            if resp.clicked() {
                let now = chrono::Utc::now();
                let start = now - chrono::Duration::seconds(*secs);
                state.start_iso = start.to_rfc3339();
                state.end_iso = now.to_rfc3339();
                actions.refresh = true;
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("起");
        ui.add(egui::TextEdit::singleline(&mut state.start_iso).desired_width(220.0));
        ui.label("止");
        ui.add(egui::TextEdit::singleline(&mut state.end_iso).desired_width(220.0));
        ui.label("最多");
        ui.add(egui::DragValue::new(&mut state.max_values).range(10..=50_000));
        let busy = state.pending_req.is_some();
        let resp = ui.add_enabled(
            !busy,
            egui::Button::new(if busy { "加载中…" } else { "🔄 刷新" }),
        );
        if resp.clicked() {
            actions.refresh = true;
        }
        if !state.points.is_empty() {
            ui.separator();
            ui.label(
                egui::RichText::new(format!("{} 个点", state.points.len()))
                    .small()
                    .color(opcuaegui_shared::theme::TEXT_MUTED),
            );
        }
    });

    if let Some(err) = &state.error {
        opcuaegui_shared::widgets::toast_card(
            ui,
            opcuaegui_shared::theme::STATUS_BAD,
            err,
        );
    }

    ui.separator();

    let plot_points: Vec<[f64; 2]> = state
        .points
        .iter()
        .filter_map(|p| {
            let ts = parse_to_unix_secs(&p.source_timestamp)?;
            let v = p.numeric?;
            Some([ts, v])
        })
        .collect();

    egui_plot::Plot::new(format!("history_plot_{}", state.node_id))
        .height(220.0)
        .x_axis_formatter(|gm, _| format_time_axis(gm.value))
        .label_formatter(|name, value| {
            let ts = format_time_axis(value.x);
            if name.is_empty() {
                format!("{ts}\n{:.4}", value.y)
            } else {
                format!("{name}\n{ts}\n{:.4}", value.y)
            }
        })
        .show(ui, |plot_ui| {
            if plot_points.is_empty() {
                return;
            }
            plot_ui.line(
                egui_plot::Line::new(
                    state.display_name.clone(),
                    egui_plot::PlotPoints::from(plot_points),
                )
                .color(opcuaegui_shared::theme::ACCENT),
            );
        });

    ui.separator();

    if state.points.is_empty() && state.pending_req.is_none() {
        opcuaegui_shared::widgets::empty_state(
            ui,
            "📊",
            "暂无历史数据",
            Some("调整时间范围后点击刷新"),
        );
        return actions;
    }

    egui_extras::TableBuilder::new(ui)
        .id_salt(format!("history_table_{}", state.node_id))
        .striped(true)
        .column(egui_extras::Column::auto().at_least(220.0))
        .column(egui_extras::Column::auto().at_least(120.0))
        .column(egui_extras::Column::remainder().at_least(80.0))
        .header(20.0, |mut h| {
            h.col(|ui| {
                ui.strong("Source Timestamp");
            });
            h.col(|ui| {
                ui.strong("Value");
            });
            h.col(|ui| {
                ui.strong("Status");
            });
        })
        .body(|body| {
            let total = state.points.len();
            body.rows(18.0, total, |mut row| {
                let i = row.index();
                let p = &state.points[i];
                row.col(|ui| {
                    ui.label(&p.source_timestamp);
                });
                row.col(|ui| {
                    ui.monospace(&p.value);
                });
                row.col(|ui| {
                    let color = super::quality_color(&p.status);
                    ui.colored_label(color, &p.status);
                });
            });
        });

    actions
}

pub fn dispatch_refresh(
    state: &mut HistoryTabState,
    backend: &BackendHandle,
    next_req_id: &mut u64,
) {
    *next_req_id = next_req_id.wrapping_add(1);
    let req_id = *next_req_id;
    state.pending_req = Some(req_id);
    state.error = None;
    backend.send(UiCommand::ReadHistory {
        conn_id: state.conn_id.clone(),
        node_id: state.node_id.clone(),
        start_iso: state.start_iso.clone(),
        end_iso: state.end_iso.clone(),
        max_values: state.max_values,
        req_id,
    });
}

fn parse_to_unix_secs(rfc3339: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|dt| dt.timestamp_millis() as f64 / 1000.0)
}

fn format_time_axis(unix_secs: f64) -> String {
    let millis = (unix_secs * 1000.0) as i64;
    let Some(dt) = DateTime::from_timestamp_millis(millis) else {
        return String::new();
    };
    let local: DateTime<Local> = Local.from_utc_datetime(&dt.naive_utc());
    local.format("%H:%M:%S").to_string()
}

/// Round-trip the current start/end through a difference and match it to one of
/// the quick ranges so the active button can be highlighted.
fn active_quick_range(state: &HistoryTabState) -> Option<i64> {
    let start = DateTime::parse_from_rfc3339(&state.start_iso).ok()?;
    let end = DateTime::parse_from_rfc3339(&state.end_iso).ok()?;
    let diff = end.timestamp() - start.timestamp();
    QUICK_RANGES
        .iter()
        .map(|(_, s)| *s)
        .find(|s| (diff - s).abs() <= 2)
}
