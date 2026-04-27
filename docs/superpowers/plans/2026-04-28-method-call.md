# Method Call Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 浏览到 OPC UA Method 节点时,master 的 BrowsePanel 显示一个图标并支持右键"调用…",弹对话框,按 InputArguments 渲染输入控件,执行后展示 OutputArguments。`opcusim-core` 提供 `call_method` + `read_method_arguments` 客户端 API,并支持在测试 fixture 中注册 `Demo.Echo(input: String) -> output: String` 用于 e2e 验证。

**Architecture:**
- core 端新增 `method.rs`:`call_method(session, object_id, method_id, inputs) -> Vec<Variant>`,以及 `read_method_arguments(session, method_id) -> MethodArgumentsInfo`(读 Method 子节点 InputArguments / OutputArguments 的 Value)
- core 端新增 `register_demo_echo_method(server)`(test-only API),通过 `MethodBuilder` + `simple_node_manager.add_method_callback` 注册回显方法
- master-egui:`UiCommand::ReadMethodArgs` / `UiCommand::CallMethod` + `BackendEvent::MethodArgs` / `MethodCallResult`;新 `Modal::MethodCall(MethodCallState)` + 对话框
- BrowsePanel:Method 节点用 ⚙ 图标,右键菜单加"调用..."

**Tech Stack:** async-opcua-client `Session::call`, async-opcua-nodes `MethodBuilder`, `Argument` ExtensionObject 解码

---

## File Structure

| 文件 | 责任 |
|---|---|
| `crates/opcuasim-core/src/method.rs` (新建) | call_method + read_method_arguments |
| `crates/opcuasim-core/src/lib.rs` (修改) | `pub mod method;` |
| `crates/opcuasim-core/src/server/test_methods.rs` (新建) | `register_demo_echo_method(server)` test helper |
| `crates/opcuasim-core/src/server/mod.rs` (修改) | `pub mod test_methods;` |
| `crates/opcuamaster-egui/src/events.rs` (修改) | UiCommand × 2 + BackendEvent × 2 + DTO |
| `crates/opcuamaster-egui/src/backend/dispatcher.rs` (修改) | 2 个 handler + Variant 解码 |
| `crates/opcuamaster-egui/src/widgets/method_call_dialog.rs` (新建) | 输入参数表单 + 输出展示 |
| `crates/opcuamaster-egui/src/widgets/mod.rs` (修改) | 暴露新模块 |
| `crates/opcuamaster-egui/src/model.rs` (修改) | `Modal::MethodCall` + `MethodCallState` |
| `crates/opcuamaster-egui/src/panels/browse_panel.rs` (修改) | Method 图标 + 右键菜单 |
| `crates/opcuamaster-egui/src/app.rs` (修改) | 路由新事件 / 渲染新 modal |
| `crates/opcuamaster-egui/tests/e2e.rs` (修改) | 新增 `method_call_echo` 测试 |

---

## Task 1: core — method.rs

**Files:**
- Create: `crates/opcuasim-core/src/method.rs`
- Modify: `crates/opcuasim-core/src/lib.rs`

- [ ] **Step 1: 创建 method.rs**

写入 `crates/opcuasim-core/src/method.rs`:

```rust
//! Method service helpers: read InputArguments / OutputArguments Property
//! nodes of a Method, then call the method.

use std::sync::Arc;

use opcua_client::Session;
use opcua_types::{
    Argument, AttributeId, BrowseDescription, BrowseDirection, BrowseResultMask, CallMethodRequest,
    DataValue, ExtensionObject, NodeClassMask, NodeId, ReadValueId, ReferenceTypeId, StatusCode,
    TimestampsToReturn, Variant,
};

use crate::error::OpcUaSimError;

#[derive(Debug, Clone, Default)]
pub struct ArgumentInfo {
    pub name: String,
    /// Display string for the argument's data type (e.g. "String", "Double").
    pub data_type: String,
    /// Raw OPC UA DataType NodeId.
    pub data_type_id: NodeId,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct MethodArgumentsInfo {
    pub inputs: Vec<ArgumentInfo>,
    pub outputs: Vec<ArgumentInfo>,
}

/// Browse the given Method node for HasProperty children named
/// "InputArguments" and "OutputArguments", read each one's Value attribute,
/// and decode the Argument[] payload.
pub async fn read_method_arguments(
    session: &Arc<Session>,
    method_id: &NodeId,
) -> Result<MethodArgumentsInfo, OpcUaSimError> {
    let browse = vec![BrowseDescription {
        node_id: method_id.clone(),
        browse_direction: BrowseDirection::Forward,
        reference_type_id: ReferenceTypeId::HasProperty.into(),
        include_subtypes: true,
        node_class_mask: NodeClassMask::VARIABLE.bits(),
        result_mask: BrowseResultMask::All as u32,
    }];

    let results = session
        .browse(browse, 0, None)
        .await
        .map_err(|e| OpcUaSimError::SubscriptionError(format!("Browse method props failed: {e}")))?;

    let mut input_id: Option<NodeId> = None;
    let mut output_id: Option<NodeId> = None;
    for r in results {
        if let Some(refs) = r.references {
            for rf in refs {
                let name = rf.browse_name.name.to_string();
                if name == "InputArguments" {
                    input_id = Some(rf.node_id.node_id.clone());
                } else if name == "OutputArguments" {
                    output_id = Some(rf.node_id.node_id.clone());
                }
            }
        }
    }

    let inputs = match input_id {
        Some(id) => decode_argument_array(session, &id).await?,
        None => Vec::new(),
    };
    let outputs = match output_id {
        Some(id) => decode_argument_array(session, &id).await?,
        None => Vec::new(),
    };

    Ok(MethodArgumentsInfo { inputs, outputs })
}

async fn decode_argument_array(
    session: &Arc<Session>,
    args_node_id: &NodeId,
) -> Result<Vec<ArgumentInfo>, OpcUaSimError> {
    let read_request = vec![ReadValueId {
        node_id: args_node_id.clone(),
        attribute_id: AttributeId::Value as u32,
        index_range: opcua_types::NumericRange::None,
        data_encoding: opcua_types::QualifiedName::null(),
    }];
    let dvs: Vec<DataValue> = session
        .read(&read_request, TimestampsToReturn::Neither, 0.0)
        .await
        .map_err(|e| OpcUaSimError::SubscriptionError(format!("Read args failed: {e}")))?;

    let dv = dvs.into_iter().next().ok_or_else(|| {
        OpcUaSimError::SubscriptionError("Read args returned no DataValue".into())
    })?;
    let value = dv.value.unwrap_or(Variant::Empty);

    let extension_objects: Vec<ExtensionObject> = match value {
        Variant::Array(arr) => arr
            .values
            .into_iter()
            .filter_map(|v| match v {
                Variant::ExtensionObject(eo) => Some(*eo),
                _ => None,
            })
            .collect(),
        Variant::ExtensionObject(eo) => vec![*eo],
        _ => return Ok(Vec::new()),
    };

    let mut out = Vec::with_capacity(extension_objects.len());
    for eo in extension_objects {
        if let Some(arg) = eo.into_inner_as::<Argument>() {
            out.push(ArgumentInfo {
                name: arg.name.to_string(),
                data_type_id: arg.data_type.clone(),
                data_type: data_type_label(&arg.data_type),
                description: arg.description.text.to_string(),
            });
        }
    }
    Ok(out)
}

fn data_type_label(id: &NodeId) -> String {
    use opcua_types::DataTypeId;
    if let Ok(d) = DataTypeId::try_from(id) {
        format!("{d:?}")
    } else {
        format!("{id}")
    }
}

#[derive(Debug, Clone)]
pub struct MethodCallOutcome {
    pub status: StatusCode,
    pub outputs: Vec<Variant>,
}

pub async fn call_method(
    session: &Arc<Session>,
    object_id: &NodeId,
    method_id: &NodeId,
    inputs: Vec<Variant>,
) -> Result<MethodCallOutcome, OpcUaSimError> {
    let req: CallMethodRequest = (object_id.clone(), method_id.clone(), Some(inputs)).into();
    let result = session
        .call_one(req)
        .await
        .map_err(|e| OpcUaSimError::ConnectionFailed(format!("Call failed: {e}")))?;
    Ok(MethodCallOutcome {
        status: result.status_code,
        outputs: result.output_arguments.unwrap_or_default(),
    })
}
```

