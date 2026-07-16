# Codex Duck Overlay

> 当前为工作名称；首次公开 push 前仍会确认最终项目名称和仓库 slug。

Codex Duck Overlay 是一个面向 Windows 的本地 Codex 额度悬浮窗，用于在不打开仪表盘的情况下查看动态额度窗口。

## 当前状态

- 内部开发基线已完成，并标记为 `v0.0.0-overlay-baseline`。
- 尚未发布正式安装包或公开 Release。
- 当前处于早期开发阶段，不是 OpenAI 官方产品。

未来会补充经过审核的真实截图或 GIF；当前不放置虚构演示资源。

## 已完成能力

- Windows Codex 额度悬浮窗与动态额度窗口。
- 优先使用官方 Codex App Server。
- 标明来源的 Legacy 兼容回退。
- 剩余额度、重置时间和 reset credits。
- 手动 HTTP/HTTPS 代理、环境代理与安全的连接错误分类。
- 支持 DPI 的位置保存、恢复和可见区域兜底。
- 托盘入口、Settings/Overlay 单实例与三种启动显示策略。

## 系统与账号前提

- Windows 10 或 Windows 11。
- 已安装并登录官方 Codex CLI。
- 当前真实验证范围有限，不能据此宣称覆盖所有 Windows 设备、版本、显示器组合或 DPI 环境。

## 当前使用流程概览

1. 使用官方 Codex CLI 登录。
2. 启动本地 Windows 应用。
3. 通过托盘显示或隐藏 Codex Overlay。
4. 按需刷新额度；优先数据源为 Codex App Server。
5. 在设置中配置代理、启动显示策略和位置恢复。

## 隐私与安全摘要

- 项目不运营自己的用户账号或云服务。
- 不向项目维护者上传 Codex 对话、凭证或使用遥测。
- 为读取额度，应用会与官方 Codex/OpenAI 服务通信。
- App Server 是首选来源；Legacy 额度接口仅作兼容回退。
- 不接受在代理 URL 中嵌入用户名或密码。

请阅读 [PRIVACY.md](PRIVACY.md)、[SECURITY.md](SECURITY.md) 和
[docs/security-review.md](docs/security-review.md)。

## 已知限制

- 本项目是非官方第三方项目，不受 OpenAI 支持、认可或运营。
- Legacy 额度接口可能随时变化。
- 当前基线没有公开安装包、自动更新或代码签名声明。
- 本阶段不支持 macOS 或 Linux。

## Roadmap

见 [docs/roadmap.md](docs/roadmap.md)。Roadmap 不是交付时间承诺。

## 构建与开发

当前桌面壳为 Tauri + React，共享 Rust 逻辑。请从仓库根目录按修改范围运行：

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

请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。GitHub Issue 启用后，请使用模板并提供脱敏的 Windows 验证信息。

## 上游与致谢

本项目保留原始 [CodexBar](https://github.com/steipete/CodexBar)（Peter Steinberger）以及 Windows/Tauri 派生基础 Win-CodexBar 的 Git 历史与 MIT 许可。当前 Codex Overlay 是派生后的内部开发方向，不代表与上游或 OpenAI 存在隶属关系。

详见 [NOTICE.md](NOTICE.md) 与 [docs/upstream-sync.md](docs/upstream-sync.md)。

## 许可与非官方声明

仓库保留现有 [MIT License](LICENSE)。OpenAI、ChatGPT 和 Codex 是其各自权利人的商标。本项目不是 OpenAI 官方工具。
