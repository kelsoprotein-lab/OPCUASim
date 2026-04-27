use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::events::{BrowseItem, ConnectionInfo, LogRow, MonitoredRow, NodeAttrsDto, NodeGroupDto};
use crate::widgets::connection_dialog::ConnDialogState;

#[derive(Default)]
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

#[derive(Default)]
pub struct LogState {
    pub per_conn: HashMap<String, LogPerConn>,
    pub expanded: bool,
    pub filter: LogDirectionFilter,
    pub search: String,
}

#[derive(Default)]
pub struct LogPerConn {
    pub entries: Vec<LogRow>,
}

impl LogPerConn {
    pub fn append(&mut self, mut rows: Vec<LogRow>) {
        const MAX: usize = 10_000;
        self.entries.append(&mut rows);
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
        }
    }
}

pub struct Toast {
    pub level: crate::events::ToastLevel,
    pub message: String,
    pub created_at: std::time::Instant,
}
