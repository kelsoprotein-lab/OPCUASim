//! Endpoint discovery: thin wrapper around OPC UA `GetEndpoints`.
//!
//! Independent of `OpcUaConnection` — builds a one-shot client, asks the
//! target server for its advertised endpoints, then drops the client.

use std::time::Duration;

use log::info;
use opcua_client::ClientBuilder;
use opcua_types::{EndpointDescription, MessageSecurityMode};

use crate::error::OpcUaSimError;

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredEndpoint {
    pub endpoint_url: String,
    pub security_policy_uri: String,
    pub security_policy: String,
    pub security_mode: String,
    pub security_level: u8,
    pub user_token_policies: Vec<DiscoveredUserToken>,
    pub server_cert_thumbprint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredUserToken {
    pub policy_id: String,
    pub token_type: String,
    pub security_policy_uri: String,
}

pub async fn discover_endpoints(
    url: &str,
    timeout_ms: u64,
) -> Result<Vec<DiscoveredEndpoint>, OpcUaSimError> {
    info!("Discovering endpoints at {}", url);
    let client = ClientBuilder::new()
        .application_name("OPCUAMaster Discovery")
        .application_uri("urn:OPCUAMaster:Discovery")
        .create_sample_keypair(true)
        .trust_server_certs(true)
        .session_retry_limit(0)
        .request_timeout(Duration::from_millis(timeout_ms))
        .client()
        .map_err(|errs| OpcUaSimError::ConnectionFailed(errs.join("; ")))?;

    let raw = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        client.get_server_endpoints_from_url(url.to_string()),
    )
    .await
    .map_err(|_| {
        OpcUaSimError::ConnectionFailed(format!("Discovery timed out after {timeout_ms}ms"))
    })?
    .map_err(|e| OpcUaSimError::ConnectionFailed(format!("GetEndpoints failed: {e}")))?;

    Ok(raw.into_iter().map(map_endpoint).collect())
}

fn map_endpoint(e: EndpointDescription) -> DiscoveredEndpoint {
    let policy_uri = e.security_policy_uri.to_string();
    let policy = policy_uri
        .rsplit_once('#')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_else(|| policy_uri.clone());
    let mode = match e.security_mode {
        MessageSecurityMode::None => "None",
        MessageSecurityMode::Sign => "Sign",
        MessageSecurityMode::SignAndEncrypt => "SignAndEncrypt",
        _ => "Invalid",
    };
    let user_token_policies = e
        .user_identity_tokens
        .unwrap_or_default()
        .into_iter()
        .map(|t| DiscoveredUserToken {
            policy_id: t.policy_id.to_string(),
            token_type: format!("{:?}", t.token_type),
            security_policy_uri: t.security_policy_uri.to_string(),
        })
        .collect();
    let cert_bytes = e.server_certificate.value.unwrap_or_default();
    let server_cert_thumbprint = if cert_bytes.is_empty() {
        String::new()
    } else {
        sha1_hex(&cert_bytes)
    };
    DiscoveredEndpoint {
        endpoint_url: e.endpoint_url.to_string(),
        security_policy_uri: policy_uri,
        security_policy: policy,
        security_mode: mode.to_string(),
        security_level: e.security_level,
        user_token_policies,
        server_cert_thumbprint,
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = sha1_smol::Sha1::from(bytes).digest().bytes();
    let mut s = String::with_capacity(40);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}
