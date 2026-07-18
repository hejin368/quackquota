//! Debug-only, bounded lifecycle bridge diagnostics.
//!
//! Release builds retain the call sites as no-ops so they never create a
//! lifecycle diagnostic file. Debug records contain only status enums, known
//! QuackQuota window labels, a process-local generation number, and safe error
//! categories. They never contain paths, titles, command lines, IDs, proxy
//! endpoints, credentials, or provider data.

use codexbar::chatgpt_desktop::ChatGptDesktopStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrontendDiagnosticEvent {
    #[serde(rename = "frontend-initial-invoke-success")]
    InitialInvokeSuccess,
    #[serde(rename = "frontend-initial-invoke-error")]
    InitialInvokeError,
    #[serde(rename = "frontend-event-listen-success")]
    EventListenSuccess,
    #[serde(rename = "frontend-event-listen-error")]
    EventListenError,
    #[serde(rename = "frontend-payload-validation-success")]
    PayloadValidationSuccess,
    #[serde(rename = "frontend-payload-validation-error")]
    PayloadValidationError,
}

impl FrontendDiagnosticEvent {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitialInvokeSuccess => "frontend_initial_invoke_success",
            Self::InitialInvokeError => "frontend_initial_invoke_error",
            Self::EventListenSuccess => "frontend_event_listen_success",
            Self::EventListenError => "frontend_event_listen_error",
            Self::PayloadValidationSuccess => "frontend_payload_validation_success",
            Self::PayloadValidationError => "frontend_payload_validation_error",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrontendDiagnosticWindow {
    CodexOverlay,
    Settings,
    Unknown,
}

impl FrontendDiagnosticWindow {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CodexOverlay => "codex-overlay",
            Self::Settings => "settings",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleDiagnosticErrorCategory {
    InvokeCommandNotFound,
    InvokePermissionDenied,
    InvokeBackendUnavailable,
    InvokeInvalidResponse,
    EventListenPermissionDenied,
    EventListenFailed,
    EventInvalidPayload,
    SharedStateNotReady,
    ObserverComInitialization,
    ObserverPackageIdentityQuery,
    ObserverTopLevelWindowEnumeration,
    UnknownSafeCategory,
    EventEmitFailed,
}

impl LifecycleDiagnosticErrorCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InvokeCommandNotFound => "invoke-command-not-found",
            Self::InvokePermissionDenied => "invoke-permission-denied",
            Self::InvokeBackendUnavailable => "invoke-backend-unavailable",
            Self::InvokeInvalidResponse => "invoke-invalid-response",
            Self::EventListenPermissionDenied => "event-listen-permission-denied",
            Self::EventListenFailed => "event-listen-failed",
            Self::EventInvalidPayload => "event-invalid-payload",
            Self::SharedStateNotReady => "shared-state-not-ready",
            Self::ObserverComInitialization => "observer-com-initialization",
            Self::ObserverPackageIdentityQuery => "observer-package-identity-query",
            Self::ObserverTopLevelWindowEnumeration => "observer-top-level-window-enumeration",
            Self::UnknownSafeCategory => "unknown-safe-category",
            Self::EventEmitFailed => "event-emit-failed",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendDiagnosticInput {
    pub event: FrontendDiagnosticEvent,
    pub window_label: FrontendDiagnosticWindow,
    pub status: Option<ChatGptDesktopStatus>,
    pub error_category: Option<LifecycleDiagnosticErrorCategory>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticRecord<'a> {
    timestamp_ms: u128,
    event: &'a str,
    status: Option<ChatGptDesktopStatus>,
    managed_state_generation: Option<u64>,
    window_label: Option<&'a str>,
    error_category: Option<&'a str>,
}

#[cfg(all(debug_assertions, not(test)))]
const MAX_DIAGNOSTIC_BYTES: u64 = 64 * 1024;

#[cfg(all(debug_assertions, not(test)))]
fn write_record(record: DiagnosticRecord<'_>) {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let Ok(_guard) = WRITE_LOCK.get_or_init(|| Mutex::new(())).lock() else {
        return;
    };
    let directory = std::env::temp_dir().join("QuackQuota");
    if fs::create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join("chatgpt-lifecycle-diagnostic.jsonl");
    let Ok(mut line) = serde_json::to_vec(&record) else {
        return;
    };
    line.push(b'\n');
    let Ok(line_len) = u64::try_from(line.len()) else {
        return;
    };
    let current_len = path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    if current_len.saturating_add(line_len) > MAX_DIAGNOSTIC_BYTES
        && OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .is_err()
    {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = file.write_all(&line);
}

#[cfg(any(not(debug_assertions), test))]
fn write_record(_record: DiagnosticRecord<'_>) {}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub fn record_backend(
    event: &'static str,
    status: Option<ChatGptDesktopStatus>,
    managed_state_generation: Option<u64>,
    window_label: Option<&'static str>,
    error_category: Option<LifecycleDiagnosticErrorCategory>,
) {
    write_record(DiagnosticRecord {
        timestamp_ms: now_ms(),
        event,
        status,
        managed_state_generation,
        window_label,
        error_category: error_category.map(LifecycleDiagnosticErrorCategory::as_str),
    });
}

pub fn record_frontend(input: FrontendDiagnosticInput) {
    write_record(DiagnosticRecord {
        timestamp_ms: now_ms(),
        event: input.event.as_str(),
        status: input.status,
        managed_state_generation: None,
        window_label: Some(input.window_label.as_str()),
        error_category: input
            .error_category
            .map(LifecycleDiagnosticErrorCategory::as_str),
    });
}

pub const fn enabled() -> bool {
    cfg!(debug_assertions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontend_records_only_safe_enum_fields() {
        let input = FrontendDiagnosticInput {
            event: FrontendDiagnosticEvent::InitialInvokeError,
            window_label: FrontendDiagnosticWindow::CodexOverlay,
            status: None,
            error_category: Some(LifecycleDiagnosticErrorCategory::InvokePermissionDenied),
        };
        assert_eq!(input.event.as_str(), "frontend_initial_invoke_error");
        assert_eq!(input.window_label.as_str(), "codex-overlay");
        assert_eq!(
            input.error_category.unwrap().as_str(),
            "invoke-permission-denied"
        );
    }

    #[test]
    fn frontend_event_wire_names_match_the_typescript_dto() {
        let input: FrontendDiagnosticInput = serde_json::from_value(serde_json::json!({
            "event": "frontend-initial-invoke-success",
            "windowLabel": "settings",
            "status": "running"
        }))
        .expect("the frontend DTO should deserialize");

        assert!(matches!(
            input.event,
            FrontendDiagnosticEvent::InitialInvokeSuccess
        ));
        assert!(matches!(
            input.window_label,
            FrontendDiagnosticWindow::Settings
        ));
        assert!(matches!(input.status, Some(ChatGptDesktopStatus::Running)));
    }

    #[test]
    fn debug_diagnostics_are_disabled_for_release_builds() {
        assert_eq!(enabled(), cfg!(debug_assertions));
    }
}
