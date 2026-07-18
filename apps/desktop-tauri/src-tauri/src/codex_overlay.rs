//! Detached Codex-only usage overlay.
//!
//! The window is independent from the main surface state machine. Its own
//! visibility preference and monitor-relative geometry are persisted so the
//! tray, Settings, and the generic multi-provider surfaces remain untouched.

use codexbar::settings::{CodexOverlayStartupMode, Settings};
use tauri::{Manager, PhysicalPosition, Position, WebviewUrl};

use crate::geometry_store::{self, StoredMonitorInfo, StoredOverlayGeometry};
use crate::window_positioner::{self, PanelSize, Rect};

pub const CODEX_OVERLAY_LABEL: &str = "codex-overlay";
const CODEX_OVERLAY_WIDTH: f64 = 420.0;
const CODEX_OVERLAY_HEIGHT: f64 = 260.0;

#[derive(Debug, Clone)]
struct OverlayMonitor {
    name: Option<String>,
    work_area: Rect,
    scale_factor: f64,
}

pub fn initial_size() -> (f64, f64) {
    (CODEX_OVERLAY_WIDTH, CODEX_OVERLAY_HEIGHT)
}

pub fn is_visible(app: &tauri::AppHandle) -> bool {
    app.get_webview_window(CODEX_OVERLAY_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(false))
}

pub fn should_show_at_start(settings: &Settings) -> bool {
    if !settings.codex_overlay_has_launched {
        return true;
    }
    match settings.codex_overlay_startup_mode {
        CodexOverlayStartupMode::RememberLast => settings.codex_overlay_last_visible,
        CodexOverlayStartupMode::AlwaysShow => true,
        CodexOverlayStartupMode::AlwaysHide => false,
    }
}

fn remember_visibility(visible: bool) {
    let mut settings = Settings::load();
    settings.codex_overlay_has_launched = true;
    settings.codex_overlay_last_visible = visible;
    if let Err(error) = settings.save() {
        tracing::warn!(target: "codexbar::overlay", %error, "failed to persist overlay visibility");
    }
}

fn valid_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn default_position(monitor: &OverlayMonitor) -> (i32, i32) {
    let panel = PanelSize {
        width: CODEX_OVERLAY_WIDTH as u32,
        height: CODEX_OVERLAY_HEIGHT as u32,
    };
    let scale = valid_scale(monitor.scale_factor);
    let target_x = monitor.work_area.x
        + (monitor.work_area.width as i32 - (panel.width as f64 * scale).round() as i32) / 2;
    let target_y = monitor.work_area.y
        + (monitor.work_area.height as i32 - (panel.height as f64 * scale).round() as i32) / 2;
    window_positioner::clamp_position_to_work_area(
        target_x,
        target_y,
        &monitor.work_area,
        &panel,
        scale,
    )
}

fn same_monitor_name(stored: &StoredMonitorInfo, current: &OverlayMonitor) -> bool {
    stored.name.as_ref().is_some_and(|name| {
        current
            .name
            .as_ref()
            .is_some_and(|current_name| current_name == name)
    })
}

fn same_monitor_bounds(stored: &StoredMonitorInfo, current: &OverlayMonitor) -> bool {
    stored.x == current.work_area.x
        && stored.y == current.work_area.y
        && stored.width == current.work_area.width
        && stored.height == current.work_area.height
}

fn resolve_position(
    stored: Option<&StoredOverlayGeometry>,
    monitors: &[OverlayMonitor],
    primary_index: usize,
) -> Option<(i32, i32)> {
    let primary = monitors.get(primary_index).or_else(|| monitors.first())?;
    let Some(stored) = stored else {
        return Some(default_position(primary));
    };
    if !stored.logical_x.is_finite()
        || !stored.logical_y.is_finite()
        || stored.logical_width == 0
        || stored.logical_height == 0
    {
        return Some(default_position(primary));
    }

    let monitor = monitors
        .iter()
        .find(|monitor| same_monitor_name(&stored.monitor, monitor))
        .or_else(|| {
            monitors
                .iter()
                .find(|monitor| same_monitor_bounds(&stored.monitor, monitor))
        })
        .unwrap_or(primary);
    let scale = valid_scale(monitor.scale_factor);
    let target_x = monitor.work_area.x + (stored.logical_x * scale).round() as i32;
    let target_y = monitor.work_area.y + (stored.logical_y * scale).round() as i32;
    let panel = PanelSize {
        width: stored.logical_width.max(1),
        height: stored.logical_height.max(1),
    };
    Some(window_positioner::clamp_position_to_work_area(
        target_x,
        target_y,
        &monitor.work_area,
        &panel,
        scale,
    ))
}

