use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::events::{BrowseItem, ConnectionInfo, LogRow, MonitoredRow, NodeAttrsDto, NodeGroupDto};
use crate::widgets::connection_dialog::ConnDialogState;

pub struct AppModel {
    pub connections: Vec<ConnectionInfo>,
    pub selected_conn: Option<String>,
    pub modal: Option<Modal>,
    pub browse: BrowseState,
    pub monitor: MonitorState,
    pub value_panel: ValuePanelState,
    pub logs: LogState,
    pub groups: Vec<NodeGroupDto>,
    pub group_input: String,
    pub toasts: Vec<Toast>,
    pub next_req_id: u64,
    pub central_tab: CentralPanelTab,
    pub history_tabs: Vec<HistoryTabState>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CentralPanelTab {
    DataTable,
    History(usize),
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
    /// Cached `[unix_secs, value]` pairs derived from `points`. Refreshed on
    /// `set_points` rather than every frame.
    pub plot_cache: Vec<[f64; 2]>,
}

impl HistoryTabState {
    pub fn new(conn_id: String, node_id: String, display_name: String) -> Self {
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
            plot_cache: Vec::new(),
        }
    }

    /// Replace `points` and refresh the cached `[ts, value]` plot data.
    pub fn set_points(&mut self, points: Vec<crate::events::HistoryPointDto>) {
        use chrono::DateTime;
        self.plot_cache.clear();
        self.plot_cache.reserve(points.len());
        for p in &points {
            let Ok(dt) = DateTime::parse_from_rfc3339(&p.source_timestamp) else {
                continue;
            };
            let Some(v) = p.numeric else { continue };
            self.plot_cache
                .push([dt.timestamp_millis() as f64 / 1000.0, v]);
        }
        self.points = points;
    }
}

impl Default for LogState {
    fn default() -> Self {
        Self {
            per_conn: HashMap::new(),
            expanded: false,
            filter: LogDirectionFilter::All,
            search: String::new(),
            paused: false,
            auto_scroll: true,
            filter_dirty: true,
            filtered_cache: Vec::new(),
            last_filter_key: (String::new(), LogDirectionFilter::All, String::new(), 0),
        }
    }
}

impl LogState {
    /// Rebuilds `filtered_cache` lazily — only when conn / filter / search /
    /// entry count changed since the last call.
    pub fn ensure_filter(&mut self, conn_id: &str) {
        let entries_len = self
            .per_conn
            .get(conn_id)
            .map(|p| p.entries.len())
            .unwrap_or(0);
        let key = (
            conn_id.to_string(),
            self.filter,
            self.search.clone(),
            entries_len,
        );
        if !self.filter_dirty && key == self.last_filter_key {
            return;
        }
        self.filtered_cache.clear();
        let needle = self.search.trim().to_lowercase();
        if let Some(per) = self.per_conn.get(conn_id) {
            for (i, e) in per.entries.iter().enumerate() {
                let dir_ok = matches!(
                    (self.filter, e.direction.as_str()),
                    (LogDirectionFilter::All, _)
                        | (LogDirectionFilter::Request, "Request")
                        | (LogDirectionFilter::Response, "Response")
                );
                if !dir_ok {
                    continue;
                }
                if !needle.is_empty()
                    && !e.service.to_lowercase().contains(&needle)
                    && !e.detail.to_lowercase().contains(&needle)
                {
                    continue;
                }
                self.filtered_cache.push(i);
            }
        }
        self.filter_dirty = false;
        self.last_filter_key = key;
    }
}

impl AppModel {
    pub fn alloc_req_id(&mut self) -> u64 {
        self.next_req_id = self.next_req_id.wrapping_add(1);
        self.next_req_id
    }

    pub fn push_toast(&mut self, level: crate::events::ToastLevel, msg: impl Into<String>) {
        self.toasts.push(Toast {
            level,
            message: msg.into(),
            created_at: std::time::Instant::now(),
        });
    }

