use codexbar::locale::{self, LocaleKey};
use codexbar::settings::Language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrayMenuEntry {
    pub(crate) id: Option<String>,
    pub(crate) label: String,
    pub(crate) children: Vec<Self>,
    pub(crate) is_separator: bool,
    pub(crate) disabled: bool,
    pub(crate) checked: Option<bool>,
}

impl TrayMenuEntry {
    fn item(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            label: label.into(),
            children: Vec::new(),
            is_separator: false,
            disabled: false,
            checked: None,
        }
    }

    fn separator() -> Self {
        Self {
            id: None,
            label: String::new(),
            children: Vec::new(),
            is_separator: true,
            disabled: false,
            checked: None,
        }
    }

    fn path_segment(&self) -> Option<String> {
        if self.is_separator {
            return None;
        }
        Some(
            self.id
                .clone()
                .unwrap_or_else(|| self.label.to_ascii_lowercase().replace(' ', "_")),
        )
    }
}

pub(crate) fn build_tray_menu_with(overlay_visible: bool, lang: Language) -> Vec<TrayMenuEntry> {
    let text = |key| locale::get_text(lang, key);
    vec![
        TrayMenuEntry::item(
            "toggle_codex_overlay",
            text(if overlay_visible {
                LocaleKey::TrayHideCodexOverlay
            } else {
                LocaleKey::TrayShowCodexOverlay
            }),
        ),
        TrayMenuEntry::separator(),
        TrayMenuEntry::item("settings", text(LocaleKey::TraySettings)),
        TrayMenuEntry::item("about", text(LocaleKey::MenuAbout)),
        TrayMenuEntry::separator(),
        TrayMenuEntry::item("quit", text(LocaleKey::MenuQuit)),
    ]
}

pub(crate) fn proof_menu_items(entries: &[TrayMenuEntry], menu_path: &str) -> Option<Vec<String>> {
    proof_menu_entries(entries, menu_path).map(|visible_entries| {
        visible_entries
            .iter()
            .filter(|entry| !entry.is_separator)
            .map(|entry| entry.label.clone())
            .collect()
    })
}

pub(crate) fn proof_menu_context_for_item(
    entries: &[TrayMenuEntry],
    item_id: &str,
) -> Option<(String, Vec<String>)> {
    proof_menu_context_for_item_inner(entries, item_id, "tray")
}

fn proof_menu_context_for_item_inner(
    entries: &[TrayMenuEntry],
    item_id: &str,
    menu_path: &str,
) -> Option<(String, Vec<String>)> {
    for entry in entries {
        if entry.is_separator {
            continue;
        }
        if entry.id.as_deref() == Some(item_id) {
            return proof_menu_items(entries, menu_path)
                .map(|items| (menu_path.to_string(), items));
        }
        if entry.children.is_empty() {
            continue;
        }
        let next_path = format!("{menu_path}/{}", entry.path_segment()?);
        if let Some(context) =
            proof_menu_context_for_item_inner(&entry.children, item_id, &next_path)
        {
            return Some(context);
        }
    }
    None
}

fn proof_menu_entries<'a>(
    entries: &'a [TrayMenuEntry],
    menu_path: &str,
) -> Option<&'a [TrayMenuEntry]> {
    let mut segments = menu_path.split('/');
    if segments.next()? != "tray" {
        return None;
    }

    let mut current = entries;
    for segment in segments {
        let submenu = current.iter().find(|entry| {
            !entry.is_separator
                && !entry.children.is_empty()
                && entry.path_segment().as_deref() == Some(segment)
        })?;
        current = &submenu.children;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(menu: &[TrayMenuEntry]) -> Vec<&str> {
        menu.iter()
            .filter_map(|entry| entry.id.as_deref())
            .collect()
    }

    #[test]
    fn public_menu_contains_only_overlay_settings_about_and_quit() {
        let menu = build_tray_menu_with(false, Language::English);
        assert_eq!(
            ids(&menu),
            vec!["toggle_codex_overlay", "settings", "about", "quit"]
        );
        assert_eq!(
            proof_menu_items(&menu, "tray").unwrap(),
            vec!["Show QuackQuota", "Settings...", "About QuackQuota", "Quit"]
        );
    }

    #[test]
    fn public_menu_has_no_dashboard_provider_refresh_or_update_entries() {
        let menu = build_tray_menu_with(false, Language::English);
        for removed in [
            "refresh",
            "pop_out",
            "show_panel",
            "providers",
            "toggle_float_bar",
            "check_for_updates",
        ] {
            assert!(!ids(&menu).contains(&removed), "unexpected item: {removed}");
        }
    }

    #[test]
    fn overlay_menu_label_tracks_visibility() {
        let shown = build_tray_menu_with(false, Language::English);
        let hidden = build_tray_menu_with(true, Language::English);
        assert_eq!(shown[0].label, "Show QuackQuota");
        assert_eq!(hidden[0].label, "Hide QuackQuota");
    }

    #[test]
    fn public_menu_labels_follow_language() {
        let menu = build_tray_menu_with(false, Language::Japanese);
        let items = proof_menu_items(&menu, "tray").unwrap();
        assert!(items.iter().any(|item| item == "QuackQuota を表示"));
        assert!(items.iter().any(|item| item == "設定..."));
        assert!(items.iter().any(|item| item == "終了"));
        assert!(!items.iter().any(|item| item == "Show QuackQuota"));
    }

    #[test]
    fn about_context_is_the_root_tray_menu() {
        let menu = build_tray_menu_with(false, Language::English);
        let (path, items) = proof_menu_context_for_item(&menu, "about").unwrap();
        assert_eq!(path, "tray");
        assert_eq!(items.len(), 4);
    }
}
