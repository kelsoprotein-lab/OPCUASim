use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use log::{info, warn};

use opcua_client::{ClientBuilder, IdentityToken, Session};
use opcua_types::{
    EndpointDescription, MessageSecurityMode, UserTokenPolicy,
};

use crate::config::{AuthConfig, ConnectionConfig};
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
    session: Arc<RwLock<Option<Arc<Session>>>>,
    event_loop_handle: Arc<RwLock<Option<JoinHandle<opcua_types::StatusCode>>>>,
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
            session: Arc::new(RwLock::new(None)),
            event_loop_handle: Arc::new(RwLock::new(None)),
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

    /// Map config security_mode string to async-opcua MessageSecurityMode
    fn map_security_mode(mode: &str) -> MessageSecurityMode {
        match mode {
            "Sign" => MessageSecurityMode::Sign,
            "SignAndEncrypt" => MessageSecurityMode::SignAndEncrypt,
            _ => MessageSecurityMode::None,
        }
    }

    /// Map config security_policy string to the URI used by EndpointDescription
    fn map_security_policy_uri(policy: &str) -> &'static str {
        match policy {
            "Basic128Rsa15" => "http://opcfoundation.org/UA/SecurityPolicy#Basic128Rsa15",
            "Basic256" => "http://opcfoundation.org/UA/SecurityPolicy#Basic256",
            "Basic256Sha256" => "http://opcfoundation.org/UA/SecurityPolicy#Basic256Sha256",
            "Aes128_Sha256_RsaOaep" => "http://opcfoundation.org/UA/SecurityPolicy#Aes128_Sha256_RsaOaep",
            "Aes256_Sha256_RsaPss" => "http://opcfoundation.org/UA/SecurityPolicy#Aes256_Sha256_RsaPss",
            _ => "http://opcfoundation.org/UA/SecurityPolicy#None",
        }
    }

    /// Map AuthConfig to IdentityToken
    fn map_identity_token(auth: &AuthConfig) -> IdentityToken {
        match auth {
            AuthConfig::Anonymous => IdentityToken::Anonymous,
            AuthConfig::UserPassword { username, password } => {
                IdentityToken::new_user_name(username.clone(), password.clone())
            }
            AuthConfig::Certificate { cert_path, key_path } => {
                match IdentityToken::new_x509_path(cert_path, key_path) {
                    Ok(token) => token,
                    Err(e) => {
                        warn!("Failed to load X509 certificate: {}. Falling back to anonymous.", e);
                        IdentityToken::Anonymous
                    }
                }
            }
        }
    }

    pub async fn connect(&self) -> Result<(), OpcUaSimError> {
        self.set_state(ConnectionState::Connecting).await;
        self.log_request("Session", &format!("Connecting to {}", self.config.endpoint_url));

        info!("Connecting to OPC UA server: {}", self.config.endpoint_url);

        // Build client
        let mut client = ClientBuilder::new()
            .application_name("OPCUAMaster")
            .application_uri("urn:OPCUAMaster")
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .session_retry_limit(-1)
            .keep_alive_interval(std::time::Duration::from_secs(5))
            .request_timeout(std::time::Duration::from_secs(30))
            .max_array_length(65535)
            .max_message_size(64 * 1024 * 1024)
            .max_string_length(65535)
            .client()
            .map_err(|errs| OpcUaSimError::ConnectionFailed(errs.join("; ")))?;

        // Build endpoint
        let security_mode = Self::map_security_mode(&self.config.security_mode);
        let security_policy_uri = Self::map_security_policy_uri(&self.config.security_policy);
        let identity_token = Self::map_identity_token(&self.config.auth);

        let endpoint: EndpointDescription = (
            self.config.endpoint_url.as_str(),
            security_policy_uri,
            security_mode,
            UserTokenPolicy::anonymous(),
        ).into();

        // Try connect_to_matching_endpoint first (does endpoint discovery),
        // fall back to connect_to_endpoint_directly
        let connect_result = client
            .connect_to_matching_endpoint(endpoint.clone(), identity_token.clone())
            .await;

        let (session, event_loop) = match connect_result {
            Ok(result) => result,
            Err(e) => {
                info!("Endpoint discovery failed ({}), trying direct connection...", e);
                self.log_response("Session", &format!("Discovery failed: {}, trying direct...", e), None);
                client
                    .connect_to_endpoint_directly(endpoint, identity_token)
                    .map_err(|e| OpcUaSimError::ConnectionFailed(e.to_string()))?
            }
        };

        // Spawn the event loop
        let handle = event_loop.spawn();

        // Wait for connection with timeout
        let timeout_ms = self.config.timeout_ms.max(5000);
        let wait_result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            session.wait_for_connection(),
        ).await;

        if wait_result.is_err() {
            // Timeout — abort event loop and fail
            handle.abort();
            self.set_state(ConnectionState::Disconnected).await;
            let msg = format!("Connection timeout after {}ms to {}", timeout_ms, self.config.endpoint_url);
            self.log_response("Session", &msg, Some("BadTimeout"));
            return Err(OpcUaSimError::ConnectionFailed(msg));
        }

        // Store session and event loop handle
        {
            let mut s = self.session.write().await;
            *s = Some(session);
        }
        {
            let mut h = self.event_loop_handle.write().await;
            *h = Some(handle);
        }

        self.set_state(ConnectionState::Connected).await;
        self.log_response("Session", "Connected", Some("Good"));
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), OpcUaSimError> {
        // Cancel reconnect loop if running
        let mut tx_guard = self.shutdown_tx.write().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(());
        }
        drop(tx_guard);

        // Disconnect session
        let session_opt = {
            let s = self.session.read().await;
            s.clone()
        };
        if let Some(session) = session_opt {
            let _ = session.disconnect().await;
        }

        // Clear stored session
        {
            let mut s = self.session.write().await;
            *s = None;
        }

        // Abort event loop handle
        {
            let mut h = self.event_loop_handle.write().await;
            if let Some(handle) = h.take() {
                handle.abort();
            }
        }

        self.set_state(ConnectionState::Disconnected).await;
        self.log_request("Session", "Disconnecting");
        self.log_response("Session", "Disconnected", Some("Good"));
        info!("Disconnected from: {}", self.config.endpoint_url);
        Ok(())
    }

    /// Get the current session, if connected.
    pub async fn get_session(&self) -> Option<Arc<Session>> {
        self.session.read().await.clone()
    }

    /// Get a cheap clone of the session holder Arc, for use when you can't hold
    /// a std::sync::RwLock guard across an await point.
    pub fn get_session_holder(&self) -> Arc<RwLock<Option<Arc<Session>>>> {
        self.session.clone()
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

                info!("Reconnect attempt {} to {}", attempt + 1, endpoint);
                attempt += 1;
            }
        });
    }
}
