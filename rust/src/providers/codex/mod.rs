//! Codex (OpenAI/ChatGPT) provider implementation
//!
//! Fetches usage data from ChatGPT's backend API using OAuth credentials
//! stored by the Codex CLI in ~/.codex/auth.json

mod api;
mod proxy;
mod quota;

use async_trait::async_trait;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::core::{
    FetchContext, Provider, ProviderError, ProviderFetchResult, ProviderId, ProviderMetadata,
    SourceMode,
};

pub use api::CodexApi;
pub use proxy::{
    CodexConnectionErrorKind, CodexProxySource, CodexProxyStatus, CodexProxyTestResult,
    proxy_status, test_codex_proxy_connection, validate_manual_proxy,
};
pub use quota::{
    CodexQuotaSnapshot, CodexQuotaSource, CodexQuotaStatus, CodexRateLimitLevel,
    CodexRateLimitWindow, CodexResetCredits, find_codex_binary, read_codex_quota_snapshot,
};

/// Codex provider for fetching AI usage limits
pub struct CodexProvider {
    metadata: ProviderMetadata,
    api: CodexApi,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            metadata: ProviderMetadata {
                id: ProviderId::Codex,
                display_name: "Codex",
                session_label: "Session",
                weekly_label: "Weekly",
                supports_opus: false,
                supports_credits: true,
                default_enabled: true,
                is_primary: true,
                dashboard_url: Some("https://chatgpt.com/codex/settings/usage"),
                status_page_url: Some("https://status.openai.com"),
            },
            api: CodexApi::new(),
        }
    }
}

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Codex
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn fetch_usage(&self, _ctx: &FetchContext) -> Result<ProviderFetchResult, ProviderError> {
        tracing::debug!("Fetching Codex usage via OAuth API");

        match self.api.fetch_usage().await {
            Ok((usage, cost)) => {
                let mut result = ProviderFetchResult::new(usage, "oauth");
                if let Some(c) = cost {
                    result = result.with_cost(c);
                }
                Ok(result)
            }
            Err(e) => {
                tracing::warn!(
                    "Codex API fetch failed: {}",
                    crate::logging::safe_error_message(&e)
                );
                Err(e)
            }
        }
    }

    fn available_sources(&self) -> Vec<SourceMode> {
        vec![SourceMode::Auto, SourceMode::OAuth, SourceMode::Cli]
    }

    fn supports_oauth(&self) -> bool {
        true
    }

    fn supports_cli(&self) -> bool {
        true
    }

    fn detect_version(&self) -> Option<String> {
        detect_codex_version()
    }
}

/// Detect the version of the codex CLI
fn detect_codex_version() -> Option<String> {
    let codex_path = find_codex_binary()?;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new(codex_path);
    cmd.args(["--version"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().ok()?;

    if output.status.success() {
        let version_str = String::from_utf8_lossy(&output.stdout);
        extract_version(&version_str)
    } else {
        None
    }
}

/// Extract version number from a string like "codex 1.2.3"
fn extract_version(s: &str) -> Option<String> {
    let re = regex_lite::Regex::new(r"(\d+(?:\.\d+)+)").ok()?;
    re.find(s).map(|m| m.as_str().to_string())
}
