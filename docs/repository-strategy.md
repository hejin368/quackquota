# Repository strategy

## Current direction

QuackQuota is an independent derived-project repository, not a long-term
UI-only fork of Win-CodexBar. The public display name is `QuackQuota`; the
public GitHub repository is `hejin368/quackquota`.

## History and remotes

- Preserve the complete existing Git history.
- `origin` points to the independent `hejin368/quackquota` repository.
- `upstream` points to `Finesssee/Win-CodexBar`.
- Local recovery refs under `refs/backup/` are never pushed.

## Upstream policy

Provider, security, and compatibility fixes can be reviewed through a separate
upstream-sync workflow. Upstream UI, branding, and product-direction changes
are never merged automatically. Every sync must use a dedicated branch, review
affected surfaces, and pass the complete validation matrix.

## Public repository launch

The authorized destination repository exists, GitHub Private Vulnerability
Reporting is enabled, and the scoped Windows branding check is complete. The
first public push remains gated on the complete automated checks and history
audit documented for the launch task.
