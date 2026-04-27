# Endpoint Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `opcuamaster-egui` 在 ConnectionDialog 内通过 OPC UA `GetEndpoints` 服务调用拉取目标服务器宣告的 endpoint 列表,用户可选定其中一行自动填充 SecurityPolicy / SecurityMode / 用户令牌策略,无需事先猜测。

**Architecture:**
- core: 新建 `opcuasim-core/src/discovery.rs`,提供 `discover_endpoints(url, timeout_ms) -> Result<Vec<DiscoveredEndpoint>>`,内部用一次性 `Client::get_server_endpoints_from_url` 完成,不复用 `OpcUaConnection`
- master-egui: 新增 `UiCommand::DiscoverEndpoints { url, timeout_ms, req_id }` 与 `BackendEvent::EndpointsDiscovered { req_id, endpoints }`(失败走现有 `Toast`),dispatcher 加 handler
- ConnectionDialog: URL 输入框旁新增"发现"按钮 → 命中后在对话框中段显示一张 endpoint 表;点选行时把 policy/mode 写回下半部已有字段;userTokenPolicy 也用来约束认证选项(scalar 改善体验,本期不做强约束)

**Tech Stack:** opcua-client 0.18, egui 0.34, egui_extras::TableBuilder

---

## File Structure

| 文件 | 责任 |
|---|---|
| `crates/opcuasim-core/src/discovery.rs` (新建) | 协议层:GetEndpoints 调用 + DTO |
| `crates/opcuasim-core/src/lib.rs` (修改) | 加 `pub mod discovery;` |
| `crates/opcuasim-core/tests/discovery.rs` (新建) | 单元测试:对内嵌 server 拉 endpoints |
| `crates/opcuamaster-egui/src/events.rs` (修改) | 新 UiCommand 变体 / BackendEvent 变体 / DTO |
| `crates/opcuamaster-egui/src/backend/dispatcher.rs` (修改) | DiscoverEndpoints handler |
| `crates/opcuamaster-egui/src/widgets/connection_dialog.rs` (修改) | URL 旁"发现"按钮 + 端点表;ConnDialogState 扩字段 |
| `crates/opcuamaster-egui/src/app.rs` (修改) | render_modal 把 EndpointsDiscovered 路由到 dialog state |
| `crates/opcuamaster-egui/src/model.rs` (修改) | 已有 `next_req_id` / `Modal::NewConnection`,无新结构,只在 dialog state 内扩 |
| `crates/opcuamaster-egui/tests/e2e.rs` (修改) | 在 `master_full_flow` 中追加 DiscoverEndpoints 断言 |

---

## Task 1: core — 新建 discovery 模块与 DTO

**Files:**
- Create: `crates/opcuasim-core/src/discovery.rs`
- Modify: `crates/opcuasim-core/src/lib.rs`

- [ ] **Step 1: 创建 discovery.rs 文件**

写入 `crates/opcuasim-core/src/discovery.rs`:

```rust
//! Endpoint discovery: thin wrapper around OPC UA `GetEndpoints`.
//!
//! Independent of `OpcUaConnection` — builds a one-shot client, asks the
//! target server for its advertised endpoints, then drops the client.

use std::time::Duration;

use log::info;
use opcua_client::ClientBuilder;
use opcua_types::{EndpointDescription, MessageSecurityMode};

use crate::error::OpcUaSimError;

/// Result of one discovery call. UI-friendly: all fields are owned strings.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredEndpoint {
    pub endpoint_url: String,
    pub security_policy_uri: String,
    /// Stripped tail of `security_policy_uri`, e.g. "Basic256Sha256".
    pub security_policy: String,
    /// "None" | "Sign" | "SignAndEncrypt"
    pub security_mode: String,
    pub security_level: u8,
    pub user_token_policies: Vec<DiscoveredUserToken>,
    /// Hex thumbprint of the server certificate, lowercase, no separators.
    /// Empty when no cert was offered (Security=None).
    pub server_cert_thumbprint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredUserToken {
    pub policy_id: String,
    pub token_type: String, // "Anonymous" | "UserName" | "Certificate" | "IssuedToken"
    pub security_policy_uri: String,
}

/// Issue a GetEndpoints to `url`, return the parsed list. Wraps the call in
/// a tokio timeout so a dead server fails fast.
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
    .map_err(|_| OpcUaSimError::ConnectionFailed(format!("Discovery timed out after {timeout_ms}ms")))?
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

/// Lowercase hex SHA-1 — uses `opcua_crypto`'s already-bundled openssl backend
/// (transitively via async-opcua-client) but we don't want to take a direct
/// dep, so do it via a tiny inline impl. SHA-1 is fine here: it's only an
/// identifier for UI display, not a security primitive.
fn sha1_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let digest = sha1_smol::Sha1::from(bytes).digest().bytes();
    let mut s = String::with_capacity(40);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}
```

