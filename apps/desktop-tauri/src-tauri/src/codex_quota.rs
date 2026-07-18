//! Minimal Codex-only bridge for the detached overlay.

use codexbar::providers::codex::{
    CodexProxyStatus, CodexProxyTestResult, CodexQuotaSnapshot, CodexQuotaSource, CodexQuotaStatus,
    CodexRateLimitWindow, CodexResetCredits,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const QUOTA_CACHE_SCHEMA_VERSION: u32 = 1;
const QUOTA_CACHE_FILE_NAME: &str = "last-successful-codex-quota.json";

/// On-disk cache is intentionally narrower than the IPC DTO: it contains only
/// displayable quota fields and no error strings, paths, credentials, or raw
/// provider responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LastSuccessfulQuotaSnapshot {
    schema_version: u32,
    plan: Option<String>,
    windows: Vec<CodexRateLimitWindow>,
    reset_credits: Option<CodexResetCredits>,
    source: CodexQuotaSource,
    fetched_at: String,
}

impl LastSuccessfulQuotaSnapshot {
    fn from_live(snapshot: &CodexQuotaSnapshot) -> Option<Self> {
        (snapshot.status == CodexQuotaStatus::Ready && !snapshot.windows.is_empty()).then(|| Self {
            schema_version: QUOTA_CACHE_SCHEMA_VERSION,
            plan: snapshot.plan.clone(),
            windows: snapshot.windows.clone(),
            reset_credits: snapshot.reset_credits.clone(),
            source: snapshot.source,
            fetched_at: snapshot.updated_at.clone(),
        })
    }

    fn into_snapshot(self) -> Option<CodexQuotaSnapshot> {
        (self.schema_version == QUOTA_CACHE_SCHEMA_VERSION && !self.windows.is_empty()).then(|| {
            CodexQuotaSnapshot {
                plan: self.plan,
                windows: self.windows,
                reset_credits: self.reset_credits,
                status: CodexQuotaStatus::Cached,
                source: CodexQuotaSource::Cache,
                updated_at: self.fetched_at,
                message: Some("Showing the last successful Codex quota snapshot.".to_string()),
                error_kind: None,
            }
        })
    }
}

fn quota_cache_path() -> Option<PathBuf> {
    codexbar::settings::Settings::settings_path().and_then(|path| {
        path.parent()
            .map(|parent| parent.join(QUOTA_CACHE_FILE_NAME))
    })
}

fn load_last_successful_quota_from(path: &Path) -> Option<CodexQuotaSnapshot> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<LastSuccessfulQuotaSnapshot>(&content)
        .ok()?
        .into_snapshot()
}

fn load_last_successful_quota() -> Option<CodexQuotaSnapshot> {
    quota_cache_path()
        .as_deref()
        .and_then(load_last_successful_quota_from)
}

