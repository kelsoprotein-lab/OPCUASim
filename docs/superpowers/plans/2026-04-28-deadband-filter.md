# DataChangeFilter / Deadband Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 给 `opcuamaster-egui` 的订阅添加 OPC UA `DataChangeFilter` 支持(trigger + 绝对/百分比 deadband),客户端默认行为不变;过滤项随节点持久化到 `.opcuaproj`。

**Architecture:**
- `MonitoredNode`(opcusim-core)新增 `filter: Option<DataChangeFilterCfg>` 字段,`#[serde(default)]` 兼容老项目文件
- `SubscriptionManager::add_nodes` 不改签名,从 `node.filter` 透传到 `MonitoringParameters.filter`(包成 `ExtensionObject::from_message(DataChangeFilter)`)
- 主站 UI 端,`MonitoredNodeReq` 与 `AddVariablesUnderNode` 都加可选 filter,`browse_panel` 顶部"模式/间隔/深度"那一行下方增加一个 `CollapsingHeader::new("高级 (deadband)")` 默认折叠

**Tech Stack:** async-opcua-types (DataChangeFilter / ExtensionObject), egui CollapsingHeader / ComboBox / DragValue

---

## File Structure

| 文件 | 责任 |
|---|---|
| `crates/opcuasim-core/src/node.rs` (修改) | 加 `DataChangeFilterCfg` 与 `MonitoredNode.filter` 字段 |
| `crates/opcuasim-core/src/subscription.rs` (修改) | `add_nodes` 用 filter 构造 `MonitoredItemCreateRequest` |
| `crates/opcuamaster-egui/src/events.rs` (修改) | `MonitoredNodeReq` + `AddVariablesUnderNode` 加 filter,新 `DataChangeFilterReq` DTO |
| `crates/opcuamaster-egui/src/backend/dispatcher.rs` (修改) | 把 filter 从 Req → core MonitoredNode 透传 |
| `crates/opcuamaster-egui/src/model.rs` (修改) | `BrowseState` 加 `filter_*` UI state |
| `crates/opcuamaster-egui/src/panels/browse_panel.rs` (修改) | "高级"折叠区 + 把 filter 写进 MonitoredNodeReq |
| `crates/opcuamaster-egui/tests/e2e.rs` (修改) | 新加 `deadband_reduces_samples` 测试,断言带 deadband 的订阅样本数显著少 |

---

## Task 1: core — 给 MonitoredNode 加 filter 字段

**Files:**
- Modify: `crates/opcuasim-core/src/node.rs`

- [ ] **Step 1: 加 DataChangeFilterCfg 与 子枚举**

在 `pub enum AccessMode { ... }` 之后插入:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataChangeTriggerKind {
    Status,
    StatusValue,
    StatusValueTimestamp,
}

