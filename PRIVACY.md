# Privacy

QuackQuota is a local Windows desktop application. It does not operate
its own user-account system or cloud service.

## Data handling

- The project does not send Codex conversations, tokens, cookies, API keys,
  `auth.json` contents, or usage telemetry to project maintainers.
- To read quota data, the application communicates with official Codex/OpenAI
  services through the official Codex CLI/App Server where available.
- Codex App Server is the preferred quota source. A Legacy interface is kept
  only as a compatibility fallback.
- Proxy settings are stored locally with application settings. Proxy URLs with
  embedded usernames or passwords are rejected.
- The application currently has no automatic crash-reporting service.

## Local control

You can stop the application by exiting it and remove it by uninstalling or
deleting the local installation when a supported distribution is available.

If telemetry, crash reporting, or any new external service is introduced in a
future version, it must be opt-in or otherwise clearly disclosed before use,
and this document must be updated first.
