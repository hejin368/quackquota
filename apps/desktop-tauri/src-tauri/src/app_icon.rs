//! Single runtime source for the Windows application, window, and tray icon.
//!
//! `rust/icons/icon.png` is the editable master. `rust/icons/icon.ico` is a
//! generated bundle artifact used by Windows executable resources.

use tauri::image::Image;

#[cfg(test)]
pub const APP_ICON_SOURCE_PATH: &str = "rust/icons/icon.png";
const APP_ICON_PNG: &[u8] = include_bytes!("../../../../rust/icons/icon.png");
#[cfg(test)]
const APP_ICON_ICO: &[u8] = include_bytes!("../../../../rust/icons/icon.ico");

pub fn image() -> Result<Image<'static>, String> {
    Image::from_bytes(APP_ICON_PNG).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_icon_uses_the_canonical_blue_source() {
        assert_eq!(APP_ICON_SOURCE_PATH, "rust/icons/icon.png");
        assert_eq!(&APP_ICON_PNG[..8], b"\x89PNG\r\n\x1a\n");
        let icon = image().expect("decode canonical app icon");
        assert_eq!((icon.width(), icon.height()), (1024, 1024));
    }

    #[test]
    fn tauri_bundle_uses_only_the_canonical_png_and_derived_ico() {
        let config = include_str!("../tauri.conf.json");
        let about_tab = include_str!("../../src/surfaces/settings/tabs/AboutTab.tsx");
        assert!(config.contains("../../../rust/icons/icon.png"));
        assert!(config.contains("../../../rust/icons/icon.ico"));
        assert!(!config.contains("about-icon.png"));
        assert!(!config.contains("CodexBar-app-icon.png"));
        assert!(about_tab.contains("../../../../../../rust/icons/icon.png"));
        assert!(!about_tab.contains("assets/codexbar-icon.png"));
    }

    #[test]
    fn windows_ico_contains_every_supported_shell_size() {
        assert_eq!(&APP_ICON_ICO[..4], &[0, 0, 1, 0]);
        let count = u16::from_le_bytes([APP_ICON_ICO[4], APP_ICON_ICO[5]]) as usize;
        let sizes = (0..count)
            .map(|index| match APP_ICON_ICO[6 + index * 16] {
                0 => 256,
                size => u16::from(size),
            })
            .collect::<Vec<_>>();
        assert_eq!(sizes, vec![16, 20, 24, 32, 48, 64, 128, 256]);
    }

    #[test]
    fn tray_refresh_never_replaces_the_canonical_icon() {
        let tray_bridge = include_str!("tray_bridge.rs");
        assert!(!tray_bridge.contains("tray.set_icon"));
        assert!(!tray_bridge.contains("render_bar_icon_rgba"));
        assert!(!tray_bridge.contains("render_percent_icon_rgba"));
    }
}
