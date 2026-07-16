# QuackQuota

A lightweight Codex quota companion for Windows.

QuackQuota is a Windows-local desktop companion that keeps Codex quota windows
visible without opening a dashboard. It is an independent, non-official
project; Codex support does not imply OpenAI endorsement.

The maintained project documentation is available in English and Simplified
Chinese: [README.md](README.md) and [README.zh-CN.md](README.zh-CN.md). The
other historical translations are explicitly marked unmaintained.

## Project status

- The internal baseline is complete and tagged `v0.0.0-overlay-baseline`.
- No public installer, release package, or public QuackQuota repository exists
  yet. The planned repository slug is `quackquota`.
- This is early Windows-only development software, not an OpenAI product.

## Current capabilities

- A Windows overlay with dynamic Codex quota windows.
- Official Codex App Server as the preferred quota path.
- A compatibility-only Legacy fallback with explicit source labeling.
- Remaining quota, reset time, reset-credit display, proxy support, and safe
  connection-error categories.
- DPI-aware overlay position recovery, tray access, single-instance Settings
  and Overlay windows, and three startup display policies.

## Requirements and validation scope

- Windows 10 or Windows 11.
- The official Codex CLI installed and signed in.
- Current real-device validation is intentionally scoped; it does not claim
  coverage of every Windows version, device, display topology, or DPI setup.

## Privacy and security

- QuackQuota has no project-operated user accounts or cloud service.
- It does not upload Codex conversations, credentials, or telemetry to project
  maintainers.
- It reads quota data through official Codex/OpenAI services where required;
  App Server is preferred and the Legacy path is compatibility-only.
- Proxy URLs with embedded usernames or passwords are rejected.

Read [PRIVACY.md](PRIVACY.md), [SECURITY.md](SECURITY.md), and
[docs/security-review.md](docs/security-review.md) before testing a new path.

## Roadmap

The current product remains a quota overlay. A later mascot phase may add a
low-frame-rate duck and let users give their local duck a name; that name will
remain local-only and will not be added to Settings until a mascot surface
exists. See [docs/roadmap.md](docs/roadmap.md) and
[docs/mascot-architecture.md](docs/mascot-architecture.md).

## Build and development

The active Windows desktop shell is Tauri with a React frontend and shared Rust
logic. From the repository root, run the checks appropriate to your change:

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

There are no public installation instructions yet. Do not treat a debug build
as a supported release artifact.

## Feedback and contributions

Read [CONTRIBUTING.md](CONTRIBUTING.md). When GitHub issue tracking is enabled,
use the provided templates and include only redacted Windows validation details.

## Upstream, license, and marks

QuackQuota preserves the Git history and MIT license of the original
[CodexBar](https://github.com/steipete/CodexBar) project by Peter Steinberger
and the Windows/Tauri derivative Win-CodexBar. QuackQuota changes are derived
work and do not claim authorship of upstream code or affiliation with OpenAI.

See [NOTICE.md](NOTICE.md), [docs/repository-strategy.md](docs/repository-strategy.md),
and [docs/upstream-sync.md](docs/upstream-sync.md). OpenAI, ChatGPT, and Codex
are trademarks of their respective owners.
