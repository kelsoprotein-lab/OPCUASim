# HistoryRead Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** master 端支持读取 OPC UA Variable 节点的历史值,以折线图 + 表格双视图展示;入口在 BrowsePanel 与 DataTable 节点的右键菜单"查看历史"。

**Architecture:**
- core 端新增 `history.rs`:`history_read_raw(session, node_id, start, end, max_values, return_bounds) -> Vec<HistoryDataPoint>`,内部用 `Session::history_read` + `ReadRawModifiedDetails`,处理 `continuationPoint` 直至取尽或达到 max_values
- master-egui 增加中央面板 Tab 切换:`enum CentralPanelTab { DataTable, History(usize) }`,Vec<HistoryTabState> 持有每个打开的历史视图
- HistoryTab UI:工具栏(快捷时间范围按钮 + 自定义起止时间 + 点数上限 + 刷新)+ 上半部 `egui_plot::Plot` 折线 + 下半部 `egui_extras::TableBuilder` 列表
- 不写 e2e(spec §4.2 与 §5.5);提供手测脚本 `docs/manual-tests/history-read.md`

**Tech Stack:** async-opcua-client `Session::history_read`, async-opcua-types `ReadRawModifiedDetails / HistoryData`, **新依赖 `egui_plot` 0.34**

---

## File Structure

| 文件 | 责任 |
|---|---|
| `crates/opcuasim-core/src/history.rs` (新建) | history_read_raw + DTO |
| `crates/opcuasim-core/src/lib.rs` (修改) | 暴露 history 模块 |
| `Cargo.toml` (workspace,修改) | 加 `egui_plot = "0.34"` |
| `crates/opcuamaster-egui/Cargo.toml` (修改) | 引用 workspace 的 egui_plot |
| `crates/opcuamaster-egui/src/events.rs` (修改) | UiCommand::ReadHistory + BackendEvent::HistoryResult + DTO |
| `crates/opcuamaster-egui/src/backend/dispatcher.rs` (修改) | history handler |
| `crates/opcuamaster-egui/src/model.rs` (修改) | `CentralPanelTab` 枚举 + `Vec<HistoryTabState>` |
| `crates/opcuamaster-egui/src/panels/history_tab.rs` (新建) | 工具栏 + Plot + 表格 |
| `crates/opcuamaster-egui/src/panels/mod.rs` (修改) | 暴露 history_tab 模块 |
| `crates/opcuamaster-egui/src/panels/browse_panel.rs` (修改) | Variable 节点右键加"查看历史" |
| `crates/opcuamaster-egui/src/panels/data_table.rs` (修改) | DataTable 行右键加"查看历史" |
| `crates/opcuamaster-egui/src/app.rs` (修改) | Tab 切换栏渲染 + 路由 HistoryResult |
| `docs/manual-tests/history-read.md` (新建) | 手测脚本 |

---

## Task 1: core — history.rs

**Files:**
- Create: `crates/opcuasim-core/src/history.rs`
- Modify: `crates/opcuasim-core/src/lib.rs`

- [ ] **Step 1: 创建 history.rs**

