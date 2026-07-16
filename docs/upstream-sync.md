# Upstream synchronization

This document describes a future process only. It does not authorize or run an
upstream sync.

1. Fetch the configured upstream remote on a dedicated `sync/*` branch.
2. Review provider, security, and Windows-compatibility changes individually.
3. Do not automatically merge upstream UI, branding, release, or product
   direction changes.
4. Resolve conflicts with the local QuackQuota architecture and security
   boundaries in mind.
5. Run frontend, shared Rust, Tauri Rust, localization, and build checks.
6. Perform targeted Windows manual validation for surface, tray, proxy, and
   credential-boundary changes.
7. Document the source commits and rationale in the sync pull request.

Any future remote configuration change is a separate, explicit operation.