注:`BrowseDescription / NodeClassMask / BrowseResultMask` 等类型路径需要验证;若不通,在编译期 hop。`ExtensionObject::into_inner_as::<Argument>` 是已确认存在的 API(见 extension_object.rs)。

- [ ] **Step 2: 暴露模块**

修改 `crates/opcuasim-core/src/lib.rs`,在 `pub mod cert_manager;` 之后加:

```rust
pub mod method;
```

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -25`
Expected: 成功;若类型路径有偏差,按报错信息一一调整。

---

## Task 2: core — server 端 echo method 注册器

**Files:**
- Create: `crates/opcuasim-core/src/server/test_methods.rs`
- Modify: `crates/opcuasim-core/src/server/mod.rs`

- [ ] **Step 1: 创建 test_methods.rs**

写入 `crates/opcuasim-core/src/server/test_methods.rs`:

```rust
//! Test-only address-space helpers: register a callable Demo.Echo method
//! that echoes a single String input. Not used in production startup.

use std::sync::Arc;

use opcua_nodes::MethodBuilder;
use opcua_server::node_manager::memory::SimpleNodeManager;
use opcua_types::{
    Argument, DataTypeId, LocalizedText, NodeId, ObjectId, StatusCode, UAString, Variant,
};

use crate::error::OpcUaSimError;
use crate::server::server::OpcUaServer;

/// Register Demo.Echo as a child of Server (i=2253) in the server's namespace.
/// Method node id: ns=2;s=Demo.Echo
/// Returns the method node id for caller convenience.
pub async fn register_demo_echo_method(server: &OpcUaServer) -> Result<NodeId, OpcUaSimError> {
    let nm = server
        .node_manager()
        .await
        .ok_or_else(|| OpcUaSimError::ServerError("Server not started".into()))?;
    let ns = server.namespace_index().await;

    let parent_node_id: NodeId = ObjectId::ObjectsFolder.into();
    let method_id = NodeId::new(ns, "Demo.Echo");
    let in_args_id = NodeId::new(ns, "Demo.Echo.InputArguments");
    let out_args_id = NodeId::new(ns, "Demo.Echo.OutputArguments");

    {
        let mut addr = nm.address_space().write();
        let _ = MethodBuilder::new(&method_id, "Echo", "Echo")
            .component_of(parent_node_id)
            .executable(true)
            .user_executable(true)
            .input_args(
                &mut *addr,
                &in_args_id,
                &[Argument {
                    name: UAString::from("input"),
                    data_type: DataTypeId::String.into(),
                    value_rank: -1,
                    array_dimensions: None,
                    description: LocalizedText::from("Echoed back as output"),
                }],
            )
            .output_args(
                &mut *addr,
                &out_args_id,
                &[Argument {
                    name: UAString::from("output"),
                    data_type: DataTypeId::String.into(),
                    value_rank: -1,
                    array_dimensions: None,
                    description: LocalizedText::from("Same string as input"),
                }],
            )
            .insert(&mut *addr);
    }

    nm.add_method_callback(method_id.clone(), |inputs: &[Variant]| {
        match inputs.first() {
            Some(Variant::String(s)) => Ok(vec![Variant::String(s.clone())]),
            _ => Err(StatusCode::BadInvalidArgument),
        }
    });

    Ok(method_id)
}

#[allow(unused_imports)]
use _ as _; // placeholder to keep mod compiled if all uses gated behind cfg(test)
```

(末尾 `use _ as _` 是不必要的,删掉。)

- [ ] **Step 2: 暴露模块**

修改 `crates/opcuasim-core/src/server/mod.rs`,加:

```rust
pub mod test_methods;
```

- [ ] **Step 3: 编译**

Run: `cargo build -p opcuasim-core 2>&1 | tail -25`
Expected: 成功。`MethodBuilder` API 调用名/签名若不准,按编译错误调整。

- [ ] **Step 4: Commit Task 1+2**

```bash
git add crates/opcuasim-core/src/method.rs \
        crates/opcuasim-core/src/lib.rs \
        crates/opcuasim-core/src/server/test_methods.rs \
        crates/opcuasim-core/src/server/mod.rs
git commit -m "feat(core): method service + Demo.Echo test fixture

