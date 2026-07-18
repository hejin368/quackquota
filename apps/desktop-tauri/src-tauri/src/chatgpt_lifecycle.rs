//! Low-frequency Windows ChatGPT desktop lifecycle watcher.
//!
//! The watcher consumes the shared package/window recognizer and never scans
//! browsers, command lines, or provider credentials. It only emits stable
//! lifecycle transitions after consecutive samples.

use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use codexbar::chatgpt_desktop::{
    ChatGptDesktopSnapshot, ChatGptDesktopStatus, ChatGptDesktopUnavailableReason,
};
use codexbar::settings::Settings;
use tauri::{AppHandle, Emitter, Manager};

use crate::chatgpt_lifecycle_diagnostic::{
    self, FrontendDiagnosticInput, LifecycleDiagnosticErrorCategory,
};

const POLL_INTERVAL: Duration = Duration::from_millis(1_500);
const RUNNING_CONFIRMATION_SAMPLES: u8 = 2;
const NOT_RUNNING_CONFIRMATION_SAMPLES: u8 = 3;
const STATUS_CHANGED_EVENT: &str = "chatgpt-desktop-status-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LifecyclePreferences {
    monitor: bool,
    show_on_start: bool,
    hide_on_exit: bool,
}

impl From<&Settings> for LifecyclePreferences {
    fn from(settings: &Settings) -> Self {
        Self {
            monitor: settings.monitor_chatgpt_desktop,
            show_on_start: settings.show_overlay_on_chatgpt_start,
            hide_on_exit: settings.hide_overlay_on_chatgpt_exit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleAction {
    ShowOverlay,
    HideOverlay,
}

#[derive(Debug)]
struct LifecycleTracker {
    preferences: LifecyclePreferences,
    snapshot: ChatGptDesktopSnapshot,
    pending_status: Option<ChatGptDesktopStatus>,
    pending_samples: u8,
    initialized: bool,
    manual_hide_during_current_run: bool,
}

impl LifecycleTracker {
    fn new(settings: &Settings) -> Self {
        let preferences = LifecyclePreferences::from(settings);
        let status = if preferences.monitor {
            ChatGptDesktopStatus::Detecting
        } else {
            ChatGptDesktopStatus::MonitoringDisabled
        };
        Self {
            preferences,
            snapshot: ChatGptDesktopSnapshot::new(status),
            pending_status: None,
            pending_samples: 0,
            initialized: false,
            manual_hide_during_current_run: false,
        }
    }

    fn update_preferences(&mut self, settings: &Settings) -> bool {
        self.preferences = LifecyclePreferences::from(settings);
        if !self.preferences.monitor {
            self.pending_status = None;
            self.pending_samples = 0;
            self.snapshot = ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::MonitoringDisabled);
            return true;
        }

        if self.snapshot.status == ChatGptDesktopStatus::MonitoringDisabled {
            self.snapshot = ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Detecting);
            self.pending_status = None;
            self.pending_samples = 0;
            self.initialized = false;
            self.manual_hide_during_current_run = false;
            return true;
        }
        false
    }

    fn snapshot(&self) -> ChatGptDesktopSnapshot {
        self.snapshot.clone()
    }

    fn note_manual_hide(&mut self) {
        if self.snapshot.status == ChatGptDesktopStatus::Running {
            self.manual_hide_during_current_run = true;
        }
    }

    fn note_manual_show(&mut self) {
        if self.snapshot.status == ChatGptDesktopStatus::Running {
            self.manual_hide_during_current_run = false;
        }
    }

    fn accept(&mut self, status: ChatGptDesktopStatus) -> (bool, Option<LifecycleAction>) {
        let previous = self.snapshot.status;
        self.snapshot = ChatGptDesktopSnapshot::new(status);
        let changed = previous != status;
        let was_initialized = self.initialized;
        self.initialized = true;

        let action = match (previous, status) {
            (_, ChatGptDesktopStatus::NotRunning) => {
                self.manual_hide_during_current_run = false;
                (was_initialized
                    && previous == ChatGptDesktopStatus::Running
                    && self.preferences.hide_on_exit)
                    .then_some(LifecycleAction::HideOverlay)
            }
            (_, ChatGptDesktopStatus::Running) => {
                let is_start = !was_initialized || previous == ChatGptDesktopStatus::NotRunning;
                (is_start && self.preferences.show_on_start && !self.manual_hide_during_current_run)
                    .then_some(LifecycleAction::ShowOverlay)
            }
            _ => None,
        };
        (changed, action)
    }

    fn sample(&mut self, observed: ChatGptDesktopStatus) -> (bool, Option<LifecycleAction>) {
        if !self.preferences.monitor {
            let changed = self.snapshot.status != ChatGptDesktopStatus::MonitoringDisabled;
            self.snapshot = ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::MonitoringDisabled);
            return (changed, None);
        }

        if !matches!(
            observed,
            ChatGptDesktopStatus::Running | ChatGptDesktopStatus::NotRunning
        ) {
            self.pending_status = None;
            self.pending_samples = 0;
            let changed = self.snapshot.status != observed;
            self.snapshot = ChatGptDesktopSnapshot::new(observed);
            return (changed, None);
        }

        if self.pending_status == Some(observed) {
            self.pending_samples = self.pending_samples.saturating_add(1);
        } else {
            self.pending_status = Some(observed);
            self.pending_samples = 1;
        }

        let required_samples = match observed {
            ChatGptDesktopStatus::Running => RUNNING_CONFIRMATION_SAMPLES,
            ChatGptDesktopStatus::NotRunning => NOT_RUNNING_CONFIRMATION_SAMPLES,
            _ => unreachable!(),
        };
        if self.pending_samples < required_samples || self.snapshot.status == observed {
            return (false, None);
        }
        self.accept(observed)
    }
}