```rust
//! Historical data access wrapper around Session::history_read with
//! ReadRawModifiedDetails. Loops continuation points up to max_values.

use std::sync::Arc;

use opcua_client::Session;
use opcua_types::{
    ContinuationPoint, DataValue, DateTime, ExtensionObject, HistoryData, HistoryReadResult,
    HistoryReadValueId, NodeId, NumericRange, QualifiedName, ReadRawModifiedDetails,
    TimestampsToReturn,
};

use crate::error::OpcUaSimError;

#[derive(Debug, Clone)]
pub struct HistoryDataPoint {
    /// Source timestamp as RFC3339 UTC string (or empty if missing).
    pub source_timestamp: String,
    /// Server timestamp as RFC3339 UTC string.
    pub server_timestamp: String,
    /// Display string of the Variant value, or empty.
    pub value: String,
    /// Numeric coercion of the Variant value for plotting; None if non-numeric.
    pub numeric: Option<f64>,
    /// Status code as a string label.
    pub status: String,
}

/// Read raw historical samples between [start, end].
/// Loops continuation points until `max_values` reached or server returns
/// none. `return_bounds` asks the server to include the boundary values.
pub async fn history_read_raw(
    session: &Arc<Session>,
    node_id: &NodeId,
    start: DateTime,
    end: DateTime,
    max_values: u32,
    return_bounds: bool,
) -> Result<Vec<HistoryDataPoint>, OpcUaSimError> {
    use opcua_client::session::services::attributes::HistoryReadAction;

    let mut out: Vec<HistoryDataPoint> = Vec::new();
    let mut continuation_point = ContinuationPoint::null();

    loop {
        let action = HistoryReadAction::ReadRawModifiedDetails(ReadRawModifiedDetails {
            is_read_modified: false,
            start_time: start,
            end_time: end,
            num_values_per_node: max_values.saturating_sub(out.len() as u32),
            return_bounds,
        });
        let nodes_to_read = vec![HistoryReadValueId {
            node_id: node_id.clone(),
            index_range: NumericRange::None,
            data_encoding: QualifiedName::null(),
            continuation_point: continuation_point.clone(),
        }];

        let results: Vec<HistoryReadResult> = session
            .history_read(action, TimestampsToReturn::Both, false, &nodes_to_read)
            .await
            .map_err(|e| OpcUaSimError::ConnectionFailed(format!("history_read failed: {e}")))?;

        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| OpcUaSimError::ConnectionFailed("history_read empty result".into()))?;

        if !result.status_code.is_good() {
            return Err(OpcUaSimError::ConnectionFailed(format!(
                "history_read status: {}",
                result.status_code
            )));
        }

        let history_data: Option<Box<HistoryData>> =
            result.history_data.into_inner_as::<HistoryData>();
        let dvs: Vec<DataValue> = history_data
            .and_then(|hd| hd.data_values)
            .unwrap_or_default();

        for dv in dvs {
            out.push(map_data_value(dv));
            if out.len() as u32 >= max_values {
                break;
            }
        }

        if out.len() as u32 >= max_values || result.continuation_point.is_null() {
            break;
        }
        continuation_point = result.continuation_point;
    }

    Ok(out)
}

fn map_data_value(dv: DataValue) -> HistoryDataPoint {
    let value_str = dv
        .value
        .as_ref()
        .map(|v| format!("{v}"))
        .unwrap_or_default();
    let numeric = dv.value.as_ref().and_then(variant_to_f64);
    let status = dv
        .status
        .map(|s| format!("{s}"))
        .unwrap_or_else(|| "Good".to_string());
    let source_timestamp = dv
        .source_timestamp
        .map(|t| t.to_string())
        .unwrap_or_default();
    let server_timestamp = dv
        .server_timestamp
        .map(|t| t.to_string())
        .unwrap_or_default();
    HistoryDataPoint {
        source_timestamp,
        server_timestamp,
        value: value_str,
        numeric,
        status,
    }
}

fn variant_to_f64(v: &opcua_types::Variant) -> Option<f64> {
    use opcua_types::Variant;
    match v {
        Variant::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        Variant::SByte(x) => Some(*x as f64),
        Variant::Byte(x) => Some(*x as f64),
        Variant::Int16(x) => Some(*x as f64),
        Variant::UInt16(x) => Some(*x as f64),
        Variant::Int32(x) => Some(*x as f64),
        Variant::UInt32(x) => Some(*x as f64),
        Variant::Int64(x) => Some(*x as f64),
        Variant::UInt64(x) => Some(*x as f64),
        Variant::Float(x) => Some(*x as f64),
        Variant::Double(x) => Some(*x),
        _ => None,
    }
}
```

注:`HistoryReadAction` 的导出路径在编译时若不准,改成 `opcua_client::session::services::attributes::HistoryReadAction` 之外的路径(或 grep registry)。`ContinuationPoint::null()` / `is_null()` 若不存在,改用 `ContinuationPoint::default()` 然后比较 `value.is_some()`。

- [ ] **Step 2: 暴露模块**

在 `lib.rs` 加:

