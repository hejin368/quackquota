# Contributing

Thanks for helping improve QuackQuota, a Windows-first, local-first Codex quota companion.
Contributions are reviewed individually and are not guaranteed to be merged.

## Before you start

- Open or link an issue when issue tracking is available.
- Use branches named `feature/*`, `fix/*`, or `docs/*`.
- Keep one behavior or documentation concern per pull request.
- Never commit credentials, local settings, logs, screenshots containing private
  data, or build artifacts.

## Development checks

Run the checks appropriate to your change from the repository root:

```powershell
pnpm --dir apps/desktop-tauri run format:check
pnpm --dir apps/desktop-tauri exec tsc --noEmit
pnpm --dir apps/desktop-tauri test -- --reporter=basic
pnpm --dir apps/desktop-tauri run check-locale
pnpm --dir apps/desktop-tauri run build

cargo fmt --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path apps/desktop-tauri/src-tauri/Cargo.toml
powershell.exe -ExecutionPolicy Bypass -File scripts/local-check.ps1 -Rust -Tauri -Frontend -Format -Clippy
```

## Pull request expectations

Include the purpose, test results, Windows manual verification, screenshots for
UI changes, and security impact. Explain any upstream-sync relationship. Check
that credentials, local configuration, logs, and build outputs are absent from
the diff.

Future mascot and animation assets require a separate asset specification; they
are not open for contribution in the current phase. Keep changes Windows-first,
lightweight, and local-first.
