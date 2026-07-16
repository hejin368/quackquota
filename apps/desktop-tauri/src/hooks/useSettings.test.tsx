import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type SettingsEvent = { payload?: { sourceWindowLabel?: string | null } };

const eventMocks = vi.hoisted(() => {
  const listeners: Record<string, (event?: SettingsEvent) => void> = {};
  return {
    listeners,
    listen: vi.fn((name: string, callback: (event?: SettingsEvent) => void) => {
      listeners[name] = callback;
      return Promise.resolve(() => {
        delete listeners[name];
      });
    }),
  };
});

const tauriMocks = vi.hoisted(() => ({
  getSettingsSnapshot: vi.fn(),
  updateSettings: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => eventMocks);
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: "settings" }),
}));
vi.mock("../lib/tauri", () => tauriMocks);

import { useSettings } from "./useSettings";
import type { SettingsSnapshot } from "../types/bridge";

const snapshot = (windowScalePercent = 100, trayScalePercent = 100) =>
  ({ windowScalePercent, trayScalePercent }) as SettingsSnapshot;

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe("useSettings optimistic persistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.keys(eventMocks.listeners).forEach((name) => {
      delete eventMocks.listeners[name];
    });
    tauriMocks.getSettingsSnapshot.mockResolvedValue(snapshot());
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps optimistic settings visible while a save is pending", async () => {
    const pending = deferred<SettingsSnapshot>();
    tauriMocks.updateSettings.mockReturnValueOnce(pending.promise);
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));

    await act(async () => {
      void result.current.update({ windowScalePercent: 150 });
    });

    expect(result.current.settings.windowScalePercent).toBe(150);
    expect(result.current.saveFeedback).toEqual({ state: "saving" });

    await act(async () => {
      pending.resolve(snapshot(150));
    });

    await waitFor(() =>
      expect(result.current.saveFeedback).toEqual({ state: "saved" }),
    );
    expect(result.current.settings.windowScalePercent).toBe(150);
  });

  it("does not re-read the complete snapshot for its own settings event", async () => {
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));
    await waitFor(() =>
      expect(eventMocks.listeners["settings-changed"]).toBeTypeOf("function"),
    );
    const readsBeforeOwnEvent =
      tauriMocks.getSettingsSnapshot.mock.calls.length;

    act(() => {
      eventMocks.listeners["settings-changed"]({
        payload: { sourceWindowLabel: "settings" },
      });
    });

    expect(tauriMocks.getSettingsSnapshot).toHaveBeenCalledTimes(
      readsBeforeOwnEvent,
    );
    expect(result.current.settings.windowScalePercent).toBe(100);
  });

  it("syncs an event from another window when no local save is pending", async () => {
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));
    await waitFor(() =>
      expect(eventMocks.listeners["settings-changed"]).toBeTypeOf("function"),
    );
    tauriMocks.getSettingsSnapshot.mockResolvedValueOnce(snapshot(175));

    await act(async () => {
      eventMocks.listeners["settings-changed"]({
        payload: { sourceWindowLabel: "main" },
      });
    });

    await waitFor(() =>
      expect(result.current.settings.windowScalePercent).toBe(175),
    );
  });

  it("does not let an older same-field response overwrite a newer choice", async () => {
    const first = deferred<SettingsSnapshot>();
    const second = deferred<SettingsSnapshot>();
    tauriMocks.updateSettings
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));

    await act(async () => {
      void result.current.update({ windowScalePercent: 120 });
      void result.current.update({ windowScalePercent: 150 });
    });
    expect(result.current.settings.windowScalePercent).toBe(150);

    await act(async () => {
      first.resolve(snapshot(120));
    });
    await waitFor(() =>
      expect(tauriMocks.updateSettings).toHaveBeenCalledTimes(2),
    );
    expect(result.current.settings.windowScalePercent).toBe(150);

    await act(async () => {
      second.resolve(snapshot(150));
    });
    await waitFor(() =>
      expect(result.current.settings.windowScalePercent).toBe(150),
    );
  });

  it("rolls back only the failed field and preserves another successful field", async () => {
    const first = deferred<SettingsSnapshot>();
    tauriMocks.updateSettings
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce(snapshot(100, 150));
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));

    await act(async () => {
      void result.current.update({ windowScalePercent: 150 });
      void result.current.update({ trayScalePercent: 150 });
    });
    expect(result.current.settings).toMatchObject({
      windowScalePercent: 150,
      trayScalePercent: 150,
    });

    await act(async () => {
      first.reject(new Error("backend detail must not reach UI"));
    });
    await waitFor(() =>
      expect(tauriMocks.updateSettings).toHaveBeenCalledTimes(2),
    );
    await waitFor(() =>
      expect(result.current.settings).toMatchObject({
        windowScalePercent: 100,
        trayScalePercent: 150,
      }),
    );
  });

  it("keeps a failed save visible and rolls its field back to the confirmed value", async () => {
    tauriMocks.updateSettings.mockRejectedValueOnce(
      new Error("sensitive error"),
    );
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));

    await act(async () => {
      await result.current.update({ windowScalePercent: 150 });
    });

    expect(result.current.settings.windowScalePercent).toBe(100);
    expect(result.current.saveFeedback).toEqual({ state: "error" });
  });

  it("shows saved feedback briefly, then fades it without reloading settings", async () => {
    vi.useFakeTimers();
    tauriMocks.updateSettings.mockResolvedValueOnce(snapshot(150));
    const initial = snapshot();
    const { result } = renderHook(() => useSettings(initial));

    await act(async () => {
      await result.current.update({ windowScalePercent: 150 });
    });
    expect(result.current.saveFeedback).toEqual({ state: "saved" });

    await act(async () => {
      vi.advanceTimersByTime(1_500);
    });
    expect(result.current.saveFeedback).toEqual({ state: "idle" });
  });

  it("unsubscribes the listener on unmount", async () => {
    const unlisten = vi.fn();
    eventMocks.listen.mockResolvedValueOnce(unlisten);
    const initial = snapshot();
    const { unmount } = renderHook(() => useSettings(initial));
    await waitFor(() => expect(eventMocks.listen).toHaveBeenCalled());

    unmount();
    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });
});