```rust
pub mod history;
```

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -15`
Expected: 通过;hop 修类型路径直至通过。

- [ ] **Step 4: Commit**

```bash
git add crates/opcuasim-core/src/history.rs crates/opcuasim-core/src/lib.rs
git commit -m "feat(core): history_read_raw with continuation-point loop

history_read_raw(session, node_id, start, end, max_values, return_bounds)
issues HistoryReadAction::ReadRawModifiedDetails repeatedly until the
server returns a null continuation point or max_values reached. Each
DataValue is normalized into HistoryDataPoint with RFC3339 timestamps,
the Variant's display string, an optional f64 coercion for plotting,
and a status label."
```

---

## Task 2: workspace — egui_plot 依赖

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/opcuamaster-egui/Cargo.toml`

- [ ] **Step 1: workspace deps**

在 `[workspace.dependencies]` 末尾加:

```toml
egui_plot = "0.34"
```

- [ ] **Step 2: master-egui 引用**

在 `crates/opcuamaster-egui/Cargo.toml` `[dependencies]` 加:

```toml
egui_plot.workspace = true
```

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -10`
Expected: 拉取 egui_plot,无报错。

(暂不 commit,与后续合并提交)

---

## Task 3: master-egui — events.rs

**Files:**
- Modify: `crates/opcuamaster-egui/src/events.rs`

- [ ] **Step 1: UiCommand 变体**

在 `UiCommand::CallMethod { ... },` 之后加:

```rust
    ReadHistory {
        conn_id: String,
        node_id: String,
        start_iso: String,
        end_iso: String,
        max_values: u32,
        req_id: u64,
    },
```

- [ ] **Step 2: BackendEvent 变体 + DTO**

在 `BackendEvent::MethodCallResult` 之后加:

```rust
    HistoryResult {
        req_id: u64,
        node_id: String,
        points: Vec<HistoryPointDto>,
        error: Option<String>,
    },
```

文件末尾加:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPointDto {
    pub source_timestamp: String,
    pub server_timestamp: String,
    pub value: String,
    pub numeric: Option<f64>,
    pub status: String,
}
```

---

## Task 4: master-egui — dispatcher handler

**Files:**
- Modify: `crates/opcuamaster-egui/src/backend/dispatcher.rs`

- [ ] **Step 1: 加 use**

`use opcuasim_core::method::{...};` 之下加:

```rust
use opcuasim_core::history::history_read_raw;
```

events use 列表加 `HistoryPointDto`。

- [ ] **Step 2: handle_cmd 加分支**

在 `UiCommand::CallMethod { ... }` 分支之后加:

```rust
        UiCommand::ReadHistory {
            conn_id,
            node_id,
            start_iso,
            end_iso,
            max_values,
            req_id,
        } => {
            do_read_history(
                conn_id, node_id, start_iso, end_iso, max_values, req_id, &state, &event_tx,
            )
            .await
        }
```

- [ ] **Step 3: 写 handler**

文件末尾追加:

```rust
#[allow(clippy::too_many_arguments)]
async fn do_read_history(
    conn_id: String,
    node_id: String,
    start_iso: String,
    end_iso: String,
    max_values: u32,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let nid: opcua_types::NodeId = node_id
        .parse()
        .map_err(|_| format!("invalid node id: {node_id}"))?;

    let start = parse_iso_to_datetime(&start_iso)?;
    let end = parse_iso_to_datetime(&end_iso)?;

    let send_result = |error: Option<String>, points: Vec<HistoryPointDto>| {
        let _ = event_tx.send(BackendEvent::HistoryResult {
            req_id,
            node_id: node_id.clone(),
            points,
            error,
        });
    };

    match history_read_raw(&session, &nid, start, end, max_values, true).await {
        Ok(pts) => {
            let dtos: Vec<HistoryPointDto> = pts
                .into_iter()
                .map(|p| HistoryPointDto {
                    source_timestamp: p.source_timestamp,
                    server_timestamp: p.server_timestamp,
                    value: p.value,
                    numeric: p.numeric,
                    status: p.status,
                })
                .collect();
            send_result(None, dtos);
            Ok(())
        }
        Err(e) => {
            send_result(Some(e.to_string()), Vec::new());
            // Surface as toast as well.
            Err(format!("HistoryRead failed: {e}"))
        }
    }
}

fn parse_iso_to_datetime(s: &str) -> Result<opcua_types::DateTime, String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(s.trim())
        .map_err(|e| format!("invalid time '{s}': {e}"))?;
    let utc: chrono::DateTime<chrono::Utc> = parsed.with_timezone(&chrono::Utc);
    Ok(opcua_types::DateTime::from(utc))
}
```

