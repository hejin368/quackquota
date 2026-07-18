//! Windows ChatGPT desktop lifecycle recognition.
//!
//! This module deliberately recognizes only the installed Windows desktop
//! package. It does not inspect browsers, web tabs, command lines, or chat
//! content. The package version directory is resolved from Windows metadata
//! every time an identity is refreshed rather than stored or hard-coded.

use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::cell::Cell;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::{Mutex, OnceLock};
#[cfg(windows)]
use std::time::{Duration, Instant};

const SUPPORTED_PACKAGE_NAMES: &[&str] = &["OpenAI.Codex", "OpenAI.ChatGPT"];
const CHATGPT_EXECUTABLE: &str = "ChatGPT.exe";
#[cfg(windows)]
const IDENTITY_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[cfg(windows)]
thread_local! {
    static COM_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatGptDesktopStatus {
    MonitoringDisabled,
    Detecting,
    Running,
    NotRunning,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatGptDesktopSnapshot {
    pub status: ChatGptDesktopStatus,
}

impl ChatGptDesktopSnapshot {
    pub const fn new(status: ChatGptDesktopStatus) -> Self {
        Self { status }
    }
}

/// Safe diagnostic category for an unavailable Windows desktop observation.
/// It deliberately contains no HRESULT, path, package version, PID, or window
/// metadata so it can be used by debug-only lifecycle diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatGptDesktopUnavailableReason {
    ComInitialization,
    PackageIdentityQuery,
    TopLevelWindowEnumeration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatGptDesktopObservation {
    pub snapshot: ChatGptDesktopSnapshot,
    pub unavailable_reason: Option<ChatGptDesktopUnavailableReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatGptPackageIdentity {
    package_family_name: String,
    app_user_model_id: String,
    install_root: PathBuf,
    executable_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TopLevelWindowEvidence {
    visible: bool,
    has_title: bool,
    package_family_name: Option<String>,
    app_user_model_id: Option<String>,
    executable_path: Option<PathBuf>,
}

#[cfg(windows)]
#[derive(Debug, Clone)]
struct CachedIdentity {
    identity: Option<ChatGptPackageIdentity>,
    refreshed_at: Instant,
}

#[cfg(windows)]
fn ensure_com_initialized() -> Result<(), ()> {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    COM_INITIALIZED.with(|initialized| {
        if initialized.get() {
            return Ok(());
        }
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|_| ())?;
        initialized.set(true);
        Ok(())
    })
}

fn path_file_name_matches(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn is_valid_main_window(
    identity: &ChatGptPackageIdentity,
    evidence: &TopLevelWindowEvidence,
) -> bool {
    evidence.visible
        && evidence.has_title
        && evidence
            .package_family_name
            .as_deref()
            .is_some_and(|value| value == identity.package_family_name)
        && evidence
            .app_user_model_id
            .as_deref()
            .is_some_and(|value| value == identity.app_user_model_id)
        && evidence.executable_path.as_ref().is_some_and(|path| {
            path.starts_with(&identity.install_root)
                && path_file_name_matches(path, &identity.executable_name)
        })
}

/// Read the current desktop state. It never reports raw paths, process IDs,
/// package versions, window titles, or any authentication information.
pub fn observe() -> ChatGptDesktopSnapshot {
    observe_with_diagnostic().snapshot
}

/// Read the current desktop state together with a safe, bounded unavailable
/// category for local diagnostics. The category cannot expose personal or
/// authentication data and is not sent to the frontend.
pub fn observe_with_diagnostic() -> ChatGptDesktopObservation {
    #[cfg(windows)]
    {
        match cached_identity() {
            Ok(Some(identity)) => match enumerate_top_level_windows(&identity) {
                Ok(has_main_window) => ChatGptDesktopObservation {
                    snapshot: ChatGptDesktopSnapshot::new(if has_main_window {
                        ChatGptDesktopStatus::Running
                    } else {
                        ChatGptDesktopStatus::NotRunning
                    }),
                    unavailable_reason: None,
                },
                Err(reason) => ChatGptDesktopObservation {
                    snapshot: ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Unavailable),
                    unavailable_reason: Some(reason),
                },
            },
            Ok(None) => ChatGptDesktopObservation {
                snapshot: ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Unsupported),
                unavailable_reason: None,
            },
            Err(reason) => ChatGptDesktopObservation {
                snapshot: ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Unavailable),
                unavailable_reason: Some(reason),
            },
        }
    }

    #[cfg(not(windows))]
    {
        ChatGptDesktopObservation {
            snapshot: ChatGptDesktopSnapshot::new(ChatGptDesktopStatus::Unsupported),
            unavailable_reason: None,
        }
    }
}

#[cfg(windows)]
fn cached_identity() -> Result<Option<ChatGptPackageIdentity>, ChatGptDesktopUnavailableReason> {
    static CACHE: OnceLock<Mutex<Option<CachedIdentity>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    let mut entry = cache
        .lock()
        .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
    if let Some(cached) = entry.as_ref()
        && cached.refreshed_at.elapsed() < IDENTITY_REFRESH_INTERVAL
    {
        return Ok(cached.identity.clone());
    }

    let identity = resolve_identity()?;
    *entry = Some(CachedIdentity {
        identity: identity.clone(),
        refreshed_at: Instant::now(),
    });
    Ok(identity)
}

#[cfg(windows)]
fn resolve_identity() -> Result<Option<ChatGptPackageIdentity>, ChatGptDesktopUnavailableReason> {
    use windows::Management::Deployment::PackageManager;

    ensure_com_initialized().map_err(|_| ChatGptDesktopUnavailableReason::ComInitialization)?;
    let packages = PackageManager::new()
        .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
        .FindPackages()
        .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
    let packages = packages
        .First()
        .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
    let mut candidates = Vec::new();

    while packages
        .HasCurrent()
        .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
    {
        let package = packages
            .Current()
            .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
        let id = package
            .Id()
            .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
        let name = id
            .Name()
            .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
            .to_string();
        if SUPPORTED_PACKAGE_NAMES
            .iter()
            .any(|candidate| *candidate == name)
        {
            let family_name = id
                .FamilyName()
                .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
                .to_string();
            let root = package
                .InstalledLocation()
                .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
                .Path()
                .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?
                .to_string();
            if let Some((app_id, executable_name)) =
                manifest_chatgpt_application(Path::new(&root).join("AppxManifest.xml").as_path())
            {
                candidates.push(ChatGptPackageIdentity {
                    app_user_model_id: format!("{family_name}!{app_id}"),
                    package_family_name: family_name,
                    install_root: PathBuf::from(root),
                    executable_name,
                });
            }
        }
        packages
            .MoveNext()
            .map_err(|_| ChatGptDesktopUnavailableReason::PackageIdentityQuery)?;
    }

    Ok((candidates.len() == 1).then(|| candidates.remove(0)))
}

#[cfg(windows)]
fn manifest_chatgpt_application(path: &Path) -> Option<(String, String)> {
    let manifest = std::fs::read_to_string(path).ok()?;
    let mut cursor = manifest.as_str();
    while let Some(start) = cursor.find("<Application ") {
        let after_start = &cursor[start..];
        let end = after_start.find('>')?;
        let element = &after_start[..end];
        if let Some(executable) = xml_attribute(element, "Executable")
            && path_file_name_matches(Path::new(executable), CHATGPT_EXECUTABLE)
            && let Some(id) = xml_attribute(element, "Id")
        {
            return Some((id.to_string(), CHATGPT_EXECUTABLE.to_string()));
        }
        cursor = &after_start[end + 1..];
    }
    None
}

#[cfg(windows)]
fn xml_attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=\"");
    let start = element.find(&prefix)? + prefix.len();
    let rest = &element[start..];
    Some(&rest[..rest.find('"')?])
}