- [ ] **Step 2: 加依赖 sha1_smol**

修改 `crates/opcuasim-core/Cargo.toml`,在 `[dependencies]` 段尾加:
```toml
sha1_smol = "1"
```

- [ ] **Step 3: 暴露模块**

修改 `crates/opcuasim-core/src/lib.rs`,在 `pub mod browse;` 下方插入:
```rust
pub mod discovery;
```

- [ ] **Step 4: 编译通过**

Run: `cargo build -p opcuasim-core`
Expected: 成功,无 warning。

- [ ] **Step 5: Commit**

```bash
git add crates/opcuasim-core/src/discovery.rs crates/opcuasim-core/src/lib.rs crates/opcuasim-core/Cargo.toml Cargo.lock
git commit -m "feat(core): add discovery module for GetEndpoints

discover_endpoints(url, timeout_ms) returns DiscoveredEndpoint list
mapped from OPC UA EndpointDescription. Builds a one-shot client and
issues GetEndpoints, with tokio timeout. Server certificate thumbprint
computed via SHA-1 for display only."
```

---

## Task 2: core — 单元测试 discovery 对内嵌 server

**Files:**
- Create: `crates/opcuasim-core/tests/discovery.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/opcuasim-core/tests/discovery.rs`:

```rust
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
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,opcua=warn"),
    )
    .is_test(true)
    .try_init();

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
```

- [ ] **Step 2: 跑测试,确认通过**

Run: `cargo test -p opcuasim-core --test discovery -- --nocapture`
Expected: PASS in <5s。

- [ ] **Step 3: Commit**

```bash
git add crates/opcuasim-core/tests/discovery.rs
git commit -m "test(core): cover discover_endpoints against embedded server

Asserts non-empty list, presence of None/None endpoint, and a
Basic256Sha256/Sign endpoint matching the server config."
```

---

## Task 3: master-egui — 扩 events.rs

**Files:**
- Modify: `crates/opcuamaster-egui/src/events.rs`

- [ ] **Step 1: 加 UiCommand 变体**

在 `pub enum UiCommand { ... }` 内,紧跟 `CreateConnection(CreateConnectionReq),` 之后插入:

```rust
    DiscoverEndpoints {
        url: String,
        timeout_ms: u64,
        req_id: u64,
    },
```

- [ ] **Step 2: 加 BackendEvent 变体 + DTO**

在 `pub enum BackendEvent { ... }` 内,在 `Toast { ... }` 之前插入:

```rust
    EndpointsDiscovered {
        req_id: u64,
        endpoints: Vec<DiscoveredEndpointDto>,
    },
```

文件末尾追加 DTO:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredEndpointDto {
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub security_level: u8,
    pub server_cert_thumbprint: String,
    pub user_token_policy_ids: Vec<String>,
}
```

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuamaster-egui`
Expected: FAIL — dispatcher 未处理新变体。这是预期。

- [ ] **Step 4: Commit(暂不 commit,留到 Task 4 一起)**

无 commit,留待下一任务 atomically 提交。

---

## Task 4: master-egui — dispatcher 实现 DiscoverEndpoints handler

**Files:**
- Modify: `crates/opcuamaster-egui/src/backend/dispatcher.rs`

- [ ] **Step 1: 加 use**

在文件顶部 `use crate::events::{...}` 内追加 `DiscoveredEndpointDto`,并加:
```rust
use opcuasim_core::discovery::discover_endpoints;
```

- [ ] **Step 2: 在 handle_cmd 的 match 中加分支**

在 `UiCommand::CreateConnection(req) => create_connection(req, &state, &event_tx).await,` 之后加:

```rust
        UiCommand::DiscoverEndpoints { url, timeout_ms, req_id } => {
            do_discover_endpoints(url, timeout_ms, req_id, &event_tx).await
        }
```

- [ ] **Step 3: 写 handler 函数**

在文件末尾(其它 async fn 同级)追加:

```rust
async fn do_discover_endpoints(
    url: String,
    timeout_ms: u64,
    req_id: u64,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    match discover_endpoints(&url, timeout_ms).await {
        Ok(list) => {
            let endpoints: Vec<DiscoveredEndpointDto> = list
                .into_iter()
                .map(|e| DiscoveredEndpointDto {
                    endpoint_url: e.endpoint_url,
                    security_policy: e.security_policy,
                    security_mode: e.security_mode,
                    security_level: e.security_level,
                    server_cert_thumbprint: e.server_cert_thumbprint,
                    user_token_policy_ids: e
                        .user_token_policies
                        .into_iter()
                        .map(|t| t.policy_id)
                        .collect(),
                })
                .collect();
            let _ = event_tx.send(BackendEvent::EndpointsDiscovered { req_id, endpoints });
            Ok(())
        }
        Err(e) => Err(format!("Discovery failed: {e}")),
    }
}
```

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuamaster-egui`
Expected: 成功(`app.rs` 收到 EndpointsDiscovered 事件还没处理 → match 将报 non-exhaustive,继续 step 5 修)。

- [ ] **Step 5: 在 app.rs apply_event 中临时吞掉新变体**

修改 `crates/opcuamaster-egui/src/app.rs`,在 `apply_event` 的 `match ev { ... }` 内,`Toast { ... }` 之前插入:

```rust
            BackendEvent::EndpointsDiscovered { req_id, endpoints } => {
                if let Some(crate::model::Modal::NewConnection(state)) = self.model.modal.as_mut() {
                    if state.discovery_req_id == Some(req_id) {
                        state.discovered = endpoints;
                        state.discovery_in_flight = false;
                        state.discovery_req_id = None;
                    }
                }
            }
```

(这一步引用 `discovered` / `discovery_in_flight` / `discovery_req_id`,Task 5 中将在 ConnDialogState 加上。先放这里,Task 5 完成后整体编译能过。)

- [ ] **Step 6: 暂时跳过编译验证,与 Task 5 atomically 完成**

无 commit。

---

## Task 5: master-egui — ConnectionDialog 加发现 UI

**Files:**
- Modify: `crates/opcuamaster-egui/src/widgets/connection_dialog.rs`

- [ ] **Step 1: 给 ConnDialogState 加字段**

修改 `pub struct ConnDialogState`,在 `pub error: Option<String>,` 之后加:

```rust
    pub discovery_in_flight: bool,
    pub discovery_req_id: Option<u64>,
    pub discovered: Vec<crate::events::DiscoveredEndpointDto>,
```

并修改 `Default` 实现,补上对应初始值:
```rust
            discovery_in_flight: false,
            discovery_req_id: None,
            discovered: Vec::new(),
```

- [ ] **Step 2: 把 show 函数签名改为返回结构**

当前 `show` 只回 `Option<CreateConnectionReq>`。需要新增"用户按了发现"信号。改签名为:

```rust
pub struct DialogActions {
    pub submit: Option<CreateConnectionReq>,
    /// User clicked "发现": (url, timeout_ms). UI assigns req_id and stores
    /// it on `state.discovery_req_id` after this returns.
    pub discover: Option<(String, u64)>,
}

pub fn show(
    ctx: &egui::Context,
    state: &mut ConnDialogState,
    close: &mut bool,
) -> DialogActions {
```

把当前 `let mut submitted: Option<CreateConnectionReq> = None;` 改成 `let mut actions = DialogActions { submit: None, discover: None };`,并把后续 `submitted = Some(req);` 改成 `actions.submit = Some(req);`,函数尾返回 `actions`。

- [ ] **Step 3: 加发现按钮**

在 `ui.label("Endpoint URL"); ui.text_edit_singleline(&mut state.endpoint_url); ui.end_row();` 这块,把 URL 一行改成:

```rust
                    ui.label("Endpoint URL");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.endpoint_url)
                                .desired_width(280.0),
                        );
                        let btn_label = if state.discovery_in_flight { "发现中…" } else { "发现" };
                        let btn = egui::Button::new(btn_label);
                        let resp = ui.add_enabled(!state.discovery_in_flight, btn);
                        if resp.clicked() {
                            actions.discover = Some((
                                state.endpoint_url.trim().to_string(),
                                state.timeout_ms,
                            ));
                        }
                    });
                    ui.end_row();
