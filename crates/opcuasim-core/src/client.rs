use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn};

use crate::config::ConnectionConfig;
use crate::error::OpcUaSimError;
use crate::log_collector::LogCollector;
use crate::log_entry::{Direction, LogEntry};
use crate::reconnect::{ReconnectPolicy, ReconnectState};

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting => write!(f, "Reconnecting"),
        }
    }
}

impl serde::Serialize for ConnectionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub struct OpcUaConnection {
    pub config: ConnectionConfig,
    pub state: Arc<RwLock<ConnectionState>>,
    pub log_collector: LogCollector,
    reconnect_policy: ReconnectPolicy,
    reconnect_state: Arc<RwLock<ReconnectState>>,
    shutdown_tx: Arc<RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl OpcUaConnection {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            log_collector: LogCollector::new(),
            reconnect_policy: ReconnectPolicy::default(),
            reconnect_state: Arc::new(RwLock::new(ReconnectState::Idle)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    async fn set_state(&self, new_state: ConnectionState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    fn log_request(&self, service: &str, detail: &str) {
        let seq = self.log_collector.next_seq();
        self.log_collector.add(LogEntry::new(
            seq,
            self.config.id.clone(),
            Direction::Request,
            service.to_string(),
            detail.to_string(),
            None,
        ));
    }

    fn log_response(&self, service: &str, detail: &str, status: Option<&str>) {
        let seq = self.log_collector.next_seq();
        self.log_collector.add(LogEntry::new(
            seq,
            self.config.id.clone(),
            Direction::Response,
            service.to_string(),
            detail.to_string(),
            status.map(|s| s.to_string()),
        ));
    }

    pub async fn connect(&self) -> Result<(), OpcUaSimError> {
        self.set_state(ConnectionState::Connecting).await;
        self.log_request("Session", &format!("Connecting to {}", self.config.endpoint_url));

        // TODO: Task 8 will implement actual async-opcua session creation here.
        info!("Connecting to OPC UA server: {}", self.config.endpoint_url);

        self.set_state(ConnectionState::Connected).await;
        self.log_response("Session", "Connected", Some("Good"));
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), OpcUaSimError> {
        let mut tx_guard = self.shutdown_tx.write().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }

        self.set_state(ConnectionState::Disconnected).await;
        self.log_request("Session", "Disconnecting");
        self.log_response("Session", "Disconnected", Some("Good"));
        info!("Disconnected from: {}", self.config.endpoint_url);
        Ok(())
    }

    pub async fn start_reconnect_loop<F>(&self, on_state_change: F)
    where
        F: Fn(ConnectionState) + Send + Sync + 'static,
    {
        let state = self.state.clone();
        let reconnect_state = self.reconnect_state.clone();
        let policy = self.reconnect_policy.clone();
        let endpoint = self.config.endpoint_url.clone();

        let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
        {
            let mut tx_guard = self.shutdown_tx.write().await;
            *tx_guard = Some(tx);
        }

        tokio::spawn(async move {
            let mut attempt: u32 = 0;
            loop {
                if !policy.should_retry(attempt) {
                    *reconnect_state.write().await = ReconnectState::GaveUp;
                    warn!("Gave up reconnecting to {}", endpoint);
                    break;
                }

                *reconnect_state.write().await = ReconnectState::Reconnecting { attempt };
                *state.write().await = ConnectionState::Reconnecting;
                on_state_change(ConnectionState::Reconnecting);

                let delay = policy.delay_for_attempt(attempt);
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {}
                    _ = &mut rx => {
                        info!("Reconnect loop cancelled");
                        return;
                    }
                }

                // TODO: Task 8 will implement actual reconnection attempt.
                info!("Reconnect attempt {} to {}", attempt + 1, endpoint);
                attempt += 1;
            }
        });
    }
}
