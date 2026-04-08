# OPCUASim

跨平台 OPC UA 仿真套件 — 当前包含 **OPCUAMaster**（采集主站/客户端），基于 Tauri 2、Rust 和 Vue 3 构建。可连接任意 OPC UA 服务器，浏览地址空间，实时监控数据。

[English](README.md)

## 下载

**[最新版本下载](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest)**

| 平台 | OPCUAMaster |
|------|------------|
| macOS (Apple Silicon) | [.dmg](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_aarch64.dmg) |
| macOS (Intel) | [.dmg](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_x64.dmg) |
| Windows | [.exe](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_x64-setup.exe) / [.msi](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_x64_en-US.msi) |
| Linux | [.deb](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_amd64.deb) / [.AppImage](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster_0.2.0_amd64.AppImage) / [.rpm](https://github.com/kelsoprotein-lab/OPCUASim/releases/latest/download/OPCUAMaster-0.2.0-1.x86_64.rpm) |

## 功能

### OPCUAMaster — 采集主站（客户端）

- **OPC UA DA 数据采集** — 连接任意 OPC UA 服务器，浏览地址空间，读写变量值
- **安全模式** — 支持 None、Sign、SignAndEncrypt；匿名、用户名密码、证书三种认证方式
- **地址空间浏览器** — 无限深度懒加载树，展开文件夹发现 Variable 节点
- **智能节点采集** — 选择 Object 节点自动收集其下所有 Variable 子节点
- **订阅 + 轮询** — 通过 OPC UA 订阅（服务器推送）或可配置轮询间隔监控节点
- **实时数据表格** — 虚拟滚动表格，支持搜索/过滤，NodeId 简化显示，列宽自适应
- **值详情面板** — 查看选中节点属性（NodeId、DisplayName、DataType、Value、Quality、Timestamp）
- **通信日志** — 实时请求/响应日志，支持方向过滤、服务类型过滤、搜索、CSV 导出
- **项目文件** — 保存/加载连接配置为 `.opcuaproj` 文件
- **自定义分组** — 将监控节点组织到命名分组中
- **自动重连** — 指数退避重连策略（1s → 2s → 4s → ... → 最大 60s）
- **大地址空间** — 支持 65535 数组元素、128MB 消息，适应大规模服务器

## 使用说明

### 快速开始

1. 启动 OPCUAMaster，点击工具栏中的 **New Connection**
2. 输入连接名称（如 "本地服务器"）和 OPC UA 端点地址（如 `opc.tcp://localhost:4840`）
3. 在左侧面板选中该连接，点击 **Connect**
4. 连接成功后，点击 **Browse Nodes** 打开地址空间浏览器

### 浏览地址空间

- **Browse Panel** 以懒加载树形结构展示服务器地址空间
- 点击文件夹节点可展开查看子节点
- 选择 **Variable** 节点将其添加到监控数据表
- 选择 **Object** 节点可自动采集其下所有 Variable 子节点（智能采集）
- 选中节点时会显示其属性（NodeId、DisplayName、DataType 等）

### 数据监控

- 被监控的节点显示在中央 **Data Table** 中，实时更新数值
- 两种采集模式：
  - **Subscription**（默认）— 服务器主动推送数据变化
  - **Polling** — 客户端按可配置间隔轮询读取
- 数据表支持搜索/过滤、大量节点虚拟滚动、列宽自适应
- 选中一行可在右侧 **Value Panel** 查看详细属性（NodeId、DisplayName、DataType、Value、Quality、Timestamp）
- 可通过右键菜单或工具栏按钮移除监控节点

### 通信日志

- 底部 **Log Panel** 实时记录所有 OPC UA 请求/响应消息
- 点击日志栏可展开/收起面板
- 支持按方向（Request/Response）、服务类型或自由文本过滤
- 点击工具栏 **Export Logs** 将日志导出为 CSV 文件
- 使用 **Clear** 清空日志缓冲区

### 项目文件

- 点击工具栏 **Save** 将所有连接配置保存为 `.opcuaproj` 文件
- 点击 **Open** 加载之前保存的项目，恢复所有连接和分组
- 项目文件存储：连接名称、端点地址、安全设置、认证配置、监控节点分组

### 架构

- **纯 Rust 后端** — `opcuasim-core` 库使用 `async-opcua` 客户端，Tokio 全异步
- **Tauri 2 桌面应用** — 通过 WebView 的原生桌面应用，跨平台（macOS、Windows、Linux）
- **Vue 3 + TypeScript** — 响应式 UI，Composition API，@tanstack/vue-virtual 虚拟滚动
- **Catppuccin Mocha 主题** — 深色主题，与 [ModbusSim](https://github.com/kelsoprotein-lab/ModbusSim) 和 [IEC104 Simulator](https://github.com/kelsoprotein-lab/IEC60870-5-104-Simulator) 风格一致
- **可插拔输出** — `DataOutput` trait，未来可扩展 MQTT、InfluxDB、REST API 等输出

## 开发

### 环境要求

- Rust 1.77+
- Node.js 18+
- npm

### 构建与运行

```bash
# 安装前端依赖
npm install

# 开发模式运行
cd crates/opcuamaster-app
cargo tauri dev

# 生产构建
cargo tauri build
```

### 项目结构

```
OPCUASim/
├── crates/
│   ├── opcuasim-core/          # 核心 OPC UA 库
│   │   └── src/
│   │       ├── client.rs       # 连接管理
│   │       ├── browse.rs       # 节点浏览 + Variable 收集
│   │       ├── subscription.rs # OPC UA 订阅管理
│   │       ├── polling.rs      # 轮询管理
│   │       ├── config.rs       # 配置 + 项目文件
│   │       └── ...
│   └── opcuamaster-app/        # Tauri 桌面应用
│       └── src/
│           ├── commands.rs     # 22 个 Tauri IPC 命令
│           └── state.rs        # 应用状态 + DTO
├── master-frontend/            # Vue 3 前端
│   └── src/
│       ├── App.vue             # 网格布局
│       └── components/         # Toolbar、Tree、DataTable 等
└── shared-frontend/            # 共享 composables + 组件
```

## 参与贡献

欢迎贡献代码！请遵循以下流程：

1. Fork 本仓库
2. 从 `master` 创建功能分支（`git checkout -b feat/your-feature`）
3. 进行修改
4. 确保代码编译通过且无警告：
   ```bash
   cargo clippy --workspace -- -D warnings
   cd master-frontend && npx vue-tsc --noEmit
   ```
5. 使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范提交（如 `feat:`、`fix:`、`refactor:`）
6. 推送到你的 Fork 并创建 Pull Request

### 代码风格

- **Rust** — 遵循标准 Rust 规范。提交前运行 `cargo fmt` 和 `cargo clippy`。零警告策略。
- **TypeScript/Vue** — 使用 Composition API + `<script setup>`。组件保持职责单一。
- **提交信息** — 使用规范化前缀：`feat:`、`fix:`、`refactor:`、`docs:`、`chore:` 等。

### 报告问题

请在 [GitHub Issues](https://github.com/kelsoprotein-lab/OPCUASim/issues) 中提交，包含：
- 操作系统及版本
- 复现步骤
- 期望行为与实际行为
- OPC UA 服务器信息（如相关）

## 许可证

MIT