```

- [ ] **Step 4: 在对话框中段渲染端点表**

在 Grid 之后、`if let Some(err) = &state.error` 之前插入:

```rust
            if !state.discovered.is_empty() {
                ui.separator();
                ui.label(format!("发现的端点({}):", state.discovered.len()));
                egui_extras::TableBuilder::new(ui)
                    .id_salt("discovered_endpoints")
                    .striped(true)
                    .column(egui_extras::Column::auto().at_least(120.0)) // policy
                    .column(egui_extras::Column::auto().at_least(110.0)) // mode
                    .column(egui_extras::Column::auto().at_least(60.0))  // level
                    .column(egui_extras::Column::remainder().at_least(180.0)) // url
                    .header(20.0, |mut header| {
                        header.col(|ui| { ui.strong("Policy"); });
                        header.col(|ui| { ui.strong("Mode"); });
                        header.col(|ui| { ui.strong("Level"); });
                        header.col(|ui| { ui.strong("URL"); });
                    })
                    .body(|mut body| {
                        let rows: Vec<_> = state.discovered.clone();
                        for ep in rows {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    if ui.selectable_label(
                                        state.security_policy == ep.security_policy
                                            && state.security_mode == ep.security_mode,
                                        &ep.security_policy,
                                    ).clicked() {
                                        state.security_policy = ep.security_policy.clone();
                                        state.security_mode = ep.security_mode.clone();
                                        state.endpoint_url = ep.endpoint_url.clone();
                                    }
                                });
                                row.col(|ui| { ui.label(&ep.security_mode); });
                                row.col(|ui| { ui.label(format!("{}", ep.security_level)); });
                                row.col(|ui| { ui.label(&ep.endpoint_url); });
                            });
                        }
                    });
            }
```

- [ ] **Step 5: 让对话框宽度可变**

把 `.default_width(440.0)` 改为 `.min_width(440.0).default_width(640.0).resizable(true)`。

- [ ] **Step 6: 调整 toolbar 调用 / app.rs render_modal 适配新返回**

修改 `crates/opcuamaster-egui/src/app.rs` 的 render_modal:

```rust
            Modal::NewConnection(state) => {
                let mut close = false;
                let actions = connection_dialog::show(ctx, state, &mut close);
                if let Some(req) = actions.submit {
                    self.backend.send(UiCommand::CreateConnection(req));
                }
                if let Some((url, timeout_ms)) = actions.discover {
                    if !url.is_empty() {
                        let req_id = self.model.alloc_req_id();
                        state.discovery_in_flight = true;
                        state.discovery_req_id = Some(req_id);
                        state.discovered.clear();
                        state.error = None;
                        self.backend.send(UiCommand::DiscoverEndpoints {
                            url,
                            timeout_ms,
                            req_id,
                        });
                    }
                }
                if close {
                    self.model.modal = None;
                }
            }
```

注意 `self.model.alloc_req_id()` 必须在借用 `state` 之前/之后保证 NLL 满足。当前 `let Some(modal) = &mut self.model.modal else { return };` 已借走 `self.model`,需要在该 fn 顶部一开始 `let req_id_alloc: Box<dyn FnMut() -> u64 + '_> = ...;` 之类。简化做法:把 `next_req_id` 改成在 `Modal::NewConnection(state)` arm 内直接操作:把 `next_req_id` 字段从 model 拷一份本地,处理结束写回。或者更简单:把 `next_req_id` 提到 `AppModel` 之外用 `Cell<u64>`。

最干净的实现:在 render_modal 顶部一次性 take 出 modal,处理完再放回:

```rust
fn render_modal(&mut self, ctx: &egui::Context) {
    let Some(mut modal) = self.model.modal.take() else { return };
    match &mut modal {
        Modal::NewConnection(state) => {
            let mut close = false;
            let actions = connection_dialog::show(ctx, state, &mut close);
            if let Some(req) = actions.submit {
                self.backend.send(UiCommand::CreateConnection(req));
            }
            if let Some((url, timeout_ms)) = actions.discover {
                if !url.is_empty() {
                    let req_id = self.model.alloc_req_id();
                    state.discovery_in_flight = true;
                    state.discovery_req_id = Some(req_id);
                    state.discovered.clear();
                    state.error = None;
                    self.backend.send(UiCommand::DiscoverEndpoints {
                        url,
                        timeout_ms,
                        req_id,
                    });
                }
            }
            if !close {
                self.model.modal = Some(modal);
            }
        }
    }
}
```

- [ ] **Step 7: 编译**

Run: `cargo build -p opcuamaster-egui`
Expected: 成功,无 warning。

- [ ] **Step 8: 启动手测**

Run(可选,只用于自验): `cargo run -p opcuamaster-egui` 后:
1. 在另一终端启动 server(用 e2e fixture 或随便外部 server),例如 `cargo test -p opcuasim-core --test discovery -- --nocapture`(测试运行期间 server 在线)
2. 主站点"新建连接",URL 填测试 URL,点"发现"
3. 期望:几百毫秒内表里出现 None/None 与 Basic256Sha256/Sign 等行;点行后下方 policy/mode 自动改

