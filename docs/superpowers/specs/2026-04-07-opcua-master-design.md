# OPC UA 采集主站设计文档

## 概述

开发一个 OPC UA 采集主站（Client），用于主动连接 OPC UA Server 读取数据。作为 OPCUASim 模拟器套件的第一个组件，后续将加入从站（Server）模拟器。

项目风格与现有 IEC 60870-5-104 Simulator、ModbusSim 保持一致。

## 技术栈

| 层级 | 技术 |
|------|------|
| 后端 | Rust + Tokio 异步运行时 |
| OPC UA 库 | async-opcua（async-opcua-client crate） |
| 桌面框架 | Tauri 2 |
| 前端框架 | Vue 3 + TypeScript + Vite |
| 虚拟滚动 | @tanstack/vue-virtual |
| 主题 | Catppuccin Mocha 深色主题 |

## 功能范围

- OPC UA DA（Data Access）数据采集
- 订阅模式（Subscription）+ 轮询模式（Polling），按节点灵活配置
- 节点浏览（Browse）+ 手动输入 NodeId 两种方式发现节点
- 安全模式：None / Sign / SignAndEncrypt，支持匿名、用户名密码、证书认证
- 可插拔数据输出（初期实现日志输出，预留数据库、消息队列、API 接口）
- 灵活可配的规模：1 到 20+ Server，几百到上万节点
- 项目文件持久化（.opcuaproj）
- 通信日志记录与导出

## 项目结构

```
OPCUASim/
├── Cargo.toml                    # Cargo workspace
├── package.json                  # npm workspace
├── crates/
│   ├── opcuasim-core/            # 核心库
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # OPC UA 客户端连接管理
│   │       ├── browse.rs         # 节点浏览
│   │       ├── subscription.rs   # 订阅模式管理
│   │       ├── polling.rs        # 轮询模式管理
│   │       ├── node.rs           # 节点数据结构、类型映射
│   │       ├── group.rs          # 自定义分组
│   │       ├── output.rs         # 输出插件 trait + 日志输出实现
│   │       ├── security.rs       # 安全配置
│   │       ├── config.rs         # 配置序列化、项目文件
│   │       ├── log_collector.rs  # 通信日志收集（环形缓冲区）
│   │       ├── log_entry.rs      # 日志条目定义
│   │       └── error.rs          # 统一错误类型
│   └── opcuamaster-app/          # 主站 Tauri 应用
│       ├── tauri.conf.json
│       └── src/
│           ├── main.rs
│           ├── lib.rs            # Tauri 初始化、命令注册
│           ├── commands.rs       # Tauri IPC 命令
│           └── state.rs          # 应用状态
├── master-frontend/              # 主站 Vue 3 前端
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── App.vue
│       ├── main.ts
│       ├── style.css
│       ├── types.ts
│       ├── components/
│       │   ├── Toolbar.vue
│       │   ├── ConnectionTree.vue
│       │   ├── DataTable.vue
│       │   ├── ValuePanel.vue
│       │   ├── BrowsePanel.vue
│       │   ├── LogPanel.vue
│       │   └── AppDialog.vue
│       └── composables/
│           └── useDialog.ts
└── shared-frontend/              # 共享前端库
    └── src/
        ├── components/AppDialog.vue
        └── composables/
            ├── useDialog.ts
            ├── useLogPanel.ts
            ├── useLogFilter.ts
            └── useErrorHandler.ts
```

## 核心数据模型

### 连接配置

```rust
struct ConnectionConfig {
    id: String,                              // UUID
    name: String,                            // 用户自定义名称
    endpoint_url: String,                    // opc.tcp://host:port/path
    security_policy: SecurityPolicy,         // None / Basic256Sha256 等
    security_mode: MessageSecurityMode,      // None / Sign / SignAndEncrypt
    auth: AuthConfig,                        // 认证方式
    timeout_ms: u64,
}

enum AuthConfig {
    Anonymous,
    UserPassword { username: String, password: String },
    Certificate { cert_path: String, key_path: String },
}
```

### 监控节点

