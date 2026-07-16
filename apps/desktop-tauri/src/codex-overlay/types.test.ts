import { describe, expect, it } from "vitest";
import {
  formatCodexRateLimitLabel,
  getMascotQuotaInput,
  selectVisibleCodexQuotaWindows,
  type CodexQuotaLabelStrings,
  type CodexQuotaSnapshot,
  type CodexRateLimitWindow,
} from "./types";

const labels: CodexQuotaLabelStrings = {
  oneHour: "1小时额度",
  fiveHours: "5小时额度",
  twentyFourHours: "24小时额度",
  sevenDays: "7天额度",
  hourQuotaUnit: "小时额度",
  dayQuotaUnit: "天额度",
  generic: "Codex额度",
};

function window(
  id: string,
  usedPercent: number,
  duration?: number,
  limitName?: string,
): CodexRateLimitWindow {
  return {
    id,
    label: "server fallback",
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
    plan: "ChatGPT Plus",
    windows,
    status: "ready",
    source: "app-server",
    updatedAt: "2026-07-16T10:00:00Z",
    ...overrides,
  };
}

describe("dynamic Codex quota model", () => {
  it("formats trusted names and common duration labels", () => {
    expect(
      formatCodexRateLimitLabel(
        window("named:primary", 20, 300, "Codex Spark"),
        "chinese",
        labels,
      ),
    ).toBe("Codex Spark");
    expect(
      formatCodexRateLimitLabel(
        window("five:primary", 20, 300),
        "chinese",
        labels,
      ),
    ).toBe("5小时额度");
    expect(
      formatCodexRateLimitLabel(
        window("day:primary", 20, 1_440),
        "chinese",
        labels,
      ),
    ).toBe("24小时额度");
    expect(
      formatCodexRateLimitLabel(
        window("week:primary", 20, 10_080),
        "chinese",
        labels,
      ),
    ).toBe("7天额度");
  });

  it("does not guess an unknown duration as five-hour or weekly", () => {
    expect(
      formatCodexRateLimitLabel(
        window("unknown:primary", 20, 90),
        "chinese",
        labels,
      ),
    ).toBe("Codex额度");
    expect(
      formatCodexRateLimitLabel(window("none:primary", 20), "chinese", labels),
    ).toBe("Codex额度");
  });

  it("shows only the two tightest of three or more valid windows", () => {
    const selected = selectVisibleCodexQuotaWindows([
      window("comfortable:primary", 10, 300),
      window("critical:primary", 90, 1_440),
      window("tight:secondary", 70, 10_080),
    ]);

    expect(selected.windows.map((item) => item.id)).toEqual([
      "critical:primary",
      "tight:secondary",
    ]);
    expect(selected.additionalCount).toBe(1);
  });

  it("drops a missing or invalid percentage instead of drawing a fake quota", () => {
    const invalid = {
      ...window("invalid:primary", 20, 300),
      usedPercent: Number.NaN,
      remainingPercent: Number.NaN,
    };
    expect(selectVisibleCodexQuotaWindows([invalid]).windows).toEqual([]);
  });

  it("derives future mascot input from the tightest dynamic window", () => {
    const previous = snapshot([
      window("codex:primary", 80, 300),
      window("codex:secondary", 30, 10_080),
    ]);
    const currentPrimary = {
      ...window("codex:primary", 2, 300),
      resetsAt: "2026-07-24T08:00:00Z",
    };
    const current = snapshot([
      currentPrimary,
      window("codex:secondary", 30, 10_080),
    ]);

    expect(getMascotQuotaInput(current, previous)).toEqual({
      lowestRemainingPercent: 70,
      tightestWindowId: "codex:secondary",
      justReset: true,
      offlineOrInvalid: false,
    });
    expect(
      getMascotQuotaInput(
        snapshot([currentPrimary]),
        snapshot([previous.windows[0]]),
      ).justReset,
    ).toBe(true);
    expect(
      getMascotQuotaInput(snapshot([], { status: "offline" })).offlineOrInvalid,
    ).toBe(true);
  });
});