fn enumeration_completed_or_found_match(found_match: bool, enumeration_completed: bool) -> bool {
    found_match || enumeration_completed
}

#[cfg(windows)]
fn enumerate_top_level_windows(
    identity: &ChatGptPackageIdentity,
) -> Result<bool, ChatGptDesktopUnavailableReason> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct Search<'a> {
        identity: &'a ChatGptPackageIdentity,
        found: bool,
    }

    unsafe extern "system" fn inspect_window(hwnd: HWND, data: LPARAM) -> BOOL {
        let search = unsafe { &mut *(data.0 as *mut Search<'_>) };
        if !unsafe { IsWindowVisible(hwnd).as_bool() } || unsafe { GetWindowTextLengthW(hwnd) } <= 0
        {
            return BOOL(1);
        }

        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        if process_id != 0
            && let Some(evidence) = process_window_evidence(process_id)
            && is_valid_main_window(search.identity, &evidence)
        {
            search.found = true;
            return BOOL(0);
        }
        BOOL(1)
    }

    let mut search = Search {
        identity,
        found: false,
    };
    let enumeration_completed =
        unsafe { EnumWindows(Some(inspect_window), LPARAM(&mut search as *mut _ as isize)) }
            .is_ok();
    if !enumeration_completed_or_found_match(search.found, enumeration_completed) {
        return Err(ChatGptDesktopUnavailableReason::TopLevelWindowEnumeration);
    }
    Ok(search.found)
}

#[cfg(windows)]
fn process_window_evidence(process_id: u32) -> Option<TopLevelWindowEvidence> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::Packaging::Appx::{
        GetApplicationUserModelId, GetPackageFamilyName,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::core::PWSTR;

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
    let result = (|| {
        let package_family_name = process_string(|length, buffer| unsafe {
            GetPackageFamilyName(handle, length, buffer)
        })?;
        let app_user_model_id = process_string(|length, buffer| unsafe {
            GetApplicationUserModelId(handle, length, buffer)
        })?;
        let mut image = vec![0u16; 32_768];
        let mut length = image.len() as u32;
        unsafe {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(image.as_mut_ptr()),
                &mut length,
            )
            .ok()?;
        }
        Some(TopLevelWindowEvidence {
            visible: true,
            has_title: true,
            package_family_name: Some(package_family_name),
            app_user_model_id: Some(app_user_model_id),
            executable_path: Some(PathBuf::from(String::from_utf16_lossy(
                &image[..length as usize],
            ))),
        })
    })();
    let _ = unsafe { CloseHandle(handle) };
    result
}

