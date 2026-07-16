import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LocaleProvider } from "../i18n/LocaleProvider";
import { buildBundle } from "../test/localeHarness";
import type { CodexQuotaSnapshot, CodexRateLimitWindow } from "./types";

const tauriMocks = vi.hoisted(() => ({
  getLocaleStrings: vi.fn(),
  setUiLanguage: vi.fn(),
}));

const eventMocks = vi.hoisted(() => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("../lib/tauri", () => tauriMocks);
vi.mock("@tauri-apps/api/event", () => eventMocks);

import { CodexOverlayView } from "./CodexOverlay";

function quotaWindow(
  id: string,
  usedPercent: number,
  duration: number,
  limitName?: string,
): CodexRateLimitWindow {
  return {
    id,
    label: "fallback",
    usedPercent,
    remainingPercent: 100 - usedPercent,
    windowDurationMinutes: duration,
    resetsAt: "2026-07-23T08:00:00Z",
    limitId: id.split(":")[0],
    limitName,
    level: id.endsWith("secondary") ? "secondary" : "primary",
  };
}

function snapshot(
  windows: CodexRateLimitWindow[],
  overrides: Partial<CodexQuotaSnapshot> = {},
): CodexQuotaSnapshot {
  return {
    status: "ready",
    plan: "ChatGPT Plus",
    windows,
    source: "app-server",
    updatedAt: "2026-07-16T10:00:00Z",
    ...overrides,
  };
}

function renderOverlay(value: CodexQuotaSnapshot | null, refreshing = false) {
  return render(
    <LocaleProvider>
      <CodexOverlayView
        snapshot={value}
        isRefreshing={refreshing}
        onRefresh={() => {}}
        onClose={() => {}}
      />
    </LocaleProvider>,
  );
}

describe("CodexOverlayView", () => {
  beforeEach(() => {
    tauriMocks.getLocaleStrings.mockResolvedValue(
      buildBundle({
        CodexOverlayTitle: "Codex Usage",
        CodexOverlayCachedData: "Cached data",
        CodexOverlayNoQuotaData: "No quota data",
        CodexOverlayCliNotInstalled: "CLI not installed",
        CodexOverlayOffline: "Offline",
        CodexOverlayInvalidData: "Invalid quota data",
        CodexOverlayOneHourQuota: "1 hour quota",
        CodexOverlayFiveHourQuota: "5 hour quota",
        CodexOverlayTwentyFourHourQuota: "24 hour quota",
        CodexOverlaySevenDayQuota: "7 day quota",
        CodexOverlayHourQuotaUnit: "hour quota",
        CodexOverlayDayQuotaUnit: "day quota",
        CodexOverlayGenericQuota: "Codex quota",
        CodexOverlayResetAt: "Reset at",
        CodexOverlayResetCredits: "Available resets",
        CodexOverlayAdditionalPrefix: "Plus",
        CodexOverlayAdditionalSuffix: "more quota item(s)",
        ProviderPlan: "Plan",
        PanelUsedSuffix: "used",
        FloatBarRemainingSuffix: "remaining",
        DataSource: "Data source",
        LastUpdated: "Updated",
        StateLoadingProviders: "Loading providers",
        ProviderNotSignedIn: "Not signed in",
        StatusUnableToGetUsage: "Unable to get usage",
        ActionRefresh: "Refresh",
        ActionClose: "Close",
      }),
    );
  });

  it("renders a weekly-only response as one full-width 7-day card", async () => {
    const { container } = renderOverlay(
      snapshot([quotaWindow("codex:primary", 24, 10_080)]),
    );

    await waitFor(() =>
      expect(screen.getByText("7 day quota")).toBeInTheDocument(),
    );
    expect(container.querySelectorAll(".codex-overlay__quota")).toHaveLength(1);
    expect(
      container.querySelector(".codex-overlay__quotas--single"),
    ).not.toBeNull();
    expect(screen.queryByText("5 hour quota")).not.toBeInTheDocument();
    expect(screen.getByText("76% remaining")).toBeInTheDocument();
    expect(screen.getByTestId("quota-progress-codex:primary")).toHaveStyle({
      width: "76%",
    });
  });

  it("renders two different valid periods as two cards", async () => {
    const { container } = renderOverlay(
      snapshot([
        quotaWindow("codex:primary", 20, 300),
        quotaWindow("codex:secondary", 40, 10_080),
      ]),
    );

    await screen.findByText("5 hour quota");
    expect(screen.getByText("7 day quota")).toBeInTheDocument();
    expect(container.querySelectorAll(".codex-overlay__quota")).toHaveLength(2);
  });

  it("shows only the two tightest windows and reports the remainder", async () => {
    renderOverlay(
      snapshot([
        quotaWindow("comfortable:primary", 10, 300, "Comfortable"),
        quotaWindow("critical:primary", 90, 1_440, "Critical"),
        quotaWindow("tight:primary", 70, 10_080, "Tight"),
      ]),
    );

    await screen.findByText("Critical");
    expect(screen.getByText("Tight")).toBeInTheDocument();
    expect(screen.queryByText("Comfortable")).not.toBeInTheDocument();
    expect(screen.getByText("Plus 1 more quota item(s)")).toBeInTheDocument();
  });

  it("does not draw a misleading progress bar for an invalid percentage", async () => {
    const invalid = {
      ...quotaWindow("invalid:primary", 20, 300),
      usedPercent: Number.NaN,
      remainingPercent: Number.NaN,
    };
    renderOverlay(snapshot([invalid]));

    await screen.findByText("No quota data");
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("renders loading, not-installed and cached states", async () => {
    const { rerender } = renderOverlay(null, true);
    await screen.findByText("Loading providers");

    rerender(
      <LocaleProvider>
        <CodexOverlayView
          snapshot={snapshot([], { status: "not-installed" })}
          isRefreshing={false}
          onRefresh={() => {}}
          onClose={() => {}}
        />
      </LocaleProvider>,
    );
    expect(await screen.findByText("CLI not installed")).toBeInTheDocument();

    rerender(
      <LocaleProvider>
        <CodexOverlayView
          snapshot={snapshot([quotaWindow("codex:primary", 24, 10_080)], {
            status: "cached",
            source: "cache",
          })}
          isRefreshing={false}
          onRefresh={() => {}}
          onClose={() => {}}
        />
      </LocaleProvider>,
    );
    expect(await screen.findByText("Cached data")).toBeInTheDocument();
  });
});
