# OPCUAServer Design Spec

> OPC UA Simulation Server -- a standalone Tauri 2 desktop application for simulating PLC/device data via the OPC UA protocol.

## 1. Overview

### Purpose

A **simulation/testing tool** that exposes a configurable OPC UA address space with fake data generation (static, random, sine, linear, script). Developers use it to test OPC UA client programs without real hardware.

### Form Factor

- Standalone Tauri 2 desktop application (`opcuaserver-app` + `server-frontend`)
- Independent from OPCUAMaster; shares `opcuasim-core` library and `shared-frontend` components
- Catppuccin Mocha dark theme, layout tailored for server management

### Tech Stack

| Layer | Technology |
|-------|-----------|
| OPC UA Server | `async-opcua-server` 0.18 |
| Backend | Rust, Tokio async runtime |
| Desktop | Tauri 2 |
| Frontend | Vue 3 + TypeScript, Composition API, `@tanstack/vue-virtual` |
| Shared | `opcuasim-core` (extended), `shared-frontend` |
| Expression engine | `evalexpr` (supports custom variable/function injection for `t`, `i`, `rand()`, etc.) |

## 2. Architecture

### Project Structure (Approach A: extend core)

```
OPCUASim/
├── crates/
│   ├── opcuasim-core/              # Extended: add server/ module
│   │   └── src/
│   │       ├── ... (existing client modules unchanged)
│   │       └── server/
│   │           ├── mod.rs          # Module exports
│   │           ├── server.rs       # OpcUaServer: start/stop async-opcua-server
│   │           ├── address_space.rs# Dynamic add/remove folders and Variable nodes
│   │           ├── simulation.rs   # SimulationEngine: drives data generation
│   │           ├── generator.rs    # Data generators: Static/Random/Sine/Linear/Script
│   │           └── security.rs     # Server security: certs, user auth strategies
│   ├── opcuamaster-app/            # Unchanged
│   └── opcuaserver-app/            # NEW: Tauri Server application
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── build.rs
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── commands.rs         # ~18 Tauri IPC commands
│           └── state.rs            # AppState
├── master-frontend/                # Unchanged
├── server-frontend/                # NEW: Server Vue 3 frontend
│   ├── vite.config.ts
│   ├── index.html
│   └── src/
│       ├── main.ts
│       ├── App.vue
│       ├── types.ts
│       ├── composables/
│       │   ├── useServer.ts
│       │   ├── useAddressSpace.ts
│       │   └── useSimulation.ts
│       └── components/
│           ├── Toolbar.vue
│           ├── AddressSpaceTree.vue
│           ├── NodeTable.vue
│           ├── PropertyEditor.vue
│           ├── SimModeEditor.vue
│           ├── ServerSettingsDialog.vue
│           ├── UserManageDialog.vue
│           ├── BatchAddDialog.vue
│           ├── StatusBar.vue
│           └── LogPanel.vue
└── shared-frontend/                # Extended: reuse AppDialog, LogPanel, etc.
```

## 3. Data Models

### Core Types (in `opcuasim-core::server`)

```rust
// --- Server Node ---
pub struct ServerNode {
    pub node_id: String,            // e.g. "ns=2;s=Temperature"
    pub display_name: String,
    pub parent_id: String,          // Parent folder node_id
    pub data_type: DataType,
    pub writable: bool,             // Whether clients can write
    pub simulation: SimulationMode,
}

pub enum DataType {
    Boolean, Int16, Int32, Int64, UInt16, UInt32, UInt64,
    Float, Double, String, DateTime, ByteString,
}

// --- Simulation Modes ---
pub enum SimulationMode {
    Static { value: String },
    Random { min: f64, max: f64, interval_ms: u64 },
    Sine { amplitude: f64, offset: f64, period_ms: u64, interval_ms: u64 },
    Linear { start: f64, step: f64, min: f64, max: f64, mode: LinearMode, interval_ms: u64 },
    Script { expression: String, interval_ms: u64 },
}

pub enum LinearMode { Repeat, Bounce }

// --- Folder Node ---
pub struct ServerFolder {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,          // Root parent_id = "i=85" (Objects)
}

// --- Server Config ---
pub struct ServerConfig {
    pub name: String,
    pub endpoint_url: String,       // e.g. "opc.tcp://0.0.0.0:4840"
    pub port: u16,
    pub security_policies: Vec<String>,  // ["None", "Basic256Sha256", ...]
    pub security_modes: Vec<String>,     // ["None", "Sign", "SignAndEncrypt"]
    pub users: Vec<UserAccount>,
    pub anonymous_enabled: bool,
    pub certificate_auth_enabled: bool,
    pub max_sessions: u32,               // Default 100
    pub max_subscriptions_per_session: u32, // Default 50
}

pub struct UserAccount {
    pub username: String,
    pub password: String,           // Plaintext (simulation tool, not production)
    pub role: UserRole,
}

pub enum UserRole {
    ReadOnly,
    ReadWrite,
    Admin,
}

// --- Project File ---
pub struct ServerProjectFile {
    pub project_type: String,       // "OpcUaServer"
    pub version: String,
    pub server_config: ServerConfig,
    pub folders: Vec<ServerFolder>,
    pub nodes: Vec<ServerNode>,
}
```

