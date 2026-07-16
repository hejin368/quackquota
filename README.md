# Codex Duck Overlay

> Working name — the final project name and repository slug will be confirmed
> before the first public push.

Codex Duck Overlay is an early Windows-local desktop overlay for viewing Codex
quota windows without opening a dashboard.

## Project status

- The internal baseline is complete and tagged `v0.0.0-overlay-baseline`.
- No public installer or release package is available yet.
- This is early development software, not an OpenAI product.

Screenshot and GIF placeholders will be added only when genuine, reviewable
assets are ready.

## Current capabilities

- A Windows Codex quota overlay with dynamic quota windows.
- Official Codex App Server as the preferred quota path.
- A compatibility-only Legacy fallback with explicit source labeling.
- Remaining quota, reset time, and reset-credit display.
- Manual HTTP/HTTPS proxy support, environment-proxy support, and safe
  connection error categories.
- Overlay position persistence with DPI-aware visible-area recovery.
- Tray access, single-instance Settings and Overlay windows, and three startup
  display policies.

## Requirements and current validation scope

- Windows 10 or Windows 11.
- The official Codex CLI installed and signed in.
- The current real-device validation scope is limited. It does not claim
  coverage of every Windows version, device, display topology, or DPI setup.

## How it is used today

1. Sign in with the official Codex CLI.
2. Start the local Windows application.
3. Use the tray icon to show or hide the Codex Overlay.
4. Refresh the overlay when needed; the preferred source is Codex App Server.
5. Use Settings to configure proxy behavior, startup display policy, and
   overlay position recovery.

## Privacy and security summary

- This project does not operate its own user accounts or cloud service.
- It does not upload Codex conversations, credentials, or telemetry to project
  maintainers.
- It communicates with official Codex/OpenAI services when reading quota data.
- App Server is preferred; Legacy quota access is a compatibility fallback.
- Proxy URLs with embedded usernames or passwords are rejected.

Read [PRIVACY.md](PRIVACY.md), [SECURITY.md](SECURITY.md), and
[docs/security-review.md](docs/security-review.md) before testing a new path.

## Known limitations

- This is a non-official third-party project. It is not supported, endorsed, or
  operated by OpenAI.
- Legacy quota interfaces may change without notice.
- There is no public installer, automatic update path, or code-signing claim
  for this baseline.
- macOS and Linux are not supported by this project phase.

## Roadmap

See [docs/roadmap.md](docs/roadmap.md). The roadmap is not a delivery-date
commitment.

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

The repository has no public installation instructions yet. Do not treat a
debug build as a supported release artifact.

## Feedback and contributions

Read [CONTRIBUTING.md](CONTRIBUTING.md). When GitHub issue tracking is enabled,
use the provided issue templates and include redacted Windows validation details.

## Upstream and acknowledgements

This work preserves the Git history and MIT license of the original
[CodexBar](https://github.com/steipete/CodexBar) project by Peter Steinberger,
and the Windows/Tauri derivative Win-CodexBar. Current Codex Overlay work is a
derived internal-development direction, not a claim of upstream affiliation.

See [NOTICE.md](NOTICE.md) and [docs/upstream-sync.md](docs/upstream-sync.md).

## License and trademark notice

This repository retains the existing [MIT License](LICENSE). OpenAI, ChatGPT,
and Codex are trademarks of their respective owners. This project is not an
official OpenAI tool.