```rust
struct MonitoredNode {
    node_id: NodeId,              // OPC UA NodeId
    display_name: String,
    browse_path: String,          // 浏览路径
    data_type: OpcDataType,
    value: Option<DataValue>,     // 当前值 + 时间戳 + 质量
    access_mode: AccessMode,
    group_id: Option<String>,     // 所属自定义分组
}

enum AccessMode {
    Subscription { interval_ms: f64 },
    Polling { interval_ms: u64 },
}
```

### 自定义分组

```rust
struct NodeGroup {
    id: String,
    name: String,
    node_ids: Vec<NodeId>,
}
```

### 输出插件

```rust
trait DataOutput: Send + Sync {
    async fn on_data_change(&self, connection_id: &str, items: &[DataChangeItem]);
    async fn on_connect(&self, connection_id: &str);
    async fn on_disconnect(&self, connection_id: &str);
}
```

初期实现 `LogOutput`（写入日志收集器供 UI 展示）。后续可加 `MqttOutput`、`InfluxOutput`、`RestApiOutput` 等。

## Tauri 命令

### 连接管理

| 命令 | 说明 |
|------|------|
| `create_connection(config)` | 创建连接配置 |
| `connect(id)` | 建立 OPC UA 会话 |
| `disconnect(id)` | 断开 |
| `delete_connection(id)` | 删除 |
| `list_connections()` | 列出所有连接及状态 |
| `get_endpoints(url)` | 发现 Server 端点和安全策略 |

### 节点浏览

| 命令 | 说明 |
|------|------|
| `browse_root(conn_id)` | 从 Objects 根节点开始浏览 |
| `browse_node(conn_id, node_id)` | 浏览子节点 |
| `read_node_attributes(conn_id, node_id)` | 读取节点详细属性 |

### 采集管理

| 命令 | 说明 |
|------|------|
| `add_monitored_nodes(conn_id, nodes, access_mode)` | 添加节点到采集列表 |
| `remove_monitored_nodes(conn_id, node_ids)` | 移除 |
| `update_access_mode(conn_id, node_id, mode)` | 切换订阅/轮询 |
| `get_monitored_data(conn_id, since_seq)` | 增量获取采集数据 |

### 分组管理

| 命令 | 说明 |
|------|------|
| `create_group(name)` | 创建自定义分组 |
| `delete_group(id)` | 删除 |
| `add_nodes_to_group(group_id, node_ids)` | 添加节点到分组 |
| `remove_nodes_from_group(group_id, node_ids)` | 移除 |

### 日志与持久化

| 命令 | 说明 |
|------|------|
| `get_communication_logs(conn_id, since_seq)` | 获取通信日志 |
| `clear_communication_logs(conn_id)` | 清空 |
| `export_logs_csv(conn_id, path)` | 导出 CSV |
| `save_project(path)` | 保存 .opcuaproj |
| `load_project(path)` | 加载 .opcuaproj |

### 后端事件推送

| 事件 | 说明 |
|------|------|
| `connection-state-changed` | 连接/断开/重连状态 |
| `data-changed` | 订阅数据变化通知 |
| `browse-complete` | 浏览结果返回 |
| `error` | 错误通知 |

## 数据流

```
订阅模式:
  Server --push--> async-opcua --> DataOutput trait --> LogCollector
                                                    --> Tauri Event --> 前端增量更新

轮询模式:
  tokio 定时器 --> async-opcua read --> DataOutput trait --> 同上
```

## UI 布局

```
┌──────────────────────────────────────────────────────────┐
│  工具栏（42px）                                          │
│  [新建连接] [连接] [断开] [删除] | [浏览节点] | [保存] [打开] | [导出日志] │
├───────────┬──────────────────────┬───────────────────────┤
│ 左侧树    │  数据表格            │  值详情面板            │
│ (240px)   │  (自适应, 虚拟滚动)  │  (280px)              │
│           │                      │                       │
│ ▾ 连接视图 │  NodeId | 名称 | 值  │  节点属性              │
│   ▾ Conn1 │  | 类型 | 质量 | 时间 │  当前值               │
│     ▾ 地址空间│                   │  数据类型              │
│       Objects│                   │  访问模式配置           │
│       Types  │                   │                       │
│     ▾ 分组   │                   │                       │
│       温度组 │                   │                       │
│       压力组 │                   │                       │
├───────────┴──────────────────────┴───────────────────────┤
│  日志面板（32px 标题，可展开 200px）                       │
│  时间 | 方向 | 服务 | 详情 | 状态码                       │
└──────────────────────────────────────────────────────────┘
```