pub struct ChatGptLifecycleState {
    tracker: Mutex<LifecycleTracker>,
    generation: u64,
}

impl ChatGptLifecycleState {
    pub fn new(settings: &Settings) -> Self {
        static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
        Self {
            tracker: Mutex::new(LifecycleTracker::new(settings)),
            generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn generation(&self) -> u64 {
        self.generation
    }
}

fn diagnostic_window_label(label: &str) -> &'static str {
    match label {
        crate::codex_overlay::CODEX_OVERLAY_LABEL => crate::codex_overlay::CODEX_OVERLAY_LABEL,
        "settings" => "settings",
        _ => "other",
    }
}

fn observer_error_category(
    reason: ChatGptDesktopUnavailableReason,
) -> LifecycleDiagnosticErrorCategory {
    match reason {
        ChatGptDesktopUnavailableReason::ComInitialization => {
            LifecycleDiagnosticErrorCategory::ObserverComInitialization
        }
        ChatGptDesktopUnavailableReason::PackageIdentityQuery => {
            LifecycleDiagnosticErrorCategory::ObserverPackageIdentityQuery
        }
        ChatGptDesktopUnavailableReason::TopLevelWindowEnumeration => {
            LifecycleDiagnosticErrorCategory::ObserverTopLevelWindowEnumeration
        }
    }
}

fn emit_status(app: &AppHandle, snapshot: ChatGptDesktopSnapshot, generation: u64) {
    let status = snapshot.status;
    chatgpt_lifecycle_diagnostic::record_backend(
        "event_emit_attempted",
        Some(status),
        Some(generation),
        None,
        None,
    );
    match app.emit(STATUS_CHANGED_EVENT, snapshot) {
        Ok(_) => chatgpt_lifecycle_diagnostic::record_backend(
            "event_emit_success",
            Some(status),
            Some(generation),
            None,
            None,
        ),
        Err(_) => chatgpt_lifecycle_diagnostic::record_backend(
            "event_emit_error",
            Some(status),
            Some(generation),
            None,
            Some(LifecycleDiagnosticErrorCategory::EventEmitFailed),
        ),
    }
}

fn current_snapshot(state: &ChatGptLifecycleState) -> ChatGptDesktopSnapshot {
    state.tracker.lock().unwrap().snapshot()
}

fn sample_managed_state(
    state: &ChatGptLifecycleState,
    observed: ChatGptDesktopStatus,
) -> (bool, Option<LifecycleAction>, ChatGptDesktopSnapshot) {
    let mut tracker = state.tracker.lock().unwrap();
    let (changed, action) = tracker.sample(observed);
    (changed, action, tracker.snapshot())
}

fn apply_action(app: &AppHandle, action: LifecycleAction) {
    match action {
        LifecycleAction::ShowOverlay => {
            if let Err(error) = crate::codex_overlay::show_for_chatgpt_lifecycle(app) {
                tracing::warn!(
                    target: "quackquota::chatgpt_lifecycle",
                    error = %codexbar::logging::safe_error_message(error),
                    "failed to show overlay after ChatGPT start"
                );
                return;
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::codex_quota::read_and_cache_codex_rate_limits().await;
                let _ = app;
            });
        }
        LifecycleAction::HideOverlay => {
            if let Err(error) = crate::codex_overlay::hide_for_chatgpt_lifecycle(app) {
                tracing::warn!(
                    target: "quackquota::chatgpt_lifecycle",
                    error = %codexbar::logging::safe_error_message(error),
                    "failed to hide overlay after ChatGPT exit"
                );
            }
        }
    }
}

