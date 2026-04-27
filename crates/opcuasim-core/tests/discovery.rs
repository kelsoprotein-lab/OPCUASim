//! Unit test for discovery module: spin up the embedded OpcUaServer,
//! call discover_endpoints(), assert non-empty + None policy present.

use std::sync::Arc;
use std::time::Duration;

use opcuasim_core::discovery::discover_endpoints;
use opcuasim_core::server::models::ServerConfig;
use opcuasim_core::server::server::OpcUaServer;

const TEST_PORT: u16 = 48420;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lists_advertised_endpoints() {
    let server = Arc::new(OpcUaServer::new());
    let config = ServerConfig {
        name: "DiscoveryTestServer".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{TEST_PORT}"),
        port: TEST_PORT,
        security_policies: vec!["None".into(), "Basic256Sha256".into()],
        security_modes: vec!["None".into(), "Sign".into(), "SignAndEncrypt".into()],
        users: Vec::new(),
        anonymous_enabled: true,
        max_sessions: 10,
        max_subscriptions_per_session: 10,
    };
    server.start(&config, &[], &[]).await.expect("server start");
    tokio::time::sleep(Duration::from_millis(800)).await;

    let endpoints = discover_endpoints(&format!("opc.tcp://127.0.0.1:{TEST_PORT}"), 5000)
        .await
        .expect("discover ok");

    assert!(!endpoints.is_empty(), "expected at least one endpoint");
    let none_ep = endpoints
        .iter()
        .find(|e| e.security_policy == "None" && e.security_mode == "None")
        .expect("expected a Security=None endpoint");
    assert!(
        !none_ep.user_token_policies.is_empty(),
        "endpoint should advertise user token policies"
    );
    assert!(
        endpoints
            .iter()
            .any(|e| e.security_policy == "Basic256Sha256" && e.security_mode == "Sign"),
        "expected Basic256Sha256/Sign endpoint, got {:?}",
        endpoints
            .iter()
            .map(|e| format!("{}/{}", e.security_policy, e.security_mode))
            .collect::<Vec<_>>()
    );

    server.stop().await.expect("server stop");
}
