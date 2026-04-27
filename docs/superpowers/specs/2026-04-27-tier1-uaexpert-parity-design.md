# Tier-1 UAExpert Parity Design

**日期:** 2026-04-27
**作者:** OPCUASim 维护者
**范围:** opcuamaster-egui 主站,部分 opcuasim-core server 扩展
**预计工期:** 7–10 个工作日,5 个子项目串行推进

---

## 1. 背景

`opcuamaster-egui` 已实现连接/浏览/订阅/读写/分组/项目持久化等核心功能,与 UAExpert 在"日常巡检式使用"上仍有 5 处显著差距(详见 brainstorming 记录的"一档"清单)。本 spec 总览 5 个子项目的方向、范围边界与共用决策,具体实施细节交由各子项目自己的 plan.md。

## 2. 目标

让 `opcuamaster-egui` 在以下场景与 UAExpert 持平:

1. 连接陌生服务器时无需事先猜对 SecurityPolicy/SecurityMode,可拉取 endpoint 列表选择
2. 服务器证书首次拒绝后,可在 GUI 内信任,无需手动操作文件
3. 订阅大批量模拟值时可设 Deadband,降低订阅噪声
4. 浏览到 Method 节点时可右键调用,带参数编辑
5. 读取节点历史值,以折线图与表格展示

## 3. 范围

**IN:** 上述 5 个子项目主站侧改动 + Method 测试所需的 server 端扩展(`opcuasim-core`)。

**OUT(后续可单独立项):**
- 自定义 Struct/Enum 解码
- Events / AlarmsAndConditions
- PubSub
- GDS / Push 证书签发
- Server 端 Historian 完整实现(本期手测即可)

## 4. 共用决策

### 4.1 PKI 目录:`./pki-master`(运行目录相对路径)

沿用现状,不引入家目录方案。Trust List 管理对话框直接读写 `./pki-master/{trusted,rejected,issuers}/` 三个子目录;切换工作目录会得到不同的 trust list,这是预期行为。

### 4.2 测试策略:混合

| 子项目 | 测试方式 |
|---|---|
| #1 Endpoint Discovery | e2e 自动化(扩 `master_full_flow` 或新建 test) |
| #2 Trust List | 单元测试 + 手测(纯文件操作,不在线 server) |
| #3 Deadband | e2e 自动化 |
| #4 Method Call | **opcuasim-core server 注册一个测试 method**(例如 `Demo.Echo(InputArguments=arg) -> OutputArguments=arg`) → e2e 自动化 |
| #5 HistoryRead | **不写 e2e**,提供手测脚本(对 KEPServer / Prosys / open62541 demo)。`opcuasim-core` 不实现 historian |

### 4.3 沿用既定代码风格

不在本 spec 引入新风格,所有新代码继承当前的:
- **core API 形态:** 无状态操作 = `pub async fn name(session: &Arc<Session>, ...)` 自由函数;有状态对象 = 自带 struct(参考 `subscription.rs`)
- **后端通信:** 新增 `UiCommand` / `BackendEvent` 变体走 `dispatcher.rs` 现有 match 路由;问答式命令带 `req_id`
- **CommLog:** 新协议操作(GetEndpoints / Call / HistoryRead)在 `dispatcher.rs` 入口和返回处各打一条日志条目,与 connect/browse/read 一致
- **错误 UX:** 协议错误走 `BackendEvent::Toast`,字段校验错误内联在对话框

### 4.4 不动 `opcuasim-core` server 的 default profile

server 的默认配置保持向后兼容。Method 测试节点只在 e2e 测试 fixture 中显式注册,不出现在生产启动路径中。

## 5. 子项目摘要

### #1 Endpoint Discovery

**动机:** 当前 ConnectionDialog 要求用户事先选 SecurityPolicy 与 Mode,猜错就连不上。UAExpert 通过 `GetEndpoints` 服务调用拉列表给用户挑。

**core 改动:**
- 新增 `pub async fn discover_endpoints(url: &str, timeout_ms: u64) -> Result<Vec<EndpointInfo>>`(放 `client.rs` 或新建 `discovery.rs`)
- `EndpointInfo` DTO:`endpoint_url / security_policy_uri / security_mode / user_token_policies / server_certificate_thumbprint`

**UI 改动:**
- 复用 ConnectionDialog 现有的 URL 输入框,旁边新增"发现"按钮;按下后 spawn 命令拉列表,渲染到对话框中段的 endpoint 表格
- 用户点选某行 → 自动填充下半部的 policy/mode/auth 选项
- 保留"手动模式"(用户可不点发现直接填)

**测试:** e2e 自动化,对 `opcuasim-core` 内嵌 server 跑 GetEndpoints 验证返回非空且 policy 列表与配置匹配。

---

### #2 Trust List 管理

**动机:** 服务器证书第一次连不上时落到 `./pki-master/rejected/`,用户当前只能 `mv` 到 `trusted/`。需要 GUI。

**core 改动:**
- 新建 `cert_manager.rs`(或 `pki.rs`),纯文件 IO:
  - `list_certificates(role: Trusted | Rejected) -> Vec<CertSummary>`
  - `move_certificate(from: PathBuf, to_role: Trusted | Rejected | Discard)`
  - `read_certificate_metadata(path: &Path) -> Result<CertMeta>`(subject / issuer / thumbprint / valid_from / valid_to)
- 用 `x509-parser` crate 解析证书元信息(`opcua-crypto` 已经间接依赖,加直接依赖即可)

**UI 改动:**
- 顶栏新菜单项 "证书管理…",弹出对话框
- 两栏列表(Trusted / Rejected),双击显示详情,按钮 Trust / Reject / Delete

**测试:** 单元测试覆盖文件操作(tempdir);手测覆盖 GUI 流程。