method::call_method invokes a single CallMethodRequest, method::
read_method_arguments decodes InputArguments/OutputArguments via
Browse + Read on the Method's HasProperty children. Server-side test
helper register_demo_echo_method adds a String->String echo method
under the Objects folder using MethodBuilder + add_method_callback;
test-only, not invoked from the production OpcUaServer::start path."
```

---

## Task 3: master-egui — events.rs

**Files:**
- Modify: `crates/opcuamaster-egui/src/events.rs`

- [ ] **Step 1: 加 UiCommand 变体**

在 `UiCommand::DeleteCertificate { ... },` 之后加:

```rust
    ReadMethodArgs {
        conn_id: String,
        method_id: String,
        req_id: u64,
    },
    CallMethod {
        conn_id: String,
        object_id: String,
        method_id: String,
        inputs: Vec<MethodArgValue>,
        req_id: u64,
    },
```

- [ ] **Step 2: 加 BackendEvent 变体**

在 `BackendEvent::CertificateList { ... },` 之后加:

```rust
    MethodArgs {
        req_id: u64,
        inputs: Vec<MethodArgInfo>,
        outputs: Vec<MethodArgInfo>,
    },
    MethodCallResult {
        req_id: u64,
        status: String,
        outputs: Vec<MethodArgValue>,
    },
```

- [ ] **Step 3: 加 DTO**

文件末尾加:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodArgInfo {
    pub name: String,
    pub data_type: String,
    pub description: String,
}

/// String-encoded scalar values for round-tripping with the dispatcher.
/// All numeric types serialize as decimal strings; bool as "true"/"false";
/// strings as themselves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodArgValue {
    pub data_type: String,
    pub value: String,
}
```

- [ ] **Step 4: 编译预期出错**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -10`
Expected: dispatcher / app.rs match 缺分支 → 留到后续 task 修。

---

## Task 4: master-egui — dispatcher 实现两个 handler

**Files:**
- Modify: `crates/opcuamaster-egui/src/backend/dispatcher.rs`

- [ ] **Step 1: 加 use**

`use opcuasim_core::client::...` 旁加:

```rust
use opcuasim_core::method::{call_method, read_method_arguments};
```

events use 列表加 `MethodArgInfo, MethodArgValue`。

- [ ] **Step 2: 加 match 分支**

在 `UiCommand::DeleteCertificate { path } => do_delete_cert(path, &event_tx).await,` 之后加:

```rust
        UiCommand::ReadMethodArgs {
            conn_id,
            method_id,
            req_id,
        } => do_read_method_args(conn_id, method_id, req_id, &state, &event_tx).await,
        UiCommand::CallMethod {
            conn_id,
            object_id,
            method_id,
            inputs,
            req_id,
        } => do_call_method(conn_id, object_id, method_id, inputs, req_id, &state, &event_tx).await,
```

- [ ] **Step 3: 写两个 handler**

文件末尾追加:

```rust
async fn do_read_method_args(
    conn_id: String,
    method_id: String,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let nid: opcua_types::NodeId = method_id
        .parse()
        .map_err(|_| format!("invalid node id: {method_id}"))?;
    let info = read_method_arguments(&session, &nid)
        .await
        .map_err(|e| e.to_string())?;
    let inputs = info
        .inputs
        .into_iter()
        .map(|a| MethodArgInfo {
            name: a.name,
            data_type: a.data_type,
            description: a.description,
        })
        .collect();
    let outputs = info
        .outputs
        .into_iter()
        .map(|a| MethodArgInfo {
            name: a.name,
            data_type: a.data_type,
            description: a.description,
        })
        .collect();
    let _ = event_tx.send(BackendEvent::MethodArgs {
        req_id,
        inputs,
        outputs,
    });
    Ok(())
}