File extension: `.opcuaproj` (same as Master, distinguished by `project_type` field).

## 4. Simulation Engine

### Architecture

```
SimulationEngine
├── interval_groups: HashMap<u64, IntervalGroup>  // interval_ms → group
├── start_all()     — Create a tokio task per interval group
├── stop_all()      — Cancel all tasks
├── start_node()    — Add node to its interval group
├── stop_node()     — Remove node from its group
└── update_node()   — Hot-update: stop + start with new params
```

**IntervalGroup**: one `tokio::time::interval` task handles all nodes with the same `interval_ms`. Each tick:
1. Compute new values for all nodes in the group (pure function, no locks)
2. Acquire AddressSpace write lock once
3. Batch-write all values
4. Release lock

### Generators

| Mode | Logic | Parameters |
|------|-------|-----------|
| Static | No timer, value written on manual change only | `value` |
| Random | `rand(min, max)` each interval | `min`, `max`, `interval_ms` |
| Sine | `offset + amplitude * sin(2pi * elapsed / period)` | `amplitude`, `offset`, `period_ms`, `interval_ms` |
| Linear | Increment by `step` each interval; Repeat resets at max, Bounce reverses | `start`, `step`, `min`, `max`, `mode`, `interval_ms` |
| Script | Evaluate expression with built-in vars: `t` (seconds), `i` (iteration), `rand()`, `sin()`, `cos()`, `abs()`, `min()`, `max()`, `floor()`, `ceil()` | `expression`, `interval_ms` |

Example script expression: `25.0 + 5.0 * sin(t * 0.1) + rand() * 0.5`

### Data Type Conversion

Generators produce `f64` uniformly. Conversion to target `DataType` on write:

| DataType | Conversion |
|----------|-----------|
| Boolean | `value > 0.5` |
| Int16/32/64, UInt16/32/64 | `value as T` (clamped to type range) |
| Float, Double | Direct |
| String | `format!("{:.2}", value)` |
| DateTime | Current system time (generator value ignored) |
| ByteString | Current timestamp as bytes |

### Performance (10,000+ nodes, dozens of clients)

**Task count estimation**:

| Nodes | Distinct intervals | Tokio tasks |
|-------|--------------------|-------------|
| 10,000 | 3 (100ms/500ms/1s) | 3 |
| 10,000 | 10 | 10 |
| 50,000 | 5 | 5 |

vs. naive per-node approach: 10,000+ tasks.

**Key optimizations**:
- Group-by-interval batching: single timer per interval group
- Lock-free generation: generators are pure functions
- Batch write: one lock acquisition per interval tick per group
- Channel decoupling: simulation engine and OPC UA server communicate via `tokio::sync::mpsc`

## 5. OPC UA Server

### Lifecycle

```
Stopped → Starting → Running → Stopping → Stopped
```

### Server Startup Sequence

1. Build `async-opcua-server` via `ServerBuilder` with configured security policies, modes, and user tokens
2. Populate address space: create FolderType nodes for each `ServerFolder`, Variable nodes for each `ServerNode`
3. Register write callback for writable nodes
4. Start server (binds to configured port)
5. Start `SimulationEngine` for all non-Static nodes
6. Update state to `Running`

### Server Shutdown Sequence

1. Stop `SimulationEngine` (cancel all interval tasks)
2. Shutdown `async-opcua-server` instance
3. Update state to `Stopped`

### Client Write Handling

When a client writes to a Variable node:
1. `async-opcua-server` triggers write callback
2. Check node `writable` flag and user `UserRole` permission
3. If node is in non-Static simulation mode: **pause simulation for that node**, switch to `Static` mode with the written value
4. User can re-enable simulation from GUI

### Security

- **Certificates**: Auto-generate self-signed cert on first launch (stored in app data dir)
- **Security policies**: None / Basic256Sha256 / Aes128_Sha256_RsaOaep / Aes256_Sha256_RsaPss
- **Authentication**: Anonymous (configurable on/off), Username/Password, X.509 Certificate
- **Password storage**: Plaintext in project file (simulation tool, not production use)

## 6. GUI Layout

