use std::sync::{Arc, RwLock};
use crate::log_entry::LogEntry;

const MAX_LOG_ENTRIES: usize = 10_000;

#[derive(Clone)]
pub struct LogCollector {
    entries: Arc<RwLock<Vec<LogEntry>>>,
    seq_counter: Arc<RwLock<u64>>,
}

impl LogCollector {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            seq_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub fn next_seq(&self) -> u64 {
        let mut counter = self.seq_counter.write().unwrap();
        *counter += 1;
        *counter
    }

    pub fn add(&self, entry: LogEntry) {
        let mut entries = self.entries.write().unwrap();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.remove(0);
        }
        entries.push(entry);
    }

    pub fn get_since(&self, since_seq: u64) -> Vec<LogEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().filter(|e| e.seq > since_seq).cloned().collect()
    }

    pub fn get_all(&self) -> Vec<LogEntry> {
        self.entries.read().unwrap().clone()
    }

    pub fn clear(&self) {
        self.entries.write().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn export_csv(&self) -> String {
        let entries = self.entries.read().unwrap();
        let mut csv = String::from(LogEntry::csv_header());
        csv.push('\n');
        for entry in entries.iter() {
            csv.push_str(&entry.to_csv_row());
            csv.push('\n');
        }
        csv
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_entry::Direction;

    fn make_entry(collector: &LogCollector, service: &str) -> LogEntry {
        LogEntry::new(
            collector.next_seq(),
            "test-conn".to_string(),
            Direction::Request,
            service.to_string(),
            "test detail".to_string(),
            None,
        )
    }

    #[test]
    fn test_add_and_get_all() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.add(make_entry(&collector, "Read"));
        assert_eq!(collector.len(), 2);
        let all = collector.get_all();
        assert_eq!(all[0].service, "Browse");
        assert_eq!(all[1].service, "Read");
    }

    #[test]
    fn test_get_since() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.add(make_entry(&collector, "Read"));
        let since = collector.get_since(1);
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].service, "Read");
    }

    #[test]
    fn test_clear() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        collector.clear();
        assert!(collector.is_empty());
    }

    #[test]
    fn test_csv_export() {
        let collector = LogCollector::new();
        collector.add(make_entry(&collector, "Browse"));
        let csv = collector.export_csv();
        assert!(csv.starts_with("Timestamp,"));
        assert!(csv.contains("Browse"));
    }
}
