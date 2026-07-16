# Security policy

## Scope

Report security issues in the Windows desktop shell, local credential boundary,
proxy handling, Codex quota DTO boundary, and project-controlled build or
release material.

Do not post tokens, cookies, API keys, `auth.json`, complete logs, or private
account data in a public issue. Provide a redacted reproduction, affected
version or commit, Windows version, Codex CLI version, and minimal steps.

## Reporting channel

A formal private security reporting channel has not been established yet. Do
not disclose a suspected vulnerability publicly until the project provides a
private channel. This placeholder intentionally does not invent an email
address or response-time promise.

## Trust boundaries and known risks

- The Overlay receives a minimal quota DTO, not OAuth material or raw
  authentication-file content.
- The preferred path depends on the official Codex CLI and App Server, whose
  commands and protocol can change.
- The non-public Legacy quota interface is compatibility-only and may change
  without notice.
- OpenAI account or platform security issues must be reported to OpenAI through
  its official channels; this project cannot handle them.

See [docs/security-review.md](docs/security-review.md) for the current
implementation review.
