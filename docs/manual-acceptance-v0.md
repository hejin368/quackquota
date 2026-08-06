# QuackQuota v0 Windows 手动验收清单

本文档用于 `v0.0.0-overlay-baseline` 之前的真实 Windows 桌面验收。自动化测试和构建通过不代表本清单通过；每一项都需要在目标 Windows 环境中记录结果。

## 已完成的真实账号验证（2026-07）

- 独立 Codex CLI `0.144.5` 可执行，`codex app-server` 可正常启动。
- Overlay 来源显示 `Codex App Server`，当前账号返回一个 7 天额度窗口。
- 实测已用 `35%`、剩余 `65%`，进度条从左侧填充约 `65%`，可用额度重置次数为 `2`。
- 未再出现空白“5小时额度”卡片。
- 以上仅证明该账号和该网络条件下的成功路径，不替代本清单中的代理、故障、显示器和生命周期矩阵。

## 验收边界与安全规则

- 仅使用测试人员自己的 Codex 账号和本机环境。
- 不打开、复制、截图或粘贴 `auth.json` 内容。
- 不在问题单、截图、录屏或日志附件中包含 Token、Cookie、Authorization Header、邮箱或用户目录。
- 不为制造失败场景而把 Codex Bearer Token 重定向到第三方服务器。
- 卸载 CLI、修改网络或切换显示器的测试优先在可还原的 Windows 测试账号、虚拟机或备用机器中完成。
- 每项结果填写：`通过 / 失败 / 阻塞`、Windows 版本、应用构建 SHA、时间和不含个人信息的备注。

## 验收前记录

- [ ] Git 提交或补丁标识：`________________`
- [ ] Windows 版本与构建号：`________________`
- [ ] WebView2 版本：`________________`
- [ ] Codex CLI 版本（若已安装）：`________________`
- [ ] 显示器数量、分辨率和缩放：`________________`
- [ ] 通过 `scripts/dev.ps1` 或 Tauri debug 构建启动，未直接运行指向 Vite dev URL 的裸 Cargo 构建。
- [ ] 测试前确认没有旧的 QuackQuota 进程残留；若需按内部可执行名检查，使用 `QuackQuota.exe`；旧构建可能仍有 `codexbar-desktop-tauri.exe` 残留。

## A. 安装与认证矩阵

### A1. Codex CLI 已安装并已登录

- [ ] `codex --version` 成功，不记录可识别用户身份的信息。
- [ ] 使用 Codex CLI 自身支持的方式确认登录可用，不读取认证文件。
- [ ] 启动应用后从托盘打开 Overlay，未出现登录错误。
- [ ] 结果与备注：`________________`

### A2. Codex CLI 未安装

- [ ] 在干净测试环境中确认 `codex` 命令不可用。
- [ ] 若该环境也没有 CLI 留下的认证文件，Overlay 显示“未登录/无可用数据”，应用不崩溃、不循环弹窗。
- [ ] 若测试环境保留了有效认证文件，单独记录“CLI 不存在但认证仍可用”的结果；当前 provider 读取认证文件，不依赖启动 CLI 进程。
- [ ] 托盘、设置和完整退出仍可用。
- [ ] 结果与备注：`________________`

### A3. Codex CLI 已安装但未登录

- [ ] 使用干净测试账号或 CLI 官方登出流程准备未登录状态，不手工编辑 `auth.json`。
- [ ] Overlay 显示未登录状态，不显示伪造的 `0%` 额度。
- [ ] 手动刷新不会导致崩溃或创建额外窗口。
- [ ] 结果与备注：`________________`

## B. 真实额度与刷新

### B1. 单个动态额度窗口

- [ ] 对当前仅返回一个 7 天窗口的账号，Overlay 只显示一张全宽卡片。
- [x] 已验证样本显示已用 `35%`、剩余 `65%`；进度条从左侧填充约 `65%`，消耗时从右向左缩短。
- [ ] 标题按周期显示“7天额度”，不出现空白“5小时额度”卡片。
- [ ] 已用与剩余之和允许有四舍五入误差，但不得明显超过或低于 100%。
- [ ] 同时显示相对重置倒计时和本地时区绝对重置时间。
- [ ] 数据来源优先显示 `Codex App Server`；若为 `Legacy Codex Provider`，记录 App Server 失败原因类别但不记录原始响应。
- [ ] 结果与备注：`________________`

### B2. 多个动态额度窗口

