use serde::{Deserialize, Serialize};
use crate::node::{AccessMode, NodeGroup};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum AuthConfig {
    #[default]
    Anonymous,
    UserPassword { username: String, password: String },
    Certificate { cert_path: String, key_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthConfig,
    pub timeout_ms: u64,
}

impl ConnectionConfig {
    pub fn new(id: String, name: String, endpoint_url: String) -> Self {
        Self {
            id,
            name,
            endpoint_url,
            security_policy: "None".to_string(),
            security_mode: "None".to_string(),
            auth: AuthConfig::default(),
            timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredNodeConfig {
    pub node_id: String,
    pub display_name: String,
    pub access_mode: AccessMode,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProjectEntry {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthConfig,
    pub timeout_ms: u64,
    pub monitored_nodes: Vec<MonitoredNodeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    #[serde(rename = "type")]
    pub project_type: String,
    pub version: String,
    pub connections: Vec<ConnectionProjectEntry>,
    pub groups: Vec<NodeGroup>,
}

impl ProjectFile {
    pub fn new_master() -> Self {
        Self {
            project_type: "OpcUaMaster".to_string(),
            version: "0.1.0".to_string(),
            connections: vec![],
            groups: vec![],
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_file_roundtrip() {
        let mut project = ProjectFile::new_master();
        project.connections.push(ConnectionProjectEntry {
            name: "Test".to_string(),
            endpoint_url: "opc.tcp://localhost:4840".to_string(),
            security_policy: "None".to_string(),
            security_mode: "None".to_string(),
            auth: AuthConfig::Anonymous,
            timeout_ms: 5000,
            monitored_nodes: vec![MonitoredNodeConfig {
                node_id: "ns=2;s=Temperature".to_string(),
                display_name: "Temperature".to_string(),
                access_mode: AccessMode::Subscription { interval_ms: 1000.0 },
                group_id: None,
            }],
        });
        project.groups.push(NodeGroup {
            id: "g1".to_string(),
            name: "Group 1".to_string(),
            node_ids: vec!["ns=2;s=Temperature".to_string()],
        });

        let json = project.to_json().unwrap();
        let parsed = ProjectFile::from_json(&json).unwrap();
        assert_eq!(parsed.project_type, "OpcUaMaster");
        assert_eq!(parsed.connections.len(), 1);
        assert_eq!(parsed.connections[0].monitored_nodes[0].node_id, "ns=2;s=Temperature");
        assert_eq!(parsed.groups.len(), 1);
    }

    #[test]
    fn test_auth_config_variants() {
        let anon = serde_json::to_string(&AuthConfig::Anonymous).unwrap();
        assert!(anon.contains("Anonymous"));

        let user = serde_json::to_string(&AuthConfig::UserPassword {
            username: "admin".to_string(),
            password: "pass".to_string(),
        }).unwrap();
        assert!(user.contains("admin"));
    }
}
