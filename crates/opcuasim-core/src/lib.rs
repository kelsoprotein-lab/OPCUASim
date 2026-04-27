pub mod error;
pub mod node;       // MonitoredNode, NodeGroup, BrowseResultItem, NodeAttributes
pub mod config;     // ConnectionConfig, AuthConfig, ProjectFile (includes security config)
pub mod output;
pub mod log_entry;
pub mod log_collector;
pub mod reconnect;
pub mod client;
pub mod browse;
pub mod cert_manager;
pub mod discovery;
pub mod history;
pub mod method;
pub mod subscription;
pub mod polling;
pub mod server;     // OPC UA server simulation module

/// Re-export the OPC UA Session type for downstream crates.
pub use opcua_client::Session as OpcUaSession;
