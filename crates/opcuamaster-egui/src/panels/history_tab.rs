use crate::events::UiCommand;
use crate::model::HistoryTabState;
use crate::runtime::BackendHandle;

pub struct TabActions {
    pub close: bool,
    pub refresh: bool,
}

pub fn show(ui: &mut egui::Ui, state: &mut HistoryTabState) -> TabActions {
    let mut actions = TabActions {
        close: false,
        refresh: false,
    };

    ui.horizontal(|ui| {
        ui.label(format!("📈 {}", state.display_name));
        ui.label(format!("({})", state.node_id));
        ui.separator();
        if ui.button("✕ 关闭").clicked() {
            actions.close = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("快捷:");
        for (label, secs) in [
            ("1m", 60i64),
            ("5m", 300),
            ("30m", 1800),
            ("1h", 3600),
            ("6h", 21600),
            ("24h", 86400),
        ] {
            if ui.small_button(label).clicked() {
                let now = chrono::Utc::now();
                let start = now - chrono::Duration::seconds(secs);
                state.start_iso = start.to_rfc3339();
                state.end_iso = now.to_rfc3339();
                actions.refresh = true;
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("起:");
        ui.add(egui::TextEdit::singleline(&mut state.start_iso).desired_width(220.0));
        ui.label("止:");
        ui.add(egui::TextEdit::singleline(&mut state.end_iso).desired_width(220.0));
        ui.label("最多:");
        ui.add(egui::DragValue::new(&mut state.max_values).range(10..=50_000));
        let busy = state.pending_req.is_some();
        let resp = ui.add_enabled(
            !busy,
            egui::Button::new(if busy { "加载中…" } else { "🔄 刷新" }),
        );
        if resp.clicked() {
            actions.refresh = true;
        }
    });

    if let Some(err) = &state.error {
        ui.colored_label(egui::Color32::LIGHT_RED, err);
    }

    ui.separator();

    let plot_points: Vec<[f64; 2]> = state
        .points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.numeric.map(|n| [i as f64, n]))
        .collect();

    egui_plot::Plot::new(format!("history_plot_{}", state.node_id))
        .height(220.0)
        .show(ui, |plot_ui| {
            if !plot_points.is_empty() {
                plot_ui.line(egui_plot::Line::new(
                    state.display_name.clone(),
                    egui_plot::PlotPoints::from(plot_points),
                ));
            }
        });

    ui.separator();

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
                    ui.label(&p.value);
                });
                row.col(|ui| {
                    ui.label(&p.status);
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
