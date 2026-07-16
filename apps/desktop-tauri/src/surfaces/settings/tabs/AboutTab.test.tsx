import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  getAppInfo: vi.fn(),
  openExternalUrl: vi.fn(),
}));

vi.mock("../../../lib/tauri", () => tauriMocks);
vi.mock("../../../hooks/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key }),
}));
import AboutTab from "./AboutTab";
import type { SettingsSnapshot } from "../../../types/bridge";

const settings: SettingsSnapshot = {
  enabledProviders: [],
  refreshIntervalSecs: 300,
  refreshAllProvidersOnMenuOpen: false,
  startAtLogin: false,
  startMinimized: false,
  showNotifications: true,
  soundEnabled: true,
  soundVolume: 100,
  highUsageThreshold: 70,
  criticalUsageThreshold: 90,
  predictivePaceWarningEnabled: false,
  trayIconMode: "single",
  switcherShowsIcons: true,
  menuBarShowsHighestUsage: true,
  menuBarShowsPercent: true,
  showAsUsed: false,
  showAllTokenAccountsInMenu: true,
  enableAnimations: true,
  resetTimeRelative: true,
  showResetWhenExhausted: false,
  menuBarDisplayMode: "compact",
  hidePersonalInfo: false,
  autoDownloadUpdates: false,
  installUpdatesOnQuit: false,
  globalShortcut: "",
  codexCustomSessionsDirs: [],
  updateChannel: "stable",
  uiLanguage: "english",
  theme: "dark",
  windowScalePercent: 125,
  trayScalePercent: 100,
  powertoysStatusPipeEnabled: false,
  claudeAvoidKeychainPrompts: true,
  codexSparkUsageVisible: true,
  disableKeychainAccess: false,
  providerMetrics: {},
  floatBarEnabled: false,
  floatBarOpacity: 0.9,
  floatBarScale: 100,
  floatBarOrientation: "horizontal",
  floatBarStyle: "floating",
  floatBarClickThrough: false,
  floatBarProviderIds: [],
  floatBarDarkText: false,
  floatBarShowResetInline: false,
  floatBarShowCost: false,
};

describe("AboutTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    tauriMocks.getAppInfo.mockResolvedValue({
      name: "QuackQuota",
      version: "0.30.3",
      buildNumber: "dev",
      updateChannel: "stable",
      tagline: "A lightweight Codex quota companion for Windows",
    });
    tauriMocks.openExternalUrl.mockResolvedValue(undefined);
  });

  it("opens about links through the Tauri URL bridge", async () => {
    render(<AboutTab settings={settings} set={vi.fn()} saving={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "AboutLinkOriginalProject" }),
    );

    expect(tauriMocks.openExternalUrl).toHaveBeenCalledTimes(1);
    expect(tauriMocks.openExternalUrl).toHaveBeenCalledWith(
      "https://github.com/steipete/CodexBar",
    );
  });

  it("renders the QuackQuota product identity and non-official notice", async () => {
    render(<AboutTab settings={settings} set={vi.fn()} saving={false} />);

    expect(await screen.findByText("QuackQuota")).toBeInTheDocument();
    expect(screen.getByText("AboutNonOfficial")).toBeInTheDocument();
    expect(screen.queryByText("AboutCheckForUpdates")).not.toBeInTheDocument();
  });

  it("shows a link error if the OS browser launch fails", async () => {
    tauriMocks.openExternalUrl.mockRejectedValue("no browser");

    render(<AboutTab settings={settings} set={vi.fn()} saving={false} />);

    fireEvent.click(
      await screen.findByRole("button", { name: "AboutLinkOriginalProject" }),
    );

    await waitFor(() => {
      expect(screen.getByText("ErrorPrefix no browser")).toBeInTheDocument();
    });
  });
});
