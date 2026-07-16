# Repository strategy

## Current direction

Codex Duck Overlay is planned as an independent derived-project repository,
not as a long-term UI-only fork of Win-CodexBar. The working name remains
subject to confirmation before the first public push.

## History and remotes

- Preserve the complete existing Git history.
- For a future public repository, `origin` is planned to be the independent
  Codex Duck Overlay repository.
- `upstream` is planned to be the current Win-CodexBar repository.
- This phase changes neither remote and does not push.

## Upstream policy

Provider, security, and compatibility fixes can be reviewed through a separate
upstream-sync workflow. Upstream UI, branding, and product-direction changes
are never merged automatically. Every sync must use a dedicated branch,
review the affected surfaces, and pass the complete validation matrix.

## Public-push gate

Before the first public push, confirm the final display name, repository slug,
ownership, contributor channels, and the branding decisions in
`docs/branding-audit.md`.