async fn do_call_method(
    conn_id: String,
    object_id: String,
    method_id: String,
    inputs: Vec<MethodArgValue>,
    req_id: u64,
    state: &Arc<BackendState>,
    event_tx: &UnboundedSender<BackendEvent>,
) -> Result<(), String> {
    let session = get_session(state, &conn_id).await?;
    let oid: opcua_types::NodeId = object_id
        .parse()
        .map_err(|_| format!("invalid object id: {object_id}"))?;
    let mid: opcua_types::NodeId = method_id
        .parse()
        .map_err(|_| format!("invalid method id: {method_id}"))?;

    let variants: Vec<opcua_types::Variant> = inputs
        .iter()
        .map(|a| string_to_variant(&a.data_type, &a.value))
        .collect::<Result<_, String>>()?;

    let outcome = call_method(&session, &oid, &mid, variants)
        .await
        .map_err(|e| e.to_string())?;

    let outputs: Vec<MethodArgValue> = outcome
        .outputs
        .into_iter()
        .map(|v| MethodArgValue {
            data_type: variant_type_label(&v),
            value: format!("{v}"),
        })
        .collect();

    let _ = event_tx.send(BackendEvent::MethodCallResult {
        req_id,
        status: format!("{}", outcome.status),
        outputs,
    });
    Ok(())
}

fn string_to_variant(data_type: &str, value: &str) -> Result<opcua_types::Variant, String> {
    use opcua_types::Variant;
    match data_type {
        "Boolean" => value
            .parse::<bool>()
            .map(Variant::Boolean)
            .map_err(|e| e.to_string()),
        "SByte" => value.parse::<i8>().map(Variant::SByte).map_err(|e| e.to_string()),
        "Byte" => value.parse::<u8>().map(Variant::Byte).map_err(|e| e.to_string()),
        "Int16" => value.parse::<i16>().map(Variant::Int16).map_err(|e| e.to_string()),
        "UInt16" => value.parse::<u16>().map(Variant::UInt16).map_err(|e| e.to_string()),
        "Int32" => value.parse::<i32>().map(Variant::Int32).map_err(|e| e.to_string()),
        "UInt32" => value.parse::<u32>().map(Variant::UInt32).map_err(|e| e.to_string()),
        "Int64" => value.parse::<i64>().map(Variant::Int64).map_err(|e| e.to_string()),
        "UInt64" => value.parse::<u64>().map(Variant::UInt64).map_err(|e| e.to_string()),
        "Float" => value.parse::<f32>().map(Variant::Float).map_err(|e| e.to_string()),
        "Double" => value.parse::<f64>().map(Variant::Double).map_err(|e| e.to_string()),
        "String" => Ok(Variant::String(value.into())),
        other => Err(format!("unsupported method arg type: {other}")),
    }
}

fn variant_type_label(v: &opcua_types::Variant) -> String {
    use opcua_types::variant::VariantTypeId;
    match v.type_id() {
        VariantTypeId::Empty => "Empty".to_string(),
        VariantTypeId::Scalar(s) => format!("{s}"),
        VariantTypeId::Array(s, _) => format!("Array<{s}>"),
    }
}
```

- [ ] **Step 4: 编译**

Run: `cargo build -p opcuamaster-egui 2>&1 | tail -10`
Expected: app.rs 仍有 non-exhaustive match;留到 Task 6。

---

## Task 5: master-egui — Modal::MethodCall + dialog

**Files:**
- Modify: `crates/opcuamaster-egui/src/model.rs`
- Create: `crates/opcuamaster-egui/src/widgets/method_call_dialog.rs`
- Modify: `crates/opcuamaster-egui/src/widgets/mod.rs`

- [ ] **Step 1: model.rs**

在 `pub enum Modal { ... }` 内,在 `CertManager(...)` 之后加:

```rust
    MethodCall(MethodCallState),
```

文件末尾加:

```rust
pub struct MethodCallState {
    pub conn_id: String,
    pub object_id: String,
    pub method_id: String,
    pub display_name: String,
    pub inputs_meta: Vec<crate::events::MethodArgInfo>,
    pub outputs_meta: Vec<crate::events::MethodArgInfo>,
    pub input_values: Vec<String>,
    pub pending_args_req: Option<u64>,
    pub pending_call_req: Option<u64>,
    pub last_result_status: Option<String>,
    pub last_result_outputs: Vec<crate::events::MethodArgValue>,
    pub error: Option<String>,
}

impl MethodCallState {
    pub fn new(conn_id: String, object_id: String, method_id: String, display_name: String) -> Self {
        Self {
            conn_id,
            object_id,
            method_id,
            display_name,
            inputs_meta: Vec::new(),
            outputs_meta: Vec::new(),
            input_values: Vec::new(),
            pending_args_req: None,
            pending_call_req: None,
            last_result_status: None,
            last_result_outputs: Vec::new(),
            error: None,
        }
    }
}
```

- [ ] **Step 2: widgets/method_call_dialog.rs**

写入新文件:

```rust
use crate::events::MethodArgValue;
use crate::model::MethodCallState;

