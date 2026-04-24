//! End-to-end integration test: spins up a real OPC UA server via
//! opcuasim-core, drives the Master dispatcher, and asserts that the
//! full channel plumbing (commands -> core -> events -> model) works.

use std::sync::Arc;
use std::time::Duration;

use opcuasim_core::server::models::{
    DataType, ServerConfig, ServerFolder, ServerNode, SimulationMode,
};
use opcuasim_core::server::server::OpcUaServer;

use opcuamaster_egui::events::{
    AuthKindReq, BackendEvent, CreateConnectionReq, MonitoredNodeReq, UiCommand,
};
use opcuamaster_egui::runtime::BackendHandle;
use tokio::sync::mpsc::UnboundedReceiver;

const TEST_PORT: u16 = 48410;

async fn recv_until<F>(
    rx: &mut UnboundedReceiver<BackendEvent>,
    timeout_secs: u64,
    saw_log: &mut bool,
    mut matcher: F,
) -> BackendEvent
where
    F: FnMut(&BackendEvent) -> bool,
{
    let fut = async {
        loop {
            let Some(ev) = rx.recv().await else {
                panic!("event channel closed before match");
            };
            if let BackendEvent::CommLogEntries { entries, .. } = &ev {
                if !entries.is_empty() {
                    *saw_log = true;
                }
            }
            if matcher(&ev) {
                return ev;
            }
        }
    };
    match tokio::time::timeout(Duration::from_secs(timeout_secs), fut).await {
        Ok(ev) => ev,
        Err(_) => panic!("timed out waiting for event after {timeout_secs}s"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn master_full_flow() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,opcua=warn"),
    )
    .is_test(true)
    .try_init();

    // --- Start a real OPC UA server ---
    let server = Arc::new(OpcUaServer::new());
    let config = ServerConfig {
        name: "E2ETestServer".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{TEST_PORT}"),
        port: TEST_PORT,
        security_policies: vec!["None".into()],
        security_modes: vec!["None".into()],
        users: Vec::new(),
        anonymous_enabled: true,
        max_sessions: 10,
        max_subscriptions_per_session: 10,
    };
    let folders = vec![ServerFolder {
        node_id: "Demo".into(),
        display_name: "Demo".into(),
        parent_id: "i=85".into(),
    }];
    let nodes = vec![
        ServerNode {
            node_id: "Demo.Sine".into(),
            display_name: "Sine".into(),
            parent_id: "Demo".into(),
            data_type: DataType::Double,
            writable: false,
            simulation: SimulationMode::Sine {
                amplitude: 10.0,
                offset: 0.0,
                period_ms: 4000,
                interval_ms: 200,
            },
            update_seq: 0,
            current_value: None,
        },
        ServerNode {
            node_id: "Demo.Setpoint".into(),
            display_name: "Setpoint".into(),
            parent_id: "Demo".into(),
            data_type: DataType::Double,
            writable: true,
            simulation: SimulationMode::Static { value: "0".into() },
            update_seq: 0,
            current_value: None,
        },
    ];
    server
        .start(&config, &folders, &nodes)
        .await
        .expect("server start");
    tokio::time::sleep(Duration::from_millis(800)).await;

    // --- Boot Master backend ---
    let ctx = egui::Context::default();
    let (backend, mut rx) =
        BackendHandle::new(ctx, "e2e-master", opcuamaster_egui::backend::dispatcher::run);

    // --- 1. CreateConnection ---
    backend.send(UiCommand::CreateConnection(CreateConnectionReq {
        name: "e2e".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{TEST_PORT}"),
        security_policy: "None".into(),
        security_mode: "None".into(),
        auth: AuthKindReq::Anonymous,
        timeout_ms: 5000,
    }));

    let mut saw_log = false;

    let conn_id = loop {
        let ev = recv_until(&mut rx, 5, &mut saw_log, |e| matches!(e, BackendEvent::Connections(_))).await;
        if let BackendEvent::Connections(list) = ev {
            if let Some(c) = list.into_iter().find(|c| c.name == "e2e") {
                break c.id;
            }
        }
    };

    // --- 2. Connect ---
    backend.send(UiCommand::Connect(conn_id.clone()));
    let _ = recv_until(&mut rx, 8, &mut saw_log, |e| {
        matches!(e, BackendEvent::ConnectionStateChanged { state, .. } if state == "Connected")
    })
    .await;

    // --- 3. BrowseRoot (which browses Objects i=85) -> find Demo ---
    backend.send(UiCommand::BrowseRoot {
        conn_id: conn_id.clone(),
        req_id: 1,
    });
    let root_ev = recv_until(&mut rx, 5, &mut saw_log, |e| {
        matches!(e, BackendEvent::BrowseResult { req_id: 1, .. })
    })
    .await;
    let BackendEvent::BrowseResult { items: root_items, .. } = root_ev else {
        unreachable!()
    };
    let demo = root_items
        .iter()
        .find(|i| i.display_name == "Demo")
        .unwrap_or_else(|| {
            panic!(
                "Demo folder not found under Objects; got: {:?}",
                root_items.iter().map(|i| &i.display_name).collect::<Vec<_>>()
            )
        });

    // --- 4. AddMonitoredNodes for Sine + Setpoint ---
    let _ = demo;
    backend.send(UiCommand::AddMonitoredNodes {
        conn_id: conn_id.clone(),
        nodes: vec![
            MonitoredNodeReq {
                node_id: "ns=2;s=Demo.Sine".into(),
                display_name: "Sine".into(),
                data_type: Some("Double".into()),
                access_mode: "Subscription".into(),
                interval_ms: 250.0,
            },
            MonitoredNodeReq {
                node_id: "ns=2;s=Demo.Setpoint".into(),
                display_name: "Setpoint".into(),
                data_type: Some("Double".into()),
                access_mode: "Subscription".into(),
                interval_ms: 250.0,
            },
        ],
    });

    // Wait for two monitored snapshots with different values -> verifies sim engine + subscription
    let mut seen_values: Vec<String> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tokio::time::Instant::now() < deadline && seen_values.len() < 3 {
        let ev = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .ok()
            .flatten();
        if let Some(BackendEvent::CommLogEntries { entries, .. }) = &ev {
            if !entries.is_empty() {
                saw_log = true;
            }
        }
        if let Some(BackendEvent::MonitoredSnapshot { nodes, .. }) = ev {
            for n in nodes {
                if n.node_id == "ns=2;s=Demo.Sine" {
                    if let Some(v) = n.value {
                        if !seen_values.contains(&v) {
                            seen_values.push(v);
                        }
                    }
                }
            }
        }
    }
    assert!(
        seen_values.len() >= 2,
        "expected at least 2 distinct Sine values, got {seen_values:?}"
    );

    // --- 5. WriteValue on Setpoint ---
    backend.send(UiCommand::WriteValue {
        conn_id: conn_id.clone(),
        node_id: "ns=2;s=Demo.Setpoint".into(),
        value: "42.5".into(),
        data_type: "Double".into(),
        req_id: 10,
    });
    let _ = recv_until(&mut rx, 5, &mut saw_log, |e| {
        matches!(e, BackendEvent::WriteOk { req_id: 10, .. })
    })
    .await;

    // --- 6. ReadAttrs on Setpoint returns 42.5 ---
    backend.send(UiCommand::AddMonitoredNodes {
        conn_id: conn_id.clone(),
        nodes: vec![MonitoredNodeReq {
            node_id: "ns=2;s=Demo.Setpoint".into(),
            display_name: "Setpoint".into(),
            data_type: Some("Double".into()),
            access_mode: "Subscription".into(),
            interval_ms: 500.0,
        }],
    });
    backend.send(UiCommand::ReadAttrs {
        conn_id: conn_id.clone(),
        node_id: "ns=2;s=Demo.Setpoint".into(),
        req_id: 11,
    });
    let attrs_ev = recv_until(&mut rx, 5, &mut saw_log, |e| {
        matches!(e, BackendEvent::NodeAttrs { req_id: 11, .. })
    })
    .await;
    let BackendEvent::NodeAttrs { attrs, .. } = attrs_ev else {
        unreachable!()
    };
    assert!(
        attrs.value.as_deref().unwrap_or("").contains("42.5"),
        "expected Setpoint to read back 42.5 after write, got {:?}",
        attrs.value
    );

    // --- 7. Ensure a CommLog batch was observed during the session ---
    // The log_timer fires every 1.5s; connect produces log entries and any recv_until above
    // captures them into saw_log. If we have not seen one yet, wait briefly for the next tick.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && !saw_log {
        let ev = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .ok()
            .flatten();
        if let Some(BackendEvent::CommLogEntries { entries, .. }) = ev {
            if !entries.is_empty() {
                saw_log = true;
            }
        }
    }
    assert!(saw_log, "expected at least one CommLog batch during session");

    // --- 8. Clean shutdown (drop BackendHandle off the async context) ---
    tokio::task::spawn_blocking(move || drop(backend))
        .await
        .expect("drop backend");
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.stop().await.expect("server stop");
}
