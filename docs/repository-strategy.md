# Repository strategy

## Current direction

QuackQuota is planned as an independent derived-project repository, not as a
long-term UI-only fork of Win-CodexBar. The confirmed public display name is
`QuackQuota`; the planned GitHub repository slug is `quackquota`.

## History and remotes

- Preserve the complete existing Git history.
- A future public `origin` is planned to be the independent QuackQuota
  repository. It has not been created in this phase.
- The current Win-CodexBar repository remains the planned upstream reference.
- This phase changes neither remote and does not push.

## Upstream policy

Provider, security, and compatibility fixes can be reviewed through a separate
upstream-sync workflow. Upstream UI, branding, and product-direction changes
are never merged automatically. Every sync must use a dedicated branch, review
affected surfaces, and pass the complete validation matrix.

## Public-push gate

Before the first public push, create the authorized destination repository,
confirm contributor and security-reporting channels, complete the manual
Windows branding check, and review `docs/branding-audit.md`.