    pub fn apply_monitored_snapshot(
        &mut self,
        conn_id: &str,
        seq: u64,
        full: bool,
        rows: Vec<MonitoredRow>,
    ) {
        let per = self
            .monitor
            .per_conn
            .entry(conn_id.to_string())
            .or_default();
        if full {
            per.rows.clear();
            for row in rows {
                per.rows.insert(row.node_id.clone(), row);
            }
        } else {
            for row in rows {
                per.rows.insert(row.node_id.clone(), row);
            }
        }
        per.seq = seq;
        self.monitor.filter_dirty = true;
    }
}

pub enum Modal {
    NewConnection(ConnDialogState),
    CertManager(CertManagerState),
    MethodCall(MethodCallState),
}

#[derive(Default)]
pub struct CertManagerState {
    pub trusted: Vec<crate::events::CertSummaryDto>,
    pub rejected: Vec<crate::events::CertSummaryDto>,
    pub pending_trusted_req: Option<u64>,
    pub pending_rejected_req: Option<u64>,
    pub selected_path: Option<std::path::PathBuf>,
    pub error: Option<String>,
}

pub struct MethodCallState {
    pub conn_id: String,
    pub object_id: String,
    pub method_id: String,
    pub display_name: String,
    pub inputs_meta: Vec<crate::events::MethodArgInfo>,
    pub outputs_meta: Vec<crate::events::MethodArgInfo>,
    pub input_values: Vec<String>,
    pub pending_args_req: Option<u64>,
    pub pending_call_req: Option<u64>,
    pub last_result_status: Option<String>,
    pub last_result_outputs: Vec<crate::events::MethodArgValue>,
    pub error: Option<String>,
}

impl MethodCallState {
    pub fn new(
        conn_id: String,
        object_id: String,
        method_id: String,
        display_name: String,
    ) -> Self {
        Self {
            conn_id,
            object_id,
            method_id,
            display_name,
            inputs_meta: Vec::new(),
            outputs_meta: Vec::new(),
            input_values: Vec::new(),
            pending_args_req: None,
            pending_call_req: None,
            last_result_status: None,
            last_result_outputs: Vec::new(),
            error: None,
        }
    }
}

pub struct BrowseState {
    pub open: bool,
    pub conn_id: Option<String>,
    pub root_loaded: bool,
    pub nodes: HashMap<String, BrowseNodeState>,
    pub roots: Vec<String>,
    pub pending: HashSet<u64>,
    pub selected: HashSet<String>,
    pub access_mode: String,
    pub interval_ms: f64,
    pub max_depth: u32,
    pub filter_enabled: bool,
    pub trigger: crate::events::DataChangeTriggerKindReq,
    pub deadband_kind: crate::events::DeadbandKindReq,
    pub deadband_value: f64,
    /// child node id → parent node id, populated incrementally as browse
    /// results arrive. Used to find a method's owner Object O(1).
    pub parent_of: HashMap<String, String>,
}

pub struct BrowseNodeState {
    pub item: BrowseItem,
    pub expanded: bool,
    pub children: Option<Vec<String>>,
    pub loading: bool,
}

#[derive(Default)]
pub struct MonitorState {
    pub per_conn: HashMap<String, MonitorPerConn>,
    pub search: String,
    pub selected_rows: HashSet<String>,
    pub focused_row: Option<String>,
    pub filter_dirty: bool,
    pub filtered_cache: Vec<String>,
    pub last_conn_for_filter: Option<String>,
    pub last_search_for_filter: String,
    /// Anchor index in `filtered_cache` for Shift+Click range selection.
    pub last_clicked_filtered_idx: Option<usize>,
}