- [ ] 两个有效周期窗口显示为左右两张卡片，不依赖 primary/secondary 的固定语义。
- [ ] 三个及以上有效窗口时只显示剩余百分比最低的两个，并显示“另外 N 项额度”。
- [ ] `limitName` 存在时用作标题；缺失时按周期生成标题。
- [ ] 60、300、1440、10080 分钟分别显示为 1小时、5小时、24小时、7天额度。
- [ ] 未知周期显示通用“Codex额度”，不得猜测为 5 小时或每周。
- [ ] 某个窗口缺失/非法时不影响其他有效窗口；非法窗口不显示误导进度条。
- [ ] 与同一账号的 Codex 官方额度界面交叉核对，记录差异但不截图账号信息。
- [ ] 结果与备注：`________________`

### B3. 手动刷新

- [ ] 点击刷新后按钮进入忙碌状态，完成后恢复。
- [ ] 最后刷新时间更新；已有窗口不会闪退或重复创建。
- [ ] 刷新只调用 Codex 专用命令；其他 Provider 的更新时间、错误和登录状态均不应变化。
- [ ] 连续快速点击刷新不会产生并发刷新风暴。
- [ ] 结果与备注：`________________`

### B4. 自动刷新

- [ ] 记录当前设置中的刷新间隔，不通过编辑认证文件缩短测试。
- [ ] 保持应用运行超过一个完整刷新周期，确认最后刷新时间和数据自动更新。
- [ ] 自动刷新期间打开/关闭 Overlay，状态不应永久停留在“刷新中”。
- [ ] 设置为手动刷新时，后台周期刷新停止。
- [ ] 结果与备注：`________________`

### B5. 重置倒计时

- [ ] 每个有效窗口的倒计时自然递减，不出现负数、`NaN` 或跳到错误日期。
- [ ] Windows 睡眠/唤醒或系统时间恢复后，倒计时能按绝对重置时间重新计算。
- [ ] 到达重置时刻后应用触发刷新，或明确显示正在等待新数据。
- [ ] 结果与备注：`________________`

## C. 故障与缓存

### C1. 断网

- [ ] 在 Overlay 已有成功数据后断开网络并手动刷新。
- [ ] 应用不崩溃；若保留最后成功数据，必须明确标记为缓存并显示错误。
- [ ] 恢复网络后再次刷新可恢复实时数据。
- [ ] 结果与备注：`________________`

### C2. Provider 请求失败

- [ ] 使用可撤销的本机防火墙/DNS 测试规则阻止官方 Codex 用量主机；不要把请求重定向到第三方地址。
- [ ] 错误状态可读，不含完整 URL 查询秘密、Token、Cookie 或认证文件内容。
- [ ] 解除规则后刷新恢复。
- [ ] 结果与备注：`________________`

### C3. 缓存行为

- [ ] 成功读取后关闭再重新打开 Overlay，进程内缓存可立即显示并明确标记缓存状态。
- [ ] 完整退出程序再启动时，不把进程内旧数据误称为实时数据。
- [ ] 结果与备注：`________________`

### C4. 代理策略与故障矩阵

- [ ] 手动代理为空、系统/环境代理开启且进程具有 `HTTPS_PROXY` 或 `HTTP_PROXY` 时，“当前连接方式”显示系统/环境代理，地址只显示为 `http(s)://***`。
- [ ] 同时存在手动代理与环境代理时，手动代理优先；清空手动代理后自动恢复环境代理。
- [ ] 关闭“使用系统/环境代理”且手动代理为空时使用直接连接。
- [ ] 手动 HTTP 代理能够让 App Server 和 legacy fallback 使用同一连接策略；不得填写或记录真实测试地址。
- [ ] `socks5://`、无主机地址、带查询/片段的地址和嵌入用户名密码的 URL 被拒绝，错误信息不回显原始地址。
- [ ] “测试连接”成功时不读取或发送 Codex Token、Cookie、Authorization Header 或认证文件内容。
- [ ] 分别制造 DNS/网络不可达、代理进程未运行、Codex 未登录、App Server 无法启动和服务端错误，界面显示对应的普通用户文案而非底层原始错误。
- [ ] 手动代理、环境代理、直接连接三种模式下都执行一次真实额度刷新并记录来源标签。
- [ ] 结果与备注：`________________`

## D. 窗口、托盘和生命周期

### D1. 关闭并重新打开 Overlay

- [ ] 点击 Overlay 关闭按钮后窗口消失，托盘程序继续运行。
- [ ] 再次点击托盘“显示 Codex 悬浮窗”可显示同一个隐藏窗口并恢复数据流。
- [ ] 重复至少 10 次，没有白屏、僵尸窗口或句柄异常。
- [ ] 结果与备注：`________________`

