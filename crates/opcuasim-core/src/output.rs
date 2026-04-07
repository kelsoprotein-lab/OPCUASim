use crate::log_collector::LogCollector;
use crate::log_entry::{Direction, LogEntry};

pub struct DataChangeItem {
    pub node_id: String,
    pub value: String,
    pub quality: String,
    pub timestamp: String,
}

#[allow(async_fn_in_trait)]
pub trait DataOutput: Send + Sync {
    async fn on_data_change(&self, connection_id: &str, items: &[DataChangeItem]);
    async fn on_connect(&self, connection_id: &str);
    async fn on_disconnect(&self, connection_id: &str);
}

pub struct LogOutput {
    collector: LogCollector,
}

impl LogOutput {
    pub fn new(collector: LogCollector) -> Self {
        Self { collector }
    }
}

impl DataOutput for LogOutput {
    async fn on_data_change(&self, connection_id: &str, items: &[DataChangeItem]) {
        for item in items {
            let seq = self.collector.next_seq();
            self.collector.add(LogEntry::new(
                seq,
                connection_id.to_string(),
                Direction::Response,
                "DataChange".to_string(),
                format!("{} = {} [{}]", item.node_id, item.value, item.quality),
                None,
            ));
        }
    }

    async fn on_connect(&self, connection_id: &str) {
        let seq = self.collector.next_seq();
        self.collector.add(LogEntry::new(
            seq,
            connection_id.to_string(),
            Direction::Response,
            "Session".to_string(),
            "Connected".to_string(),
            None,
        ));
    }

    async fn on_disconnect(&self, connection_id: &str) {
        let seq = self.collector.next_seq();
        self.collector.add(LogEntry::new(
            seq,
            connection_id.to_string(),
            Direction::Response,
            "Session".to_string(),
            "Disconnected".to_string(),
            None,
        ));
    }
}