注:`opcua_types::DateTime::from(chrono::DateTime<Utc>)` 若不存在,改 `DateTime::new(utc)` 或解构成 ticks。编译时确认。

---

## Task 5: master-egui — model.rs Tab + HistoryTabState

**Files:**
- Modify: `crates/opcuamaster-egui/src/model.rs`

- [ ] **Step 1: 加枚举与状态**

在 `pub struct AppModel { ... }` 中,在 `pub next_req_id: u64,` 之后加:

```rust
    pub central_tab: CentralPanelTab,
    pub history_tabs: Vec<HistoryTabState>,
```

并在 Default impl 里给默认值(若是 derive,改成手写 Default)。

文件中找 `#[derive(Default)] pub struct AppModel`,把 derive 拿掉,改:

```rust
impl Default for AppModel {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            selected_conn: None,
            modal: None,
            browse: BrowseState::default(),
            monitor: MonitorState::default(),
            value_panel: ValuePanelState::default(),
            logs: LogState::default(),
            groups: Vec::new(),
            group_input: String::new(),
            toasts: Vec::new(),
            next_req_id: 0,
            central_tab: CentralPanelTab::DataTable,
            history_tabs: Vec::new(),
        }
    }
}
```

定义枚举与状态:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CentralPanelTab {
    DataTable,
    History(usize), // index into history_tabs
}

pub struct HistoryTabState {
    pub conn_id: String,
    pub node_id: String,
    pub display_name: String,
    pub start_iso: String,
    pub end_iso: String,
    pub max_values: u32,
    pub points: Vec<crate::events::HistoryPointDto>,
    pub pending_req: Option<u64>,
    pub error: Option<String>,
    pub last_loaded: Option<std::time::Instant>,
}

impl HistoryTabState {
    pub fn new(conn_id: String, node_id: String, display_name: String) -> Self {
        // Default: last 5 minutes from "now". Use local naive ISO so the
        // user can adjust easily; serialization to UTC happens in dispatcher.
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::minutes(5);
        Self {
            conn_id,
            node_id,
            display_name,
            start_iso: start.to_rfc3339(),
            end_iso: now.to_rfc3339(),
            max_values: 5000,
            points: Vec::new(),
            pending_req: None,
            error: None,
            last_loaded: None,
        }
    }
}
```

---

## Task 6: master-egui — history_tab.rs panel

**Files:**
- Create: `crates/opcuamaster-egui/src/panels/history_tab.rs`
- Modify: `crates/opcuamaster-egui/src/panels/mod.rs`

- [ ] **Step 1: history_tab.rs**

```rust
use crate::events::UiCommand;
use crate::model::{AppModel, HistoryTabState};
use crate::runtime::BackendHandle;

