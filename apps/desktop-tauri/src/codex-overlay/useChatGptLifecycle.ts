import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import {
  getChatGptLifecycleStatus,
  lifecycleDiagnosticsEnabled,
  recordChatGptLifecycleFrontendDiagnostic,
} from "./api";
import type {
  ChatGptDesktopSnapshot,
  ChatGptDesktopStatus,
  LifecycleDiagnosticErrorCategory,
  LifecycleDiagnosticEvent,
  LifecycleDiagnosticWindow,
} from "./types";

const FALLBACK: ChatGptDesktopSnapshot = { status: "unavailable" };
export const CHATGPT_LIFECYCLE_STATUS_EVENT = "chatgpt-desktop-status-changed";

const VALID_STATUSES = new Set<ChatGptDesktopStatus>([
  "monitoring-disabled",
  "detecting",
  "running",
  "not-running",
  "unsupported",
  "unavailable",
]);

export function isChatGptDesktopSnapshot(
  value: unknown,
): value is ChatGptDesktopSnapshot {
  return (
    typeof value === "object" &&
    value !== null &&
    "status" in value &&
    typeof value.status === "string" &&
    VALID_STATUSES.has(value.status as ChatGptDesktopStatus)
  );
}

function diagnosticWindowLabel(): LifecycleDiagnosticWindow {
  try {
    const label = getCurrentWindow().label;
    if (label === "codex-overlay" || label === "settings") return label;
  } catch {
    // Diagnostics must not affect the lifecycle UI if window metadata is not
    // available yet (or is absent in a non-Tauri test environment).
  }
  return "unknown";
}

function categorizeInvokeError(
  error: unknown,
): LifecycleDiagnosticErrorCategory {
  const message = error instanceof Error ? error.message.toLowerCase() : "";
  if (message.includes("not found") || message.includes("unknown command")) {
    return "invoke-command-not-found";
  }
  if (
    message.includes("permission") ||
    message.includes("denied") ||
    message.includes("not allowed")
  ) {
    return "invoke-permission-denied";
  }
  if (message.includes("backend") || message.includes("not initialized")) {
    return "invoke-backend-unavailable";
  }
  return "unknown-safe-category";
}

function categorizeEventListenError(
  error: unknown,
): LifecycleDiagnosticErrorCategory {
  const message = error instanceof Error ? error.message.toLowerCase() : "";
  if (
    message.includes("permission") ||
    message.includes("denied") ||
    message.includes("not allowed")
  ) {
    return "event-listen-permission-denied";
  }
  return "event-listen-failed";
}

/** Consume backend lifecycle state; the frontend never enumerates processes. */
export function useChatGptLifecycle(): ChatGptDesktopSnapshot {
  const [snapshot, setSnapshot] = useState<ChatGptDesktopSnapshot>(FALLBACK);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    let receivedEvent = false;
    let diagnosticsEnabled: boolean | undefined;
    const pendingDiagnostics: Array<{
      event: LifecycleDiagnosticEvent;
      status?: ChatGptDesktopStatus;
      errorCategory?: LifecycleDiagnosticErrorCategory;
    }> = [];

    const reportDiagnostic = (input: {
      event: LifecycleDiagnosticEvent;
      status?: ChatGptDesktopStatus;
      errorCategory?: LifecycleDiagnosticErrorCategory;
    }) => {
      if (diagnosticsEnabled === false) return;
      if (diagnosticsEnabled === undefined) {
        pendingDiagnostics.push(input);
        return;
      }
      void recordChatGptLifecycleFrontendDiagnostic({
        ...input,
        windowLabel: diagnosticWindowLabel(),
      }).catch(() => {});
    };

    void lifecycleDiagnosticsEnabled()
      .then((enabled) => {
        diagnosticsEnabled = enabled;
        if (!enabled) return;
        for (const input of pendingDiagnostics.splice(0)) {
          reportDiagnostic(input);
        }
      })
      .catch(() => {
        diagnosticsEnabled = false;
        pendingDiagnostics.length = 0;
      });

    const applySnapshot = (next: unknown, fromEvent = false) => {
      if (!active || !isChatGptDesktopSnapshot(next)) return false;
      if (fromEvent) receivedEvent = true;
      setSnapshot(next);
      return true;
    };

    void getChatGptLifecycleStatus()
      .then((next) => {
        if (!isChatGptDesktopSnapshot(next)) {
          reportDiagnostic({
            event: "frontend-initial-invoke-error",
            errorCategory: "invoke-invalid-response",
          });
          return;
        }
        reportDiagnostic({
          event: "frontend-initial-invoke-success",
          status: next.status,
        });
        // A later event is newer than an in-flight initial query.
        if (!receivedEvent) applySnapshot(next);
      })
      .catch((error) => {
        reportDiagnostic({
          event: "frontend-initial-invoke-error",
          errorCategory: categorizeInvokeError(error),
        });
        // Keep the initial fallback only when the query fails. A later valid
        // lifecycle event must still be able to recover this UI state.
      });
    void (async () => {
      try {
        const nextDispose = await listen<unknown>(
          CHATGPT_LIFECYCLE_STATUS_EVENT,
          (event) => {
            if (isChatGptDesktopSnapshot(event.payload)) {
              applySnapshot(event.payload, true);
              reportDiagnostic({
                event: "frontend-payload-validation-success",
                status: event.payload.status,
              });
            } else {
              reportDiagnostic({
                event: "frontend-payload-validation-error",
                errorCategory: "event-invalid-payload",
              });
            }
          },
        );
        reportDiagnostic({ event: "frontend-event-listen-success" });
        if (active) {
          dispose = nextDispose;
        } else {
          nextDispose();
        }
      } catch (error) {
        reportDiagnostic({
          event: "frontend-event-listen-error",
          errorCategory: categorizeEventListenError(error),
        });
        // Event subscription is an enhancement. Do not overwrite a valid
        // command response when this window cannot subscribe temporarily.
      }
    })();
    return () => {
      active = false;
      dispose?.();
    };
  }, []);

  return snapshot;
}