fn overlay_monitor(monitor: &tauri::Monitor) -> OverlayMonitor {
    let work_area = monitor.work_area();
    OverlayMonitor {
        name: monitor.name().cloned(),
        work_area: Rect {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width,
            height: work_area.size.height,
        },
        scale_factor: monitor.scale_factor(),
    }
}

fn primary_index(monitors: &[tauri::Monitor], primary: Option<&tauri::Monitor>) -> usize {
    primary
        .and_then(|primary| {
            monitors.iter().position(|monitor| {
                monitor.name() == primary.name()
                    && monitor.position() == primary.position()
                    && monitor.size() == primary.size()
            })
        })
        .unwrap_or(0)
}

fn apply_restored_position(window: &tauri::WebviewWindow) {
    let Ok(monitors) = window.available_monitors() else {
        let _ = window.center();
        return;
    };
    let primary = window.primary_monitor().ok().flatten();
    let primary_index = primary_index(&monitors, primary.as_ref());
    let monitors = monitors.iter().map(overlay_monitor).collect::<Vec<_>>();
    let stored = geometry_store::load_overlay(CODEX_OVERLAY_LABEL);
    if let Some((x, y)) = resolve_position(stored.as_ref(), &monitors, primary_index) {
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    } else {
        let _ = window.center();
    }
}

trait OverlayWindowGeometry<R: tauri::Runtime> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>>;
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>>;
    fn available_monitors(&self) -> tauri::Result<Vec<tauri::Monitor>>;
    fn primary_monitor(&self) -> tauri::Result<Option<tauri::Monitor>>;
}

impl<R: tauri::Runtime> OverlayWindowGeometry<R> for tauri::WebviewWindow<R> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>> {
        tauri::WebviewWindow::outer_position(self)
    }
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>> {
        tauri::WebviewWindow::outer_size(self)
    }
    fn available_monitors(&self) -> tauri::Result<Vec<tauri::Monitor>> {
        tauri::WebviewWindow::available_monitors(self)
    }
    fn primary_monitor(&self) -> tauri::Result<Option<tauri::Monitor>> {
        tauri::WebviewWindow::primary_monitor(self)
    }
}

impl<R: tauri::Runtime> OverlayWindowGeometry<R> for tauri::Window<R> {
    fn outer_position(&self) -> tauri::Result<tauri::PhysicalPosition<i32>> {
        tauri::Window::outer_position(self)
    }
    fn outer_size(&self) -> tauri::Result<tauri::PhysicalSize<u32>> {
        tauri::Window::outer_size(self)
    }
    fn available_monitors(&self) -> tauri::Result<Vec<tauri::Monitor>> {
        tauri::Window::available_monitors(self)
    }
    fn primary_monitor(&self) -> tauri::Result<Option<tauri::Monitor>> {
        tauri::Window::primary_monitor(self)
    }
}

fn remember_geometry<R: tauri::Runtime, W: OverlayWindowGeometry<R>>(window: &W) {
    let (Ok(position), Ok(size), Ok(monitors)) = (
        window.outer_position(),
        window.outer_size(),
        window.available_monitors(),
    ) else {
        return;
    };
    let primary = window.primary_monitor().ok().flatten();
    let primary_index = primary_index(&monitors, primary.as_ref());
    let center_x = position.x + size.width as i32 / 2;
    let center_y = position.y + size.height as i32 / 2;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let area = monitor.work_area();
            center_x >= area.position.x
                && center_x < area.position.x + area.size.width as i32
                && center_y >= area.position.y
                && center_y < area.position.y + area.size.height as i32
        })
        .or_else(|| monitors.get(primary_index));
    let Some(monitor) = monitor else {
        return;
    };
    let area = monitor.work_area();
    let scale = valid_scale(monitor.scale_factor());
    geometry_store::save_overlay(
        CODEX_OVERLAY_LABEL,
        StoredOverlayGeometry {
            monitor: StoredMonitorInfo {
                name: monitor.name().cloned(),
                x: area.position.x,
                y: area.position.y,
                width: area.size.width,
                height: area.size.height,
                scale_factor: scale,
            },
            logical_x: (position.x - area.position.x) as f64 / scale,
            logical_y: (position.y - area.position.y) as f64 / scale,
            logical_width: (size.width as f64 / scale).round().max(1.0) as u32,
            logical_height: (size.height as f64 / scale).round().max(1.0) as u32,
        },
    );
}