fn store_last_successful_quota_at(
    path: &Path,
    snapshot: &CodexQuotaSnapshot,
) -> std::io::Result<()> {
    let Some(cache) = LastSuccessfulQuotaSnapshot::from_live(snapshot) else {
        return Ok(());
    };
    let encoded = serde_json::to_vec_pretty(&cache).map_err(std::io::Error::other)?;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("quota cache has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temporary, encoded)?;
    std::fs::rename(&temporary, path).inspect_err(|_error| {
        let _ = std::fs::remove_file(&temporary);
    })
}

fn store_last_successful_quota(snapshot: &CodexQuotaSnapshot) {
    if let Some(path) = quota_cache_path()
        && let Err(error) = store_last_successful_quota_at(&path, snapshot)
    {
        tracing::warn!(
            target: "quackquota::codex_quota_cache",
            error = %codexbar::logging::safe_error_message(error),
            "failed to store last successful Codex quota snapshot"
        );
    }
}

/// Read the Codex-only snapshot and update the safe local cache only after a
/// successful live response. Failed refreshes retain, and return, the last
/// successful disk cache when available.
pub async fn read_and_cache_codex_rate_limits() -> CodexQuotaSnapshot {
    let snapshot = codexbar::providers::codex::read_codex_quota_snapshot().await;
    if LastSuccessfulQuotaSnapshot::from_live(&snapshot).is_some() {
        store_last_successful_quota(&snapshot);
        return snapshot;
    }
    load_last_successful_quota().unwrap_or(snapshot)
}

/// Read only Codex rate limits. This command never enters the global provider
/// refresh pipeline and therefore cannot load credentials for other providers.
#[tauri::command]
pub async fn read_codex_rate_limits() -> CodexQuotaSnapshot {
    read_and_cache_codex_rate_limits().await
}

#[tauri::command]
pub fn get_codex_proxy_status() -> CodexProxyStatus {
    codexbar::providers::codex::proxy_status()
}

#[tauri::command]
pub async fn test_codex_proxy_connection() -> CodexProxyTestResult {
    codexbar::providers::codex::test_codex_proxy_connection().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use codexbar::providers::codex::CodexRateLimitLevel;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestCacheDir(PathBuf);

    impl Drop for TestCacheDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temporary_cache_path() -> (TestCacheDir, PathBuf) {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "quackquota-quota-cache-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let guard = TestCacheDir(directory);
        let path = guard.0.join("quota.json");
        (guard, path)
    }

    fn ready_snapshot() -> CodexQuotaSnapshot {
        CodexQuotaSnapshot {
            plan: Some("Plus".to_string()),
            windows: vec![CodexRateLimitWindow {
                id: "weekly:primary".to_string(),
                label: "7 day quota".to_string(),
                used_percent: 35.0,
                remaining_percent: 65.0,
                window_duration_minutes: Some(10_080),
                resets_at: Some("2026-07-25T00:00:00Z".to_string()),
                limit_id: Some("weekly".to_string()),
                limit_name: None,
                level: CodexRateLimitLevel::Primary,
            }],
            reset_credits: Some(CodexResetCredits { available_count: 2 }),
            status: CodexQuotaStatus::Ready,
            source: CodexQuotaSource::AppServer,
            updated_at: "2026-07-18T00:00:00Z".to_string(),
            message: None,
            error_kind: None,
        }
    }

    #[test]
    fn cache_roundtrip_contains_only_displayable_quota_data() {
        let (_directory, path) = temporary_cache_path();
        let snapshot = ready_snapshot();
        store_last_successful_quota_at(&path, &snapshot).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        for forbidden in [
            "accessToken",
            "refreshToken",
            "cookie",
            "authorization",
            "auth.json",
        ] {
            assert!(
                !raw.to_ascii_lowercase()
                    .contains(&forbidden.to_ascii_lowercase())
            );
        }
        let cached = load_last_successful_quota_from(&path).unwrap();
        assert_eq!(cached.status, CodexQuotaStatus::Cached);
        assert_eq!(cached.source, CodexQuotaSource::Cache);
        assert_eq!(cached.windows, snapshot.windows);
    }

    #[test]
    fn corrupt_or_unknown_cache_is_ignored() {
        let (_directory, path) = temporary_cache_path();
        std::fs::write(&path, "not json").unwrap();
        assert!(load_last_successful_quota_from(&path).is_none());

        std::fs::write(&path, r#"{"schemaVersion":999,"windows":[]}"#).unwrap();
        assert!(load_last_successful_quota_from(&path).is_none());
    }

    #[test]
    fn failed_snapshots_do_not_overwrite_a_last_successful_record() {
        let (_directory, path) = temporary_cache_path();
        let ready = ready_snapshot();
        store_last_successful_quota_at(&path, &ready).unwrap();
        let failed = CodexQuotaSnapshot {
            status: CodexQuotaStatus::Offline,
            windows: Vec::new(),
            ..ready.clone()
        };
        store_last_successful_quota_at(&path, &failed).unwrap();
        assert_eq!(
            load_last_successful_quota_from(&path).unwrap().windows,
            ready.windows
        );
    }
}