pub fn install(app: AppHandle) {
    if let Some(state) = app.try_state::<ChatGptLifecycleState>() {
        let snapshot = current_snapshot(state.inner());
        chatgpt_lifecycle_diagnostic::record_backend(
            "watcher_started",
            Some(snapshot.status),
            Some(state.generation()),
            None,
            None,
        );
        chatgpt_lifecycle_diagnostic::record_backend(
            "managed_state_generation",
            Some(snapshot.status),
            Some(state.generation()),
            None,
            None,
        );
    }
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let (changed, action, snapshot, generation, unavailable_reason) = {
                let Some(state) = app.try_state::<ChatGptLifecycleState>() else {
                    return;
                };
                let monitoring = state.tracker.lock().unwrap().preferences.monitor;
                let observation = if monitoring {
                    codexbar::chatgpt_desktop::observe_with_diagnostic()
                } else {
                    codexbar::chatgpt_desktop::ChatGptDesktopObservation {
                        snapshot: ChatGptDesktopSnapshot::new(
                            ChatGptDesktopStatus::MonitoringDisabled,
                        ),
                        unavailable_reason: None,
                    }
                };
                let (changed, action, snapshot) =
                    sample_managed_state(state.inner(), observation.snapshot.status);
                (
                    changed,
                    action,
                    snapshot,
                    state.generation(),
                    observation.unavailable_reason,
                )
            };
            if changed {
                chatgpt_lifecycle_diagnostic::record_backend(
                    "watcher_internal_status",
                    Some(snapshot.status),
                    Some(generation),
                    None,
                    unavailable_reason.map(observer_error_category),
                );
                emit_status(&app, snapshot, generation);
            }
            if let Some(action) = action {
                apply_action(&app, action);
            }
        }
    });
}

pub fn update_preferences(app: &AppHandle, settings: &Settings) {
    let Some(state) = app.try_state::<ChatGptLifecycleState>() else {
        return;
    };
    let (changed, snapshot, generation) = {
        let mut tracker = state.tracker.lock().unwrap();
        let changed = tracker.update_preferences(settings);
        (changed, tracker.snapshot(), state.generation())
    };
    if changed {
        emit_status(app, snapshot, generation);
    }
}

pub fn note_manual_overlay_hide(app: &AppHandle) {
    if let Some(state) = app.try_state::<ChatGptLifecycleState>() {
        state.tracker.lock().unwrap().note_manual_hide();
    }
}

pub fn note_manual_overlay_show(app: &AppHandle) {
    if let Some(state) = app.try_state::<ChatGptLifecycleState>() {
        state.tracker.lock().unwrap().note_manual_show();
    }
}

#[tauri::command]
pub fn get_chatgpt_lifecycle_status(
    app: AppHandle,
    window: tauri::WebviewWindow,
) -> ChatGptDesktopSnapshot {
    let (snapshot, generation) = app
        .try_state::<ChatGptLifecycleState>()
        .map(|state| (current_snapshot(state.inner()), Some(state.generation())))
        .unwrap_or_else(|| {
            (
                ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Unavailable),
                None,
            )
        });
    let window_label = diagnostic_window_label(window.label());
    chatgpt_lifecycle_diagnostic::record_backend(
        "command_invoked",
        Some(snapshot.status),
        generation,
        Some(window_label),
        generation
            .is_none()
            .then_some(LifecycleDiagnosticErrorCategory::SharedStateNotReady),
    );
    chatgpt_lifecycle_diagnostic::record_backend(
        "command_returned_status",
        Some(snapshot.status),
        generation,
        Some(window_label),
        None,
    );
    snapshot
}

#[tauri::command]
pub fn record_chatgpt_lifecycle_frontend_diagnostic(input: FrontendDiagnosticInput) {
    chatgpt_lifecycle_diagnostic::record_frontend(input);
}

