//! Codex-only quota reader used by the detached overlay.
//!
//! The preferred path talks to the official Codex App Server over its
//! JSONL/stdio protocol. A legacy OAuth provider fallback remains available
//! for Codex CLI versions that do not expose `account/rateLimits/read`.

use super::api::CodexApi;
use super::proxy::{
    CodexConnectionErrorKind, ResolvedCodexProxy, classify_connection_text, resolve_codex_proxy,
};
use crate::core::{ProviderError, RateWindow, UsageSnapshot};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStdout, Command};
use tokio::time::timeout;

const OFFICIAL_CHATGPT_BASE_URL_OVERRIDE: &str =
    "chatgpt_base_url=\"https://chatgpt.com/backend-api\"";
const APP_SERVER_TIMEOUT: Duration = Duration::from_secs(10);
const APP_SERVER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

static QUOTA_CACHE: OnceLock<Mutex<Option<CodexQuotaSnapshot>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexQuotaStatus {
    Ready,
    Cached,
    NotInstalled,
    NotLoggedIn,
    Offline,
    ProviderError,
    InvalidData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexQuotaSource {
    AppServer,
    LegacyProvider,
    Cache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexRateLimitLevel {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitWindow {
    pub id: String,
    pub label: String,
    pub used_percent: f64,
    pub remaining_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_duration_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_name: Option<String>,
    pub level: CodexRateLimitLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexResetCredits {
    pub available_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexQuotaSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    pub windows: Vec<CodexRateLimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_credits: Option<CodexResetCredits>,
    pub status: CodexQuotaStatus,
    pub source: CodexQuotaSource,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<CodexConnectionErrorKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuotaFailureKind {
    NotInstalled,
    NotLoggedIn,
    DnsOrNetwork,
    ProxyConnection,
    ProxyConfiguration,
    AppServerUnavailable,
    ServiceError,
    InvalidData,
}

#[derive(Debug, Clone, Copy)]
struct QuotaFailure {
    kind: QuotaFailureKind,
}

impl QuotaFailure {
    const fn new(kind: QuotaFailureKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitsResponse {
    rate_limits: Option<AppServerRateLimitSnapshot>,
    #[serde(default)]
    rate_limits_by_limit_id: Option<HashMap<String, AppServerRateLimitSnapshot>>,
    rate_limit_reset_credits: Option<AppServerResetCredits>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitSnapshot {
    limit_id: Option<String>,
    limit_name: Option<String>,
    primary: Option<AppServerRateLimitWindow>,
    secondary: Option<AppServerRateLimitWindow>,
    plan_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerRateLimitWindow {
    used_percent: Option<f64>,
    window_duration_mins: Option<i64>,
    resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppServerResetCredits {
    available_count: i64,
}

/// Read a Codex-only quota snapshot without loading any other provider.
///
/// App Server is attempted first. The legacy provider is used only when the
/// process cannot be started, the RPC is unsupported/fails, or it returns no
/// valid rate-limit windows. A last-good in-memory snapshot is the final
/// fallback and is explicitly marked as cached.
pub async fn read_codex_quota_snapshot() -> CodexQuotaSnapshot {
    let proxy = match resolve_codex_proxy() {
        Ok(proxy) => proxy,
        Err(_) => {
            return failure_snapshot(QuotaFailure::new(QuotaFailureKind::ProxyConfiguration));
        }
    };
    let codex_binary = find_codex_binary();
    let app_server_result = match codex_binary.as_deref() {
        Some(binary) => fetch_from_app_server(binary, &proxy).await,
        None => Err(QuotaFailure::new(QuotaFailureKind::NotInstalled)),
    };

    if let Ok(snapshot) = app_server_result.as_ref() {
        store_cached_snapshot(snapshot);
        return snapshot.clone();
    }

    let legacy_result = CodexApi::with_proxy(&proxy).fetch_usage().await;
    match legacy_result {
        Ok((usage, _cost)) => {
            let mut snapshot = snapshot_from_legacy_usage(usage);
            if !snapshot.windows.is_empty() {
                snapshot.message = Some(
                    "Codex App Server unavailable; using the legacy Codex provider.".to_string(),
                );
                store_cached_snapshot(&snapshot);
                return snapshot;
            }
        }
        Err(error) => {
            if let Some(cached) = cached_snapshot() {
                return as_cached(cached);
            }

            let cli_is_usable = app_server_result
                .as_ref()
                .err()
                .is_none_or(|failure| failure.kind != QuotaFailureKind::NotInstalled);
            let failure = classify_legacy_error(&error, cli_is_usable, proxy.is_active());
            return failure_snapshot(failure);
        }
    }

    if let Some(cached) = cached_snapshot() {
        return as_cached(cached);
    }

    failure_snapshot(
        app_server_result
            .err()
            .unwrap_or_else(|| QuotaFailure::new(QuotaFailureKind::InvalidData)),
    )
}

/// Locate the standalone Codex CLI used to launch App Server.
pub fn find_codex_binary() -> Option<PathBuf> {
    let possible_paths = [
        dirs::data_dir().map(|path| path.join("npm").join("codex.cmd")),
        dirs::data_local_dir().map(|path| path.join("Programs").join("codex").join("codex.exe")),
        which::which("codex.exe").ok(),
        which::which("codex").ok(),
    ];

    possible_paths
        .into_iter()
        .flatten()
        .find(|path| path.exists())
}

async fn fetch_from_app_server(
    binary: &Path,
    proxy: &ResolvedCodexProxy,
) -> Result<CodexQuotaSnapshot, QuotaFailure> {
    let mut command = Command::new(binary);
    command
        .args([
            "-s",
            "read-only",
            "-a",
            "untrusted",
            "-c",
            OFFICIAL_CHATGPT_BASE_URL_OVERRIDE,
            "app-server",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // App Server diagnostics can contain account or backend details. The
        // overlay never forwards or logs its raw stderr.
        .stderr(Stdio::null())
        .kill_on_drop(true);
    proxy.apply_to_app_server(&mut command);

    #[cfg(windows)]
    command.creation_flags(0x08000000);

    let mut child = command
        .spawn()
        .map_err(|error| classify_spawn_error(&error))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?;
    let mut lines = BufReader::new(stdout).lines();

    let exchange = async {
        write_rpc_message(
            &mut stdin,
            &json!({
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": {
                        "name": "quackquota",
                        "title": "QuackQuota",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }),
        )
        .await?;
        let _ = read_rpc_result(&mut lines, 1, proxy.is_active()).await?;

        write_rpc_message(
            &mut stdin,
            &json!({ "method": "initialized", "params": {} }),
        )
        .await?;
        write_rpc_message(
            &mut stdin,
            &json!({ "id": 2, "method": "account/rateLimits/read" }),
        )
        .await?;

        let result = read_rpc_result(&mut lines, 2, proxy.is_active()).await?;
        let response: AppServerRateLimitsResponse = serde_json::from_value(result)
            .map_err(|_| QuotaFailure::new(QuotaFailureKind::InvalidData))?;
        snapshot_from_app_server(response)
    }
    .await;

    let _ = child.start_kill();
    let _ = timeout(APP_SERVER_SHUTDOWN_TIMEOUT, child.wait()).await;
    exchange
}

async fn write_rpc_message(
    stdin: &mut tokio::process::ChildStdin,
    message: &Value,
) -> Result<(), QuotaFailure> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|_| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?;
    encoded.push(b'\n');
    stdin
        .write_all(&encoded)
        .await
        .map_err(|_| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?;
    stdin
        .flush()
        .await
        .map_err(|_| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))
}

async fn read_rpc_result(
    lines: &mut Lines<BufReader<ChildStdout>>,
    expected_id: i64,
    proxy_active: bool,
) -> Result<Value, QuotaFailure> {
    timeout(APP_SERVER_TIMEOUT, async {
        loop {
            let line = lines
                .next_line()
                .await
                .map_err(|_| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?
                .ok_or_else(|| QuotaFailure::new(QuotaFailureKind::AppServerUnavailable))?;
            let message: Value = serde_json::from_str(&line)
                .map_err(|_| QuotaFailure::new(QuotaFailureKind::InvalidData))?;
            if message.get("id").and_then(Value::as_i64) != Some(expected_id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                let message = error.get("message").and_then(Value::as_str).unwrap_or("");
                return Err(classify_rpc_error(message, proxy_active));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| QuotaFailure::new(QuotaFailureKind::InvalidData));
        }
    })
    .await
    .map_err(|_| {
        QuotaFailure::new(if proxy_active {
            QuotaFailureKind::ProxyConnection
        } else {
            QuotaFailureKind::DnsOrNetwork
        })
    })?
}

fn snapshot_from_app_server(
    response: AppServerRateLimitsResponse,
) -> Result<CodexQuotaSnapshot, QuotaFailure> {
    let top_level = response.rate_limits;
    let mut buckets: Vec<(String, AppServerRateLimitSnapshot)> = response
        .rate_limits_by_limit_id
        .filter(|buckets| !buckets.is_empty())
        .map(|buckets| buckets.into_iter().collect())
        .unwrap_or_else(|| {
            top_level
                .clone()
                .map(|bucket| {
                    let key = nonblank(bucket.limit_id.as_deref()).unwrap_or("codex");
                    vec![(key.to_string(), bucket)]
                })
                .unwrap_or_default()
        });
    buckets.sort_by(|left, right| left.0.cmp(&right.0));

    let plan = top_level
        .as_ref()
        .and_then(|bucket| bucket.plan_type.as_deref())
        .or_else(|| {
            buckets
                .iter()
                .find_map(|(_, bucket)| bucket.plan_type.as_deref())
        })
        .and_then(display_plan);

    let mut windows = Vec::new();
    for (bucket_key, bucket) in buckets {
        let limit_id = nonblank(bucket.limit_id.as_deref())
            .unwrap_or(&bucket_key)
            .to_string();
        let limit_name = nonblank(bucket.limit_name.as_deref()).map(str::to_string);
        if let Some(window) = bucket.primary.as_ref().and_then(|window| {
            app_server_window(
                &limit_id,
                limit_name.as_deref(),
                CodexRateLimitLevel::Primary,
                window,
            )
        }) {
            windows.push(window);
        }
        if let Some(window) = bucket.secondary.as_ref().and_then(|window| {
            app_server_window(
                &limit_id,
                limit_name.as_deref(),
                CodexRateLimitLevel::Secondary,
                window,
            )
        }) {
            windows.push(window);
        }
    }
    sort_windows(&mut windows);

    if windows.is_empty() {
        return Err(QuotaFailure::new(QuotaFailureKind::InvalidData));
    }

    let reset_credits = response
        .rate_limit_reset_credits
        .filter(|credits| credits.available_count >= 0)
        .and_then(|credits| u32::try_from(credits.available_count).ok())
        .map(|available_count| CodexResetCredits { available_count });

    Ok(CodexQuotaSnapshot {
        plan,
        windows,
        reset_credits,
        status: CodexQuotaStatus::Ready,
        source: CodexQuotaSource::AppServer,
        updated_at: Utc::now().to_rfc3339(),
        message: None,
        error_kind: None,
    })
}

fn app_server_window(
    limit_id: &str,
    limit_name: Option<&str>,
    level: CodexRateLimitLevel,
    window: &AppServerRateLimitWindow,
) -> Option<CodexRateLimitWindow> {
    let used_percent = valid_percent(window.used_percent?)?;
    let remaining_percent = (100.0 - used_percent).clamp(0.0, 100.0);
    let window_duration_minutes = window
        .window_duration_mins
        .filter(|minutes| *minutes > 0)
        .and_then(|minutes| u32::try_from(minutes).ok());
    let resets_at = window.resets_at.and_then(timestamp_to_rfc3339);
    let level_id = match level {
        CodexRateLimitLevel::Primary => "primary",
        CodexRateLimitLevel::Secondary => "secondary",
    };

    Some(CodexRateLimitWindow {
        id: format!("{limit_id}:{level_id}"),
        label: quota_label(limit_name, window_duration_minutes),
        used_percent,
        remaining_percent,
        window_duration_minutes,
        resets_at,
        limit_id: Some(limit_id.to_string()),
        limit_name: limit_name.map(str::to_string),
        level,
    })
}

fn snapshot_from_legacy_usage(usage: UsageSnapshot) -> CodexQuotaSnapshot {
    let mut windows = Vec::new();
    if let Some(window) = legacy_window(
        "legacy:primary",
        None,
        CodexRateLimitLevel::Primary,
        &usage.primary,
    ) {
        windows.push(window);
    }
    if let Some(window) = usage.secondary.as_ref().and_then(|window| {
        legacy_window(
            "legacy:secondary",
            None,
            CodexRateLimitLevel::Secondary,
            window,
        )
    }) {
        windows.push(window);
    }
    if let Some(window) = usage.model_specific.as_ref().and_then(|window| {
        legacy_window(
            "legacy:model-specific",
            Some("Code review"),
            CodexRateLimitLevel::Primary,
            window,
        )
    }) {
        windows.push(window);
    }
    if let Some(window) = usage.tertiary.as_ref().and_then(|window| {
        legacy_window(
            "legacy:tertiary",
            None,
            CodexRateLimitLevel::Primary,
            window,
        )
    }) {
        windows.push(window);
    }
    for extra in &usage.extra_rate_windows {
        if let Some(window) = legacy_window(
            &format!("legacy:extra:{}", extra.id),
            Some(&extra.title),
            CodexRateLimitLevel::Primary,
            &extra.window,
        ) {
            windows.push(window);
        }
    }
    sort_windows(&mut windows);

    CodexQuotaSnapshot {
        plan: usage.login_method,
        windows,
        reset_credits: None,
        status: CodexQuotaStatus::Ready,
        source: CodexQuotaSource::LegacyProvider,
        updated_at: usage.updated_at.to_rfc3339(),
        message: None,
        error_kind: None,
    }
}

fn legacy_window(
    id: &str,
    limit_name: Option<&str>,
    level: CodexRateLimitLevel,
    window: &RateWindow,
) -> Option<CodexRateLimitWindow> {
    if window.is_informational {
        return None;
    }
    let used_percent = valid_percent(window.used_percent)?;
    let limit_name = nonblank(limit_name).map(str::to_string);
    Some(CodexRateLimitWindow {
        id: id.to_string(),
        label: quota_label(limit_name.as_deref(), window.window_minutes),
        used_percent,
        remaining_percent: (100.0 - used_percent).clamp(0.0, 100.0),
        window_duration_minutes: window.window_minutes,
        resets_at: window.resets_at.map(|value| value.to_rfc3339()),
        limit_id: None,
        limit_name,
        level,
    })
}

fn quota_label(limit_name: Option<&str>, minutes: Option<u32>) -> String {
    if let Some(limit_name) = nonblank(limit_name) {
        return limit_name.to_string();
    }
    match minutes {
        Some(60) => "1 hour quota".to_string(),
        Some(300) => "5 hour quota".to_string(),
        Some(1_440) => "24 hour quota".to_string(),
        Some(10_080) => "7 day quota".to_string(),
        Some(value) if value >= 1_440 && value % 1_440 == 0 => {
            format!("{} day quota", value / 1_440)
        }
        Some(value) if value % 60 == 0 => format!("{} hour quota", value / 60),
        _ => "Codex quota".to_string(),
    }
}

fn sort_windows(windows: &mut [CodexRateLimitWindow]) {
    windows.sort_by(|left, right| {
        left.remaining_percent
            .total_cmp(&right.remaining_percent)
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn valid_percent(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(value)
}

fn timestamp_to_rfc3339(value: i64) -> Option<String> {
    (value > 0)
        .then(|| Utc.timestamp_opt(value, 0).single())
        .flatten()
        .map(|timestamp| timestamp.to_rfc3339())
}

fn nonblank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn display_plan(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(match value.to_ascii_lowercase().as_str() {
        "guest" => "Guest".to_string(),
        "free" => "ChatGPT Free".to_string(),
        "go" => "Codex Go".to_string(),
        "plus" => "ChatGPT Plus".to_string(),
        "pro" => "ChatGPT Pro".to_string(),
        "pro_lite" | "prolite" | "pro-lite" => "Pro Lite".to_string(),
        "team" => "ChatGPT Team".to_string(),
        "business" => "ChatGPT Business".to_string(),
        "enterprise" => "ChatGPT Enterprise".to_string(),
        "education" | "edu" => "ChatGPT Education".to_string(),
        _ => value.to_string(),
    })
}

fn classify_spawn_error(error: &std::io::Error) -> QuotaFailure {
    let kind = if matches!(error.kind(), std::io::ErrorKind::NotFound) {
        QuotaFailureKind::NotInstalled
    } else {
        QuotaFailureKind::AppServerUnavailable
    };
    QuotaFailure::new(kind)
}

fn classify_rpc_error(message: &str, proxy_active: bool) -> QuotaFailure {
    let kind = match classify_connection_text(message, proxy_active) {
        CodexConnectionErrorKind::NotLoggedIn => QuotaFailureKind::NotLoggedIn,
        CodexConnectionErrorKind::ProxyConnection => QuotaFailureKind::ProxyConnection,
        CodexConnectionErrorKind::DnsOrNetwork => QuotaFailureKind::DnsOrNetwork,
        _ => QuotaFailureKind::ServiceError,
    };
    QuotaFailure::new(kind)
}

fn classify_legacy_error(
    error: &ProviderError,
    cli_installed: bool,
    proxy_active: bool,
) -> QuotaFailure {
    let kind = match error {
        ProviderError::AuthRequired | ProviderError::OAuth(_) => QuotaFailureKind::NotLoggedIn,
        ProviderError::NotInstalled(_) => {
            if cli_installed {
                QuotaFailureKind::NotLoggedIn
            } else {
                QuotaFailureKind::NotInstalled
            }
        }
        ProviderError::Network(error) => {
            if proxy_active && error.is_connect() {
                QuotaFailureKind::ProxyConnection
            } else {
                QuotaFailureKind::DnsOrNetwork
            }
        }
        ProviderError::Timeout => {
            if proxy_active {
                QuotaFailureKind::ProxyConnection
            } else {
                QuotaFailureKind::DnsOrNetwork
            }
        }
        ProviderError::Parse(message)
            if message.to_ascii_lowercase().contains("credential")
                || message.to_ascii_lowercase().contains("access_token") =>
        {
            QuotaFailureKind::NotLoggedIn
        }
        ProviderError::Parse(_) => QuotaFailureKind::InvalidData,
        ProviderError::Other(message)
            if message.to_ascii_lowercase().contains("proxy configuration") =>
        {
            QuotaFailureKind::ProxyConfiguration
        }
        _ => QuotaFailureKind::ServiceError,
    };
    QuotaFailure::new(kind)
}

fn failure_snapshot(failure: QuotaFailure) -> CodexQuotaSnapshot {
    let (status, error_kind, message) = match failure.kind {
        QuotaFailureKind::NotInstalled => (
            CodexQuotaStatus::NotInstalled,
            CodexConnectionErrorKind::AppServerUnavailable,
            "Codex CLI is not installed or cannot be found.",
        ),
        QuotaFailureKind::NotLoggedIn => (
            CodexQuotaStatus::NotLoggedIn,
            CodexConnectionErrorKind::NotLoggedIn,
            "Codex CLI is not signed in.",
        ),
        QuotaFailureKind::DnsOrNetwork => (
            CodexQuotaStatus::Offline,
            CodexConnectionErrorKind::DnsOrNetwork,
            "Codex cannot reach the network.",
        ),
        QuotaFailureKind::ProxyConnection => (
            CodexQuotaStatus::Offline,
            CodexConnectionErrorKind::ProxyConnection,
            "Codex cannot connect through the selected proxy.",
        ),
        QuotaFailureKind::ProxyConfiguration => (
            CodexQuotaStatus::ProviderError,
            CodexConnectionErrorKind::ProxyConfiguration,
            "The Codex proxy setting is invalid.",
        ),
        QuotaFailureKind::AppServerUnavailable => (
            CodexQuotaStatus::ProviderError,
            CodexConnectionErrorKind::AppServerUnavailable,
            "Codex App Server could not be started.",
        ),
        QuotaFailureKind::ServiceError => (
            CodexQuotaStatus::ProviderError,
            CodexConnectionErrorKind::ServiceError,
            "The Codex service returned an error.",
        ),
        QuotaFailureKind::InvalidData => (
            CodexQuotaStatus::InvalidData,
            CodexConnectionErrorKind::InvalidData,
            "Codex returned no valid quota windows.",
        ),
    };
    CodexQuotaSnapshot {
        plan: None,
        windows: Vec::new(),
        reset_credits: None,
        status,
        source: CodexQuotaSource::LegacyProvider,
        updated_at: Utc::now().to_rfc3339(),
        message: Some(message.to_string()),
        error_kind: Some(error_kind),
    }
}

fn cache() -> &'static Mutex<Option<CodexQuotaSnapshot>> {
    QUOTA_CACHE.get_or_init(|| Mutex::new(None))
}

fn store_cached_snapshot(snapshot: &CodexQuotaSnapshot) {
    if snapshot.status != CodexQuotaStatus::Ready || snapshot.windows.is_empty() {
        return;
    }
    if let Ok(mut cache) = cache().lock() {
        *cache = Some(snapshot.clone());
    }
}

fn cached_snapshot() -> Option<CodexQuotaSnapshot> {
    cache().lock().ok()?.clone()
}

fn as_cached(mut snapshot: CodexQuotaSnapshot) -> CodexQuotaSnapshot {
    snapshot.status = CodexQuotaStatus::Cached;
    snapshot.source = CodexQuotaSource::Cache;
    snapshot.message = Some("Showing the last successful Codex quota snapshot.".to_string());
    snapshot.error_kind = None;
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_response(value: Value) -> AppServerRateLimitsResponse {
        serde_json::from_value(value).expect("rate limit response")
    }

    #[test]
    fn maps_primary_without_secondary_as_one_window() {
        let snapshot = snapshot_from_app_server(parse_response(json!({
            "rateLimits": {
                "limitId": "codex",
                "limitName": null,
                "primary": {
                    "usedPercent": 24,
                    "windowDurationMins": 10080,
                    "resetsAt": 1784246400
                },
                "secondary": null,
                "planType": "plus"
            }
        })))
        .expect("snapshot");

        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].id, "codex:primary");
        assert_eq!(snapshot.windows[0].remaining_percent, 76.0);
        assert_eq!(snapshot.windows[0].label, "7 day quota");
    }

    #[test]
    fn maps_primary_and_secondary_as_independent_windows() {
        let snapshot = snapshot_from_app_server(parse_response(json!({
            "rateLimits": {
                "limitId": "codex",
                "primary": { "usedPercent": 20, "windowDurationMins": 300 },
                "secondary": { "usedPercent": 40, "windowDurationMins": 10080 }
            }
        })))
        .expect("snapshot");

        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].level, CodexRateLimitLevel::Secondary);
        assert_eq!(snapshot.windows[1].level, CodexRateLimitLevel::Primary);
    }

    #[test]
    fn maps_all_rate_limit_ids_and_preserves_names() {
        let snapshot = snapshot_from_app_server(parse_response(json!({
            "rateLimits": {
                "limitId": "codex",
                "primary": { "usedPercent": 10, "windowDurationMins": 300 }
            },
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "primary": { "usedPercent": 10, "windowDurationMins": 300 }
                },
                "codex_spark": {
                    "limitId": "codex_spark",
                    "limitName": "Codex Spark",
                    "primary": { "usedPercent": 70, "windowDurationMins": 1440 },
                    "secondary": { "usedPercent": 25, "windowDurationMins": 10080 }
                }
            },
            "rateLimitResetCredits": { "availableCount": 2 }
        })))
        .expect("snapshot");

        assert_eq!(snapshot.windows.len(), 3);
        assert_eq!(snapshot.windows[0].id, "codex_spark:primary");
        assert_eq!(
            snapshot.windows[0].limit_name.as_deref(),
            Some("Codex Spark")
        );
        assert_eq!(snapshot.reset_credits.unwrap().available_count, 2);
    }

    #[test]
    fn rejects_missing_and_out_of_range_percentages() {
        let result = snapshot_from_app_server(parse_response(json!({
            "rateLimits": {
                "primary": { "windowDurationMins": 300 },
                "secondary": { "usedPercent": 120, "windowDurationMins": 10080 }
            }
        })));

        assert_eq!(
            result.expect_err("invalid data").kind,
            QuotaFailureKind::InvalidData
        );
    }

    #[test]
    fn unknown_duration_is_not_guessed_as_five_hour_or_weekly() {
        assert_eq!(quota_label(None, Some(90)), "Codex quota");
        assert_eq!(quota_label(None, None), "Codex quota");
    }

    #[test]
    fn quota_dto_contains_no_credential_fields() {
        let snapshot = snapshot_from_app_server(parse_response(json!({
            "rateLimits": {
                "primary": { "usedPercent": 24, "windowDurationMins": 10080 }
            }
        })))
        .expect("snapshot");
        let encoded = serde_json::to_string(&snapshot).expect("serialize snapshot");
        for forbidden in [
            "accessToken",
            "refreshToken",
            "cookie",
            "authorization",
            "auth.json",
        ] {
            assert!(
                !encoded
                    .to_ascii_lowercase()
                    .contains(&forbidden.to_ascii_lowercase())
            );
        }
    }
}
