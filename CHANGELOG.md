# Changelog

All notable changes to this project are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-05-02

### Highlights / 亮点

- 🎨 **统一视觉系统** / Unified industrial-dark theme + shared widget kit (`status_chip`, `info_row`, `empty_state`, `toast_card`) across master and server.
- 🧩 **Master UI 全面打磨** / Master UI overhaul: grouped toolbar with shortcut tooltips, status chips on connection list, inline-closable history tabs, friendly empty states.
- 📈 **历史数据可读性提升** / History tab now plots against real timestamps (HH:MM:SS axis), highlights the active quick-range, and shows hover values.
- 🔧 **Server UI 多选 + 防抖编辑** / Server gains Ctrl/Cmd multi-select with bulk delete, right-click "新建子文件夹", and lost-focus property commits (no more per-pixel `UpdateNode` spam).
- 🔐 **新增 Endpoint Discovery / 证书管理 / 方法调用 / DataChangeFilter / 历史读取** / New since v0.3.0: endpoint discovery, PKI trust manager, method call dialog, `DataChangeFilter`/deadband, history read raw — all from prior commits, polished and shipped together.
- 🧹 **架构清理** / Cleanup: removed legacy `master-frontend/`, `server-frontend/`, `crates/opcuamaster-app/` Vue/Tauri leftovers; rewrote `release.yml` to plain `cargo build` + `softprops/action-gh-release`.

### Added 新增

- `opcuaegui-shared::theme` — industrial-dark palette (teal accent, status colours) + `apply(ctx)` / `opcuaegui-shared::theme` — 工业暗色主题（teal 强调色 + 状态色）+ 一键 `apply(ctx)`.
- `opcuaegui-shared::widgets` — reusable `section_label`, `info_row`, `status_chip`, `empty_state`, `toast_card` / 通用展示组件 5 件套，统一两个 app 的视觉.
- Master: closable per-tab history view with rounded "browser-tab" visuals / Master 中央 tab 内嵌 ✕ 关闭 + 圆角浏览器风格.
- Master: history plot uses real timestamps + active quick-range highlight + hover label / 历史 Plot 时间轴 + 激活范围高亮 + hover tooltip.
- Server: Ctrl/Cmd multi-select in node table with bulk-delete chip / 节点表 Ctrl/Cmd 多选 + 顶部批量删除.
- Server: right-click "新建子文件夹" on any folder in the address tree / 地址空间任意文件夹右键新建子文件夹.
- CI: `release.yml` extracts the active CHANGELOG section into `RELEASE_BODY.md` and feeds it to `softprops/action-gh-release` so the GitHub release page mirrors the changelog / CI 自动从 CHANGELOG 抽取当版本 section 写入 release body.

### Changed 改进

- All toasts now render through `widgets::toast_card` (rounded card, accent border, theme-aware text) instead of inline `Frame::popup` / 所有 toast 统一走 `toast_card`，圆角带强调色描边.
- Master toolbar regrouped into Connection / Data / Project / System with shortcut tooltips and a header-side status chip / Master toolbar 按"连接 / 数据 / 项目 / 系统"分组并显示当前选中连接的 status chip.
- Master `value_panel` switches to shared `section_label` + `info_row`, drops redundant write-success text in favour of the toast / Master 详情面板改用共享 widget，删除冗余写入提示文本.
- Server property editor commits on `lost_focus + changed`, eliminating per-pixel network spam during DragValue interaction / Server 属性编辑改为 `lost_focus + changed` 才提交.
- Server `node_table` shows `RW` in accent colour + selection chip; address tree, status bar and toolbar all migrate to theme constants / Server 节点表 `RW` 强调色，状态栏/工具栏/地址树统一用主题常量.
- README/README_CN rewritten to reflect the actual egui-based architecture (no more Tauri/Vue references) / README 重写为反映实际 egui 架构.

### Fixed 修复

- Connection-tree state badge is now an actual chip (background + border + label) instead of a single hard-to-read coloured dot / 连接树状态徽章改为完整的 chip（背景+描边+文字），不再只有难辨识的彩色圆点.
- Browse panel and data table no longer show bare `(无数据)` strings; they render proper `empty_state` cards with guidance / 浏览面板/监控表移除裸的 `(无数据)` 文案，改为带操作引导的空状态卡片.

### Removed 移除

- Deleted orphan `master-frontend/` and `server-frontend/` directories (Vue dist remains, no source) / 删除孤立的 `master-frontend/` 与 `server-frontend/` 目录.
- Deleted empty `crates/opcuamaster-app/` (Tauri shell from the pre-rewrite era) / 删除空壳 `crates/opcuamaster-app/`.
- Removed `model::ValuePanelState::last_result` rendering — toast already covers it / 移除 `value_panel.last_result` 的冗余渲染.

### Internal

- Added `subfolder_inputs: HashMap<String, String>` and `selected_node_ids: HashSet<String>` to server `AppModel` to back the new sub-folder + multi-select flows / 新增字段支撑子文件夹与多选.
- All clippy `-D warnings` clean across the workspace; full test suite passes (28 tests, 0 failed) / 全 workspace clippy `-D warnings` 通过；测试套件 28 个全绿.

## [0.3.0] - prior

Endpoint discovery, PKI trust-list manager, method call dialog with auto-discovered I/O,
`DataChangeFilter` + deadband on subscriptions, history read raw with continuation-point
loop and Plot/Table viewer. See `git log v0.2.0..v0.3.0` for details.

## [0.2.0] - prior

Initial Rust+egui rewrite of master and server, replacing the Tauri/Vue prototype.
See `git log v0.1.0..v0.2.0` for details.

## [0.1.0] - prior

Initial public release (Tauri 2 + Vue 3 prototype). Superseded by the egui rewrite in v0.2.0.
