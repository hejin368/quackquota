import type { Language } from "../types/bridge";

export type CodexQuotaStatus =
  | "ready"
  | "cached"
  | "not-installed"
  | "not-logged-in"
  | "offline"
  | "provider-error"
  | "invalid-data";

export type CodexQuotaSource = "app-server" | "legacy-provider" | "cache";

export type ChatGptDesktopStatus =
  | "monitoring-disabled"
  | "detecting"
  | "running"
  | "not-running"
  | "unsupported"
  | "unavailable";

/** Minimal lifecycle DTO emitted only by the Windows backend watcher. */
export interface ChatGptDesktopSnapshot {
  status: ChatGptDesktopStatus;
}

export type LifecycleDiagnosticEvent =
  | "frontend-initial-invoke-success"
  | "frontend-initial-invoke-error"
  | "frontend-event-listen-success"
  | "frontend-event-listen-error"
  | "frontend-payload-validation-success"
  | "frontend-payload-validation-error";

export type LifecycleDiagnosticWindow =
  "codex-overlay" | "settings" | "unknown";

export type LifecycleDiagnosticErrorCategory =
  | "invoke-command-not-found"
  | "invoke-permission-denied"
  | "invoke-backend-unavailable"
  | "invoke-invalid-response"
  | "event-listen-permission-denied"
  | "event-listen-failed"
  | "event-invalid-payload"
  | "shared-state-not-ready"
  | "unknown-safe-category"
  | "event-emit-failed";

export interface LifecycleFrontendDiagnostic {
  event: LifecycleDiagnosticEvent;
  windowLabel: LifecycleDiagnosticWindow;
  status?: ChatGptDesktopStatus;
  errorCategory?: LifecycleDiagnosticErrorCategory;
}

export type CodexConnectionErrorKind =
  | "dns-or-network"
  | "proxy-connection"
  | "proxy-configuration"
  | "not-logged-in"
  | "app-server-unavailable"
  | "service-error"
  | "invalid-data";

export type CodexProxySource = "manual" | "environment" | "direct";

export interface CodexProxyStatus {
  source: CodexProxySource;
  redactedEndpoint?: string;
  errorKind?: CodexConnectionErrorKind;
}

export interface CodexProxyTestResult extends CodexProxyStatus {
  ok: boolean;
}

export interface CodexRateLimitWindow {
  id: string;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  windowDurationMinutes?: number;
  resetsAt?: string;
  limitId?: string;
  limitName?: string;
  level: "primary" | "secondary";
}

/** Minimal Codex-only DTO returned by `read_codex_rate_limits`. */
export interface CodexQuotaSnapshot {
  plan?: string;
  windows: CodexRateLimitWindow[];
  resetCredits?: {
    availableCount: number;
  };
  status: CodexQuotaStatus;
  source: CodexQuotaSource;
  updatedAt: string;
  message?: string;
  errorKind?: CodexConnectionErrorKind;
}

export interface CodexQuotaLabelStrings {
  oneHour: string;
  fiveHours: string;
  twentyFourHours: string;
  sevenDays: string;
  hourQuotaUnit: string;
  dayQuotaUnit: string;
  generic: string;
  localizedLimitNames?: Record<string, string>;
}

export interface VisibleCodexQuotaWindows {
  windows: CodexRateLimitWindow[];
  additionalCount: number;
}

export interface MascotQuotaInput {
  lowestRemainingPercent: number | null;
  tightestWindowId: string | null;
  justReset: boolean;
  offlineOrInvalid: boolean;
}

function isCompactLanguage(language: Language): boolean {
  return ["chinese", "chinesetraditional", "japanese", "korean"].includes(
    language,
  );
}

function countedLabel(count: number, unit: string, language: Language): string {
  return `${count}${isCompactLanguage(language) ? "" : " "}${unit}`;
}

/**
 * Produce a localized label without assigning semantic meaning from
 * primary/secondary ordering. Unknown durations remain generic.
 */
export function formatCodexRateLimitLabel(
  window: Pick<
    CodexRateLimitWindow,
    "label" | "limitName" | "windowDurationMinutes"
  >,
  language: Language,
  strings: CodexQuotaLabelStrings,
): string {
  const limitName = window.limitName?.trim();
  if (limitName) {
    return (
      strings.localizedLimitNames?.[limitName.toLocaleLowerCase()] ?? limitName
    );
  }

  const minutes = window.windowDurationMinutes;
  switch (minutes) {
    case 60:
      return strings.oneHour;
    case 300:
      return strings.fiveHours;
    case 1_440:
      return strings.twentyFourHours;
    case 10_080:
      return strings.sevenDays;
    default:
      break;
  }

  if (
    typeof minutes === "number" &&
    Number.isFinite(minutes) &&
    minutes >= 1_440 &&
    minutes % 1_440 === 0
  ) {
    return countedLabel(minutes / 1_440, strings.dayQuotaUnit, language);
  }
  if (
    typeof minutes === "number" &&
    Number.isFinite(minutes) &&
    minutes > 0 &&
    minutes % 60 === 0
  ) {
    return countedLabel(minutes / 60, strings.hourQuotaUnit, language);
  }

  return strings.generic;
}

export function isValidCodexRateLimitWindow(
  window: CodexRateLimitWindow,
): boolean {
  return (
    window.id.trim().length > 0 &&
    Number.isFinite(window.usedPercent) &&
    window.usedPercent >= 0 &&
    window.usedPercent <= 100 &&
    Number.isFinite(window.remainingPercent) &&
    window.remainingPercent >= 0 &&
    window.remainingPercent <= 100
  );
}

/** Select at most the two most constrained windows for the fixed-size UI. */
export function selectVisibleCodexQuotaWindows(
  windows: CodexRateLimitWindow[],
): VisibleCodexQuotaWindows {
  const valid = windows
    .filter(isValidCodexRateLimitWindow)
    .sort(
      (left, right) =>
        left.remainingPercent - right.remainingPercent ||
        left.id.localeCompare(right.id),
    );
  return {
    windows: valid.slice(0, 2),
    additionalCount: Math.max(0, valid.length - 2),
  };
}

/** Dynamic inputs reserved for a future mascot renderer. */
export function getMascotQuotaInput(
  snapshot: CodexQuotaSnapshot,
  previous?: CodexQuotaSnapshot | null,
): MascotQuotaInput {
  const validWindows = snapshot.windows
    .filter(isValidCodexRateLimitWindow)
    .sort(
      (left, right) =>
        left.remainingPercent - right.remainingPercent ||
        left.id.localeCompare(right.id),
    );
  const tightest = validWindows[0] ?? null;
  const previousById = new Map(
    (previous?.windows ?? [])
      .filter(isValidCodexRateLimitWindow)
      .map((window) => [window.id, window]),
  );
  const justReset = validWindows.some((window) => {
    const previousWindow = previousById.get(window.id);
    return Boolean(
      previousWindow &&
      window.usedPercent < previousWindow.usedPercent &&
      window.resetsAt !== previousWindow.resetsAt,
    );
  });

  return {
    lowestRemainingPercent: tightest?.remainingPercent ?? null,
    tightestWindowId: tightest?.id ?? null,
    justReset,
    offlineOrInvalid: !["ready", "cached"].includes(snapshot.status),
  };
}
