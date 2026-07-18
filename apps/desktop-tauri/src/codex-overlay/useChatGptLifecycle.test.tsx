import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const lifecycleMocks = vi.hoisted(() => ({
  getStatus: vi.fn(),
  diagnosticsEnabled: vi.fn(),
  recordDiagnostic: vi.fn(),
  listen: vi.fn(),
  listener: undefined as undefined | ((event: { payload: unknown }) => void),
}));

vi.mock("./api", () => ({
  getChatGptLifecycleStatus: lifecycleMocks.getStatus,
  lifecycleDiagnosticsEnabled: lifecycleMocks.diagnosticsEnabled,
  recordChatGptLifecycleFrontendDiagnostic: lifecycleMocks.recordDiagnostic,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: lifecycleMocks.listen,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ label: "codex-overlay" }),
}));

import {
  CHATGPT_LIFECYCLE_STATUS_EVENT,
  isChatGptDesktopSnapshot,
  useChatGptLifecycle,
} from "./useChatGptLifecycle";

function LifecycleProbe({ name }: { name: string }) {
  const snapshot = useChatGptLifecycle();
  return <output data-testid={name}>{snapshot.status}</output>;
}

describe("useChatGptLifecycle", () => {
  beforeEach(() => {
    lifecycleMocks.getStatus.mockReset();
    lifecycleMocks.diagnosticsEnabled.mockReset();
    lifecycleMocks.recordDiagnostic.mockReset();
    lifecycleMocks.listen.mockReset();
    lifecycleMocks.listener = undefined;
    lifecycleMocks.getStatus.mockResolvedValue({ status: "running" });
    lifecycleMocks.diagnosticsEnabled.mockResolvedValue(true);
    lifecycleMocks.recordDiagnostic.mockResolvedValue(undefined);
    lifecycleMocks.listen.mockImplementation(
      async (
        eventName: string,
        listener: (event: { payload: unknown }) => void,
      ) => {
        expect(eventName).toBe(CHATGPT_LIFECYCLE_STATUS_EVENT);
        lifecycleMocks.listener = listener;
        return () => {};
      },
    );
  });

  it("uses the current command result when the first event was missed", async () => {
    render(<LifecycleProbe name="overlay" />);

    await waitFor(() =>
      expect(screen.getByTestId("overlay")).toHaveTextContent("running"),
    );
  });

  it("preserves a valid command result when event subscription fails", async () => {
    lifecycleMocks.listen.mockRejectedValueOnce(
      new Error("bridge unavailable"),
    );
    render(<LifecycleProbe name="settings" />);

    await waitFor(() =>
      expect(screen.getByTestId("settings")).toHaveTextContent("running"),
    );
  });

  it("recovers from an initial command failure when a later valid event arrives", async () => {
    lifecycleMocks.getStatus.mockRejectedValueOnce(
      new Error("command unavailable"),
    );
    render(<LifecycleProbe name="overlay" />);

    await waitFor(() => expect(lifecycleMocks.listener).toBeTypeOf("function"));
    expect(screen.getByTestId("overlay")).toHaveTextContent("unavailable");

    act(() => {
      lifecycleMocks.listener?.({ payload: { status: "not-running" } });
    });
    expect(screen.getByTestId("overlay")).toHaveTextContent("not-running");
  });

  it("updates both settings and overlay consumers from the same DTO shape", async () => {
    lifecycleMocks.getStatus.mockResolvedValue({
      status: "monitoring-disabled",
    });
    render(
      <>
        <LifecycleProbe name="overlay" />
        <LifecycleProbe name="settings" />
      </>,
    );

    await waitFor(() => {
      expect(screen.getByTestId("overlay")).toHaveTextContent(
        "monitoring-disabled",
      );
      expect(screen.getByTestId("settings")).toHaveTextContent(
        "monitoring-disabled",
      );
    });
  });

  it("accepts the serialized statuses and rejects wrapped or malformed payloads", () => {
    expect(isChatGptDesktopSnapshot({ status: "running" })).toBe(true);
    expect(isChatGptDesktopSnapshot({ status: "not-running" })).toBe(true);
    expect(isChatGptDesktopSnapshot({ status: "unsupported" })).toBe(true);
    expect(isChatGptDesktopSnapshot({ status: "unavailable" })).toBe(true);
    expect(isChatGptDesktopSnapshot({ status: "monitoring-disabled" })).toBe(
      true,
    );
    expect(isChatGptDesktopSnapshot("running")).toBe(false);
    expect(isChatGptDesktopSnapshot({ snapshot: "running" })).toBe(false);
    expect(isChatGptDesktopSnapshot({ status: "unknown" })).toBe(false);
  });

  it("records only safe categories when IPC or event setup fails", async () => {
    lifecycleMocks.getStatus.mockRejectedValueOnce(
      new Error("permission denied"),
    );
    lifecycleMocks.listen.mockRejectedValueOnce(new Error("permission denied"));
    render(<LifecycleProbe name="overlay" />);

    await waitFor(() => {
      expect(lifecycleMocks.recordDiagnostic).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "frontend-initial-invoke-error",
          errorCategory: "invoke-permission-denied",
          windowLabel: "codex-overlay",
        }),
      );
      expect(lifecycleMocks.recordDiagnostic).toHaveBeenCalledWith(
        expect.objectContaining({
          event: "frontend-event-listen-error",
          errorCategory: "event-listen-permission-denied",
          windowLabel: "codex-overlay",
        }),
      );
    });
  });
});