fn show_inner(app: &tauri::AppHandle, focus: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(CODEX_OVERLAY_LABEL) {
        window.show().map_err(|error| error.to_string())?;
        if focus {
            window
                .set_always_on_top(true)
                .map_err(|error| error.to_string())?;
            window.set_focus().map_err(|error| error.to_string())?;
        }
        remember_visibility(true);
        crate::tray_bridge::rebuild_tray_menu(app);
        return Ok(());
    }

    let (width, height) = initial_size();
    let url = WebviewUrl::App("index.html?window=codex-overlay".into());
    let builder = tauri::WebviewWindowBuilder::new(app, CODEX_OVERLAY_LABEL, url)
        .title("QuackQuota")
        .inner_size(width, height)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true);

    #[cfg(windows)]
    let builder = builder.transparent(true);

    let window = builder
        .icon(crate::app_icon::image()?)
        .map_err(|error| error.to_string())?
        .background_color(tauri::utils::config::Color(0, 0, 0, 0))
        .visible(false)
        .build()
        .map_err(|error| error.to_string())?;

    apply_restored_position(&window);
    window.show().map_err(|error| error.to_string())?;
    if focus {
        window.set_focus().map_err(|error| error.to_string())?;
    }
    remember_visibility(true);
    crate::tray_bridge::rebuild_tray_menu(app);
    Ok(())
}

/// Open the Codex overlay, or bring the existing window to the front.
pub fn show(app: &tauri::AppHandle) -> Result<(), String> {
    crate::chatgpt_lifecycle::note_manual_overlay_show(app);
    show_inner(app, true)
}

/// Lifecycle-triggered display preserves the user's focus and existing window
/// attributes. It is not a tray/manual interaction.
pub fn show_for_chatgpt_lifecycle(app: &tauri::AppHandle) -> Result<(), String> {
    show_inner(app, false)
}

pub fn toggle(app: &tauri::AppHandle) -> Result<(), String> {
    if is_visible(app) {
        hide(app)
    } else {
        show(app)
    }
}

fn hide_inner(app: &tauri::AppHandle, manual: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(CODEX_OVERLAY_LABEL) {
        remember_geometry(&window);
        window.hide().map_err(|error| error.to_string())?;
    }
    if manual {
        crate::chatgpt_lifecycle::note_manual_overlay_hide(app);
    }
    remember_visibility(false);
    crate::tray_bridge::rebuild_tray_menu(app);
    Ok(())
}

pub fn hide(app: &tauri::AppHandle) -> Result<(), String> {
    hide_inner(app, true)
}

pub fn hide_for_chatgpt_lifecycle(app: &tauri::AppHandle) -> Result<(), String> {
    hide_inner(app, false)
}

pub fn handle_window_event(window: &tauri::Window, event: &tauri::WindowEvent) -> bool {
    if window.label() != CODEX_OVERLAY_LABEL {
        return false;
    }
    match event {
        tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
            remember_geometry(window);
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            remember_geometry(window);
            api.prevent_close();
            let _ = window.hide();
            crate::chatgpt_lifecycle::note_manual_overlay_hide(window.app_handle());
            remember_visibility(false);
            crate::tray_bridge::rebuild_tray_menu(window.app_handle());
        }
        _ => {}
    }
    true
}

#[tauri::command]
pub async fn show_codex_overlay(app: tauri::AppHandle) -> Result<(), String> {
    show(&app)
}

#[tauri::command]
pub async fn hide_codex_overlay(app: tauri::AppHandle) -> Result<(), String> {
    hide(&app)
}

