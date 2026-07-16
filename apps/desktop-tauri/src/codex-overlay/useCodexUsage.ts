import { useCallback, useEffect, useRef, useState } from "react";
import { getSettingsSnapshot } from "../lib/tauri";
import { readCodexRateLimits } from "./api";
import type { CodexQuotaSnapshot } from "./types";

const DEFAULT_REFRESH_INTERVAL_MS = 60_000;
const MINIMUM_REFRESH_INTERVAL_MS = 15_000;

export interface UseCodexUsageResult {
  snapshot: CodexQuotaSnapshot | null;
  isRefreshing: boolean;
  refresh: () => Promise<void>;
}

/**
 * Fetch the dedicated Codex quota DTO. This hook deliberately does not use
 * `useProviders` or `refresh_providers`, so no non-Codex credential source is
 * loaded when the overlay opens or refreshes.
 */
export function useCodexUsage(): UseCodexUsageResult {
  const [snapshot, setSnapshot] = useState<CodexQuotaSnapshot | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(true);
  const mountedRef = useRef(true);
  const inFlightRef = useRef<Promise<void> | null>(null);

  const refresh = useCallback((): Promise<void> => {
    if (inFlightRef.current) return inFlightRef.current;
    if (mountedRef.current) setIsRefreshing(true);

    const request = readCodexRateLimits()
      .then((next) => {
        if (mountedRef.current) setSnapshot(next);
      })
      .catch(() => {
        if (!mountedRef.current) return;
        setSnapshot((current) => {
          if (current?.windows.length) {
            return {
              ...current,
              status: "cached",
              source: "cache",
              message: "Showing the last successful Codex quota snapshot.",
            };
          }
          return {
            windows: [],
            status: "provider-error",
            source: "legacy-provider",
            updatedAt: new Date().toISOString(),
            message: "Codex quota provider failed.",
          };
        });
      })
      .finally(() => {
        inFlightRef.current = null;
        if (mountedRef.current) setIsRefreshing(false);
      });
    inFlightRef.current = request;
    return request;
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    void refresh();

    let intervalId: number | null = null;
    let cancelled = false;
    const installInterval = (intervalMs: number) => {
      if (cancelled) return;
      intervalId = window.setInterval(() => void refresh(), intervalMs);
    };

    getSettingsSnapshot()
      .then((settings) => {
        const configured = settings.refreshIntervalSecs * 1_000;
        if (configured <= 0) return;
        installInterval(
          Number.isFinite(configured)
            ? Math.max(MINIMUM_REFRESH_INTERVAL_MS, configured)
            : DEFAULT_REFRESH_INTERVAL_MS,
        );
      })
      .catch(() => installInterval(DEFAULT_REFRESH_INTERVAL_MS));

    return () => {
      cancelled = true;
      mountedRef.current = false;
      if (intervalId !== null) window.clearInterval(intervalId);
    };
  }, [refresh]);

  useEffect(() => {
    if (!snapshot || !["ready", "cached"].includes(snapshot.status)) return;
    const now = Date.now();
    const nextReset = snapshot.windows
      .map((window) => Date.parse(window.resetsAt ?? ""))
      .filter((value) => Number.isFinite(value) && value > now)
      .sort((left, right) => left - right)[0];
    if (!nextReset) return;

    const delay = Math.min(nextReset - now + 1_000, 2_147_000_000);
    const timeoutId = window.setTimeout(() => void refresh(), delay);
    return () => window.clearTimeout(timeoutId);
  }, [refresh, snapshot]);

  return { snapshot, isRefreshing, refresh };
}
