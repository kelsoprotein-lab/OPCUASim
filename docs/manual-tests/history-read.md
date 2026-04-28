# 手测:HistoryRead

子项目 #5(Tier-1 第 5 项)的验证脚本。`opcuasim-core` 自身不带 historian,所以 e2e 测试不能用内嵌 server,只能对外部 historian 验证。

## 前置条件

装好任一带 historian 的 OPC UA Server:

- **Prosys OPC UA Simulation Server**(免费 GUI,自带 Counter / Sinusoid 节点和 historian),推荐
- **KEPServerEX**(商业,试用即可),勾上 channel 的 "Enable historian"
- **open62541** 自带 `examples/server_history` 示例

启动 server,在其 UI/配置中开启历史归档。

## 步骤

1. `cargo run --release -p opcuamaster-egui`
2. 顶栏 → 新建连接,填入服务器 URL,Anonymous,连接成功
3. 顶栏 → 浏览节点,找到一个有历史数据的 Variable(例如 Prosys 的 `Objects/Simulation/Counter` 或 `Sinusoid`)
4. 等订阅几分钟,让 historian 攒下数据
5. **在浏览面板中右键该 Variable → "📈 查看历史"**
6. 中央面板自动切换到新 Tab,观察:
   - 默认时间范围:过去 5 分钟
   - 折线图应有上升 / 周期波形(Counter 单调递增,Sinusoid 正弦)
   - 表格列出每个采样点(Source Timestamp / Value / Status)
7. 点 "1h" 快捷按钮,刷新自动触发,看到更长时间范围的数据
8. 自定义起止时间(改 RFC3339 字符串,如 `2026-04-28T08:00:00Z`),点 "🔄 刷新",数据应反映新范围
9. **在监控表中右键已订阅节点 → "📈 查看历史"** —— 应同样开新 Tab
10. 点 Tab 标题左侧的"✕ 关闭"按钮,Tab 消失,数据表 Tab 应仍可切回
11. 同时打开 2-3 个 History Tab,验证 Tab 之间切换不会数据串扰

## 已知限制(本期)

- 仅 Float / Double / Int* / UInt* / Bool 等数值类型才会画折线;String/Bool 显示为文本表
- 时间输入是 RFC3339 字符串,没有日历选择器
- 单 Tab 单节点;不支持多节点叠加图
- 默认 5 分钟范围、上限 5000 点;改更大需 server 配合
- 第一次打开 Tab 自动触发首次刷新;若服务器 5 分钟内无数据,会看到空表(无错误提示)

## 失败模式排查

- "history_read failed: Bad…" → 检查服务器是否已启用历史归档(很多 server 默认不开)
- 折线图为空但表格有数据 → 该节点是非数值类型(String/ByteString 等),`numeric` 列均为 None
- "invalid time '...'" 错误 → RFC3339 格式不合法,起止字段必须含时区(如 `Z` 或 `+08:00`)
