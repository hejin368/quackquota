# QuackQuota 安全审查基线

本文档描述动态额度窗口版本的当前安全边界，不是安全认证或公开发布批准。审查过程中没有读取、复制或记录任何真实认证文件内容。

## 已验证的 App Server 基线

2026-07 的真实 Windows 账号验收中，独立 Codex CLI `0.144.5` 已成功启动 App Server，Overlay 通过 `account/rateLimits/read` 取得单个 7 天窗口、reset credits 和 `Codex App Server` 来源标记。文档不记录账号身份、认证文件内容或本机代理地址。

## 1. 两条 Codex 数据路径

### 首选：Codex App Server

Overlay 调用 Tauri 命令 `read_codex_rate_limits`，Rust 启动本机 Codex CLI 的 `app-server` 子进程，通过 JSONL/stdio 完成：

1. `initialize`
2. `initialized`
3. `account/rateLimits/read`

认证、Token 刷新和上游请求由官方 Codex CLI 管理。本项目不读取 App Server 使用的 access token、refresh token、Cookie 或 Authorization Header。

子进程使用只读/不受信任审批参数，并通过命令行配置层把 `chatgpt_base_url` 强制覆盖为官方 `https://chatgpt.com/backend-api`。它不继承用户配置中的自定义 ChatGPT 后端作为额度请求目标。

### 回退：legacy Codex provider

仅当 App Server 不存在、无法启动、不支持目标方法、请求失败或没有返回有效窗口时，才调用 `CodexApi::fetch_usage`：

- 若设置 `CODEX_HOME`，认证文件位置为该目录下的 `auth.json`。
- 否则为通用用户主目录下的 `.codex/auth.json`。
- 只解析请求所需的 access credential 和可选 account ID。
- refresh token 不进入 `CodexCredentials`。
- access credential 的进程内缓存 TTL 为 5 秒。
- legacy 请求目标固定为官方 ChatGPT backend，不再读取或使用自定义 `chatgpt_base_url`。

legacy 路径使用的 `/backend-api/wham/usage` 与 reset-credit 路径不是稳定公开 API，只作为兼容回退。

## 2. Codex 专用代理边界

代理只作用于 Codex App Server 子进程和 legacy Codex HTTP 客户端，不改变其他 Provider：

1. 非空且校验通过的应用内手动代理；
2. 开关启用时，按 `HTTPS_PROXY`、`https_proxy`、`HTTP_PROXY`、`http_proxy` 读取当前进程环境；
3. 否则显式直接连接。

手动代理仅接受 `http://` 或 `https://`，拒绝嵌入用户名、密码、查询参数或片段。Rust 保留实际 URL 用于连接，前端的“当前连接方式”只接收来源枚举和 `http(s)://***`。App Server 子进程会得到解析后的 `HTTP_PROXY`/`HTTPS_PROXY`；直接连接会移除代理环境。legacy 客户端使用同一解析结果并先禁用 reqwest 的隐式系统代理发现，避免两条路径策略分叉。

手动代理地址作为非凭证设置以明文保存在应用 `settings.json` 中，因此不得允许在 URL 内携带代理账号或密码。设置页面需要回显该值供用户编辑，但诊断日志、状态 DTO、截图和公开问题单必须使用脱敏形式。

“测试连接”使用不带认证的请求检查公开 ChatGPT 入口是否可达，不读取 `auth.json`，不附加 Token、Cookie、Authorization Header 或 account ID。连接错误在 Rust 中映射为固定枚举，前端只显示本地化的 DNS/网络、代理连接、代理配置、未登录、App Server 不可用、服务端错误或无效数据文案。

## 3. 可以访问凭证的代码

| 代码 | 权限和用途 |
|---|---|
| 官方 Codex App Server 子进程 | 由 Codex CLI 自己管理登录凭证、刷新和上游请求 |
| `rust/src/providers/codex/api.rs` | legacy fallback 定位认证文件、解析最小凭证并请求官方后端 |
| `rust/src/core/http.rs` | 携带凭证请求的同源重定向保护 |
| `rust/src/providers/codex/quota.rs` | 启动 App Server、解析额度 RPC、调用 Codex-only legacy fallback；不接收其他 Provider 上下文 |

`read_codex_rate_limits` 不调用 `refresh_providers`，也不加载 Claude、Cursor 或其他 Provider 的 Cookie、API key、token account。

## 4. 发送到前端的数据

Overlay 只接收 `CodexQuotaSnapshot`：

- 套餐名称（如可用）。
- 动态额度窗口数组：稳定 ID、标签、used/remaining 百分比、周期、重置时间、limit ID/name、primary/secondary 层级。
- 可用 reset credit 数量（如 App Server 返回）。
- 状态、来源、更新时间和静态脱敏状态说明。

不会发送：

- access token 或 refresh token；
- Cookie 或 Authorization Header；
- `auth.json` 原文或文件路径；
- 账号邮箱、organization 或 account ID；
- 其他 Provider 的任何快照或凭证；
- App Server 原始错误对象、stderr 或完整响应。
- 真实代理主机、端口或代理认证信息。

Rust 单元测试会序列化 DTO 并检查常见凭证字段不存在。前端不再监听全局 `provider-updated` 事件。

## 5. 日志行为

- App Server 原始 stderr 被丢弃，不转发到应用日志或前端。
- RPC 原始响应和 RPC error body 不写日志。
- legacy HTTP 非成功响应只报告状态类别，不记录响应正文。
- Codex provider 的通用错误仍经过 `safe_error_message`。
- Overlay 前端只接收固定的安全状态说明，不显示底层进程错误字符串。
- 代理解析、连接测试和错误映射不记录原始代理 URL；日志中只能出现代理来源或固定错误类别。