```
+-------------------------------------------------------------------+
| Toolbar: [New Folder] [New Node] [Start/Stop] [Save] [Open] [Settings] |
+---------------+---------------------------+-----------------------+
|               |                           |                       |
| Address Space |    Node Table             |  Property Editor      |
| Tree          |    (all Variable nodes)   |  (selected node)      |
|               |                           |                       |
| - Folders     |  NodeId | Name | Type     |  Simulation mode      |
| - Variables   |  | SimMode | Value        |  Parameter config     |
|               |                           |  Live value preview   |
| Drag to       |  Virtual scroll           |                       |
| reorder       |  Search/filter            |                       |
|               |                           |                       |
+---------------+---------------------------+-----------------------+
| StatusBar: [Server: Running] [Clients: 3] [Nodes: 1200] | LogPanel |
+-------------------------------------------------------------------+
```

### Panel Responsibilities

| Panel | Function |
|-------|----------|
| **Toolbar** | Create folders/nodes, start/stop server, save/load project, server settings (port, security, user management) |
| **Address Space Tree (left)** | Tree view of folders and Variable nodes. Right-click menu: add, delete, rename, batch add. Drag to reorder. |
| **Node Table (center)** | Virtual-scrolled table of all Variables. Columns: NodeId, DisplayName, DataType, SimMode, CurrentValue, Status. Search/filter. Live value refresh when running. |
| **Property Editor (right)** | Edit selected node: NodeId, DisplayName, DataType (dropdown), Writable (toggle), simulation mode and parameters. Live value curve preview when running. |
| **StatusBar + LogPanel (bottom)** | Server status indicator, connected client count, active node count. Communication log (reuses shared-frontend LogPanel pattern). |

### Frontend Components

```
server-frontend/src/
├── main.ts
├── App.vue                    # CSS Grid layout
├── types.ts                   # Server TS type definitions
├── composables/
│   ├── useServer.ts           # Server start/stop, status polling
│   ├── useAddressSpace.ts     # Address space CRUD via IPC
│   └── useSimulation.ts       # Incremental data polling (since_seq pattern)
└── components/
    ├── Toolbar.vue
    ├── AddressSpaceTree.vue
    ├── NodeTable.vue          # @tanstack/vue-virtual
    ├── PropertyEditor.vue
    ├── SimModeEditor.vue      # Simulation mode config sub-component
    ├── ServerSettingsDialog.vue
    ├── UserManageDialog.vue
    ├── BatchAddDialog.vue
    ├── StatusBar.vue
    └── LogPanel.vue
```

### Shared Frontend Reuse

Directly reused from `shared-frontend`:
- `AppDialog.vue` — Dialog base component
- `useDialog.ts` — Dialog state management
- `useLogFilter.ts` — Log filtering logic
- `useErrorHandler.ts` — Error handling
- `common.ts` — Shared types

## 7. Tauri IPC Commands (~18)

| Category | Command | Description |
|----------|---------|-------------|
| **Server** | `start_server` | Start OPC UA server with current config |
| | `stop_server` | Stop server and simulation engine |
| | `get_server_status` | Returns state, client count, node count |
| | `update_server_config` | Update port, security, session limits |
| **Address Space** | `add_folder` | Add folder node |
| | `add_node` | Add Variable node with simulation config |
| | `batch_add_nodes` | Add multiple nodes at once |
| | `remove_node` | Remove node or folder (recursive) |
| | `update_node` | Update node properties or simulation mode |
| | `get_address_space` | Get full tree of folders + nodes |
| **Simulation** | `get_simulation_data` | Incremental poll (since_seq) for current values |
| | `set_static_value` | Manually write a value to a Static node |
| **Users** | `add_user` | Add user account |
| | `remove_user` | Remove user account |
| | `list_users` | List all user accounts |
| **Project** | `save_server_project` | Save to .opcuaproj file |
| | `load_server_project` | Load from .opcuaproj file |
| **Logs** | `get_server_logs` | Get communication logs (since_seq) |
| | `clear_server_logs` | Clear log buffer |

## 8. Data Flow

```
User GUI configures nodes
       |
       v
  Tauri IPC commands
       |
       v
  AppState (nodes, folders, server_config)
       |
       v  start_server
  +----------------------------------+
  | OpcUaServer                      |
  | +- async-opcua-server instance   |<---- Client connections (Browse/Read/Subscribe/Write)
  | +- AddressSpace (OPC UA nodes)   |
  +----------------------------------+
       |
       v  address space built
  +----------------------------------+
  | SimulationEngine                 |
  | +- interval_groups:              |
  |     100ms -> [node1, node2...]   |
  |     500ms -> [node3, node4...]   |
  |     1000ms -> [node5, node6...]  |
  | +- one tokio task per group      |
  +----------------------------------+
       |  batch DataChange via channel
       v
  Write into async-opcua-server AddressSpace
       |
       v  automatic
  OPC UA Subscription push to connected clients
```

## 9. Non-Goals (out of scope)

- No data forwarding / gateway functionality (Modbus, MQTT, etc.)
- No historical data access (OPC UA HDA)
- No method nodes (OPC UA Method calls)
- No complex/structured data types (only scalar built-in types)
- No multi-server instances in one application (one server per app instance)
- No CLI mode (GUI only for this application)
