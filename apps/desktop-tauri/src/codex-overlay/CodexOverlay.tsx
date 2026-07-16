import { useCallback, useEffect, type MouseEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useFormattedResetTime } from "../hooks/useFormattedResetTime";
import { useLocale } from "../hooks/useLocale";
import { useResetCountdown } from "../surfaces/tray/useResetCountdown";
import type { Language } from "../types/bridge";
import {
  formatCodexRateLimitLabel,
  selectVisibleCodexQuotaWindows,
  type CodexQuotaLabelStrings,
  type CodexQuotaSnapshot,
  type CodexQuotaSource,
  type CodexRateLimitWindow,
} from "./types";
import { useCodexUsage } from "./useCodexUsage";
import { hideCodexOverlay } from "./api";
import { connectionErrorLocaleKey } from "./errors";
import "./CodexOverlay.css";

function formatPercent(value: number): string {
  return Number.isFinite(value) ? `${Math.round(value)}%` : "—";
}

function formatSource(value: CodexQuotaSource | undefined): string {
  switch (value) {
    case "app-server":
      return "Codex App Server";
    case "legacy-provider":
      return "Legacy Codex Provider";
    case "cache":
      return "Cache";
    default:
      return "—";
  }
}

function formatUpdatedAt(value: string | undefined): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

