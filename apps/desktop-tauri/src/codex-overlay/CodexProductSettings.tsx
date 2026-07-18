import { useEffect, useState } from "react";
import { Field, Toggle } from "../components/FormControls";
import { useLocale } from "../hooks/useLocale";
import type { LocaleKey } from "../i18n/keys";
import type { TabProps } from "../surfaces/Settings";
import type { CodexOverlayStartupMode } from "../types/bridge";
import type {
  ChatGptDesktopStatus,
  CodexConnectionErrorKind,
  CodexProxySource,
  CodexProxyStatus,
} from "./types";
import { useChatGptLifecycle } from "./useChatGptLifecycle";
import {
  getCodexProxyStatus,
  resetCodexOverlayPosition,
  testCodexProxyConnection,
} from "./api";
import { connectionErrorLocaleKey } from "./errors";

export function isValidManualCodexProxy(value: string): boolean {
  if (!value.trim()) return true;
  try {
    const url = new URL(value.trim());
    return (
      ["http:", "https:"].includes(url.protocol) &&
      Boolean(url.hostname) &&
      !url.username &&
      !url.password &&
      !url.search &&
      !url.hash
    );
  } catch {
    return false;
  }
}

function sourceLocaleKey(source: CodexProxySource): LocaleKey {
  switch (source) {
    case "manual":
      return "CodexProxyRouteManual";
    case "environment":
      return "CodexProxyRouteEnvironment";
    case "direct":
      return "CodexProxyRouteDirect";
  }
}

function isCodexProxyStatus(value: unknown): value is CodexProxyStatus {
  if (!value || typeof value !== "object") return false;
  return ["manual", "environment", "direct"].includes(
    String((value as { source?: unknown }).source),
  );
}

function lifecycleLocaleKey(status: ChatGptDesktopStatus): LocaleKey {
  switch (status) {
    case "running":
      return "ChatGptLifecycleRunning";
    case "not-running":
      return "ChatGptLifecycleNotRunning";
    case "detecting":
      return "ChatGptLifecycleDetecting";
    case "unsupported":
      return "ChatGptLifecycleUnsupported";
    case "monitoring-disabled":
      return "ChatGptLifecycleInactive";
    case "unavailable":
      return "ChatGptLifecycleUnavailable";
  }
}