仍必须在真实 Windows 环境检查成功、未登录、离线、RPC 不支持、legacy 失败和缓存回退场景的实际日志。静态检查不能证明运行时零泄漏。

## 6. 当前风险与状态

| ID | 风险 | 当前状态 | 发布前要求 |
|---|---|---|---|
| SEC-01 | 自定义 `chatgpt_base_url` 接收 Codex Bearer Token | **Overlay/legacy 代码边界已关闭**：两条路径均强制官方目标 | Windows 抓包或受控代理验证实际目标；确认未来改动不移除覆盖 |
| SEC-02 | legacy `/backend-api/wham/*` 非公开且可变化 | 仅在 App Server 失败时使用；动态解析、非法窗口丢弃 | 保留兼容测试和明确的 fallback 标记；考虑未来删除 legacy |
| SEC-03 | Overlay 刷新加载所有 Provider 凭证 | **已关闭**：专用 `read_codex_rate_limits` 命令只进入 Codex 路径 | 保留“不调用 `refresh_providers`”测试 |
| SEC-04 | 全局 Provider 事件把邮箱/organization 等宽数据发送到 Overlay | **已关闭**：Overlay 不再使用 `useProviders` 或全局 Provider 事件 | 保持最小 DTO，新增字段必须安全审查 |
| SEC-05 | App Server 可执行文件发现和启动在不同 Codex 安装方式下可能失败 | 有 legacy fallback；失败不暴露路径或原始错误 | 验收 npm CLI、独立 CLI、Codex Desktop 打包版本和 CLI 缺失场景 |
| SEC-06 | App Server 命令和协议可能变化 | 使用官方当前稳定方法，但 `codex app-server` 命令本身仍可能变化 | 记录最低支持 CLI 版本并增加协议兼容矩阵 |
| SEC-07 | 真实错误日志矩阵尚未完成 | 原始 stderr/响应不记录，状态消息静态化 | 完成 Windows 日志人工验收和哨兵 secret 自动测试 |
| SEC-08 | 进程内额度缓存可能短暂显示旧账号数据 | 只在当前进程保留并标为 `cache` | 多账号切换验收；必要时按非敏感账号指纹隔离或切换时清空 |
| SEC-09 | 代理地址或代理认证信息进入 UI/日志 | 手动地址只在设置输入框编辑；状态 DTO 仅含来源和协议级脱敏标记；嵌入凭证被拒绝 | 完成三种连接模式的 Windows 日志哨兵检查，检查 crash dump/诊断包 |
| SEC-10 | 环境代理与 App Server/legacy 策略分叉 | 两条路径共享同一 Rust 解析结果；子进程显式继承，reqwest 显式配置 | 在需要代理的真实网络中分别验证 App Server 与 legacy fallback |
| SEC-11 | 手动代理地址在设置文件中明文持久化 | URL 凭证、查询和片段被拒绝；地址不进入状态 DTO 或日志 | 文档提示设置文件边界；若未来支持认证代理，必须改用系统凭证存储而不是 URL |

## 7. 当前泄漏判断

基于已检查代码和自动测试，没有发现把 Token、Cookie、Authorization Header、认证文件原文或其他 Provider 凭证发送到 Overlay 前端或主动写入日志的路径。该结论不替代真实 Windows 日志和网络目标验收。

## 8. 公开发布前必须完成

- [x] 完成 App Server 成功路径的 Windows 实机验证并记录来源标签（Codex CLI `0.144.5`）。
- [ ] 完成 legacy fallback 的 Windows 实机验证并记录来源标签。
- [ ] 完成手动代理、环境代理、直接连接及代理失效矩阵，确认日志和 UI 不泄露真实代理地址。
- [ ] 验证请求实际只到官方 ChatGPT/Codex 主机。
- [ ] 完成未安装、未登录、断网、401/403、429、5xx、超时、RPC method-not-found 和非法 JSON 测试。
- [ ] 完成多账号切换和缓存隔离验收。
- [ ] 用哨兵 secret 自动测试日志脱敏，并人工检查 Windows 运行日志。
- [ ] 记录最低支持 Codex CLI 版本及 App Server 兼容策略。
- [ ] 评估何时可以删除 legacy 非公开接口 fallback。
- [ ] 检查 crash dump、诊断包和 issue 模板，禁止收集认证文件或原始 App Server 输出。
- [ ] 保留上游 MIT LICENSE、版权声明和来源归属。

## Baseline security confirmation (Windows, 2026-07)

The following statements are confirmed for the internal QuackQuota baseline.
This section deliberately contains no real proxy endpoint, user directory,
email address, token, cookie, authorization header, or authentication-file
content.

- The frontend receives only the minimum `CodexQuotaSnapshot` DTO required by
  the Overlay. It does not receive Codex OAuth material, cookies, authorization
  headers, or raw `auth.json` content.
- Overlay refresh uses its dedicated Codex command and does not invoke global
  `refresh_providers` or load other-provider credentials.
- A Codex token is not sent to a custom `chatgpt_base_url`; the App Server path
  is constrained to the official ChatGPT backend route.
- App Server and Legacy fallback use the same selected proxy policy. Proxy
  status is rendered only in redacted form.
- Settings-save and user-facing error feedback use safe localized summaries;
  they do not expose raw Rust, endpoint, or credential-bearing errors.
- The App Server primary path completed real Windows validation with a real
  account. The non-public Legacy usage endpoint remains a compatibility-only
  fallback and therefore remains an interface-change risk.
- No real endpoint or credential is written into this review document.