impl Default for DataChangeTriggerKind {
    fn default() -> Self {
        DataChangeTriggerKind::StatusValue
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadbandKind {
    None,
    Absolute,
    Percent,
}

impl Default for DeadbandKind {
    fn default() -> Self {
        DeadbandKind::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct DataChangeFilterCfg {
    #[serde(default)]
    pub trigger: DataChangeTriggerKind,
    #[serde(default)]
    pub deadband_kind: DeadbandKind,
    #[serde(default)]
    pub deadband_value: f64,
}
```

- [ ] **Step 2: 给 MonitoredNode 加 filter 字段**

修改 `pub struct MonitoredNode { ... }`,在 `pub user_access_level: u8,` 之后加:

```rust
    #[serde(default)]
    pub filter: Option<DataChangeFilterCfg>,
```

并在 `MonitoredNode::new` 末尾的字段初始化加 `filter: None,`。

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -10`
Expected: 成功(可能 warning,允许;但不允许 error)。

- [ ] **Step 4: 不 commit,留到 Task 2 一起**

---

## Task 2: core — subscription.rs 透传 filter

**Files:**
- Modify: `crates/opcuasim-core/src/subscription.rs`

当前 `add_nodes` 用 `nid.into()` 把 NodeId 隐式转成 `MonitoredItemCreateRequest`,默认 `filter` 为 null `ExtensionObject`。需要改成显式构造:对每个节点检查 `node.filter`,如有则填入 `MonitoringParameters.filter`。

- [ ] **Step 1: 加 use**

在 `use opcua_types::{...}` 行扩到:

```rust
use opcua_types::{
    DataChangeFilter, DataChangeTrigger, ExtensionObject, MonitoredItemCreateRequest,
    MonitoringMode, MonitoringParameters, NodeId, ReadValueId, TimestampsToReturn,
};
```

并在文件顶 use 区域加:

```rust
use crate::node::{DataChangeFilterCfg, DataChangeTriggerKind, DeadbandKind};
```

- [ ] **Step 2: 替换构造 items_to_create 的代码**

把当前:

```rust
let items_to_create: Vec<MonitoredItemCreateRequest> = nodes.iter()
    .filter_map(|n| {
        n.node_id.parse::<NodeId>().ok().map(|nid| nid.into())
    })
    .collect();
```

换成:

```rust
let items_to_create: Vec<MonitoredItemCreateRequest> = nodes
    .iter()
    .filter_map(|n| {
        let nid: NodeId = n.node_id.parse().ok()?;
        let interval_ms = match &n.access_mode {
            crate::node::AccessMode::Subscription { interval_ms } => *interval_ms,
            crate::node::AccessMode::Polling { .. } => 1000.0,
        };
        let filter_obj = n
            .filter
            .as_ref()
            .map(filter_cfg_to_extension_object)
            .unwrap_or_else(ExtensionObject::null);
        Some(MonitoredItemCreateRequest {
            item_to_monitor: ReadValueId {
                node_id: nid,
                attribute_id: opcua_types::AttributeId::Value as u32,
                index_range: opcua_types::NumericRange::None,
                data_encoding: opcua_types::QualifiedName::null(),
            },
            monitoring_mode: MonitoringMode::Reporting,
            requested_parameters: MonitoringParameters {
                client_handle: 0,
                sampling_interval: interval_ms,
                filter: filter_obj,
                queue_size: 1,
                discard_oldest: true,
            },
        })
    })
    .collect();
```

如类型路径 `opcua_types::AttributeId / NumericRange / QualifiedName` 的具体导入路径不对,在编译错误时再 hop 调整。

- [ ] **Step 3: 加辅助函数 filter_cfg_to_extension_object**

在文件末尾(`impl SubscriptionManager` 之外)加:

```rust
fn filter_cfg_to_extension_object(cfg: &DataChangeFilterCfg) -> ExtensionObject {
    let trigger = match cfg.trigger {
        DataChangeTriggerKind::Status => DataChangeTrigger::Status,
        DataChangeTriggerKind::StatusValue => DataChangeTrigger::StatusValue,
        DataChangeTriggerKind::StatusValueTimestamp => DataChangeTrigger::StatusValueTimestamp,
    };
    let deadband_type: u32 = match cfg.deadband_kind {
        DeadbandKind::None => 0,
        DeadbandKind::Absolute => 1,
        DeadbandKind::Percent => 2,
    };
    ExtensionObject::from_message(DataChangeFilter {
        trigger,
        deadband_type,
        deadband_value: cfg.deadband_value,
    })
}
```

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -25`
Expected: 成功;若 ReadValueId 字段名/类型路径有偏差,按编译错误提示纠正。

- [ ] **Step 5: 测试**

Run: `cargo test -p opcuasim-core 2>&1 | tail -20`
Expected: 已有测试全过(包括 cert_manager/discovery/...);新增 filter 字段不破坏序列化兼容性(`#[serde(default)]` 保证)。

- [ ] **Step 6: Commit**

```bash
git add crates/opcuasim-core/src/node.rs crates/opcuasim-core/src/subscription.rs
git commit -m "feat(core): DataChangeFilter / deadband on subscriptions

MonitoredNode gains an optional DataChangeFilterCfg (serde-default for
backward compat with older .opcuaproj files). SubscriptionManager wraps
it into a DataChangeFilter ExtensionObject and attaches to each
MonitoredItemCreateRequest. Default behavior unchanged when filter
is None."
```

---

## Task 3: master-egui — events.rs DTO

**Files:**
- Modify: `crates/opcuamaster-egui/src/events.rs`

- [ ] **Step 1: 加 DataChangeFilterReq DTO**

在文件末尾加:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataChangeTriggerKindReq {
    Status,
    StatusValue,
    StatusValueTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadbandKindReq {
    None,
    Absolute,
    Percent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DataChangeFilterReq {
    pub trigger: DataChangeTriggerKindReq,
    pub deadband_kind: DeadbandKindReq,
    pub deadband_value: f64,
}
```

- [ ] **Step 2: 给 MonitoredNodeReq 加 filter**

修改 `pub struct MonitoredNodeReq { ... }`,加字段:

```rust
    pub filter: Option<DataChangeFilterReq>,
```

- [ ] **Step 3: 给 AddVariablesUnderNode 加 filter**

修改 `UiCommand::AddVariablesUnderNode { ... }` 变体,加字段:

```rust
        filter: Option<DataChangeFilterReq>,
```

- [ ] **Step 4: 编译(预期出错)**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -20`
Expected: e2e 与 dispatcher 调用 `MonitoredNodeReq { ... }` 处缺字段,编译失败 — Task 4-6 修。

---

## Task 4: master-egui — dispatcher 透传 filter

**Files:**
- Modify: `crates/opcuamaster-egui/src/backend/dispatcher.rs`

- [ ] **Step 1: 加 use**

在 `use opcuasim_core::node::{AccessMode, MonitoredNode, NodeGroup};` 改为:

```rust
use opcuasim_core::node::{
    AccessMode, DataChangeFilterCfg, DataChangeTriggerKind, DeadbandKind, MonitoredNode,
    NodeGroup,
};
```

并在 `crate::events::{...}` 列表加:

```rust
DataChangeFilterReq, DataChangeTriggerKindReq, DeadbandKindReq,
```

- [ ] **Step 2: 加 filter 转换辅助**

在 `auth_label` 函数附近加:

```rust
fn filter_req_to_core(req: &DataChangeFilterReq) -> DataChangeFilterCfg {
    DataChangeFilterCfg {
        trigger: match req.trigger {
            DataChangeTriggerKindReq::Status => DataChangeTriggerKind::Status,
            DataChangeTriggerKindReq::StatusValue => DataChangeTriggerKind::StatusValue,
            DataChangeTriggerKindReq::StatusValueTimestamp => {
                DataChangeTriggerKind::StatusValueTimestamp
            }
        },
        deadband_kind: match req.deadband_kind {
            DeadbandKindReq::None => DeadbandKind::None,
            DeadbandKindReq::Absolute => DeadbandKind::Absolute,
            DeadbandKindReq::Percent => DeadbandKind::Percent,
        },
        deadband_value: req.deadband_value,
    }
}
```

- [ ] **Step 3: 在构造 MonitoredNode 的位置写 filter**

定位到 `add_monitored_nodes` 与 `add_variables_under_node` 中构造 `MonitoredNode { ... }` 的代码块。两处都把 `filter: None,` 替换为根据 req 来源:

```rust
filter: req.filter.as_ref().map(filter_req_to_core),
```

(具体位置待编译报错时定位;典型路径是 `add_monitored_nodes` 内对 `nodes` 迭代构造 MonitoredNode 时。)

- [ ] **Step 4: 修改 AddVariablesUnderNode 的 match 分支签名**

把:

```rust
UiCommand::AddVariablesUnderNode {
    conn_id,
    node_id,
    access_mode,
    interval_ms,
    max_depth,
} => { ... }
```

改成同时接收 `filter`,并把它一起传给 `add_variables_under_node`(同步改 helper 函数签名)。

- [ ] **Step 5: 编译**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -15`
Expected: 成功,或继续 hop 到 Task 5/6 的 model/UI 处。

---

## Task 5: master-egui — model.rs UI state

**Files:**
- Modify: `crates/opcuamaster-egui/src/model.rs`

- [ ] **Step 1: 给 BrowseState 加 filter 字段**

修改 `pub struct BrowseState`,在 `pub max_depth: u32,` 之后加:

```rust
    pub filter_enabled: bool,
    pub trigger: crate::events::DataChangeTriggerKindReq,
    pub deadband_kind: crate::events::DeadbandKindReq,
    pub deadband_value: f64,
```

并修改 `BrowseState` 的 `Default` 实现(如有);若是 `#[derive(Default)]`,需要改成手写 `impl Default`。检查文件,沿用现有风格。

- [ ] **Step 2: 在 model.rs 加 helper**

文件末尾加:

```rust
impl AppModel {
    pub fn current_filter_req(&self) -> Option<crate::events::DataChangeFilterReq> {
        if !self.browse.filter_enabled {
            return None;
        }
        Some(crate::events::DataChangeFilterReq {
            trigger: self.browse.trigger,
            deadband_kind: self.browse.deadband_kind,
            deadband_value: self.browse.deadband_value,
        })
    }
}
```

(如果 `impl AppModel { ... }` 已有,把 `current_filter_req` 加到现有 impl 里,不要重复 impl 块。)

---

## Task 6: master-egui — browse_panel.rs 折叠"高级"

**Files:**
- Modify: `crates/opcuamaster-egui/src/panels/browse_panel.rs`

- [ ] **Step 1: render_controls 加折叠区**

把当前 `render_controls` 函数的 `ui.horizontal(|ui| { ... })` 调用之后(同一函数内)加:

```rust
    egui::CollapsingHeader::new("高级 (DataChangeFilter)")
        .id_salt("browse_advanced_filter")
        .default_open(false)
        .show(ui, |ui| {
            ui.checkbox(&mut model.browse.filter_enabled, "启用 DataChangeFilter");
            ui.add_enabled_ui(model.browse.filter_enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Trigger:");
                    egui::ComboBox::from_id_salt("filter_trigger")
                        .selected_text(format!("{:?}", model.browse.trigger))
                        .show_ui(ui, |ui| {
                            for v in [
                                crate::events::DataChangeTriggerKindReq::Status,
                                crate::events::DataChangeTriggerKindReq::StatusValue,
                                crate::events::DataChangeTriggerKindReq::StatusValueTimestamp,
                            ] {
                                ui.selectable_value(&mut model.browse.trigger, v, format!("{v:?}"));
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Deadband:");
                    egui::ComboBox::from_id_salt("filter_deadband")
                        .selected_text(format!("{:?}", model.browse.deadband_kind))
                        .show_ui(ui, |ui| {
                            for v in [
                                crate::events::DeadbandKindReq::None,
                                crate::events::DeadbandKindReq::Absolute,
                                crate::events::DeadbandKindReq::Percent,
                            ] {
                                ui.selectable_value(
                                    &mut model.browse.deadband_kind,
                                    v,
                                    format!("{v:?}"),
                                );
                            }
                        });
                    ui.add_enabled(
                        !matches!(model.browse.deadband_kind, crate::events::DeadbandKindReq::None),
                        egui::DragValue::new(&mut model.browse.deadband_value)
                            .range(0.0..=100_000.0)
                            .speed(0.1),
                    );
                });
            });
        });
```

- [ ] **Step 2: render_footer 把 filter 塞进 MonitoredNodeReq**

把构造 `MonitoredNodeReq { ... }` 的字段块改为加上 `filter: model.current_filter_req(),`(注意 Rust 借用:可在外面先 `let filter_req = model.current_filter_req();`,再在 map 里 clone 进每行)。

具体改:

```rust
let filter_req = model.current_filter_req();
let nodes: Vec<MonitoredNodeReq> = model
    .browse
    .selected
    .iter()
    .filter_map(|nid| model.browse.nodes.get(nid).map(|st| (nid, st)))
    .map(|(nid, st)| MonitoredNodeReq {
        node_id: nid.clone(),
        display_name: st.item.display_name.clone(),
        data_type: st.item.data_type.clone(),
        access_mode: model.browse.access_mode.clone(),
        interval_ms: model.browse.interval_ms,
        filter: filter_req,
    })
    .collect();
```

- [ ] **Step 3: AddVariablesUnderNode 也加 filter**

在 `render_node` 里 `backend.send(UiCommand::AddVariablesUnderNode { ... })` 处,补 `filter: model.current_filter_req(),`。

- [ ] **Step 4: 编译 + clippy**

Run:
```bash
cargo build -p opcuamaster-egui 2>&1 | tail -10
cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10
```
Expected: 全过。

- [ ] **Step 5: 跑现有 e2e 不回归**

Run: `cargo test -p opcuamaster-egui --test e2e 2>&1 | tail -10`
Expected: 老 `master_full_flow` 仍然 PASS(老路径 filter=None,行为不变)。

---

## Task 7: e2e — deadband 减少样本数

**Files:**
- Modify: `crates/opcuamaster-egui/tests/e2e.rs`

- [ ] **Step 1: 加新测试**

在 `tests/e2e.rs` 文件末尾追加:

```rust
const DEADBAND_PORT: u16 = 48411;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn deadband_reduces_samples() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,opcua=warn"),
    )
    .is_test(true)
    .try_init();

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
        // amplitude=10, period=4s, sample interval=200ms -> 20 samples/period
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

    // Collect distinct numeric values for ~5 seconds (enough for >1 full sine period).
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

    // No deadband would yield ~25 distinct sine values across a 5s window
    // (sampling 200ms, 25 samples). With absolute deadband=5 against amplitude=10
    // (peak-to-peak 20), at most ~4 reports per period -> ~5 reports across 5s.
    // Use a loose bound: < 12 to allow for a couple of initial samples.
    assert!(
        distinct_values.len() <= 12,
        "expected deadband to suppress most samples, got {} distinct values across {} snapshots: {:?}",
        distinct_values.len(),
        snapshots_count,
        distinct_values,
    );
    assert!(
        distinct_values.len() >= 2,
        "expected at least 2 distinct values to confirm subscription is alive"
    );

    tokio::task::spawn_blocking(move || drop(backend))
        .await
        .expect("drop backend");
    tokio::time::sleep(Duration::from_millis(300)).await;
    server.stop().await.expect("server stop");
}
```

- [ ] **Step 2: 在 e2e.rs 顶部 use 区域加导入**

把 `opcuamaster_egui::events::{ ... }` 那行扩为:

```rust
use opcuamaster_egui::events::{
    AuthKindReq, BackendEvent, CreateConnectionReq, DataChangeFilterReq,
    DataChangeTriggerKindReq, DeadbandKindReq, MonitoredNodeReq, UiCommand,
};
```

并在原 `master_full_flow` 测试里所有构造 `MonitoredNodeReq { ... }` 的地方加 `filter: None,`(否则原测试编译失败,因为 Req 加了字段)。

- [ ] **Step 3: 跑两个测试**

Run: `cargo test -p opcuamaster-egui --test e2e 2>&1 | tail -15`
Expected: `master_full_flow` 与 `deadband_reduces_samples` 都 PASS。整个测试文件总耗时应在 15 秒内。

- [ ] **Step 4: clippy**

Run: `cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10`
Expected: 全过。

- [ ] **Step 5: Commit + push**

```bash
git add crates/opcuamaster-egui/src/events.rs \
        crates/opcuamaster-egui/src/backend/dispatcher.rs \
        crates/opcuamaster-egui/src/model.rs \
        crates/opcuamaster-egui/src/panels/browse_panel.rs \
        crates/opcuamaster-egui/tests/e2e.rs
git commit -m "feat(master): DataChangeFilter / deadband UI + e2e

BrowsePanel grows a collapsible 'high-level' section with trigger and
deadband controls. The selected filter rides along on AddMonitoredNodes
and AddVariablesUnderNode. New e2e test deadband_reduces_samples
subscribes Demo.Sine with absolute deadband=5 and asserts <=12 distinct
values reported across a 5s window."
git push origin master
```

---

## Self-Review

**Spec coverage:**
- §5.3 `SubscriptionManager::add_nodes` 扩参 → Task 2(没改签名,通过 `MonitoredNode.filter` 字段透传,等价语义)
- §5.3 `DataChangeFilterCfg { trigger, deadband_type, deadband_value }` DTO → Task 1
- §5.3 BrowsePanel "高级"折叠区 → Task 6
- §5.3 `MonitoredNodeReq` 扩字段 → Task 3
- §5.3 默认值无 filter → Task 1 `Option::None`,Task 2 fallback `ExtensionObject::null()`
- §4.2 e2e 自动化 → Task 7
- §4.3 沿用 dispatcher / Toast / serde 模式 → 全程

**Placeholder scan:** 无 TODO/TBD。Task 2 step 2 的 `ReadValueId` 字段构造若类型路径不准,在编译时再调整 — 不是占位,是已知的 hop。

**Type consistency:**
- `DataChangeFilterCfg`(core)字段:`trigger`/`deadband_kind`/`deadband_value`
- `DataChangeFilterReq`(events.rs)字段同名 + `*Req` 后缀的子枚举
- `filter_req_to_core`(dispatcher)1:1 映射
- `MonitoredNode.filter: Option<DataChangeFilterCfg>` 与 `MonitoredNodeReq.filter: Option<DataChangeFilterReq>` 一一对应
- `current_filter_req` helper 集中读 BrowseState UI 字段;`filter_enabled=false` 时返回 None,与 `DeadbandKind::None` 协同(关闭 filter 时不发请求字段,而非发"启用但 deadband=None")

**风险:** `MonitoringParameters` 结构与 `ReadValueId` 字段名/路径需要在编译期确认;若有偏差,先看 `cargo doc -p async-opcua-types --open` 或 grep registry 源码定位。

**Commit 数:** 2 条
1. Task 2: feat(core): DataChangeFilter / deadband on subscriptions(含 node.rs + subscription.rs)
2. Task 7: feat(master): DataChangeFilter / deadband UI + e2e(含 events / dispatcher / model / browse_panel / e2e)
