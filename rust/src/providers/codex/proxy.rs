//! Codex-only proxy selection and connection diagnostics.
//!
//! The resolved URL is intentionally kept inside the Rust process. Frontend
//! status DTOs expose only the source and a redacted scheme marker.

use crate::settings::Settings;
use reqwest::{ClientBuilder, Proxy, Url};
use serde::Serialize;
use std::ffi::OsString;
use std::time::Duration;
use tokio::process::Command;

const CONNECTION_TEST_URL: &str = "https://chatgpt.com/";
const CONNECTION_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const PROXY_SELECTION_ENV_KEYS: [&str; 4] =
    ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"];
const PROXY_CLEAR_ENV_KEYS: [&str; 6] = [
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "ALL_PROXY",
    "all_proxy",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexProxySource {
    Manual,
    Environment,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexConnectionErrorKind {
    DnsOrNetwork,
    ProxyConnection,
    ProxyConfiguration,
    NotLoggedIn,
    AppServerUnavailable,
    ServiceError,
    InvalidData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexProxyStatus {
    pub source: CodexProxySource,
    pub redacted_endpoint: Option<String>,
    pub error_kind: Option<CodexConnectionErrorKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexProxyTestResult {
    pub ok: bool,
    pub source: CodexProxySource,
    pub redacted_endpoint: Option<String>,
    pub error_kind: Option<CodexConnectionErrorKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexProxyConfigError {
    InvalidUrl,
    UnsupportedScheme,
    CredentialsNotAllowed,
    MissingHost,
}

#[derive(Debug, Clone)]
pub struct ResolvedCodexProxy {
    source: CodexProxySource,
    url: Option<Url>,
}

impl ResolvedCodexProxy {
    pub fn source(&self) -> CodexProxySource {
        self.source
    }

    pub fn is_active(&self) -> bool {
        self.url.is_some()
    }

    pub fn redacted_endpoint(&self) -> Option<String> {
        self.url
            .as_ref()
            .map(|url| format!("{}://***", url.scheme()))
    }

    pub fn configure_http_client(
        &self,
        builder: ClientBuilder,
    ) -> Result<ClientBuilder, CodexProxyConfigError> {
        let builder = builder.no_proxy();
        match &self.url {
            Some(url) => Proxy::all(url.as_str())
                .map(|proxy| builder.proxy(proxy))
                .map_err(|_| CodexProxyConfigError::InvalidUrl),
            None => Ok(builder),
        }
    }

    /// Apply the exact selected policy to App Server. Manual proxy selection
    /// removes NO_PROXY so a host-specific bypass cannot defeat its priority.
    pub fn apply_to_app_server(&self, command: &mut Command) {
        match &self.url {
            Some(url) => {
                command.env("HTTP_PROXY", url.as_str());
                command.env("HTTPS_PROXY", url.as_str());
                // Environment keys are case-insensitive on Windows: removing
                // the lowercase spelling after setting uppercase would remove
                // the selected value itself.
                #[cfg(not(windows))]
                {
                    command.env_remove("http_proxy");
                    command.env_remove("https_proxy");
                }
                command.env_remove("ALL_PROXY");
                command.env_remove("all_proxy");
                if self.source == CodexProxySource::Manual {
                    command.env_remove("NO_PROXY");
                    command.env_remove("no_proxy");
                }
            }
            None => {
                for key in PROXY_CLEAR_ENV_KEYS {
                    command.env_remove(key);
                }
                command.env_remove("NO_PROXY");
                command.env_remove("no_proxy");
            }
        }
    }
}

fn parse_proxy_url(raw: &str) -> Result<Url, CodexProxyConfigError> {
    let url = Url::parse(raw.trim()).map_err(|_| CodexProxyConfigError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(CodexProxyConfigError::UnsupportedScheme);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(CodexProxyConfigError::CredentialsNotAllowed);
    }
    if url.host_str().is_none() {
        return Err(CodexProxyConfigError::MissingHost);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(CodexProxyConfigError::InvalidUrl);
    }
    Ok(url)
}

fn resolve_with_lookup<F>(
    manual_proxy: &str,
    use_environment: bool,
    mut lookup: F,
) -> Result<ResolvedCodexProxy, CodexProxyConfigError>
where
    F: FnMut(&str) -> Option<OsString>,
{
    if !manual_proxy.trim().is_empty() {
        return Ok(ResolvedCodexProxy {
            source: CodexProxySource::Manual,
            url: Some(parse_proxy_url(manual_proxy)?),
        });
    }

    if use_environment {
        for key in PROXY_SELECTION_ENV_KEYS {
            let Some(value) = lookup(key) else {
                continue;
            };
            let value = value.to_string_lossy();
            if value.trim().is_empty() {
                continue;
            }
            return Ok(ResolvedCodexProxy {
                source: CodexProxySource::Environment,
                url: Some(parse_proxy_url(&value)?),
            });
        }
    }

    Ok(ResolvedCodexProxy {
        source: CodexProxySource::Direct,
        url: None,
    })
}

pub fn resolve_codex_proxy() -> Result<ResolvedCodexProxy, CodexProxyConfigError> {
    let settings = Settings::load();
    resolve_with_lookup(
        &settings.codex_manual_proxy,
        settings.codex_proxy_use_environment,
        |key| std::env::var_os(key),
    )
}

pub fn validate_manual_proxy(value: &str) -> Result<(), CodexProxyConfigError> {
    if value.trim().is_empty() {
        return Ok(());
    }
    parse_proxy_url(value).map(|_| ())
}

pub fn proxy_status() -> CodexProxyStatus {
    match resolve_codex_proxy() {
        Ok(proxy) => CodexProxyStatus {
            source: proxy.source(),
            redacted_endpoint: proxy.redacted_endpoint(),
            error_kind: None,
        },
        Err(_) => CodexProxyStatus {
            source: CodexProxySource::Direct,
            redacted_endpoint: None,
            error_kind: Some(CodexConnectionErrorKind::ProxyConfiguration),
        },
    }
}

pub fn classify_connection_text(text: &str, proxy_active: bool) -> CodexConnectionErrorKind {
    let normalized = text.to_ascii_lowercase();
    if normalized.contains("authentication required")
        || normalized.contains("not logged in")
        || normalized.contains("login required")
        || normalized.contains("unauthorized")
    {
        CodexConnectionErrorKind::NotLoggedIn
    } else if proxy_active
        && (normalized.contains("proxy")
            || normalized.contains("tunnel")
            || normalized.contains("connect"))
    {
        CodexConnectionErrorKind::ProxyConnection
    } else if normalized.contains("dns")
        || normalized.contains("network")
        || normalized.contains("timed out")
        || normalized.contains("timeout")
        || normalized.contains("resolve")
        || normalized.contains("connect")
    {
        CodexConnectionErrorKind::DnsOrNetwork
    } else {
        CodexConnectionErrorKind::ServiceError
    }
}

pub async fn test_codex_proxy_connection() -> CodexProxyTestResult {
    let proxy = match resolve_codex_proxy() {
        Ok(proxy) => proxy,
        Err(_) => {
            return CodexProxyTestResult {
                ok: false,
                source: CodexProxySource::Direct,
                redacted_endpoint: None,
                error_kind: Some(CodexConnectionErrorKind::ProxyConfiguration),
            };
        }
    };
    let source = proxy.source();
    let redacted_endpoint = proxy.redacted_endpoint();
    let builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(CONNECTION_TEST_TIMEOUT);
    let client = match proxy.configure_http_client(builder).and_then(|builder| {
        builder
            .build()
            .map_err(|_| CodexProxyConfigError::InvalidUrl)
    }) {
        Ok(client) => client,
        Err(_) => {
            return CodexProxyTestResult {
                ok: false,
                source,
                redacted_endpoint,
                error_kind: Some(CodexConnectionErrorKind::ProxyConfiguration),
            };
        }
    };

    match client.get(CONNECTION_TEST_URL).send().await {
        Ok(_) => CodexProxyTestResult {
            ok: true,
            source,
            redacted_endpoint,
            error_kind: None,
        },
        Err(error) => CodexProxyTestResult {
            ok: false,
            source,
            redacted_endpoint,
            error_kind: Some(if proxy.is_active() && error.is_connect() {
                CodexConnectionErrorKind::ProxyConnection
            } else {
                CodexConnectionErrorKind::DnsOrNetwork
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn resolve(
        manual: &str,
        use_environment: bool,
        values: &[(&str, &str)],
    ) -> Result<ResolvedCodexProxy, CodexProxyConfigError> {
        let values = values
            .iter()
            .map(|(key, value)| ((*key).to_string(), OsString::from(value)))
            .collect::<HashMap<_, _>>();
        resolve_with_lookup(manual, use_environment, |key| values.get(key).cloned())
    }

    #[test]
    fn manual_proxy_wins_over_environment() {
        let proxy = resolve(
            "http://manual.example:8080",
            true,
            &[("HTTPS_PROXY", "http://environment.example:9000")],
        )
        .unwrap();
        assert_eq!(proxy.source(), CodexProxySource::Manual);
        assert_eq!(proxy.redacted_endpoint().as_deref(), Some("http://***"));
    }

    #[test]
    fn environment_proxy_wins_over_direct() {
        let proxy = resolve(
            "",
            true,
            &[("HTTPS_PROXY", "http://environment.example:9000")],
        )
        .unwrap();
        assert_eq!(proxy.source(), CodexProxySource::Environment);
    }

    #[test]
    fn disabled_environment_proxy_selects_direct() {
        let proxy = resolve(
            "",
            false,
            &[("HTTPS_PROXY", "http://environment.example:9000")],
        )
        .unwrap();
        assert_eq!(proxy.source(), CodexProxySource::Direct);
        assert!(proxy.redacted_endpoint().is_none());
    }

    #[test]
    fn rejects_unsupported_scheme_and_embedded_credentials() {
        assert_eq!(
            parse_proxy_url("socks5://proxy.example:1080").unwrap_err(),
            CodexProxyConfigError::UnsupportedScheme
        );
        assert_eq!(
            parse_proxy_url("http://user:secret@proxy.example:8080").unwrap_err(),
            CodexProxyConfigError::CredentialsNotAllowed
        );
    }

    #[test]
    fn redaction_never_contains_host_port_or_credentials() {
        let proxy = resolve("https://private-host.example:8443", true, &[]).unwrap();
        let display = proxy.redacted_endpoint().unwrap();
        assert_eq!(display, "https://***");
        assert!(!display.contains("private-host"));
        assert!(!display.contains("8443"));
    }

    #[test]
    fn selected_proxy_is_explicitly_inherited_by_app_server() {
        let proxy = resolve("http://proxy.example:8080", true, &[]).unwrap();
        let mut command = Command::new("codex");
        proxy.apply_to_app_server(&mut command);
        let environment = command
            .as_std()
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(
            environment.get("HTTP_PROXY").cloned().flatten(),
            Some("http://proxy.example:8080/".to_string())
        );
        assert_eq!(
            environment.get("HTTPS_PROXY").cloned().flatten(),
            Some("http://proxy.example:8080/".to_string())
        );
        assert_eq!(environment.get("NO_PROXY"), Some(&None));
    }

    #[test]
    fn maps_proxy_network_auth_and_service_failures() {
        assert_eq!(
            classify_connection_text("proxy tunnel failed", true),
            CodexConnectionErrorKind::ProxyConnection
        );
        assert_eq!(
            classify_connection_text("DNS resolve failed", false),
            CodexConnectionErrorKind::DnsOrNetwork
        );
        assert_eq!(
            classify_connection_text("authentication required", true),
            CodexConnectionErrorKind::NotLoggedIn
        );
        assert_eq!(
            classify_connection_text("backend returned 503", false),
            CodexConnectionErrorKind::ServiceError
        );
    }
}