---

### #3 DataChangeFilter / Deadband

**动机:** 当前订阅一律按 sampling interval 全推,模拟连续值噪声多、UI 与日志压力大。`async-opcua` 客户端支持 `MonitoringFilter::DataChangeFilter`,只需透传。

**core 改动:**
- `SubscriptionManager::add_nodes` 增加可选参 `filter: Option<DataChangeFilterCfg>`
- DTO:`DataChangeFilterCfg { trigger: StatusValue|StatusValueTimestamp, deadband_type: None|Absolute|Percent, deadband_value: f64 }`
- 内部转 `MonitoringFilter::DataChangeFilter`,塞进 `MonitoredItemCreateRequest`

**UI 改动:**
- BrowsePanel "添加监控"区(`browse_panel.rs` 的 interval DragValue 同行/下方),增加可折叠"高级"区:trigger 下拉 + deadband 类型下拉(None/Absolute/Percent)+ 数值输入
- `MonitoredNodeReq` 扩字段 `filter: Option<DataChangeFilterCfg>`
- 默认值保留当前行为(filter=None,行为与今天一致)

**测试:** e2e 用 `Demo.Sine`(已有,4s 周期)订阅,absolute deadband=5.0,验证收到的样本数显著少于不加 filter。

---

### #4 Method Call

**动机:** OPC UA 服务器常用 Method 暴露控制操作(reset、startBatch),UAExpert 必备。当前 BrowsePanel 不区分 NodeClass,Method 节点也不能调用。

**core 改动:**
- `browse.rs` 输出补 `node_class: NodeClass` 字段(已经 RawNodeId 解析后能拿到)
- 新加 `pub async fn call_method(session: &Arc<Session>, object_id: &NodeId, method_id: &NodeId, inputs: Vec<Variant>) -> Result<Vec<Variant>>`
- 读 InputArguments / OutputArguments 属性的辅助函数(用于 UI 渲染参数表单)
- `opcuasim-core` server 端的 e2e fixture 注册一个测试 Method:`Demo.Echo(input: String) -> output: String`,直接返回入参

**UI 改动:**
- BrowsePanel 在节点旁标注图标区分 Object/Variable/Method
- Method 节点右键 → "调用…",弹 MethodDialog:上半部按 InputArguments 类型渲染输入控件,下半部空着等结果;"执行"后下半部填 OutputArguments

**测试:** e2e 调用 `Demo.Echo("hello")` 断言返回 `"hello"`。

---

### #5 HistoryRead

**动机:** 读节点历史是 UAExpert 高频用例,看趋势必备。

**core 改动:**
- 新加 `pub async fn history_read_raw(session, node_id, start, end, max_values, return_bounds) -> Result<Vec<DataValue>>`
- 走 `Session::history_read` + `ReadRawModifiedDetails`,处理 continuationPoint 直至取尽

**UI 改动:**
- 入口两处:DataTable 行右键(对当前监控项)与 BrowsePanel 节点右键(对未订阅节点),都触发 "查看历史"
- 在 model 里加 `Vec<HistoryTab>`,中央面板顶部加 Tab 切换(默认 Tab 是 DataTable,每个 HistoryTab 是单独节点的视图,可关闭)
- 工具栏:开始/结束时间(`egui_extras::DatePickerButton` 或纯输入框)、点数上限、刷新按钮
- 上半部 `egui_plot::Plot` 折线;下半部 `TableBuilder` 列出每条 (timestamp, value, status)

**测试:** 不写 e2e。提供手测脚本 `docs/manual-tests/history-read.md`,要求测试者:
- 启动 KEPServer 或 Prosys Simulation Server(配 historian)
- 用 master 连接,订阅一个 Counter 5 分钟
- 右键 → 查看历史 → 选最近 5 分钟 → 折线应单调递增

## 6. 验证与里程碑

每个子项目完成后必须满足:

1. `cargo build --workspace` 无 warning(沿袭当前要求)
2. `cargo clippy --workspace -- -D warnings` 通过
3. 该子项目对应的 e2e 测试或手测全过
4. 提交一条独立 commit(主题:`feat(master): <subproject>`),不含任何 AI 署名
5. 推送到 master(用户已授权全流程)

完成全部 5 项后:
- 整体 e2e `master_full_flow` 仍通过
- 跑一遍对 Prosys/KEPServer 的连接 + 订阅 + 读历史的烟测

## 7. 风险

| 风险 | 缓解 |
|---|---|
| `egui_plot` 在当前 egui 0.34 版本可能有 API 漂移 | 在子项目 #5 启动前先 spike 一个最小 demo,失败则回退到表格 + 后续再加图 |
| `x509-parser` 与 `opcua-crypto` 间接依赖版本冲突 | 子项目 #2 启动前 `cargo tree -i x509-parser` 检查;冲突则切到 `rcgen` 之类已用过的库 |
| Method 的 InputArguments 类型不全(Struct、Array)在表单中难渲染 | 本期只支持 scalar 内置类型(Bool/Int*/UInt*/Float/Double/String),Struct/Array 走"原始 JSON 字符串"输入,标 TODO |
| HistoryRead 大量数据(>10k 点)在 UI 卡顿 | 默认上限 5000;超过则强制走分页(continuationPoint 分批拉取并增量绘制),交互上提示 |
| Endpoint Discovery 对自签证书失败 | discover_endpoints 不验证证书(等价 UAExpert 行为),只展示 thumbprint 让用户决定 |

## 8. 流程

5 个子项目串行(`TaskList` 已建好依赖):
- 每项独立 plan.md(由 writing-plans skill 起草) → 实现 → 自测 → commit → push
- 上一项合入后再启动下一项的 plan,避免上下文堆积