#[tauri::command]
pub async fn reset_codex_overlay_position(app: tauri::AppHandle) -> Result<(), String> {
    geometry_store::remove_overlay(CODEX_OVERLAY_LABEL);
    if let Some(window) = app.get_webview_window(CODEX_OVERLAY_LABEL) {
        apply_restored_position(&window);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, y: i32, width: u32, height: u32, scale: f64) -> OverlayMonitor {
        OverlayMonitor {
            name: Some(name.to_string()),
            work_area: Rect {
                x,
                y,
                width,
                height,
            },
            scale_factor: scale,
        }
    }

    fn stored(name: &str, logical_x: f64, logical_y: f64, scale: f64) -> StoredOverlayGeometry {
        StoredOverlayGeometry {
            monitor: StoredMonitorInfo {
                name: Some(name.to_string()),
                x: 0,
                y: 0,
                width: 1920,
                height: 1040,
                scale_factor: scale,
            },
            logical_x,
            logical_y,
            logical_width: 420,
            logical_height: 260,
        }
    }

    #[test]
    fn overlay_initial_size_matches_contract() {
        assert_eq!(initial_size(), (420.0, 260.0));
    }

    #[test]
    fn startup_mode_preserves_first_launch_and_explicit_policy() {
        let mut settings = Settings {
            codex_overlay_has_launched: false,
            codex_overlay_startup_mode: CodexOverlayStartupMode::AlwaysHide,
            ..Settings::default()
        };
        assert!(should_show_at_start(&settings));

        settings.codex_overlay_has_launched = true;
        assert!(!should_show_at_start(&settings));
        settings.codex_overlay_startup_mode = CodexOverlayStartupMode::AlwaysShow;
        assert!(should_show_at_start(&settings));
        settings.codex_overlay_startup_mode = CodexOverlayStartupMode::RememberLast;
        settings.codex_overlay_last_visible = false;
        assert!(!should_show_at_start(&settings));
    }

    #[test]
    fn startup_mode_remember_last_state_matrix() {
        let mut settings = Settings {
            codex_overlay_has_launched: true,
            codex_overlay_startup_mode: CodexOverlayStartupMode::RememberLast,
            codex_overlay_last_visible: true,
            ..Settings::default()
        };
        assert!(should_show_at_start(&settings));

        settings.codex_overlay_last_visible = false;
        assert!(!should_show_at_start(&settings));

        settings.codex_overlay_startup_mode = CodexOverlayStartupMode::AlwaysShow;
        assert!(should_show_at_start(&settings));

        settings.codex_overlay_startup_mode = CodexOverlayStartupMode::AlwaysHide;
        assert!(!should_show_at_start(&settings));
    }

    #[test]
    fn restores_negative_coordinates_on_the_saved_monitor() {
        let monitors = [
            monitor("primary", 0, 0, 1920, 1040, 1.0),
            monitor("left", -1600, 0, 1600, 900, 1.0),
        ];
        let saved = stored("left", 100.0, 80.0, 1.0);
        assert_eq!(
            resolve_position(Some(&saved), &monitors, 0),
            Some((-1500, 80))
        );
    }

    #[test]
    fn converts_logical_position_for_changed_dpi() {
        let monitors = [monitor("primary", 0, 0, 2560, 1400, 1.5)];
        let saved = stored("primary", 100.0, 80.0, 1.0);
        assert_eq!(
            resolve_position(Some(&saved), &monitors, 0),
            Some((150, 120))
        );
    }

    #[test]
    fn removed_monitor_falls_back_to_primary_safe_area() {
        let monitors = [monitor("primary", 0, 0, 1920, 1040, 1.0)];
        let saved = stored("removed", 1500.0, 900.0, 1.0);
        assert_eq!(
            resolve_position(Some(&saved), &monitors, 0),
            Some((1492, 772))
        );
    }

    #[test]
    fn corrupt_geometry_falls_back_to_visible_center() {
        let monitors = [monitor("primary", 0, 0, 1920, 1040, 1.0)];
        let mut saved = stored("primary", f64::NAN, 80.0, 1.0);
        saved.logical_width = 0;
        assert_eq!(
            resolve_position(Some(&saved), &monitors, 0),
            Some((750, 390))
        );
    }

    #[test]
    fn edge_coordinates_are_clamped_inside_work_area() {
        let monitors = [monitor("primary", 0, 0, 1920, 1040, 1.0)];
        let saved = stored("primary", -500.0, -500.0, 1.0);
        assert_eq!(resolve_position(Some(&saved), &monitors, 0), Some((8, 8)));
    }
}