pub struct DialogActions {
    pub close: bool,
    pub call: Option<Vec<MethodArgValue>>,
}

pub fn show(ctx: &egui::Context, state: &mut MethodCallState) -> DialogActions {
    let mut actions = DialogActions {
        close: false,
        call: None,
    };

    egui::Window::new(format!("调用方法: {}", state.display_name))
        .collapsible(false)
        .resizable(true)
        .min_width(560.0)
        .default_width(720.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(format!("Method: {}", state.method_id));
            ui.label(format!("Object: {}", state.object_id));
            ui.separator();

            ui.heading("输入参数");
            if state.inputs_meta.is_empty() && state.pending_args_req.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("加载参数...");
                });
            } else if state.inputs_meta.is_empty() {
                ui.label("(无入参)");
            } else {
                if state.input_values.len() != state.inputs_meta.len() {
                    state.input_values =
                        state.inputs_meta.iter().map(default_for_type).collect();
                }
                for (i, arg) in state.inputs_meta.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} ({}):", arg.name, arg.data_type));
                        ui.text_edit_singleline(&mut state.input_values[i]);
                    });
                }
            }

            if let Some(err) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }

            ui.separator();

            ui.heading("输出参数");
            if state.last_result_status.is_some() {
                ui.label(format!(
                    "Status: {}",
                    state.last_result_status.as_deref().unwrap_or("?")
                ));
                if state.outputs_meta.is_empty() && state.last_result_outputs.is_empty() {
                    ui.label("(无返回)");
                } else {
                    for (i, out) in state.last_result_outputs.iter().enumerate() {
                        let name = state
                            .outputs_meta
                            .get(i)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| format!("[{i}]"));
                        ui.label(format!("{} ({}) = {}", name, out.data_type, out.value));
                    }
                }
            } else {
                ui.label("(尚未执行)");
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    actions.close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let busy = state.pending_call_req.is_some();
                    let label = if busy { "执行中…" } else { "执行" };
                    if ui.add_enabled(!busy, egui::Button::new(label)).clicked() {
                        actions.call = Some(
                            state
                                .inputs_meta
                                .iter()
                                .zip(state.input_values.iter())
                                .map(|(meta, v)| MethodArgValue {
                                    data_type: meta.data_type.clone(),
                                    value: v.clone(),
                                })
                                .collect(),
                        );
                    }
                });
            });
        });

    actions
}

fn default_for_type(arg: &crate::events::MethodArgInfo) -> String {
    match arg.data_type.as_str() {
        "Boolean" => "false".into(),
        "String" => "".into(),
        "Float" | "Double" => "0.0".into(),
        _ => "0".into(),
    }
}
```

- [ ] **Step 3: widgets/mod.rs**

加 `pub mod method_call_dialog;`(按字母顺序插入)。

---

## Task 6: master-egui — toolbar 不动,browse_panel + app.rs 接入

**Files:**
- Modify: `crates/opcuamaster-egui/src/panels/browse_panel.rs`
- Modify: `crates/opcuamaster-egui/src/app.rs`

- [ ] **Step 1: BrowsePanel — Method 节点显示 + 右键调用**

修改 `render_node`,把 `is_variable` 判断扩展为 `(is_variable, is_method)`,并在 `else if is_variable {` 之后增加 `else if is_method { ... }` 分支:

定位 `let (display, has_children, is_variable, loading, children) = { ... };`,改成同时取 `is_method`:

```rust
    let (display, has_children, is_variable, is_method, loading, children) = {
        let Some(st) = model.browse.nodes.get(node_id) else {
            return;
        };
        let icon = match st.item.node_class.as_str() {
            "Method" => "⚙",
            "Object" => "📁",
            "Variable" => "🔢",
            _ => "•",
        };
        (
            format!(
                "{icon}  {}  [{}]{}",
                st.item.display_name,
                st.item.node_class,
                st.item
                    .data_type
                    .as_ref()
                    .map(|t| format!(" : {t}"))
                    .unwrap_or_default()
            ),
            st.item.has_children,
            st.item.node_class == "Variable",
            st.item.node_class == "Method",
            st.loading,
            st.children.clone(),
        )
    };
```

在末尾 `} else if is_variable { ... }` 之后追加 `else if is_method` 分支(同一 if-else-if 链):

```rust
    } else if is_method {
        ui.horizontal(|ui| {
            let resp = ui.label(display);
            resp.context_menu(|ui| {
                if ui.button("⚙ 调用方法...").clicked() {
                    let parent_id =
                        find_parent_object(model, node_id).unwrap_or_else(|| node_id.to_string());
                    let display_name = model
                        .browse
                        .nodes
                        .get(node_id)
                        .map(|s| s.item.display_name.clone())
                        .unwrap_or_else(|| node_id.to_string());
                    let req_id = model.alloc_req_id();
                    let mut s = crate::model::MethodCallState::new(
                        conn_id.to_string(),
                        parent_id,
                        node_id.to_string(),
                        display_name,
                    );
                    s.pending_args_req = Some(req_id);
                    model.modal = Some(crate::model::Modal::MethodCall(s));
                    backend.send(UiCommand::ReadMethodArgs {
                        conn_id: conn_id.to_string(),
                        method_id: node_id.to_string(),
                        req_id,
                    });
                    ui.close();
                }
            });
        });
    }
