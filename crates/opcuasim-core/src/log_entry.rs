use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Direction {
    Request,
    Response,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Request => write!(f, "Request"),
            Direction::Response => write!(f, "Response"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub connection_id: String,
    pub direction: Direction,
    pub service: String,
    pub detail: String,
    pub status: Option<String>,
}

impl LogEntry {
    pub fn new(
        seq: u64,
        connection_id: String,
        direction: Direction,
        service: String,
        detail: String,
        status: Option<String>,
    ) -> Self {
        Self {
            seq,
            timestamp: Utc::now(),
            connection_id,
            direction,
            service,
            detail,
            status,
        }
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.direction,
            self.service,
            self.detail.replace(',', ";"),
            self.status.as_deref().unwrap_or(""),
            self.connection_id,
        )
    }

    pub fn csv_header() -> &'static str {
        "Timestamp,Direction,Service,Detail,Status,ConnectionId"
    }
}
