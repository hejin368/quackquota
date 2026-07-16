# QuackQuota

一只帮你盯着 Codex 额度的 Windows 桌面小鸭子。

QuackQuota 是一款 Windows 本机额度助手，可在不打开仪表盘的情况下查看
Codex 动态额度窗口。它是独立的非官方项目；支持 Codex 不代表获得 OpenAI
认可。

当前正式维护的项目文档为英文和简体中文：[README.md](README.md) 与
[README.zh-CN.md](README.zh-CN.md)。其他历史语言版本已明确标记为不再维护。

## 当前状态

- 内部开发基线已完成并标记为 `v0.0.0-overlay-baseline`。
- 源代码仓库已在 [hejin368/quackquota](.) 公开；目前仍无公开安装包、
  Release 或面向普通用户的受支持下载。
- 当前是仅面向 Windows 的早期 Alpha 前开发软件，不是 OpenAI 官方产品。

## 已完成能力

- 支持动态额度窗口的 Windows Codex 悬浮窗。
- 优先通过官方 Codex App Server 读取额度。
- 带明确来源标签的 Legacy 兼容回退。
- 剩余额度、重置时间、reset credits、代理支持和安全的连接错误分类。
- 支持 DPI 的位置恢复、托盘入口、Settings/Overlay 单实例和三种启动显示策略。

## 前提与验证范围

- Windows 10 或 Windows 11。
- 已安装并登录官方 Codex CLI。
- 当前真实设备验证范围有限，不代表已覆盖全部 Windows 版本、设备、显示器
  布局或 DPI 组合。

## 隐私与安全

- QuackQuota 不运营自己的用户账号或云服务。
- 不向项目维护者上传 Codex 对话、凭证或使用遥测。
- 在读取额度所需时，仅与官方 Codex/OpenAI 服务通信；App Server 为首选，
  Legacy 仅作兼容回退。
- 拒绝在代理 URL 中嵌入用户名或密码。

测试新路径前，请阅读 [PRIVACY.md](PRIVACY.md)、[SECURITY.md](SECURITY.md) 与
[docs/security-review.md](docs/security-review.md)。

## Roadmap

当前核心仍是额度悬浮窗。后续吉祥物阶段可能加入低帧率小鸭，并允许用户为本地
小鸭命名；名称只保存在本地，且会等到吉祥物界面存在后才加入设置。详见
[docs/roadmap.md](docs/roadmap.md) 与
[docs/mascot-architecture.md](docs/mascot-architecture.md)。

## 构建与开发

当前 Windows 桌面壳使用 Tauri + React，并复用共享 Rust 逻辑。请从仓库根目录
按修改范围运行检查：

```powershell
pnpm --dir apps/desktop-tauri run format:check
pnpm --dir apps/desktop-tauri exec tsc --noEmit
pnpm --dir apps/desktop-tauri test -- --reporter=basic
pnpm --dir apps/desktop-tauri run build

cargo fmt --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml
```

当前没有面向普通用户的安装说明；debug 构建不是受支持的发布产物。

## 反馈与贡献

请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。普通 Bug 请使用现有 GitHub
Issue 模板，并只提供脱敏后的 Windows 验证信息。安全漏洞请使用
[SECURITY.md](SECURITY.md) 中的私密渠道，不要发布到公开 Issue。

## 上游、许可与商标

QuackQuota 保留原始 [CodexBar](https://github.com/steipete/CodexBar)
（Peter Steinberger）及 Windows/Tauri 派生基础 Win-CodexBar 的 Git 历史和
MIT 许可。QuackQuota 的修改属于派生工作，不主张上游代码的原创权，也不表示
与 OpenAI 存在隶属或认可关系。

详见 [NOTICE.md](NOTICE.md)、[docs/repository-strategy.md](docs/repository-strategy.md)
和 [docs/upstream-sync.md](docs/upstream-sync.md)。OpenAI、ChatGPT 和 Codex 是其
各自权利人的商标。
