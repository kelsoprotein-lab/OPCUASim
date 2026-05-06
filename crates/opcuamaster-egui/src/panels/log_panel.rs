use chrono::{DateTime, Local, TimeZone};
use egui_extras::{Column, TableBuilder};
use opcuaegui_shared::theme;

use crate::events::UiCommand;
use crate::model::{AppModel, LogDirectionFilter};
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    let Some(conn_id) = model.selected_conn.clone() else {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("通信日志 (未选择连接)")
                    .color(theme::TEXT_MUTED()),
            );
        });
        return;
    };

    let total = model
        .logs
        .per_conn
        .get(&conn_id)
        .map(|p| p.entries.len())
        .unwrap_or(0);
    let buffered = model
        .logs
        .per_conn
        .get(&conn_id)
        .map(|p| p.paused_buf.len())
        .unwrap_or(0);

    ui.horizontal(|ui| {
        let icon = if model.logs.expanded { "▼" } else { "▲" };
        if ui.button(format!("{icon} 通信日志 ({total})")).clicked() {
            model.logs.expanded = !model.logs.expanded;
        }
        ui.separator();
        ui.label("方向:");
        let prev_filter = model.logs.filter;
        egui::ComboBox::from_id_salt("log_dir")
            .selected_text(match model.logs.filter {
                LogDirectionFilter::All => "All",
                LogDirectionFilter::Request => "Request",
                LogDirectionFilter::Response => "Response",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut model.logs.filter, LogDirectionFilter::All, "All");
                ui.selectable_value(
                    &mut model.logs.filter,
                    LogDirectionFilter::Request,
                    "Request",
                );
                ui.selectable_value(
                    &mut model.logs.filter,
                    LogDirectionFilter::Response,
                    "Response",
                );
            });
        if prev_filter != model.logs.filter {
            model.logs.filter_dirty = true;
        }
        ui.separator();
        ui.label("搜索:");
        let search_resp = ui.add(
            egui::TextEdit::singleline(&mut model.logs.search)
                .desired_width(200.0)
                .hint_text("Service / Detail"),
        );
        if search_resp.changed() {
            model.logs.filter_dirty = true;
        }
        ui.separator();

        // Pause / Resume
        let pause_label = if model.logs.paused {
            format!("▶ 恢复 ({buffered})")
        } else {
            "⏸ 暂停".to_string()
        };
        if ui
            .button(pause_label)
            .on_hover_text(if model.logs.paused {
                "恢复实时日志，缓冲条目将被合并"
            } else {
                "暂停实时追加，新日志先入缓冲区"
            })
            .clicked()
        {
            if model.logs.paused {
                if let Some(per) = model.logs.per_conn.get_mut(&conn_id) {
                    per.flush_paused();
                }
                model.logs.paused = false;
                model.logs.filter_dirty = true;
            } else {
                model.logs.paused = true;
            }
        }

        ui.checkbox(&mut model.logs.auto_scroll, "自动滚动")
            .on_hover_text("新日志到达时滚动到底部");

        ui.separator();
        if ui.button("清空").clicked() {
            backend.send(UiCommand::ClearCommLogs(conn_id.clone()));
        }
        if ui.button("导出 CSV").clicked() {
            let default_name = format!("opcua_logs_{conn_id}.csv");
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter("CSV", &["csv"])
                .save_file()
            {
                backend.send(UiCommand::ExportCommLogs {
                    conn_id: conn_id.clone(),
                    path,
                });
            }
        }
    });

    if !model.logs.expanded {
        return;
    }

    ui.separator();

    model.logs.ensure_filter(&conn_id);
    let filtered_len = model.logs.filtered_cache.len();
    let auto_scroll = model.logs.auto_scroll && !model.logs.paused;

    let mut builder = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(120.0).at_least(100.0))
        .column(Column::initial(90.0).at_least(70.0))
        .column(Column::initial(140.0).at_least(80.0))
        .column(Column::remainder().at_least(200.0))
        .column(Column::initial(90.0).at_least(60.0));
    if auto_scroll && filtered_len > 0 {
        builder = builder.scroll_to_row(filtered_len - 1, Some(egui::Align::BOTTOM));
    }
    builder
        .header(20.0, |mut header| {
            for label in ["Timestamp", "Direction", "Service", "Detail", "Status"] {
                header.col(|ui| {
                    ui.strong(label);
                });
            }
        })
        .body(|body| {
            let cache = &model.logs.filtered_cache;
            let entries = model
                .logs
                .per_conn
                .get(&conn_id)
                .map(|p| &p.entries[..])
                .unwrap_or(&[]);
            body.rows(18.0, cache.len(), |mut row| {
                let Some(idx) = cache.get(row.index()) else {
                    return;
                };
                let Some(entry) = entries.get(*idx) else {
                    return;
                };
                let ts = format_local_ts(entry.timestamp_ms);
                let dir_color = match entry.direction.as_str() {
                    "Request" => theme::STATUS_OK(),
                    "Response" => theme::STATUS_INFO(),
                    _ => theme::STATUS_IDLE(),
                };
                row.col(|ui| {
                    ui.label(
                        egui::RichText::new(ts)
                            .monospace()
                            .small()
                            .color(theme::TEXT_MUTED()),
                    );
                });
                row.col(|ui| {
                    let glyph = if entry.direction == "Request" { "→" } else { "←" };
                    ui.colored_label(dir_color, format!("{glyph} {}", entry.direction));
                });
                row.col(|ui| {
                    ui.label(&entry.service);
                });
                row.col(|ui| {
                    ui.label(&entry.detail);
                });
                row.col(|ui| {
                    ui.label(entry.status.as_deref().unwrap_or(""));
                });
            });
        });
}

fn format_local_ts(ms: i64) -> String {
    let Some(dt) = DateTime::from_timestamp_millis(ms) else {
        return String::from("—");
    };
    let local: DateTime<Local> = Local.from_utc_datetime(&dt.naive_utc());
    local.format("%H:%M:%S%.3f").to_string()
}