```

并在文件末尾加 helper:

```rust
fn find_parent_object(model: &AppModel, node_id: &str) -> Option<String> {
    // The node we just rendered came from a parent's children; find which
    // entry has node_id in its children list.
    for (pid, st) in &model.browse.nodes {
        if let Some(kids) = &st.children {
            if kids.iter().any(|k| k == node_id) {
                return Some(pid.clone());
            }
        }
    }
    None
}
```

- [ ] **Step 2: app.rs — apply_event 接 MethodArgs / MethodCallResult**

在 `BackendEvent::CertificateList { ... }` 分支之后加:

```rust
            BackendEvent::MethodArgs {
                req_id,
                inputs,
                outputs,
            } => {
                if let Some(Modal::MethodCall(state)) = self.model.modal.as_mut() {
                    if state.pending_args_req == Some(req_id) {
                        state.pending_args_req = None;
                        state.input_values =
                            inputs.iter().map(|m| default_input_for(m)).collect();
                        state.inputs_meta = inputs;
                        state.outputs_meta = outputs;
                    }
                }
            }
            BackendEvent::MethodCallResult {
                req_id,
                status,
                outputs,
            } => {
                if let Some(Modal::MethodCall(state)) = self.model.modal.as_mut() {
                    if state.pending_call_req == Some(req_id) {
                        state.pending_call_req = None;
                        state.last_result_status = Some(status);
                        state.last_result_outputs = outputs;
                    }
                }
            }
```

并在 `impl MasterApp` 之外加:

```rust
fn default_input_for(arg: &crate::events::MethodArgInfo) -> String {
    match arg.data_type.as_str() {
        "Boolean" => "false".into(),
        "String" => "".into(),
        "Float" | "Double" => "0.0".into(),
        _ => "0".into(),
    }
}
```

- [ ] **Step 3: app.rs — render_modal 接 MethodCall**

在 `Modal::CertManager(state) => { ... }` arm 之后追加:

```rust
            Modal::MethodCall(state) => {
                let actions = crate::widgets::method_call_dialog::show(ctx, state);
                if let Some(inputs) = actions.call {
                    let req_id = self.model.alloc_req_id();
                    state.pending_call_req = Some(req_id);
                    state.last_result_status = None;
                    state.last_result_outputs.clear();
                    self.backend.send(UiCommand::CallMethod {
                        conn_id: state.conn_id.clone(),
                        object_id: state.object_id.clone(),
                        method_id: state.method_id.clone(),
                        inputs,
                        req_id,
                    });
                }
                if !actions.close {
                    self.model.modal = Some(modal);
                }
            }
```

- [ ] **Step 4: 编译 + clippy**

Run:
```bash
cargo build -p opcuamaster-egui 2>&1 | tail -10
cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10
```
Expected: 全过。如有 too_many_arguments,加 `#[allow(...)]`。

- [ ] **Step 5: 跑现有 e2e 不回归**