function UsageWindow({
  window,
  language,
  labelStrings,
}: {
  window: CodexRateLimitWindow;
  language: Language;
  labelStrings: CodexQuotaLabelStrings;
}) {
  const { t } = useLocale();
  const relativeReset = useResetCountdown(window.resetsAt ?? null, null);
  const absoluteReset = useFormattedResetTime(
    window.resetsAt ?? null,
    null,
    false,
  );
  const remaining = Number.isFinite(window.remainingPercent)
    ? Math.max(0, Math.min(100, window.remainingPercent))
    : null;
  const label = formatCodexRateLimitLabel(window, language, labelStrings);

  return (
    <section className="codex-overlay__quota" data-window-id={window.id}>
      <div className="codex-overlay__quota-heading">
        <span title={label}>{label}</span>
        <strong>{formatPercent(window.remainingPercent)}</strong>
      </div>
      <div className="codex-overlay__metrics">
        <span>
          {formatPercent(window.usedPercent)} {t("PanelUsedSuffix")}
        </span>
        <span>
          {formatPercent(window.remainingPercent)}{" "}
          {t("FloatBarRemainingSuffix")}
        </span>
      </div>
      <div
        className="codex-overlay__track"
        role="progressbar"
        aria-label={`${label} ${t("FloatBarRemainingSuffix")}`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={remaining ?? undefined}
      >
        {remaining !== null && (
          <span
            data-testid={`quota-progress-${window.id}`}
            style={{ width: `${remaining}%` }}
          />
        )}
      </div>
      <div className="codex-overlay__reset">
        <span title={relativeReset ?? undefined}>{relativeReset ?? "—"}</span>
        <span title={absoluteReset ?? undefined}>
          {absoluteReset ? `${t("CodexOverlayResetAt")} ${absoluteReset}` : "—"}
        </span>
      </div>
    </section>
  );
}

function EmptyState({
  snapshot,
  isLoading,
}: {
  snapshot: CodexQuotaSnapshot | null;
  isLoading: boolean;
}) {
  const { t } = useLocale();
  const detailKey = connectionErrorLocaleKey(snapshot?.errorKind);
  const title = isLoading
    ? t("StateLoadingProviders")
    : snapshot?.status === "not-installed"
      ? t("CodexOverlayCliNotInstalled")
      : snapshot?.status === "not-logged-in"
        ? t("ProviderNotSignedIn")
        : snapshot?.status === "offline"
          ? t("CodexOverlayOffline")
          : snapshot?.status === "provider-error"
            ? t("StatusUnableToGetUsage")
            : snapshot?.status === "invalid-data"
              ? t("CodexOverlayInvalidData")
              : t("CodexOverlayNoQuotaData");

  return (
    <div className="codex-overlay__empty" role="status">
      <strong>{title}</strong>
      {detailKey && <span>{t(detailKey)}</span>}
    </div>
  );
}

export function CodexOverlayView({
  snapshot,
  isRefreshing,
  onRefresh,
  onClose,
  onStartDragging,
}: {
  snapshot: CodexQuotaSnapshot | null;
  isRefreshing: boolean;
  onRefresh: () => void;
  onClose: () => void;
  onStartDragging?: (event: MouseEvent<HTMLElement>) => void;
}) {
  const { t, language } = useLocale();
  const visible = selectVisibleCodexQuotaWindows(snapshot?.windows ?? []);
  const hasUsage = visible.windows.length > 0;
  const isLoading = snapshot === null && isRefreshing;
  const labelStrings: CodexQuotaLabelStrings = {
    oneHour: t("CodexOverlayOneHourQuota"),
    fiveHours: t("CodexOverlayFiveHourQuota"),
    twentyFourHours: t("CodexOverlayTwentyFourHourQuota"),
    sevenDays: t("CodexOverlaySevenDayQuota"),
    hourQuotaUnit: t("CodexOverlayHourQuotaUnit"),
    dayQuotaUnit: t("CodexOverlayDayQuotaUnit"),
    generic: t("CodexOverlayGenericQuota"),
  };
  const warningKey = connectionErrorLocaleKey(snapshot?.errorKind);

  return (
    <main className="codex-overlay">
      <header
        className="codex-overlay__header"
        data-tauri-drag-region
        onMouseDown={onStartDragging}
      >
        <div className="codex-overlay__title" data-tauri-drag-region>
          <strong data-tauri-drag-region>{t("CodexOverlayTitle")}</strong>
          {snapshot?.status === "cached" && (
            <span className="codex-overlay__badge">
              {t("CodexOverlayCachedData")}
            </span>
          )}
        </div>
        <div className="codex-overlay__actions">
          <button
            type="button"
            onClick={onRefresh}
            disabled={isRefreshing}
            aria-label={t("ActionRefresh")}
            title={t("ActionRefresh")}
          >
            <span className={isRefreshing ? "codex-overlay__spin" : undefined}>
              ↻
            </span>
          </button>
          <button
            type="button"
            onClick={onClose}
            aria-label={t("ActionClose")}
            title={t("ActionClose")}
          >
            ×
          </button>
        </div>
      </header>

      {hasUsage ? (
        <>
          <div className="codex-overlay__plan">
            <span>{t("ProviderPlan")}</span>
            <strong>{snapshot?.plan ?? "—"}</strong>
            {snapshot?.resetCredits && (
              <span className="codex-overlay__credits">
                {t("CodexOverlayResetCredits")}:{" "}
                {snapshot.resetCredits.availableCount}
              </span>
            )}
          </div>
          <div
            className={`codex-overlay__quotas${
              visible.windows.length === 1
                ? " codex-overlay__quotas--single"
                : ""
            }`}
          >
            {visible.windows.map((window) => (
              <UsageWindow
                key={window.id}
                window={window}
                language={language}
                labelStrings={labelStrings}
              />
            ))}
          </div>
          {visible.additionalCount > 0 && (
            <div className="codex-overlay__additional">
              {t("CodexOverlayAdditionalPrefix")} {visible.additionalCount}{" "}
              {t("CodexOverlayAdditionalSuffix")}
            </div>
          )}
          {warningKey && (
            <div className="codex-overlay__warning">{t(warningKey)}</div>
          )}
        </>
      ) : (
        <EmptyState snapshot={snapshot} isLoading={isLoading} />
      )}

      <footer className="codex-overlay__footer">
        <span>
          {t("DataSource")}: {formatSource(snapshot?.source)}
        </span>
        <span>
          {t("LastUpdated")}: {formatUpdatedAt(snapshot?.updatedAt)}
        </span>
      </footer>
    </main>
  );
}

export default function CodexOverlay() {
  const { snapshot, isRefreshing, refresh } = useCodexUsage();

  useEffect(() => {
    document.body.classList.add("codex-overlay-window");
    return () => document.body.classList.remove("codex-overlay-window");
  }, []);

  const close = useCallback(() => {
    void hideCodexOverlay().catch(() => {});
  }, []);

  const startDragging = useCallback((event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    if ((event.target as HTMLElement).closest("button")) return;
    event.preventDefault();
    void getCurrentWindow()
      .startDragging()
      .catch(() => {});
  }, []);

  return (
    <div className="codex-overlay__drag-shell">
      <CodexOverlayView
        snapshot={snapshot}
        isRefreshing={isRefreshing}
        onRefresh={() => void refresh()}
        onClose={close}
        onStartDragging={startDragging}
      />
    </div>
  );
}
