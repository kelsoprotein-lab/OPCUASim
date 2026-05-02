# OPCUASim

跨平台 OPC UA 仿真套件 — 基于 [egui](https://www.egui.rs/) 与 [`async-opcua`](https://crates.io/crates/async-opcua) 的纯 Rust 桌面应用。

| 可执行文件 | 角色 |
|-----------|------|
| **OPCUAMaster** | 采集主站 / 客户端 — 连接、浏览、监控、历史、方法调用 |
| **OPCUAServer** | 地址空间仿真器 — 文件夹、带仿真模式的变量、可选写入 |

[English](README.md)

## 功能

### OPCUAMaster — 客户端 / 主站

- **OPC UA DA** — 连接任意 OPC UA 服务器，浏览地址空间，读写变量值
- **安全模式** — None / Sign / SignAndEncrypt；匿名、用户名密码、证书三种认证方式
- **端点发现** — 输入服务器 URL 即可枚举所有可用端点及其安全配置
- **地址空间懒加载浏览** — 无限深度树形，按需展开
- **智能变量收集** — 选中 Object 节点一键添加其下所有 Variable 子节点
- **订阅 + 轮询** — 服务器推送或客户端按可配间隔拉取，支持按节点配置 `DataChangeFilter`
- **实时表格** — 支持搜索、`Ctrl/Cmd+Click` 多选、质量颜色编码
- **值与写入面板** — 节点属性、手动读取、向可写节点写入
- **历史读取(HA)** — 读取历史原始值到 Plot + Table Tab，提供 1m … 24h 快捷范围
- **方法调用** — 自动发现入参/出参信息并从浏览器调用
- **通信日志** — 底部面板，方向过滤、搜索、CSV 导出
- **项目文件** — 把所有连接 + 分组保存/加载为 `.opcuaproj`
- **证书管理** — 列出、信任/拒绝、删除本地 PKI 证书

### OPCUAServer — 地址空间仿真器

- **内嵌 OPC UA 服务端** — 默认监听 `opc.tcp://0.0.0.0:4840`
- **文件夹 + 变量树** — 在 `Objects` 下添加文件夹和变量
- **仿真模式** — `Static`、`Random`、`Sine`、`Linear`（Repeat / Bounce）、`Script`（`evalexpr`）
- **实时数值** — 变量按各自的间隔更新并推送到 UI
- **可写节点** — 勾选 `RW` 即可让客户端写入
- **项目文件** — 把整个地址空间保存/加载为 `.opcuaproj`

## 开发

### 环境要求

- Rust 1.77+
- 系统需有 CJK 字体（macOS 的 PingFang、Windows 的微软雅黑、Linux 的 Noto Sans CJK），启动时自动检测

### 构建与运行

```bash
# 主站
cargo run -p opcuamaster-egui --release

# 服务端仿真器
cargo run -p opcuaserver-egui --release
```

### 项目结构

```
OPCUASim/
├── crates/
│   ├── opcuasim-core/          # 核心库：client、server、browse、subscription、polling、history、methods
│   ├── opcuaegui-shared/       # 共享 egui 组件：theme、widgets、fonts、tokio runtime handle、settings
│   ├── opcuamaster-egui/       # OPCUAMaster 桌面应用
│   └── opcuaserver-egui/       # OPCUAServer 桌面应用
├── pki/                        # 本地 PKI（trusted / rejected / own）
└── docs/                       # 设计文档与实施计划
```

## 参与贡献

1. Fork 仓库并从 `master` 创建特性分支
2. 提交前执行 `cargo fmt` 和 `cargo clippy --workspace -- -D warnings`
3. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 前缀：`feat:`、`fix:`、`refactor:`、`docs:`、`chore:`
4. 向 `master` 发起 PR

## 更新日志

详见 [CHANGELOG.md](CHANGELOG.md) 与 [Releases](https://github.com/kelsoprotein-lab/OPCUASim/releases) 页面。

## 许可证

MIT