### 左侧树形菜单

双视图切换：

- **地址空间视图**：通过 Browse 实时懒加载展开 Server 节点树，可多选节点右键添加到采集列表或分组
- **分组视图**：用户自定义分组，扁平展示组内节点，支持拖拽添加、右键移除

### 数据表格

- 列：NodeId、DisplayName、数据类型、当前值、质量、时间戳、访问模式
- 虚拟滚动（@tanstack/vue-virtual）
- 增量更新（update_seq 模式）
- 右键菜单：写入值、切换订阅/轮询、移除、添加到分组

### 值详情面板

- 选中节点完整属性：NodeId、DisplayName、Description、DataType、AccessLevel
- 当前值展示（根据数据类型适配格式）
- 手动读取/写入按钮
- 访问模式切换 + 间隔配置

### 设计主题

Catppuccin Mocha 深色主题，和 104/Modbus 一致：

- 主背景：#11111b
- 组件背景：#181825 / #1e1e2e
- 文字：#cdd6f4
- 边框：#313244

## 错误处理

```rust
enum OpcUaSimError {
    // 连接层
    ConnectionFailed(String),
    SessionTimeout,
    SecurityRejected(String),
    AuthenticationFailed,
    // 协议层
    BrowseError(StatusCode),
    ReadError(StatusCode),
    WriteError(StatusCode),
    SubscriptionError(StatusCode),
    // 应用层
    ConfigError(String),
    ProjectFileError(String),
    OutputError(String),
}
```

## 重连策略

指数退避重连（和 ModbusSim 一致）：

- 连接断开后自动重连
- 退避间隔：1s → 2s → 4s → 8s → ... → 最大 60s
- 重连成功后自动恢复订阅和轮询任务
- UI 实时显示状态：连接中 / 已连接 / 断开 / 重连中

## 通信日志

```rust
struct LogEntry {
    seq: u64,
    timestamp: DateTime<Utc>,
    connection_id: String,
    direction: Direction,       // Request / Response
    service: String,            // Browse / Read / Write / CreateSubscription / Publish
    detail: String,             // 人可读摘要
    status: Option<StatusCode>,
}
```

- 环形缓冲区存储（RwLock + VecDeque）
- 按方向、服务类型过滤
- CSV 导出

## 项目文件格式（.opcuaproj）

```json
{
  "type": "OpcUaMaster",
  "version": "0.1.0",
  "connections": [
    {
      "name": "本地测试",
      "endpoint_url": "opc.tcp://127.0.0.1:4840",
      "security_policy": "None",
      "security_mode": "None",
      "auth": { "type": "Anonymous" },
      "timeout_ms": 5000,
      "monitored_nodes": [
        {
          "node_id": "ns=2;s=Temperature",
          "display_name": "温度",
          "access_mode": { "type": "Subscription", "interval_ms": 1000.0 },
          "group_id": "group-1"
        }
      ]
    }
  ],
  "groups": [
    { "id": "group-1", "name": "温度组" }
  ]
}
```

## 测试策略

**后端单元测试**：
- 节点数据结构序列化/反序列化
- 配置解析、项目文件读写
- 日志收集器
- 输出 trait mock 验证

**后端集成测试**：
- 用 async-opcua Server 端启动临时 Server
- 验证连接、浏览、订阅、轮询完整流程

**前端**：
- 手动验证 UI 交互（和现有项目一致）

**端到端验证**：
- 用 async-opcua simple-server 或自建测试 Server
- 连接 → 浏览 → 添加节点 → 订阅/轮询 → 数据展示 → 断开重连
