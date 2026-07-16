import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CodexQuotaSnapshot } from "./types";

const apiMocks = vi.hoisted(() => ({
  readCodexRateLimits: vi.fn(),
}));

const tauriMocks = vi.hoisted(() => ({
  getSettingsSnapshot: vi.fn(),
  refreshProviders: vi.fn(),
}));

vi.mock("./api", () => apiMocks);
vi.mock("../lib/tauri", () => tauriMocks);

import { useCodexUsage } from "./useCodexUsage";

const snapshot: CodexQuotaSnapshot = {
  plan: "ChatGPT Plus",
  windows: [
    {
      id: "codex:primary",
      label: "7 day quota",
      usedPercent: 24,
      remainingPercent: 76,
      windowDurationMinutes: 10_080,
      level: "primary",
    },
  ],
  status: "ready",
  source: "app-server",
  updatedAt: "2026-07-16T10:00:00Z",
};

describe("useCodexUsage", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    apiMocks.readCodexRateLimits.mockResolvedValue(snapshot);
    tauriMocks.getSettingsSnapshot.mockResolvedValue({
      refreshIntervalSecs: 15,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("uses only the Codex command for initial, manual and automatic refresh", async () => {
    const { result } = renderHook(() => useCodexUsage());

    await act(async () => {});
    expect(apiMocks.readCodexRateLimits).toHaveBeenCalledTimes(1);
    expect(result.current.snapshot).toEqual(snapshot);

    await act(async () => {
      await result.current.refresh();
    });
    expect(apiMocks.readCodexRateLimits).toHaveBeenCalledTimes(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(15_000);
    });
    expect(apiMocks.readCodexRateLimits).toHaveBeenCalledTimes(3);
    expect(tauriMocks.refreshProviders).not.toHaveBeenCalled();
  });

  it("keeps the last good DTO as cached when the bridge rejects", async () => {
    const { result } = renderHook(() => useCodexUsage());
    await act(async () => {});

    apiMocks.readCodexRateLimits.mockRejectedValueOnce(
      new Error("bridge failed"),
    );
    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.snapshot?.status).toBe("cached");
    expect(result.current.snapshot?.source).toBe("cache");
    expect(result.current.snapshot?.windows[0].id).toBe("codex:primary");
  });

  it("does not install automatic polling when refresh is manual", async () => {
    tauriMocks.getSettingsSnapshot.mockResolvedValueOnce({
      refreshIntervalSecs: 0,
    });
    renderHook(() => useCodexUsage());
    await act(async () => {});

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(apiMocks.readCodexRateLimits).toHaveBeenCalledTimes(1);
  });
});
