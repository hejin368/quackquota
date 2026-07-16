# QuackQuota branding audit

This controlled migration changes public display identity without a broad
internal rename. `QuackQuota` is the product name; `quackquota` is the planned
repository slug. Neither a public repository nor a release exists yet.

## 1. User-visible branding changed in this phase

| Surface | Decision |
|---|---|
| Active Tauri product name and primary window title | `QuackQuota` |
| Settings, Overlay, flyout, and Float Bar titles | `QuackQuota` (Settings uses `QuackQuota Settings`) |
| Tray tooltip, tooltip fallback, menu About label, and localized app strings | `QuackQuota` |
| About title, subtitle, attribution, and non-official statement | `QuackQuota`; links only to the original upstream project until an authorized project remote exists |
| User-facing launch, sign-in, cookie-extraction, CLI, and toast messages | `QuackQuota`; the underlying compatibility identifiers remain unchanged where required |
| README, project status, roadmap, repository strategy, issue/PR templates, and current Changelog | `QuackQuota`; planned slug `quackquota` is documented without inventing a remote |
| App Server client metadata | `quackquota` / `QuackQuota` without credentials or account information |

## 2. Safe metadata changed in this phase

- The active `apps/desktop-tauri/src-tauri/tauri.conf.json` `productName` is
  `QuackQuota`. The active identifier remains unchanged.
- The shared Cargo description describes a Windows Codex quota companion. Its
  package name and existing repository metadata are retained until an
  authorized remote and compatibility migration are planned.
- The temporary icon remains the existing blue development icon. It is not an
  OpenAI logo.

## 3. Internal compatibility identifiers intentionally retained

| Retained item | Reason and later migration point |
|---|---|
| Rust crate/bin `codexbar` and Tauri package `codexbar-desktop-tauri` | Changing them can affect build scripts, dependent code, and installed executable compatibility; review for a versioned migration. |
| Tauri identifier `com.codexbar.desktop` | It scopes Windows application data and cache identity; keep it to avoid losing existing local settings. |
| Executable names including `codexbar-desktop-tauri.exe` | Keep current debug/build and integration paths stable; rename only with installer, upgrade, shortcut, and migration coverage. |
| Window labels, tray ID, commands, events, modules, log targets, Keychain targets, and `CODEXBAR_*` environment variables | Internal lookup, storage, automation, and compatibility identifiers; do not rename without explicit migration design. |
| Existing local settings and application-data paths | Preserve existing settings, proxy policy, geometry, and startup preferences. |
| Windows toast AUMID `CodexBar` and its registry key | Changing an AUMID changes notification identity and can leave stale registrations. The user-facing toast display name is now `QuackQuota`; migrate the AUMID only with installer and notification migration coverage. |
| Legacy updater implementation and updater commands | The prior updater is retained for rollback compatibility, but its public About controls are removed because its configured source belongs to the upstream project. Do not re-enable it until QuackQuota has an authorized release source, signing, and update migration plan. |
| Legacy `rust/tauri.conf.json`, WiX, installer, packaging, and old build documents | Not the active validated surface; review together with future installer/release work. |

## 4. Upstream attribution and historical material retained

- `LICENSE` remains unchanged.
- `NOTICE.md` preserves the original CodexBar and Win-CodexBar attribution.
- Retained Git history, historical CHANGELOG entries, upstream-sync documents,
  old specs, and historical packaging references keep their original names.
- `README.md` and `README.zh-CN.md` are maintained. `README.zh-TW.md`,
  `README.ja-JP.md`, `README.ko-KR.md`, `README.es-MX.md`, `rust/README.md`,
  and `rust/screenshots/README.md` are historical/unmaintained until separately
  reviewed; they must not be read as current release guidance.

## Icon replacement entry point

`rust/icons/icon.png` is the only editable runtime icon source. The generated
`rust/icons/icon.ico` supplies Windows executable resources. A future duck icon
must replace the PNG and regenerate the ICO through the existing icon workflow;
do not introduce a second icon family or use OpenAI branding.

## First-public-push assessment

No unresolved active-surface branding string is intentionally left as a product
name after this phase. The legacy updater is intentionally inactive in the
public UI; publishing any update feature is a separate gate. Manual Windows
verification is still required. Creating the authorized `quackquota` remote,
choosing a private security channel, and reviewing installer/upgrade behavior
are publication gates, but no remote or release is created here.