#[derive(Default)]
pub struct ValuePanelState {
    pub attrs: Option<NodeAttrsDto>,
    pub pending_read_req: Option<u64>,
    pub pending_write_req: Option<u64>,
    pub write_value: String,
    pub last_result: Option<String>,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum LogDirectionFilter {
    #[default]
    All,
    Request,
    Response,
}

pub struct LogState {
    pub per_conn: HashMap<String, LogPerConn>,
    pub expanded: bool,
    pub filter: LogDirectionFilter,
    pub search: String,
    pub paused: bool,
    pub auto_scroll: bool,
    pub filter_dirty: bool,
    pub filtered_cache: Vec<usize>,
    pub last_filter_key: (String, LogDirectionFilter, String, usize),
}

#[derive(Default)]
pub struct LogPerConn {
    pub entries: Vec<LogRow>,
    /// Buffered while paused — flushed back into `entries` when resumed.
    pub paused_buf: Vec<LogRow>,
}

impl LogPerConn {
    pub fn append(&mut self, mut rows: Vec<LogRow>, paused: bool) {
        const MAX: usize = 10_000;
        if paused {
            self.paused_buf.append(&mut rows);
            if self.paused_buf.len() > MAX {
                let excess = self.paused_buf.len() - MAX;
                self.paused_buf.drain(0..excess);
            }
            return;
        }
        self.entries.append(&mut rows);
        if self.entries.len() > MAX {
            let excess = self.entries.len() - MAX;
            self.entries.drain(0..excess);
        }
    }

    pub fn flush_paused(&mut self) {
        const MAX: usize = 10_000;
        self.entries.append(&mut self.paused_buf);
        if self.entries.len() > MAX {
            let excess = self.entries.len() - MAX;
            self.entries.drain(0..excess);
        }
    }
}

#[derive(Default)]
pub struct MonitorPerConn {
    pub rows: IndexMap<String, MonitoredRow>,
    pub seq: u64,
}

impl MonitorState {
    pub fn ensure_filter(&mut self, conn_id: &str) -> &[String] {
        let needs_rebuild = self.filter_dirty
            || self.last_conn_for_filter.as_deref() != Some(conn_id)
            || self.last_search_for_filter != self.search;

        if needs_rebuild {
            let needle = self.search.trim().to_lowercase();
            self.filtered_cache.clear();
            if let Some(per) = self.per_conn.get(conn_id) {
                for (nid, row) in per.rows.iter() {
                    if needle.is_empty() {
                        self.filtered_cache.push(nid.clone());
                    } else {
                        let hay = format!(
                            "{} {} {}",
                            row.node_id,
                            row.display_name,
                            row.value.as_deref().unwrap_or("")
                        )
                        .to_lowercase();
                        if hay.contains(&needle) {
                            self.filtered_cache.push(nid.clone());
                        }
                    }
                }
            }
            self.last_conn_for_filter = Some(conn_id.to_string());
            self.last_search_for_filter = self.search.clone();
            self.filter_dirty = false;
        }
        &self.filtered_cache
    }
}

impl Default for BrowseState {
    fn default() -> Self {
        Self {
            open: false,
            conn_id: None,
            root_loaded: false,
            nodes: HashMap::new(),
            roots: Vec::new(),
            pending: HashSet::new(),
            selected: HashSet::new(),
            access_mode: "Subscription".into(),
            interval_ms: 1000.0,
            max_depth: 1,
            filter_enabled: false,
            trigger: crate::events::DataChangeTriggerKindReq::StatusValue,
            deadband_kind: crate::events::DeadbandKindReq::None,
            deadband_value: 0.0,
            parent_of: HashMap::new(),
        }
    }
}

impl AppModel {
    pub fn current_filter_req(&self) -> Option<crate::events::DataChangeFilterReq> {
        if !self.browse.filter_enabled {
            return None;
        }
        Some(crate::events::DataChangeFilterReq {
            trigger: self.browse.trigger,
            deadband_kind: self.browse.deadband_kind,
            deadband_value: self.browse.deadband_value,
        })
    }
}

pub struct Toast {
    pub level: crate::events::ToastLevel,
    pub message: String,
    pub created_at: std::time::Instant,
}