#[cfg(windows)]
fn process_string(
    call: impl Fn(&mut u32, windows::core::PWSTR) -> windows::Win32::Foundation::WIN32_ERROR,
) -> Option<String> {
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::core::PWSTR;

    let mut length = 0;
    let first = call(&mut length, PWSTR::null());
    if first != ERROR_INSUFFICIENT_BUFFER || length == 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    let result = call(&mut length, PWSTR(buffer.as_mut_ptr()));
    result.is_ok().then(|| {
        String::from_utf16_lossy(&buffer[..length as usize])
            .trim_end_matches('\0')
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ChatGptPackageIdentity {
        ChatGptPackageIdentity {
            package_family_name: "OpenAI.Codex_family".to_string(),
            app_user_model_id: "OpenAI.Codex_family!App".to_string(),
            install_root: PathBuf::from("C:/WindowsApps/OpenAI.Codex_dynamic"),
            executable_name: "ChatGPT.exe".to_string(),
        }
    }

    fn evidence() -> TopLevelWindowEvidence {
        TopLevelWindowEvidence {
            visible: true,
            has_title: true,
            package_family_name: Some("OpenAI.Codex_family".to_string()),
            app_user_model_id: Some("OpenAI.Codex_family!App".to_string()),
            executable_path: Some(PathBuf::from(
                "C:/WindowsApps/OpenAI.Codex_dynamic/app/ChatGPT.exe",
            )),
        }
    }

    #[test]
    fn accepts_a_current_chatgpt_package_with_a_dynamic_root() {
        assert!(is_valid_main_window(&identity(), &evidence()));
    }

    #[test]
    fn rejects_helpers_and_codex_app_server_processes() {
        let mut helper = evidence();
        helper.executable_path = Some(PathBuf::from(
            "C:/WindowsApps/OpenAI.Codex_dynamic/resources/codex.exe",
        ));
        assert!(!is_valid_main_window(&identity(), &helper));
    }

    #[test]
    fn rejects_background_processes_without_a_main_window() {
        let mut background = evidence();
        background.visible = false;
        assert!(!is_valid_main_window(&identity(), &background));

        background.visible = true;
        background.has_title = false;
        assert!(!is_valid_main_window(&identity(), &background));
    }

    #[test]
    fn rejects_an_unrelated_package_or_aumid() {
        let mut other = evidence();
        other.package_family_name = Some("Other.App_family".to_string());
        assert!(!is_valid_main_window(&identity(), &other));

        other = evidence();
        other.app_user_model_id = Some("OpenAI.Codex_family!Other".to_string());
        assert!(!is_valid_main_window(&identity(), &other));
    }

    #[test]
    fn a_match_that_stops_enum_windows_early_is_successful() {
        assert!(enumeration_completed_or_found_match(true, false));
        assert!(enumeration_completed_or_found_match(false, true));
        assert!(!enumeration_completed_or_found_match(false, false));
    }

    #[cfg(windows)]
    #[test]
    fn manifest_parser_skips_the_applications_container_element() {
        let path = std::env::temp_dir().join(format!(
            "quackquota-chatgpt-manifest-test-{}.xml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"<Package><Applications><Application Id="App" Executable="ChatGPT.exe" /></Applications></Package>"#,
        )
        .unwrap();

        assert_eq!(
            manifest_chatgpt_application(&path),
            Some(("App".to_string(), "ChatGPT.exe".to_string()))
        );
        let _ = std::fs::remove_file(path);
    }
}