pub struct TabActions {
    pub close: bool,
    pub refresh: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut HistoryTabState,
) -> TabActions {
    let mut actions = TabActions { close: false, refresh: false };

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
        let resp = ui.add_enabled(!busy, egui::Button::new(if busy { "加载中…" } else { "🔄 刷新" }));
        if resp.clicked() {
            actions.refresh = true;
        }
    });

    if let Some(err) = &state.error {
        ui.colored_label(egui::Color32::LIGHT_RED, err);
    }

    ui.separator();

    // Plot
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

    // Table
    egui_extras::TableBuilder::new(ui)
        .id_salt(format!("history_table_{}", state.node_id))
        .striped(true)
        .column(egui_extras::Column::auto().at_least(220.0))
        .column(egui_extras::Column::auto().at_least(120.0))
        .column(egui_extras::Column::remainder().at_least(80.0))
        .header(20.0, |mut h| {
            h.col(|ui| { ui.strong("Source Timestamp"); });
            h.col(|ui| { ui.strong("Value"); });
            h.col(|ui| { ui.strong("Status"); });
        })
        .body(|body| {
            let total = state.points.len();
            body.rows(18.0, total, |mut row| {
                let i = row.index();
                let p = &state.points[i];
                row.col(|ui| { ui.label(&p.source_timestamp); });
                row.col(|ui| { ui.label(&p.value); });
                row.col(|ui| { ui.label(&p.status); });
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

#[allow(dead_code)]
fn _ref_app_model(_: &AppModel) {} // keep import live; remove if unused
```

(末尾的 `_ref_app_model` 是 placeholder,如不需要就删。`AppModel` import 暂留以备后用,clippy 报 unused 时移除。)

- [ ] **Step 2: panels/mod.rs 暴露**

加:

```rust
pub mod history_tab;
```

---

## Task 7: master-egui — 入口右键 + app.rs 接入

**Files:**
- Modify: `crates/opcuamaster-egui/src/panels/browse_panel.rs`
- Modify: `crates/opcuamaster-egui/src/panels/data_table.rs`
- Modify: `crates/opcuamaster-egui/src/app.rs`

- [ ] **Step 1: BrowsePanel — Variable 节点右键加历史入口**

修改 `is_variable` 分支(checkbox 行),加 context_menu:

```rust
    } else if is_variable {
        let display_name = model.browse.nodes.get(node_id)
            .map(|s| s.item.display_name.clone())
            .unwrap_or_else(|| node_id.to_string());
        let resp = ui.horizontal(|ui| {
            let mut checked = model.browse.selected.contains(node_id);
            if ui.checkbox(&mut checked, &display).changed() {
                toggle_selection(model, node_id, checked);
            }
        }).response;
        resp.context_menu(|ui| {
            if ui.button("📈 查看历史").clicked() {
                open_history_tab(model, conn_id, node_id, &display_name);
                ui.close();
            }
        });
    }
```

并在文件末尾加 helper:

```rust
fn open_history_tab(model: &mut AppModel, conn_id: &str, node_id: &str, display_name: &str) {
    let idx = model.history_tabs.len();
    model.history_tabs.push(crate::model::HistoryTabState::new(
        conn_id.to_string(),
        node_id.to_string(),
        display_name.to_string(),
    ));
    model.central_tab = crate::model::CentralPanelTab::History(idx);
}

pub fn open_history_tab_pub(model: &mut AppModel, conn_id: &str, node_id: &str, display_name: &str) {
    open_history_tab(model, conn_id, node_id, display_name);
}
```

(后者是给 data_table 共用,改名导出。)

- [ ] **Step 2: DataTable — 行右键加历史入口**

`data_table.rs` 找到 row 渲染,在主 label 处增加 `response.context_menu`(若已有 ctx menu,增加一项;否则新加)。简化:在每行外层包一层 `interact_label` 或 `selectable_label`,挂 context_menu:

```rust
// inside the body loop, when rendering the display column
let resp = ui.label(&row_data.display_name);
resp.context_menu(|ui| {
    if ui.button("📈 查看历史").clicked() {
        crate::panels::browse_panel::open_history_tab_pub(
            model,
            &conn_id,
            &row_data.node_id,
            &row_data.display_name,
        );
        ui.close();
    }
});
```

(具体行渲染细节按当前 data_table.rs 现状调整;若 row 已有右键菜单,加入新项。)

- [ ] **Step 3: app.rs — 中央面板 Tab 切换 + 渲染 HistoryTab**

修改 update 函数中央面板渲染。先在中央面板顶部加 Tab 栏:

```rust
egui::CentralPanel::default().show(ctx, |ui| {
    ui.horizontal(|ui| {
        if ui.selectable_label(
            matches!(self.model.central_tab, crate::model::CentralPanelTab::DataTable),
            "📊 监控表",
        ).clicked() {
            self.model.central_tab = crate::model::CentralPanelTab::DataTable;
        }
        // History tabs
        let mut to_close: Option<usize> = None;
        for (i, tab) in self.model.history_tabs.iter().enumerate() {
            let label = format!("📈 {}", tab.display_name);
            let selected = matches!(self.model.central_tab, crate::model::CentralPanelTab::History(j) if j == i);
            if ui.selectable_label(selected, &label).clicked() {
                self.model.central_tab = crate::model::CentralPanelTab::History(i);
            }
        }
        // (close handled inside the panel itself via TabActions::close)
    });
    ui.separator();

    match self.model.central_tab {
        crate::model::CentralPanelTab::DataTable => {
            data_table::show(ui, &mut self.model, &self.backend);
        }
        crate::model::CentralPanelTab::History(idx) => {
            // Take the tab out to avoid borrowing conflicts.
            if let Some(state) = self.model.history_tabs.get_mut(idx) {
                let actions = crate::panels::history_tab::show(ui, state);
                if actions.refresh {
                    crate::panels::history_tab::dispatch_refresh(
                        state,
                        &self.backend,
                        &mut self.model.next_req_id,
                    );
                }
                if actions.close {
                    self.model.history_tabs.remove(idx);
                    // adjust central_tab
                    if self.model.history_tabs.is_empty() {
                        self.model.central_tab = crate::model::CentralPanelTab::DataTable;
                    } else {
                        let new_idx = idx.min(self.model.history_tabs.len() - 1);
                        self.model.central_tab =
                            crate::model::CentralPanelTab::History(new_idx);
                    }
                }
            } else {
                self.model.central_tab = crate::model::CentralPanelTab::DataTable;
            }
        }
    }
});
```

(具体 CentralPanel 调用位置取决于当前 update 实现 — hop 到现状。)

- [ ] **Step 4: app.rs — apply_event 接 HistoryResult**

在 `BackendEvent::MethodCallResult { ... }` 之后加:

```rust
            BackendEvent::HistoryResult {
                req_id,
                node_id,
                points,
                error,
            } => {
                if let Some(tab) = self
                    .model
                    .history_tabs
                    .iter_mut()
                    .find(|t| t.pending_req == Some(req_id) && t.node_id == node_id)
                {
                    tab.pending_req = None;
                    tab.points = points;
                    tab.error = error;
                    tab.last_loaded = Some(std::time::Instant::now());
                }
            }
```

- [ ] **Step 5: 自动首次刷新**

打开 HistoryTab 后立即触发一次 ReadHistory。最简单做法:在 `open_history_tab` helper 内不能直接发送(没拿到 backend 引用)。改在 `app.rs` 的 update 里检测新 tab 的 `last_loaded == None && pending_req == None` 时调 dispatch_refresh:

```rust
crate::model::CentralPanelTab::History(idx) => {
    if let Some(state) = self.model.history_tabs.get_mut(idx) {
        if state.pending_req.is_none() && state.last_loaded.is_none() {
            crate::panels::history_tab::dispatch_refresh(
                state, &self.backend, &mut self.model.next_req_id);
        }
        let actions = crate::panels::history_tab::show(ui, state);
        ...
```

- [ ] **Step 6: 编译 + clippy**

Run:
```bash
cargo build -p opcuamaster-egui 2>&1 | tail -10
cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10
```
Expected: 全过。如有 too_many_arguments / unused 等,加 `#[allow]` 或修除。

- [ ] **Step 7: 跑老 e2e 不回归**

Run: `cargo test -p opcuamaster-egui --test e2e 2>&1 | tail -10`
Expected: 3 个测试全过。

---

## Task 8: 手测脚本

**Files:**
- Create: `docs/manual-tests/history-read.md`

```markdown
# 手测:HistoryRead

## 前置条件
- 装好任一带 historian 的 OPC UA Server,如:
  - **Prosys OPC UA Simulation Server**(免费,带 historian)
  - **KEPServerEX**(商业,试用即可)
  - **open62541** 自带 `tutorial_server_history` 示例
- 启动该 server,在其 UI/配置中开启历史归档

## 步骤

1. `cargo run --release -p opcuamaster-egui`
2. 顶栏 → 新建连接,填入服务器 URL,Anonymous,连接
3. 浏览节点,找到一个有历史数据的 Variable(例如 Prosys 的 `Counter`、`Sinusoid`)
4. 等订阅几分钟,让 historian 攒下数据
5. **在 BrowsePanel 中右键该 Variable → "📈 查看历史"**
6. 中央面板切换到新 Tab,观察:
   - 默认时间范围:过去 5 分钟
   - 折线图应有上升 / 周期波形
   - 表格列出每个采样点(Source Timestamp / Value / Status)
7. 点 "1h" 快捷按钮,刷新自动触发,看到更长时间范围的数据
8. 自定义起止时间(改 RFC3339 字符串),点 "🔄 刷新",数据应反映新范围
9. 关闭 Tab,数据表 Tab 应仍可切回

## 已知限制(本期)

- Float/Double 等数值类型才会画折线;String/Bool 显示为文本表
- 时间输入是 RFC3339 字符串(如 `2026-04-28T08:00:00Z`),没有日历选择器
- 单 Tab 单节点;不支持多节点叠加
```

- [ ] **Step 1: 创建文件**(如 docs/manual-tests/ 目录不存在则 mkdir)

```bash
mkdir -p docs/manual-tests
# 写入上面内容
```

---

## Task 9: 收尾 commit + push

- [ ] **Step 1: commit**

```bash
git add Cargo.toml Cargo.lock \
        crates/opcuamaster-egui/Cargo.toml \
        crates/opcuamaster-egui/src/events.rs \
        crates/opcuamaster-egui/src/backend/dispatcher.rs \
        crates/opcuamaster-egui/src/model.rs \
        crates/opcuamaster-egui/src/panels/history_tab.rs \
        crates/opcuamaster-egui/src/panels/mod.rs \
        crates/opcuamaster-egui/src/panels/browse_panel.rs \
        crates/opcuamaster-egui/src/panels/data_table.rs \
        crates/opcuamaster-egui/src/app.rs \
        docs/manual-tests/history-read.md
git commit -m "feat(master): historical data viewer (Plot + Table)

BrowsePanel/DataTable right-click 查看历史 opens a HistoryTab in the
central panel: shortcut buttons (1m..24h) and ISO range fields drive
ReadHistory; results render as a numeric line chart (egui_plot) plus a
striped table of (timestamp, value, status). No automated test —
manual-tests/history-read.md spells out the verification flow against
Prosys / KEPServerEX / open62541 historians."
```

- [ ] **Step 2: push**

```bash
git push origin master
```

---

## Self-Review

**Spec coverage:**
- §5.5 `history_read_raw` API + continuationPoint 处理 → Task 1
- §5.5 入口 BrowsePanel + DataTable 右键 → Task 7 step 1+2
- §5.5 新 HistoryTab(中央面板 Tab 切换) → Task 5+6+7 step 3
- §5.5 工具栏 时间范围 / 点数上限 / 刷新 → Task 6
- §5.5 上半 egui_plot 折线 → Task 6
- §5.5 下半 TableBuilder 表 → Task 6
- §4.2 不写 e2e,提供手测 → Task 8

**Placeholder scan:** 无 TODO/TBD。Task 6 的 `_ref_app_model` 与 Task 1 的 hop 提示是已知调整点,非占位。

**Type consistency:**
- `HistoryDataPoint`(core)字段:source_timestamp / server_timestamp / value / numeric / status
- `HistoryPointDto`(events)字段同名同类型
- `HistoryTabState`(model)持有 `Vec<HistoryPointDto>` 直接显示
- `CentralPanelTab::History(idx)` 通过索引引用 `history_tabs`

**风险:**
- `HistoryReadAction` 的导出路径在编译期需确认
- `ContinuationPoint::null()` API 名称需确认
- `chrono::DateTime<Utc>` → `opcua_types::DateTime` 转换需确认
- `egui_plot::Line::new` API 在 0.34 可能变化;Spec §7 已记此风险,会在 Task 6 编译期 hop

**Commit 数:** 2 条
1. Task 1: feat(core): history_read_raw with continuation-point loop
2. Task 9: feat(master): historical data viewer (Plot + Table)(含 events / dispatcher / model / panel / browse / data_table / app / 手测脚本 / Cargo.toml)
