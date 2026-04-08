# OPCUASim

Cross-platform OPC UA simulation suite — currently includes **OPCUAMaster** (master station / client), built with Tauri 2, Rust, and Vue 3. Connects to any OPC UA server, browses the address space, and monitors data in real-time.

[中文文档](README_CN.md)

## Download

**[Latest Release](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest)**

| Platform | OPCUAMaster |
|----------|------------|
| macOS (Apple Silicon) | [.dmg](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_aarch64.dmg) |
| macOS (Intel) | [.dmg](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_x64.dmg) |
| Windows | [.exe](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_x64-setup.exe) / [.msi](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_x64_en-US.msi) |
| Linux | [.deb](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_amd64.deb) / [.AppImage](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.1.0_amd64.AppImage) / [.rpm](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster-0.1.0-1.x86_64.rpm) |

## Features

### OPCUAMaster — Master Station (Client)

- **OPC UA DA (Data Access)** — Connect to any OPC UA server, browse address space, read/write variable values
- **Security Support** — None, Sign, SignAndEncrypt modes; Anonymous, Username/Password, and Certificate authentication
- **Address Space Browser** — Infinite-depth lazy-loading tree, expand folders to discover Variable nodes
- **Smart Node Collection** — Select an Object node to automatically collect all Variable children underneath
- **Subscription + Polling** — Monitor nodes via OPC UA subscription (server push) or configurable polling interval
- **Real-time Data Table** — Virtual-scrolled table with search/filter, short NodeId display, flex-responsive columns
- **Value Panel** — View selected node attributes (NodeId, DisplayName, DataType, Value, Quality, Timestamp)
- **Communication Log** — Real-time request/response logging with direction filter, service filter, search, and CSV export
- **Project Files** — Save/load connection configurations as `.opcuaproj` files
- **Custom Groups** — Organize monitored nodes into named groups
- **Auto-Reconnect** — Exponential backoff reconnection (1s → 2s → 4s → ... → 60s max)
- **Robust Decoding** — Handles large address spaces (65535 array elements, 128MB messages)

## Usage

### Quick Start

1. Launch OPCUAMaster and click **New Connection** in the toolbar
2. Enter a connection name (e.g. "Local Server") and the OPC UA endpoint URL (e.g. `opc.tcp://localhost:4840`)
3. Select the connection in the left panel, then click **Connect**
4. Once connected, click **Browse Nodes** to open the address space browser

### Browsing Address Space

- The **Browse Panel** displays the server's address space as a lazy-loading tree
- Click any folder node to expand and discover child nodes
- Select a **Variable** node to add it to the monitoring data table
- Select an **Object** node to automatically collect all Variable children underneath (smart collection)
- Node attributes (NodeId, DisplayName, DataType, etc.) are shown when you select a node

### Monitoring Data

- Monitored nodes appear in the central **Data Table** with real-time value updates
- Two access modes are available:
  - **Subscription** (default) — server pushes data changes to the client
  - **Polling** — client reads values at a configurable interval
- The data table supports search/filter, virtual scrolling for large node sets, and flex-responsive columns
- Select a row to view detailed attributes in the right **Value Panel** (NodeId, DisplayName, DataType, Value, Quality, Timestamp)
- Right-click or use toolbar buttons to remove nodes from monitoring

### Communication Log

- The bottom **Log Panel** records all OPC UA request/response messages in real-time
- Click the log bar to expand/collapse the panel
- Filter logs by direction (Request/Response), service type, or free-text search
- Click **Export Logs** in the toolbar to download logs as a CSV file
- Use **Clear** to reset the log buffer

### Project Files

- Click **Save** in the toolbar to save all connection configurations to a `.opcuaproj` file
- Click **Open** to load a previously saved project, restoring all connections and groups
- Project files store: connection name, endpoint URL, security settings, authentication config, and monitored node groups

### Architecture

- **Pure Rust Backend** — `opcuasim-core` library with `async-opcua` client, fully async with Tokio
- **Tauri 2 Desktop** — Native desktop app via WebView, cross-platform (macOS, Windows, Linux)
- **Vue 3 + TypeScript** — Reactive UI with Composition API, virtual scrolling via @tanstack/vue-virtual
- **Catppuccin Mocha Theme** — Dark theme consistent with [ModbusSim](https://github.com/kelsoprotein-lab/ModbusSim) and [IEC104 Simulator](https://github.com/kelsoprotein-lab/IEC60870-5-104-Simulator)
- **Pluggable Output** — `DataOutput` trait for future integration (MQTT, InfluxDB, REST API)

## Development

### Prerequisites

- Rust 1.77+
- Node.js 18+
- npm

### Build & Run

```bash
# Install frontend dependencies
npm install

# Run in development mode
cd crates/opcuamaster-app
cargo tauri dev

# Build for production
cargo tauri build
```

### Project Structure

```
OPCUASim/
├── crates/
│   ├── opcuasim-core/          # Core OPC UA library
│   │   └── src/
│   │       ├── client.rs       # Connection management
│   │       ├── browse.rs       # Node browsing + variable collection
│   │       ├── subscription.rs # OPC UA subscription manager
│   │       ├── polling.rs      # Polling manager
│   │       ├── config.rs       # Configuration + project files
│   │       └── ...
│   └── opcuamaster-app/        # Tauri desktop app
│       └── src/
│           ├── commands.rs     # 22 Tauri IPC commands
│           └── state.rs        # App state + DTOs
├── master-frontend/            # Vue 3 frontend
│   └── src/
│       ├── App.vue             # Grid layout
│       └── components/         # Toolbar, Tree, DataTable, etc.
└── shared-frontend/            # Shared composables + components
```

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch from `master` (`git checkout -b feat/your-feature`)
3. Make your changes
4. Ensure the code compiles and passes checks:
   ```bash
   cargo clippy --workspace -- -D warnings
   cd master-frontend && npx vue-tsc --noEmit
   ```
5. Commit with a descriptive message following [Conventional Commits](https://www.conventionalcommits.org/) (e.g. `feat:`, `fix:`, `refactor:`)
6. Push to your fork and open a Pull Request

### Code Style

- **Rust** — Follow standard Rust conventions. Run `cargo fmt` and `cargo clippy` before committing. Zero warnings policy.
- **TypeScript/Vue** — Use Composition API with `<script setup>`. Keep components focused and single-purpose.
- **Commits** — Use conventional commit prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, etc.

### Reporting Issues

Please open a [GitHub Issue](https://github.com/kelsoprotein-lab/OPCUASim/issues) with:
- OS and version
- Steps to reproduce
- Expected vs actual behavior
- OPC UA server info (if relevant)

## License

MIT
