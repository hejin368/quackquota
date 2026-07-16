import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { SettingsSnapshot, SettingsUpdate } from "../types/bridge";
import { getSettingsSnapshot, updateSettings } from "../lib/tauri";

const SAVED_FEEDBACK_DURATION_MS = 1_500;

export type SettingsSaveFeedback =
  | { state: "idle" }
  | { state: "saving" }
  | { state: "saved" }
  | { state: "error" };

interface SettingsChangedPayload {
  sourceWindowLabel?: string | null;
}

interface UseSettingsReturn {
  settings: SettingsSnapshot;
  /** True only while one or more IPC saves are in flight. */
  saving: boolean;
  saveFeedback: SettingsSaveFeedback;
  update: (patch: SettingsUpdate) => Promise<void>;
}

function patchFields(patch: SettingsUpdate): string[] {
  return Object.keys(patch);
}

function mergeFields(
  current: SettingsSnapshot,
  source: SettingsSnapshot,
  fields: readonly string[],
): SettingsSnapshot {
  const normalized = Object.fromEntries(
    fields.map((field) => [field, source[field as keyof SettingsSnapshot]]),
  );
  return { ...current, ...normalized };
}

function mergePatch(
  current: SettingsSnapshot,
  patch: SettingsUpdate,
): SettingsSnapshot {
  return { ...current, ...patch };
}

function currentWindowLabel(): string | undefined {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    // Tests and non-Tauri rendering do not always expose a WebviewWindow.
    return undefined;
  }
}

/**
 * Keeps a per-webview settings copy responsive without turning a save into a
 * full settings reload. Mutations are optimistic, serialized, and merged by
 * field so an old response cannot overwrite a newer local interaction.
 */
export function useSettings(initial: SettingsSnapshot): UseSettingsReturn {
  const [settings, setSettings] = useState<SettingsSnapshot>(initial);
  const [saveFeedback, setSaveFeedback] = useState<SettingsSaveFeedback>({
    state: "idle",
  });
  const confirmedRef = useRef<SettingsSnapshot>(initial);
  const fieldVersionRef = useRef<Record<string, number>>({});
  const mutationVersionRef = useRef(0);
  const pendingSaveCountRef = useRef(0);
  const saveQueueRef = useRef<Promise<void>>(Promise.resolve());
  const savedTimerRef = useRef<number | undefined>();
  const mountedRef = useRef(true);
  const externalSyncRequestedRef = useRef(false);

  const clearSavedTimer = useCallback(() => {
    if (savedTimerRef.current !== undefined) {
      window.clearTimeout(savedTimerRef.current);
      savedTimerRef.current = undefined;
    }
  }, []);

  const showSaved = useCallback(() => {
    clearSavedTimer();
    setSaveFeedback({ state: "saved" });
    savedTimerRef.current = window.setTimeout(() => {
      if (mountedRef.current) {
        setSaveFeedback({ state: "idle" });
      }
      savedTimerRef.current = undefined;
    }, SAVED_FEEDBACK_DURATION_MS);
  }, [clearSavedTimer]);

  const syncFromAnotherWindow = useCallback(() => {
    if (pendingSaveCountRef.current > 0) {
      externalSyncRequestedRef.current = true;
      return;
    }

    const syncVersion = mutationVersionRef.current;
    void getSettingsSnapshot()
      .then((fresh) => {
        if (
          mountedRef.current &&
          pendingSaveCountRef.current === 0 &&
          mutationVersionRef.current === syncVersion
        ) {
          confirmedRef.current = fresh;
          setSettings(fresh);
        }
      })
      .catch(() => {
        // Keep the local snapshot when another window cannot be synced.
      });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    const syncVersion = mutationVersionRef.current;
    confirmedRef.current = initial;
    setSettings(initial);

    void getSettingsSnapshot()
      .then((fresh) => {
        if (
          mountedRef.current &&
          mutationVersionRef.current === syncVersion &&
          pendingSaveCountRef.current === 0
        ) {
          confirmedRef.current = fresh;
          setSettings(fresh);
        }
      })
      .catch(() => {
        // Keep the bootstrap snapshot if the background sync fails.
      });

    return () => {
      mountedRef.current = false;
    };
  }, [initial]);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const windowLabel = currentWindowLabel();

    Promise.resolve(
      listen<SettingsChangedPayload>("settings-changed", (event) => {
        if (event?.payload?.sourceWindowLabel === windowLabel) {
          return;
        }
        syncFromAnotherWindow();
      }),
    )
      .then((fn) => {
        if (active) {
          unlisten = fn;
        } else {
          fn?.();
        }
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisten?.();
    };
  }, [syncFromAnotherWindow]);

  useEffect(
    () => () => {
      mountedRef.current = false;
      clearSavedTimer();
    },
    [clearSavedTimer],
  );

  const update = useCallback(
    (patch: SettingsUpdate): Promise<void> => {
      const fields = patchFields(patch);
      if (fields.length === 0) {
        return Promise.resolve();
      }

      const sequence = ++mutationVersionRef.current;
      for (const field of fields) {
        fieldVersionRef.current[field] = sequence;
      }

      clearSavedTimer();
      setSaveFeedback({ state: "saving" });
      setSettings((current) => mergePatch(current, patch));
      pendingSaveCountRef.current += 1;

      const save = async () => {
        try {
          const next = await updateSettings(patch);
          const currentFields = fields.filter(
            (field) => fieldVersionRef.current[field] === sequence,
          );

          if (currentFields.length > 0) {
            confirmedRef.current = mergeFields(
              confirmedRef.current,
              next,
              currentFields,
            );
            if (mountedRef.current) {
              setSettings((current) =>
                mergeFields(current, next, currentFields),
              );
            }
          }

          if (sequence === mutationVersionRef.current && mountedRef.current) {
            showSaved();
          }

          if (typeof window !== "undefined") {
            window.dispatchEvent(
              new CustomEvent<SettingsSnapshot>("codexbar:settings-updated", {
                detail: next,
              }),
            );
          }
        } catch {
          const currentFields = fields.filter(
            (field) => fieldVersionRef.current[field] === sequence,
          );
          if (currentFields.length > 0 && mountedRef.current) {
            setSettings((current) =>
              mergeFields(current, confirmedRef.current, currentFields),
            );
          }
          if (sequence === mutationVersionRef.current && mountedRef.current) {
            // The raw Rust error may include local paths or endpoint details.
            setSaveFeedback({ state: "error" });
          }
        } finally {
          pendingSaveCountRef.current -= 1;
          if (
            pendingSaveCountRef.current === 0 &&
            externalSyncRequestedRef.current
          ) {
            externalSyncRequestedRef.current = false;
            syncFromAnotherWindow();
          }
        }
      };

      const queued = saveQueueRef.current.then(save, save);
      saveQueueRef.current = queued.catch(() => {});
      return queued;
    },
    [clearSavedTimer, showSaved, syncFromAnotherWindow],
  );

  return {
    settings,
    saving: saveFeedback.state === "saving",
    saveFeedback,
    update,
  };
}
