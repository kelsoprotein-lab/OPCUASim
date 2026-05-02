# OPCUASim

Cross-platform OPC UA simulation suite — pure Rust desktop apps built with [egui](https://www.egui.rs/) and the [`async-opcua`](https://crates.io/crates/async-opcua) stack.

| Binary | Role |
|--------|------|
| **OPCUAMaster** | Master station / client — connect, browse, monitor, history, methods |
| **OPCUAServer** | Address-space simulator — folders, variables with simulation modes, optional writable nodes |

[中文文档](README_CN.md)

## Features

### OPCUAMaster — Client / Master Station

- **OPC UA DA** — connect to any OPC UA server, browse address space, read/write values
- **Security** — None / Sign / SignAndEncrypt; Anonymous, Username/Password, Certificate auth
- **Endpoint discovery** — query a server URL to enumerate available endpoints and their security profiles
- **Lazy-loading address browser** — infinite-depth tree, expand on demand
- **Smart variable collection** — pick an Object node to add all Variable descendants in one click
- **Subscription + Polling** — server push or client pull at a configurable interval, per-node `DataChangeFilter`
- **Real-time table** — searchable, multi-select with `Ctrl/Cmd+Click`, quality colour coding
- **Value & Write panel** — node attributes, manual read, value write back to writable nodes
- **History (HA)** — read raw history into a plot + table tab, quick ranges (1m … 24h)
- **Method calls** — auto-discover input/output arguments and invoke methods from the browser
- **Communication log** — bottom panel with direction filter, search, CSV export
- **Project files** — save/load all connections + groups as `.opcuaproj`
- **Certificate manager** — list, trust/reject, delete certificates in the local PKI

### OPCUAServer — Address-Space Simulator

- **Embedded OPC UA server** — defaults to `opc.tcp://0.0.0.0:4840`
- **Folder + Variable tree** — add folders and variables under `Objects`
- **Simulation modes** — `Static`, `Random`, `Sine`, `Linear` (Repeat/Bounce), `Script` (`evalexpr`)
- **Live values** — variable values update at their per-node interval and stream to the UI
- **Writable nodes** — toggle `RW` to let clients write through
- **Project files** — save/load the entire address space as `.opcuaproj`

## Development

### Prerequisites

- Rust 1.77+
- A CJK font on your system (PingFang on macOS, Microsoft YaHei on Windows, Noto Sans CJK on Linux) for Chinese labels — auto-detected at startup

### Build & Run

```bash
# Master station
cargo run -p opcuamaster-egui --release

# Server simulator
cargo run -p opcuaserver-egui --release
```

### Project Structure

```
OPCUASim/
├── crates/
│   ├── opcuasim-core/          # Core library: client, server, browse, subscription, polling, history, methods
│   ├── opcuaegui-shared/       # Shared egui pieces: theme, widgets, fonts, tokio runtime handle, settings
│   ├── opcuamaster-egui/       # OPCUAMaster desktop app
│   └── opcuaserver-egui/       # OPCUAServer desktop app
├── pki/                        # Local PKI (trusted/rejected/own)
└── docs/                       # Design notes & implementation plans
```

## Contributing

1. Fork and create a feature branch from `master`
2. `cargo fmt` and `cargo clippy --workspace -- -D warnings` before committing
3. Conventional commit prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
4. Open a PR against `master`

## Changelog

See [CHANGELOG.md](CHANGELOG.md) and the [Releases](https://github.com/kelsoprotein-lab/OPCUASim/releases) page.

## License

MIT