export default function CodexProductSettings({
  settings,
  set,
  saving,
}: TabProps) {
  const { t } = useLocale();
  const lifecycle = useChatGptLifecycle();
  const [manualProxy, setManualProxy] = useState(
    settings.codexManualProxy ?? "",
  );
  const [status, setStatus] = useState<CodexProxyStatus | null>(null);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    ok: boolean;
    errorKind?: CodexConnectionErrorKind;
  } | null>(null);

  useEffect(() => {
    setManualProxy(settings.codexManualProxy ?? "");
  }, [settings.codexManualProxy]);

  useEffect(() => {
    void getCodexProxyStatus()
      .then((next) => {
        if (isCodexProxyStatus(next)) setStatus(next);
      })
      .catch(() => {
        setStatus({ source: "direct", errorKind: "proxy-configuration" });
      });
  }, [settings.codexManualProxy, settings.codexProxyUseEnvironment]);

  const saveManualProxy = async (): Promise<boolean> => {
    if (!isValidManualCodexProxy(manualProxy)) {
      setTestResult({ ok: false, errorKind: "proxy-configuration" });
      return false;
    }
    if (manualProxy.trim() === (settings.codexManualProxy ?? "")) return true;
    await set({ codexManualProxy: manualProxy.trim() });
    return true;
  };

  const testConnection = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      if (!(await saveManualProxy())) return;
      const result = await testCodexProxyConnection();
      setStatus(result);
      setTestResult({ ok: result.ok, errorKind: result.errorKind });
    } catch {
      setTestResult({ ok: false, errorKind: "dns-or-network" });
    } finally {
      setTesting(false);
    }
  };

  const errorKey = connectionErrorLocaleKey(
    testResult?.errorKind ?? status?.errorKind,
  );
  const routeText = status
    ? status.errorKind === "proxy-configuration"
      ? t("CodexProxyRouteInvalid")
      : `${t(sourceLocaleKey(status.source))}${
          status.redactedEndpoint ? ` (${status.redactedEndpoint})` : ""
        }`
    : t("CodexProxyRouteDirect");

  const startupMode = settings.codexOverlayStartupMode ?? "rememberLast";
  const monitoringEnabled = settings.monitorChatgptDesktop ?? true;
  const startupOptions: Array<{
    value: CodexOverlayStartupMode;
    label: string;
  }> = [
    {
      value: "rememberLast",
      label: t("CodexOverlayStartupRemember"),
    },
    {
      value: "alwaysShow",
      label: t("CodexOverlayStartupAlwaysShow"),
    },
    {
      value: "alwaysHide",
      label: t("CodexOverlayStartupAlwaysHide"),
    },
  ];

  return (
    <>
      <section className="settings-section">
        <h3 className="settings-section__title">{t("CodexOverlaySettings")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("CodexOverlayStartupBehavior")}
            className="codex-settings-responsive-field codex-startup-field"
          >
            <div
              className="codex-startup-options"
              role="radiogroup"
              aria-label={t("CodexOverlayStartupBehavior")}
            >
              {startupOptions.map((option) => {
                const checked = startupMode === option.value;
                return (
                  <label
                    key={option.value}
                    className={`codex-startup-option${
                      checked ? " codex-startup-option--selected" : ""
                    }${saving ? " codex-startup-option--disabled" : ""}`}
                  >
                    <input
                      type="radio"
                      name="codex-overlay-startup-mode"
                      value={option.value}
                      checked={checked}
                      disabled={saving}
                      onChange={() =>
                        void set({ codexOverlayStartupMode: option.value })
                      }
                    />
                    <span
                      className="codex-startup-option__indicator"
                      aria-hidden="true"
                    />
                    <span className="codex-startup-option__label">
                      {option.label}
                    </span>
                  </label>
                );
              })}
            </div>
          </Field>
          <Field
            label={t("ChatGptMonitorDesktop")}
            description={t("ChatGptMonitorDesktopHelper")}
            leading
            className="codex-settings-responsive-field"
          >
            <Toggle
              checked={monitoringEnabled}
              ariaLabel={t("ChatGptMonitorDesktop")}
              disabled={saving}
              onChange={(value) => void set({ monitorChatgptDesktop: value })}
            />
          </Field>
          <Field
            label={t("ChatGptShowOverlayOnStart")}
            description={
              !monitoringEnabled ? t("ChatGptLifecycleInactive") : undefined
            }
            leading
            className="codex-settings-responsive-field"
          >
            <Toggle
              checked={settings.showOverlayOnChatgptStart ?? true}
              ariaLabel={t("ChatGptShowOverlayOnStart")}
              disabled={saving}
              onChange={(value) =>
                void set({ showOverlayOnChatgptStart: value })
              }
            />
          </Field>
          <Field
            label={t("ChatGptHideOverlayOnExit")}
            description={
              !monitoringEnabled ? t("ChatGptLifecycleInactive") : undefined
            }
            leading
            className="codex-settings-responsive-field"
          >
            <Toggle
              checked={settings.hideOverlayOnChatgptExit ?? true}
              ariaLabel={t("ChatGptHideOverlayOnExit")}
              disabled={saving}
              onChange={(value) =>
                void set({ hideOverlayOnChatgptExit: value })
              }
            />
          </Field>
          <Field
            label={t("ChatGptDesktopLifecycle")}
            className="codex-settings-responsive-field"
          >
            <span role="status">{t(lifecycleLocaleKey(lifecycle.status))}</span>
          </Field>
          <Field
            label={t("CodexOverlayResetPosition")}
            description={t("CodexOverlayResetPositionHelper")}
            className="codex-settings-responsive-field"
          >
            <button
              type="button"
              className="shortcut-capture__button shortcut-capture__button--ghost"
              disabled={saving}
              onClick={() => void resetCodexOverlayPosition()}
            >
              {t("CodexOverlayResetPosition")}
            </button>
          </Field>
        </div>
      </section>

      <section className="settings-section">
        <h3 className="settings-section__title">{t("CodexNetworkSettings")}</h3>
        <div className="settings-section__group">
          <Field
            label={t("CodexUseEnvironmentProxy")}
            description={t("CodexUseEnvironmentProxyHelper")}
            leading
            className="codex-settings-responsive-field"
          >
            <Toggle
              checked={settings.codexProxyUseEnvironment ?? true}
              disabled={saving}
              onChange={(value) =>
                void set({ codexProxyUseEnvironment: value })
              }
            />
          </Field>
          <Field
            label={t("CodexManualProxy")}
            description={t("CodexManualProxyHelper")}
            className="codex-settings-responsive-field"
          >
            <input
              className="text-input"
              type="url"
              inputMode="url"
              autoComplete="off"
              spellCheck={false}
              value={manualProxy}
              placeholder="http://proxy.example:8080"
              disabled={saving || testing}
              onChange={(event) => setManualProxy(event.target.value)}
              onBlur={() => void saveManualProxy()}
              onKeyDown={(event) => {
                if (event.key === "Enter") event.currentTarget.blur();
              }}
            />
          </Field>
          <Field
            label={t("CodexProxyCurrentRoute")}
            className="codex-settings-responsive-field"
          >
            <span>{routeText}</span>
          </Field>
          {errorKey && (
            <p className="settings-field__desc" role="status">
              {t(errorKey)}
            </p>
          )}
          {testResult?.ok && (
            <p className="settings-field__desc" role="status">
              {t("CodexProxyTestSuccess")}
            </p>
          )}
          <div className="shortcut-capture__actions">
            <button
              type="button"
              className="shortcut-capture__button"
              disabled={saving || testing}
              onClick={() => void testConnection()}
            >
              {testing ? t("CodexProxyTesting") : t("CodexProxyTest")}
            </button>
            <button
              type="button"
              className="shortcut-capture__button shortcut-capture__button--ghost"
              disabled={saving || testing || manualProxy.length === 0}
              onClick={() => {
                setManualProxy("");
                setTestResult(null);
                void set({ codexManualProxy: "" });
              }}
            >
              {t("CodexProxyClear")}
            </button>
          </div>
        </div>
      </section>
    </>
  );
}
