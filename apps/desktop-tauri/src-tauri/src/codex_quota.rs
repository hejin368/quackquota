//! Minimal Codex-only bridge for the detached overlay.

use codexbar::providers::codex::{CodexProxyStatus, CodexProxyTestResult, CodexQuotaSnapshot};

/// Read only Codex rate limits. This command never enters the global provider
/// refresh pipeline and therefore cannot load credentials for other providers.
#[tauri::command]
pub async fn read_codex_rate_limits() -> CodexQuotaSnapshot {
    codexbar::providers::codex::read_codex_quota_snapshot().await
}

#[tauri::command]
pub fn get_codex_proxy_status() -> CodexProxyStatus {
    codexbar::providers::codex::proxy_status()
}

#[tauri::command]
pub async fn test_codex_proxy_connection() -> CodexProxyTestResult {
    codexbar::providers::codex::test_codex_proxy_connection().await
}