Run: `cargo test -p opcuamaster-egui --test e2e 2>&1 | tail -10`
Expected: `master_full_flow` + `deadband_reduces_samples` 全过。

---

## Task 7: e2e — Demo.Echo 调用断言

**Files:**
- Modify: `crates/opcuamaster-egui/tests/e2e.rs`

- [ ] **Step 1: 加 use**

在 `use opcuasim_core::server::server::OpcUaServer;` 之后加:

```rust
use opcuasim_core::server::test_methods::register_demo_echo_method;
```

events use 列表加 `MethodArgValue`。

- [ ] **Step 2: 加新测试**

在 deadband 测试之后追加:

```rust
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
```

- [ ] **Step 3: 跑测试**

Run: `cargo test -p opcuamaster-egui --test e2e 2>&1 | tail -15`
Expected: 3 个测试全过。

- [ ] **Step 4: clippy**

Run: `cargo clippy --workspace --tests -- -D warnings 2>&1 | tail -10`
Expected: 全过。

- [ ] **Step 5: Commit + push**

```bash
git add crates/opcuamaster-egui/src/events.rs \
        crates/opcuamaster-egui/src/backend/dispatcher.rs \
        crates/opcuamaster-egui/src/model.rs \
        crates/opcuamaster-egui/src/widgets/method_call_dialog.rs \
        crates/opcuamaster-egui/src/widgets/mod.rs \
        crates/opcuamaster-egui/src/panels/browse_panel.rs \
        crates/opcuamaster-egui/src/app.rs \
        crates/opcuamaster-egui/tests/e2e.rs
git commit -m "feat(master): method call dialog with input/output rendering

BrowsePanel marks Method nodes with ⚙, right-click 调用方法 opens a
modal: ReadMethodArgs populates input fields by data_type label,
执行 sends CallMethod, OutputArguments rendered with type+value.
e2e test method_call_echo registers Demo.Echo on the server fixture
and asserts round-trip of 'hello'."
git push origin master
```

---

## Self-Review

**Spec coverage:**
- §5.4 `browse.rs` 输出 NodeClass → 已存在(events::BrowseItem.node_class),Task 6 用之区分 Method
- §5.4 `call_method` API → Task 1
- §5.4 InputArguments / OutputArguments 读取辅助 → Task 1 `read_method_arguments`
- §5.4 `opcusim-core` server 注册 Demo.Echo → Task 2
- §5.4 BrowsePanel 标注图标 + 右键 Call → Task 6
- §5.4 MethodDialog InputArguments 编辑 + OutputArguments 显示 → Task 5
- §5.4 e2e 调用 Demo.Echo 断言返回 → Task 7
- §4.3 沿用 dispatcher / serde / Toast 模式 → 全程
- §4.4 server 默认配置不变 → Task 2 helper 仅供测试调用

**Placeholder scan:** Task 2 step 1 `use _ as _;` 是误生成的 placeholder,该步骤明确说"删掉"。Task 1 末尾的 `BrowseDescription / NodeClassMask / BrowseResultMask` 路径备注"按编译错误调整"——是 hop,不是占位。

**Type consistency:**
- `ArgumentInfo`(core)字段:name / data_type / data_type_id / description
- `MethodArgInfo`(events)字段:name / data_type / description(没带 NodeId,UI 用不上)
- `MethodArgValue`:data_type + value(string-encoded)
- `MethodCallOutcome` → `BackendEvent::MethodCallResult { status: String, outputs: Vec<MethodArgValue> }` — status 用 Display 格式化,丢失 StatusCode 数值但 UI 只需文本

**风险:**
- `MethodBuilder::component_of(parent_node_id)` API 名/签名需要在 Task 2 编译时确认
- `session.browse(...)` 返回 `Vec<BrowseResult>` 还是包装类型需要确认
- `Argument.data_type: NodeId` vs `ExpandedNodeId` 需确认
- Task 6 step 1 `find_parent_object` 在用户尚未展开过 method 父节点时会返回 None;此时使用 method_id 自身作 object_id 兜底(spec 没限定).

**Commit 数:** 2 条
1. Task 1+2 合并: feat(core): method service + Demo.Echo test fixture
2. Task 7 合并: feat(master): method call dialog with input/output rendering(含 events / dispatcher / model / dialog / browse_panel / app / e2e)