### D2. 防止重复窗口

- [ ] 快速连续点击托盘入口至少 10 次。
- [ ] 任意时刻仅存在一个 `codex-overlay` 窗口；已有窗口应被显示并聚焦。
- [ ] 结果与备注：`________________`

### D3. 拖动窗口

- [ ] 按住顶部非按钮区域可拖动窗口。
- [ ] 刷新和关闭按钮不会触发拖动。
- [ ] 拖动后仍保持置顶、透明、无边框且不出现在任务栏。
- [ ] 结果与备注：`________________`

### D4. 关闭程序后重新启动

- [ ] 通过托盘完整退出后确认进程消失。
- [ ] 重新启动应用，托盘可用并能重新创建 Overlay。
- [ ] 将 Overlay 拖到非默认位置，完整退出并重启后恢复到同一显示器的等效逻辑位置。
- [ ] 窗口关闭后从托盘重新打开，位置仍保持；设置中的“恢复悬浮窗默认位置”会把它移到主显示器安全区域。
- [ ] 结果与备注：`________________`

### D5. 启动显示策略

- [ ] 清理测试配置后的首次启动自动显示 Overlay。
- [ ] “记住上次状态”：关闭/隐藏后重启保持隐藏，显示状态退出后重启保持显示。
- [ ] “始终显示”：无论上次状态如何，重启后显示。
- [ ] “始终隐藏”：无论上次状态如何，重启后保持隐藏，但仍可从托盘手动显示。
- [ ] 从旧版本升级后原有选择不被默认值覆盖。
- [ ] 结果与备注：`________________`

### D6. 公开 Surface 收敛

- [ ] 正常启动不显示原始主窗口、Dashboard 或任务栏入口。
- [ ] 托盘左键只切换 QuackQuota；连续点击不会创建多个窗口。
- [ ] 托盘右键只包含“显示/隐藏 Codex 悬浮窗”“设置”“关于”“退出”，没有 Dashboard、仪表盘、Provider、Float Bar 或全局刷新入口。
- [ ] “设置”和“关于”复用同一个 Settings 窗口；连续点击只聚焦现有窗口。
- [ ] 关闭 Settings 只隐藏窗口，不退出 Overlay 或托盘程序；再次打开可正常恢复。
- [ ] 关闭 Overlay 不会退出托盘程序。
- [ ] 结果与备注：`________________`

### D7. 完整退出程序

- [ ] 使用托盘“退出”关闭程序。
- [ ] Task Manager 或 PowerShell 中不再存在相关进程。
- [ ] 重新启动后托盘菜单没有重复项。
- [ ] 结果与备注：`________________`

## E. Windows 显示环境

### E0. 图标一致性

- [ ] 冷启动期间 exe、窗口左上角、任务栏和托盘始终使用同一蓝色代码图标，没有黑底黄白方块闪变。
- [ ] 打开、隐藏、刷新 Overlay 及打开 Settings 后图标不发生品牌切换。
- [ ] Windows 资源管理器在小图标、中图标和大图标视图下均能显示清晰图标；必要时先清理 Windows 图标缓存后复验。
- [ ] 结果与备注：`________________`

### E1. 缩放

- [ ] 100% 缩放：文字、按钮、边距完整，无裁切。
- [ ] 125% 缩放：逻辑尺寸和点击区域正确，无模糊异常。
- [ ] 150% 缩放：内容仍在窗口内，拖动和按钮可用。
- [ ] 每次切换缩放后重新启动应用并记录结果。
- [ ] 在不同缩放间切换后，已保存的逻辑位置按当前 DPI 换算，窗口仍在工作区内。
- [ ] 结果与备注：`________________`

### E2. 单显示器与多显示器

- [ ] 单显示器启动、打开、拖动、关闭均正常。
- [ ] 多显示器中可拖到每个显示器，包含不同 DPI/缩放组合。
- [ ] 断开当前承载 Overlay 的显示器后，应用不会崩溃；重新打开时自动回到主显示器安全区域。
- [ ] 结果与备注：`________________`

### E3. 屏幕边缘重启

- [ ] 将窗口拖到每个屏幕边缘和多显示器交界处。
- [ ] 完整退出并重启后从托盘打开。
- [ ] 同一显示器仍存在时恢复边缘附近的等效逻辑位置，并自动夹紧在工作区安全边距内。
- [ ] 原显示器不存在或配置文件损坏时回到主显示器安全区域，不能永久不可见。
- [ ] 结果与备注：`________________`

