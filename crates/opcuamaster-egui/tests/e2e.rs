//! End-to-end integration test: spins up a real OPC UA server via
//! opcuasim-core, drives the Master dispatcher, and asserts that the
//! full channel plumbing (commands -> core -> events -> model) works.

use std::sync::Arc;
use std::time::Duration;

use opcuasim_core::server::models::{
    DataType, ServerConfig, ServerFolder, ServerNode, SimulationMode,
};
use opcuasim_core::server::server::OpcUaServer;
use opcuasim_core::server::test_methods::register_demo_echo_method;

use opcuamaster_egui::events::{
    AuthKindReq, BackendEvent, CreateConnectionReq, DataChangeFilterReq, DataChangeTriggerKindReq,
    DeadbandKindReq, MethodArgValue, MonitoredNodeReq, UiCommand,
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

    let mut saw_log = false;

    // --- 0. DiscoverEndpoints ---
    backend.send(UiCommand::DiscoverEndpoints {
        url: format!("opc.tcp://127.0.0.1:{TEST_PORT}"),
        timeout_ms: 5000,
        req_id: 99,
    });
    let disc_ev = recv_until(&mut rx, 8, &mut saw_log, |e| {
        matches!(e, BackendEvent::EndpointsDiscovered { req_id: 99, .. })
    })
    .await;
    let BackendEvent::EndpointsDiscovered { endpoints, .. } = disc_ev else {
        unreachable!()
    };
    assert!(
        endpoints
            .iter()
            .any(|e| e.security_policy == "None" && e.security_mode == "None"),
        "expected a None/None endpoint, got {endpoints:?}"
    );

    // --- 1. CreateConnection ---
    backend.send(UiCommand::CreateConnection(CreateConnectionReq {
        name: "e2e".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{TEST_PORT}"),
        security_policy: "None".into(),
        security_mode: "None".into(),
        auth: AuthKindReq::Anonymous,
        timeout_ms: 5000,
    }));

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
                filter: None,
            },
            MonitoredNodeReq {
                node_id: "ns=2;s=Demo.Setpoint".into(),
                display_name: "Setpoint".into(),
                data_type: Some("Double".into()),
                access_mode: "Subscription".into(),
                interval_ms: 250.0,
                filter: None,
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
            filter: None,
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

const DEADBAND_PORT: u16 = 48411;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deadband_reduces_samples() {
    let server = Arc::new(OpcUaServer::new());
    let config = ServerConfig {
        name: "DeadbandTestServer".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{DEADBAND_PORT}"),
        port: DEADBAND_PORT,
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
    let nodes = vec![ServerNode {
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
    }];
    server.start(&config, &folders, &nodes).await.expect("server start");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let ctx = egui::Context::default();
    let (backend, mut rx) =
        BackendHandle::new(ctx, "deadband-master", opcuamaster_egui::backend::dispatcher::run);

    let mut saw_log = false;
    backend.send(UiCommand::CreateConnection(CreateConnectionReq {
        name: "deadband".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{DEADBAND_PORT}"),
        security_policy: "None".into(),
        security_mode: "None".into(),
        auth: AuthKindReq::Anonymous,
        timeout_ms: 5000,
    }));
    let conn_id = loop {
        let ev = recv_until(&mut rx, 5, &mut saw_log, |e| matches!(e, BackendEvent::Connections(_))).await;
        if let BackendEvent::Connections(list) = ev {
            if let Some(c) = list.into_iter().find(|c| c.name == "deadband") {
                break c.id;
            }
        }
    };
    backend.send(UiCommand::Connect(conn_id.clone()));
    let _ = recv_until(&mut rx, 8, &mut saw_log, |e| {
        matches!(e, BackendEvent::ConnectionStateChanged { state, .. } if state == "Connected")
    })
    .await;

    backend.send(UiCommand::AddMonitoredNodes {
        conn_id: conn_id.clone(),
        nodes: vec![MonitoredNodeReq {
            node_id: "ns=2;s=Demo.Sine".into(),
            display_name: "Sine".into(),
            data_type: Some("Double".into()),
            access_mode: "Subscription".into(),
            interval_ms: 200.0,
            filter: Some(DataChangeFilterReq {
                trigger: DataChangeTriggerKindReq::StatusValue,
                deadband_kind: DeadbandKindReq::Absolute,
                deadband_value: 5.0,
            }),
        }],
    });

    let mut distinct_values: std::collections::HashSet<String> = Default::default();
    let mut snapshots_count: usize = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let ev = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .ok()
            .flatten();
        if let Some(BackendEvent::MonitoredSnapshot { nodes, .. }) = ev {
            for n in nodes {
                if n.node_id == "ns=2;s=Demo.Sine" {
                    snapshots_count += 1;
                    if let Some(v) = n.value {
                        distinct_values.insert(v);
                    }
                }
            }
        }
    }

    assert!(
        distinct_values.len() <= 12,
        "expected deadband to suppress most samples, got {} distinct values across {} snapshots: {:?}",
        distinct_values.len(),
        snapshots_count,
        distinct_values,
    );
    assert!(
        distinct_values.len() >= 2,
        "expected at least 2 distinct values to confirm subscription is alive, got {distinct_values:?}"
    );

    tokio::task::spawn_blocking(move || drop(backend))
        .await
        .expect("drop backend");
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.stop().await.expect("server stop");
}

const ECHO_PORT: u16 = 48412;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn method_call_echo() {
    let server = Arc::new(OpcUaServer::new());
    let config = ServerConfig {
        name: "EchoTestServer".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{ECHO_PORT}"),
        port: ECHO_PORT,
        security_policies: vec!["None".into()],
        security_modes: vec!["None".into()],
        users: Vec::new(),
        anonymous_enabled: true,
        max_sessions: 10,
        max_subscriptions_per_session: 10,
    };
    server.start(&config, &[], &[]).await.expect("server start");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let method_id = register_demo_echo_method(&server)
        .await
        .expect("register echo");
    let method_id_str = format!("{method_id}");

    let ctx = egui::Context::default();
    let (backend, mut rx) =
        BackendHandle::new(ctx, "echo-master", opcuamaster_egui::backend::dispatcher::run);

    let mut saw_log = false;
    backend.send(UiCommand::CreateConnection(CreateConnectionReq {
        name: "echo".into(),
        endpoint_url: format!("opc.tcp://127.0.0.1:{ECHO_PORT}"),
        security_policy: "None".into(),
        security_mode: "None".into(),
        auth: AuthKindReq::Anonymous,
        timeout_ms: 5000,
    }));
    let conn_id = loop {
        let ev = recv_until(&mut rx, 5, &mut saw_log, |e| matches!(e, BackendEvent::Connections(_))).await;
        if let BackendEvent::Connections(list) = ev {
            if let Some(c) = list.into_iter().find(|c| c.name == "echo") {
                break c.id;
            }
        }
    };
    backend.send(UiCommand::Connect(conn_id.clone()));
    let _ = recv_until(&mut rx, 8, &mut saw_log, |e| {
        matches!(e, BackendEvent::ConnectionStateChanged { state, .. } if state == "Connected")
    })
    .await;

    backend.send(UiCommand::ReadMethodArgs {
        conn_id: conn_id.clone(),
        method_id: method_id_str.clone(),
        req_id: 30,
    });
    let args_ev = recv_until(&mut rx, 5, &mut saw_log, |e| {
        matches!(e, BackendEvent::MethodArgs { req_id: 30, .. })
    })
    .await;
    let BackendEvent::MethodArgs { inputs, .. } = args_ev else {
        unreachable!()
    };
    assert_eq!(inputs.len(), 1, "expected 1 input arg, got {inputs:?}");
    assert_eq!(inputs[0].name, "input");
    assert_eq!(inputs[0].data_type, "String");

    backend.send(UiCommand::CallMethod {
        conn_id: conn_id.clone(),
        object_id: "i=85".into(),
        method_id: method_id_str.clone(),
        inputs: vec![MethodArgValue {
            data_type: "String".into(),
            value: "hello".into(),
        }],
        req_id: 31,
    });
    let call_ev = recv_until(&mut rx, 5, &mut saw_log, |e| {
        matches!(e, BackendEvent::MethodCallResult { req_id: 31, .. })
    })
    .await;
    let BackendEvent::MethodCallResult { status, outputs, .. } = call_ev else {
        unreachable!()
    };
    assert!(status.contains("Good"), "expected Good status, got {status}");
    assert_eq!(outputs.len(), 1, "expected 1 output, got {outputs:?}");
    assert!(
        outputs[0].value.contains("hello"),
        "expected output to contain 'hello', got {:?}",
        outputs[0].value
    );

    tokio::task::spawn_blocking(move || drop(backend))
        .await
        .expect("drop backend");
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.stop().await.expect("server stop");
}