#[tauri::command]
pub fn chatgpt_lifecycle_diagnostics_enabled() -> bool {
    chatgpt_lifecycle_diagnostic::enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn stable_start_requires_two_samples_and_shows_once() {
        let mut tracker = LifecycleTracker::new(&settings());
        assert_eq!(tracker.sample(ChatGptDesktopStatus::Running), (false, None));
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::Running),
            (true, Some(LifecycleAction::ShowOverlay))
        );
        assert_eq!(tracker.sample(ChatGptDesktopStatus::Running), (false, None));
    }

    #[test]
    fn exit_requires_grace_samples_and_hides_once() {
        let mut tracker = LifecycleTracker::new(&settings());
        let _ = tracker.sample(ChatGptDesktopStatus::Running);
        let _ = tracker.sample(ChatGptDesktopStatus::Running);
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (false, None)
        );
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (false, None)
        );
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (true, Some(LifecycleAction::HideOverlay))
        );
    }

    #[test]
    fn manual_hide_suppresses_reopen_until_the_next_full_cycle() {
        let mut tracker = LifecycleTracker::new(&settings());
        let _ = tracker.sample(ChatGptDesktopStatus::Running);
        let _ = tracker.sample(ChatGptDesktopStatus::Running);
        tracker.note_manual_hide();
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (false, None)
        );
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (false, None)
        );
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::NotRunning),
            (true, Some(LifecycleAction::HideOverlay))
        );
        assert_eq!(tracker.sample(ChatGptDesktopStatus::Running), (false, None));
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::Running),
            (true, Some(LifecycleAction::ShowOverlay))
        );
    }

    #[test]
    fn disabled_monitoring_never_emits_lifecycle_actions() {
        let mut settings = settings();
        settings.monitor_chatgpt_desktop = false;
        let mut tracker = LifecycleTracker::new(&settings);
        assert_eq!(tracker.sample(ChatGptDesktopStatus::Running), (false, None));
        assert_eq!(
            tracker.snapshot().status,
            ChatGptDesktopStatus::MonitoringDisabled
        );
    }

    #[test]
    fn unavailable_and_unsupported_do_not_hide_the_overlay() {
        let mut tracker = LifecycleTracker::new(&settings());
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::Unavailable),
            (true, None)
        );
        assert_eq!(
            tracker.sample(ChatGptDesktopStatus::Unsupported),
            (true, None)
        );
    }

    #[test]
    fn current_command_snapshot_reads_the_confirmed_managed_state() {
        let state = ChatGptLifecycleState::new(&settings());
        let _ = sample_managed_state(&state, ChatGptDesktopStatus::Running);
        let _ = sample_managed_state(&state, ChatGptDesktopStatus::Running);

        assert_eq!(
            current_snapshot(&state),
            ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Running)
        );
    }

    #[test]
    fn watcher_updates_and_command_reads_share_one_managed_instance() {
        let state = ChatGptLifecycleState::new(&settings());
        let generation = state.generation();
        let _ = sample_managed_state(&state, ChatGptDesktopStatus::Running);
        let _ = sample_managed_state(&state, ChatGptDesktopStatus::Running);

        assert_eq!(state.generation(), generation);
        assert_eq!(
            current_snapshot(&state).status,
            ChatGptDesktopStatus::Running
        );
    }

    #[test]
    fn lifecycle_event_name_and_snapshot_serialization_match_the_frontend_dto() {
        assert_eq!(STATUS_CHANGED_EVENT, "chatgpt-desktop-status-changed");
        assert_eq!(
            serde_json::to_string(&ChatGptDesktopSnapshot::new(
                ChatGptDesktopStatus::NotRunning
            ))
            .unwrap(),
            r#"{"status":"not-running"}"#
        );
    }

    #[test]
    fn observer_failures_are_reduced_to_safe_diagnostic_categories() {
        assert!(matches!(
            observer_error_category(ChatGptDesktopUnavailableReason::ComInitialization),
            LifecycleDiagnosticErrorCategory::ObserverComInitialization
        ));
        assert!(matches!(
            observer_error_category(ChatGptDesktopUnavailableReason::PackageIdentityQuery),
            LifecycleDiagnosticErrorCategory::ObserverPackageIdentityQuery
        ));
        assert!(matches!(
            observer_error_category(ChatGptDesktopUnavailableReason::TopLevelWindowEnumeration),
            LifecycleDiagnosticErrorCategory::ObserverTopLevelWindowEnumeration
        ));
    }
}