(Auto mode 中可跳过手测,直接跑 e2e。)

- [ ] **Step 9: Commit**

```bash
git add crates/opcuamaster-egui/src/events.rs \
        crates/opcuamaster-egui/src/backend/dispatcher.rs \
        crates/opcuamaster-egui/src/widgets/connection_dialog.rs \
        crates/opcuamaster-egui/src/app.rs
git commit -m "feat(master): endpoint discovery in connection dialog

UiCommand::DiscoverEndpoints triggers core::discovery, result rendered
as a selectable table inside the New Connection dialog. Clicking a row
fills the URL/SecurityPolicy/SecurityMode fields. Failures surface as
toasts via the existing dispatcher error path."
```

---

## Task 6: e2e — 在 master_full_flow 里加 DiscoverEndpoints 断言

**Files:**
- Modify: `crates/opcuamaster-egui/tests/e2e.rs`

- [ ] **Step 1: 在 CreateConnection 之前插入发现步骤**

在 `// --- 1. CreateConnection ---` 之前插入新一段:

```rust
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
        endpoints.iter().any(|e| e.security_policy == "None" && e.security_mode == "None"),
        "expected a None/None endpoint, got {endpoints:?}"
    );
```

注意:`saw_log` 必须在该步骤之前已声明。检查目前位置:`let mut saw_log = false;` 是在 CreateConnection 后才声明的,需要把它的声明提前到 `--- 0` 之前。

具体改法:

```rust
    let mut saw_log = false;

    // --- 0. DiscoverEndpoints ---
    backend.send(UiCommand::DiscoverEndpoints { ... });
    ...

    // --- 1. CreateConnection ---
    backend.send(UiCommand::CreateConnection(...));
    ...

    let conn_id = loop { ... };  // 后面的 let mut saw_log = false; 删除
```

- [ ] **Step 2: 跑 e2e**

Run: `cargo test -p opcuamaster-egui --test e2e -- --nocapture`
Expected: PASS,日志中能看到 EndpointsDiscovered 与原有 6 步全部通过。

- [ ] **Step 3: clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 无错。

- [ ] **Step 4: Commit + push**

```bash
git add crates/opcuamaster-egui/tests/e2e.rs
git commit -m "test(master-egui): cover DiscoverEndpoints in e2e flow"
git push origin master
```

---

## Self-Review

(填写于 plan 起草后,实施前请勿修改)

**Spec coverage:**
- §5.1 core API `discover_endpoints` → Task 1
- §5.1 EndpointInfo DTO → Task 1 (DiscoveredEndpoint) + Task 3 (DiscoveredEndpointDto)
- §5.1 UI 改动:URL 旁"发现"按钮 + 端点表 → Task 5
- §5.1 用户点选行自动填充 → Task 5 step 4
- §5.1 保留手动模式 → 满足:不点发现也能直接填,流程不变
- §4.2 测试策略 e2e 自动化 → Task 6
- §4.3 沿用 dispatcher 模式 → Task 4
- §4.3 沿用 CommLog → 注:本 plan 未在 dispatcher 显式调 log_request/log_response。这是因为 discover_endpoints 不属于某条 connection 的会话日志(它先于 connection 存在)。为保持简单,讨论范围内不写 CommLog;如需要后续补一条 system 通道。

**Placeholder scan:** 无 TBD/TODO。Task 5 Step 6 描述 NLL 借用问题给了完整 take/put-back 方案。

**Type consistency:** `DiscoveredEndpoint`(core)与 `DiscoveredEndpointDto`(UI)字段相同名+不同类型嵌套(token policies 在 UI 简化为 `Vec<String>`),映射在 dispatcher Task 4 step 3 中明确。`discovery_req_id: Option<u64>` 一致。`alloc_req_id` 在 model.rs:24 已存在。

**Commit 总览:**
1. Task 1 - feat(core): add discovery module
2. Task 2 - test(core): cover discover_endpoints
3. Task 5 - feat(master): endpoint discovery in connection dialog (含 events/dispatcher/dialog/app 一组)
4. Task 6 - test(master-egui): cover DiscoverEndpoints in e2e flow

5 个文件 4 个 commit,与 spec §6 "每子项目一条独立 commit"基本一致(本子项目分了 4 条相关 commit,每条都自包含可编译测试)。
