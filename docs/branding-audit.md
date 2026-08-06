# QuackQuota branding audit

This controlled migration changes public display identity without a broad
internal rename. `QuackQuota` is the product name and `hejin368/quackquota` is
the public repository. No public release or installer exists yet.

## 1. User-visible branding changed in this phase

| Surface | Decision |
|---|---|
| Active Tauri product name and primary window title | `QuackQuota` |
| Settings, Overlay, flyout, and Float Bar titles | `QuackQuota` (Settings uses `QuackQuota Settings`) |
| Tray tooltip, tooltip fallback, menu About label, and localized app strings | `QuackQuota` |
| About title, subtitle, attribution, and non-official statement | `QuackQuota`; the original upstream link remains for attribution, while the public QuackQuota repository is documented separately |
| User-facing launch, sign-in, cookie-extraction, CLI, and toast messages | `QuackQuota`; the underlying compatibility identifiers remain unchanged where required |
| README, project status, roadmap, repository strategy, issue/PR templates, and current Changelog | `QuackQuota`; the established `hejin368/quackquota` repository and private security channel are documented |
| App Server client metadata | `quackquota` / `QuackQuota` without credentials or account information |

## 2. Safe metadata changed in this phase

- The active `apps/desktop-tauri/src-tauri/tauri.conf.json` `productName` is
  `QuackQuota`. The active identifier remains unchanged.
- The shared Cargo description describes a Windows Codex quota companion. Its
  package name and legacy repository metadata are retained pending a separate
  compatibility review; this documentation launch does not rename packages.
- The temporary icon remains the existing blue development icon. It is not an
  OpenAI logo.

## 3. Internal compatibility identifiers intentionally retained

| Retained item | Reason and later migration point |
|---|---|
| Rust crate/bin `codexbar` and Tauri package `codexbar-desktop-tauri` | Changing them can affect build scripts, dependent code, and installed executable compatibility; review for a versioned migration. Alpha 1A added an explicit `[[bin]]` target named `QuackQuota` while preserving the package name. |
| Tauri identifier `com.codexbar.desktop` | It scopes Windows application data and cache identity; keep it to avoid losing existing local settings. |
| Executable names including `codexbar-desktop-tauri.exe` | `codexbar-desktop-tauri.exe` was the historical bare-build name. Since Alpha 1A the authoritative Windows bare-build output is `QuackQuota.exe`. The release pipeline still creates `codexbar.exe` and `codexbar-desktop.exe` from `QuackQuota.exe` as temporary installer compatibility copies; full installer brand migration, upgrade, and uninstall are not yet implemented. |
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
verification for the scoped baseline is complete.

- [x] Establish the public `hejin368/quackquota` repository.
- [x] Enable GitHub Private Vulnerability Reporting.
- [ ] Replace the temporary blue development icon with the formal duck icon.
- [ ] Validate installer, upgrade, uninstall, signing, and security-software
  behavior in the later Alpha compatibility matrix.

No release, installer, updater, or mascot implementation is claimed by this
repository-launch audit.

## Alpha 1A: executable identity baseline

The desktop Tauri package now defines an explicit `[[bin]]` target named
`QuackQuota` in `apps/desktop-tauri/src-tauri/Cargo.toml`.  A `cargo build
--release` (either directly or through `tauri:build`) produces
`target/release/QuackQuota.exe` as the sole authoritative desktop binary.

- The Cargo package name remains `codexbar-desktop-tauri` (no crate-name
  migration).
- The Tauri identifier `com.codexbar.desktop` is unchanged.
- The release pipeline still creates `codexbar.exe` and
  `codexbar-desktop.exe` as temporary installer compatibility copies from
  `QuackQuota.exe`.  These are not authoritative desktop names.
- The CLI binary remains `codexbar.exe`.
- Installer branding, application-data migration, identifier migration,
  auto-start, and silent-tray-start are not implemented in this round.