## F. 日志脱敏

- [ ] 分别覆盖成功、未登录、断网、请求失败和关闭重开场景后检查本次运行输出。
- [ ] 日志中不存在 `Authorization:` 的实际值、Bearer 值、Cookie 值、access token、refresh token 或认证 JSON 原文。
- [ ] 日志中不存在用户邮箱或真实用户目录；若发现，仅保存脱敏后的最小复现，不公开原日志。
- [ ] 日志中不存在手动代理或环境代理的真实主机、端口、用户名或密码；界面状态只允许显示 `http(s)://***`。
- [ ] 错误消息只包含状态类别、HTTP 状态码或脱敏后的诊断信息。
- [ ] 结果与备注：`________________`

## 验收结论

- [ ] 全部必须项通过。
- [ ] 失败项已经建立不含敏感信息的问题记录。
- [ ] 测试过程中产生的临时网络规则、环境变量和测试进程已清理。
- 验收人：`________________`
- 验收日期：`________________`
- 结论：`通过 / 不通过 / 有条件通过`

## Baseline acceptance record (Windows, 2026-07)

The internal QuackQuota baseline completed real Windows desktop acceptance.
This record intentionally contains no user directory, account identity, proxy
endpoint, token, cookie, authorization header, or authentication-file content.

### Accepted core behavior

- Normal double-click launch, manual proxy, environment proxy, direct-connect
  failure handling, overlay position persistence, all three startup-display
  policies, tray single-instance behavior, and complete quit with no remaining
  process all passed.
- Codex App Server was verified with a real ChatGPT Plus account. The validated
  path used the Codex CLI `0.144.5` and displayed the App Server as the data
  source.
- A dynamic single quota window, its remaining-percent progress bar, reset
  countdown, and available reset credits were verified. No empty fixed
  five-hour card was displayed.
- Manual-proxy, environment-proxy, and direct-connect error paths were
  exercised without recording any real endpoint or credential.

### Accepted product convergence

- The three startup choices are fully visible and selectable; remember-last,
  always-show, and always-hide each persist across restart.
- The legacy main window no longer opens normally, duplicate dashboard entry
  points are removed, and left-clicking the tray icon toggles QuackQuota.
- Settings and Overlay remain single-instance surfaces. Window, taskbar, tray,
  and executable icons remain visually consistent during launch.
- Settings persistence now provides non-blocking feedback. Saving does not
  replace the page, move its layout, or allow an older response to overwrite a
  newer selection. Text fields avoid per-keystroke persistence.

### Deferred to the Alpha compatibility matrix

The following are not blockers for this internal baseline and must be covered
before a broader Alpha release:

- Additional Windows versions and hardware combinations.
- Additional DPI combinations, including heterogeneous-DPI multi-monitor use.
- Expanded monitor hot-plug coverage.
- Full Legacy fallback compatibility matrix.
- Installer, upgrade, and uninstall flows.
- Long-running and sleep/resume behavior.
- Code-signing and security-software compatibility.

## ChatGPT desktop lifecycle and quota-cache acceptance

This matrix covers only the Windows ChatGPT desktop package. Do not test or
record browser tabs, page titles, chat content, command lines, account data,
installation paths, raw PIDs, credentials, or real proxy endpoints.

- [ ] With monitoring enabled, close the visible ChatGPT desktop main window;
  after the close grace period, QuackQuota reports it as not running and hides
  the Overlay only when the corresponding auto-hide option is enabled.
- [ ] Start ChatGPT again from Windows; after stable samples, QuackQuota
  reports it as running and shows the existing Overlay only when the
  corresponding auto-show option is enabled. It must not create a duplicate.
- [ ] Toggle each independently persisted setting: monitor desktop app,
  auto-show on start, and auto-hide on exit. Restart QuackQuota and confirm
  the selected values remain intact. Turning monitoring off must preserve the
  other two values while making them inactive.
- [ ] Hide the Overlay manually while ChatGPT remains running. It must stay
  hidden until a later complete close-and-start cycle, unless opened manually
  from the tray.
- [ ] Confirm the watcher does not react to browser-based ChatGPT, standalone
  Codex CLI, or App Server helper processes.
- [ ] After one successful quota refresh, disconnect the network or make the
  Codex path unavailable. The Overlay may show the last successful quota
  snapshot only when explicitly labelled cached; it must never present it as
  live data. Restart QuackQuota and repeat the fallback check.
- [ ] Corrupt or remove only a disposable test cache configuration, then
  start QuackQuota. The app must remain usable and must not leave the Overlay
  permanently hidden.
