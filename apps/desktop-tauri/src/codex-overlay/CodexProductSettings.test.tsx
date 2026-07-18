import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SettingsSnapshot } from "../types/bridge";
import CodexProductSettings, {
  isValidManualCodexProxy,
} from "./CodexProductSettings";
import { connectionErrorLocaleKey } from "./errors";

const { locale } = vi.hoisted(() => ({ locale: { current: "zh-CN" } }));

const translations: Record<string, Record<string, string>> = {
  "zh-CN": {
    CodexOverlaySettings: "CODEX 悬浮窗",
    CodexOverlayStartupBehavior: "启动时显示悬浮窗",
    CodexOverlayStartupRemember: "记住上次状态",
    CodexOverlayStartupAlwaysShow: "始终显示",
    CodexOverlayStartupAlwaysHide: "始终隐藏",
    CodexOverlayResetPosition: "恢复悬浮窗默认位置",
    CodexOverlayResetPositionHelper: "恢复到默认位置",
    CodexNetworkSettings: "Codex 网络",
    CodexUseEnvironmentProxy: "使用系统/环境代理",
    CodexUseEnvironmentProxyHelper: "使用环境变量代理",
    CodexManualProxy: "手动代理地址",
    CodexManualProxyHelper: "仅支持 HTTP/HTTPS",
    CodexProxyCurrentRoute: "当前连接方式",
    CodexProxyRouteDirect: "直接连接",
    CodexProxyTest: "测试连接",
    CodexProxyTesting: "正在测试",
    CodexProxyClear: "清空代理",
  },
  en: {
    CodexOverlaySettings: "CODEX Overlay",
    CodexOverlayStartupBehavior: "Show overlay at startup",
    CodexOverlayStartupRemember: "Remember last state",
    CodexOverlayStartupAlwaysShow: "Always show",
    CodexOverlayStartupAlwaysHide: "Always hide",
    CodexOverlayResetPosition: "Reset overlay position",
    CodexOverlayResetPositionHelper: "Restore the default position",
    CodexNetworkSettings: "Codex network",
    CodexUseEnvironmentProxy: "Use system/environment proxy",
    CodexUseEnvironmentProxyHelper: "Use environment proxy variables",
    CodexManualProxy: "Manual proxy address",
    CodexManualProxyHelper: "HTTP/HTTPS only",
    CodexProxyCurrentRoute: "Current connection route",
    CodexProxyRouteDirect: "Direct connection",
    CodexProxyTest: "Test connection",
    CodexProxyTesting: "Testing",
    CodexProxyClear: "Clear proxy",
    ChatGptMonitorDesktop: "Monitor ChatGPT desktop application",
    ChatGptMonitorDesktopHelper:
      "Only monitors the Windows ChatGPT desktop application",
    ChatGptShowOverlayOnStart: "Automatically show overlay when ChatGPT starts",
    ChatGptHideOverlayOnExit: "Automatically hide overlay when ChatGPT closes",
    ChatGptDesktopLifecycle: "ChatGPT desktop application status",
    ChatGptLifecycleRunning: "Running",
  },
};

vi.mock("../hooks/useLocale", () => ({
  useLocale: () => ({
    t: (key: string) => translations[locale.current]?.[key] ?? key,
  }),
}));

vi.mock("./api", () => ({
  getCodexProxyStatus: vi.fn(() => new Promise<never>(() => {})),
  getChatGptLifecycleStatus: vi.fn().mockResolvedValue({ status: "running" }),
  resetCodexOverlayPosition: vi.fn().mockResolvedValue(undefined),
  testCodexProxyConnection: vi.fn().mockResolvedValue({
    ok: true,
    source: "direct",
  }),
}));

vi.mock("./useChatGptLifecycle", () => ({
  useChatGptLifecycle: () => ({ status: "running" }),
}));

const settings: SettingsSnapshot = {
  enabledProviders: [],
  refreshIntervalSecs: 300,
  refreshAllProvidersOnMenuOpen: false,
  startAtLogin: false,
  startMinimized: false,
  codexProxyUseEnvironment: true,
  codexManualProxy: "",
  codexOverlayStartupMode: "rememberLast",
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
  updateChannel: "stable",
  autoDownloadUpdates: false,
  installUpdatesOnQuit: false,
  globalShortcut: "",
  codexCustomSessionsDirs: [],
  uiLanguage: "chinese",
  theme: "dark",
  windowScalePercent: 100,
  trayScalePercent: 100,
  powertoysStatusPipeEnabled: false,
  claudeAvoidKeychainPrompts: false,
  codexSparkUsageVisible: true,
  disableKeychainAccess: false,
  providerMetrics: {},
  floatBarEnabled: false,
  floatBarOpacity: 80,
  floatBarScale: 100,
  floatBarOrientation: "horizontal",
  floatBarStyle: "floating",
  floatBarClickThrough: false,
  floatBarProviderIds: [],
  floatBarDarkText: false,
  floatBarShowResetInline: false,
  floatBarShowCost: false,
};

function renderSettings(
  mode: NonNullable<
    SettingsSnapshot["codexOverlayStartupMode"]
  > = "rememberLast",
) {
  const set = vi.fn().mockResolvedValue(undefined);
  const rendered = render(
    <CodexProductSettings
      settings={{ ...settings, codexOverlayStartupMode: mode }}
      set={set}
      saving={false}
    />,
  );
  return { ...rendered, set };
}

describe("Codex proxy settings helpers", () => {
  it("accepts HTTP and HTTPS proxy URLs without credentials", () => {
    expect(isValidManualCodexProxy("http://proxy.example:8080")).toBe(true);
    expect(isValidManualCodexProxy("https://proxy.example")).toBe(true);
    expect(isValidManualCodexProxy("")).toBe(true);
  });

  it("rejects unsupported schemes and embedded credentials", () => {
    expect(isValidManualCodexProxy("socks5://proxy.example:1080")).toBe(false);
    expect(
      isValidManualCodexProxy("http://user:secret@proxy.example:8080"),
    ).toBe(false);
    expect(isValidManualCodexProxy("http://proxy.example?q=secret")).toBe(
      false,
    );
  });

  it("maps sanitized backend failures to user-facing locale keys", () => {
    expect(connectionErrorLocaleKey("dns-or-network")).toBe(
      "CodexProxyErrorDnsOrNetwork",
    );
    expect(connectionErrorLocaleKey("proxy-connection")).toBe(
      "CodexProxyErrorProxyConnection",
    );
    expect(connectionErrorLocaleKey("not-logged-in")).toBe(
      "CodexProxyErrorNotLoggedIn",
    );
  });
});

describe("Codex overlay startup radio group", () => {
  it("renders all three complete Chinese choices without a dropdown", () => {
    locale.current = "zh-CN";
    renderSettings();

    expect(
      screen.getByRole("radiogroup", { name: "启动时显示悬浮窗" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "记住上次状态" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "始终显示" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "始终隐藏" })).toBeInTheDocument();
    expect(
      screen.queryByRole("combobox", { name: "启动时显示悬浮窗" }),
    ).not.toBeInTheDocument();
  });

  it("renders all complete English choices", () => {
    locale.current = "en";
    renderSettings();

    expect(
      screen.getByRole("radio", { name: "Remember last state" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("radio", { name: "Always show" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("radio", { name: "Always hide" }),
    ).toBeInTheDocument();
  });

  it.each([
    ["rememberLast", "记住上次状态"],
    ["alwaysShow", "始终显示"],
    ["alwaysHide", "始终隐藏"],
  ] as const)("writes the selected %s mode immediately", (value, label) => {
    locale.current = "zh-CN";
    const initialMode =
      value === "rememberLast" ? "alwaysShow" : "rememberLast";
    const { set } = renderSettings(initialMode);

    fireEvent.click(screen.getByRole("radio", { name: label }));
    expect(set).toHaveBeenCalledWith({ codexOverlayStartupMode: value });
  });

  it.each([
    ["rememberLast", "记住上次状态"],
    ["alwaysShow", "始终显示"],
    ["alwaysHide", "始终隐藏"],
  ] as const)("restores %s as checked after settings reload", (mode, label) => {
    locale.current = "zh-CN";
    renderSettings(mode);

    const radio = screen.getByRole("radio", { name: label });
    expect(radio).toBeChecked();
    expect(radio.closest("label")).toHaveClass(
      "codex-startup-option--selected",
    );
  });

  it("keeps reset-position and proxy controls available", () => {
    locale.current = "zh-CN";
    renderSettings();

    expect(
      screen.getByRole("button", { name: "恢复悬浮窗默认位置" }),
    ).toBeInTheDocument();
    expect(screen.getByText("使用系统/环境代理")).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("http://proxy.example:8080"),
    ).toBeInTheDocument();
  });

  it("does not persist every manual proxy keystroke", () => {
    locale.current = "en";
    const { set } = renderSettings();
    const input = screen.getByPlaceholderText("http://proxy.example:8080");

    fireEvent.change(input, { target: { value: "http://proxy.example:8080" } });
    fireEvent.change(input, { target: { value: "http://proxy.example:8081" } });
    expect(set).not.toHaveBeenCalled();

    fireEvent.blur(input);
    expect(set).toHaveBeenCalledTimes(1);
    expect(set).toHaveBeenCalledWith({
      codexManualProxy: "http://proxy.example:8081",
    });
  });
});

describe("ChatGPT desktop lifecycle settings", () => {
  it("renders and persists three independent lifecycle toggles", () => {
    locale.current = "en";
    const { set } = renderSettings();

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: "Monitor ChatGPT desktop application",
      }),
    );
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: "Automatically show overlay when ChatGPT starts",
      }),
    );
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: "Automatically hide overlay when ChatGPT closes",
      }),
    );

    expect(set).toHaveBeenCalledWith({ monitorChatgptDesktop: false });
    expect(set).toHaveBeenCalledWith({ showOverlayOnChatgptStart: false });
    expect(set).toHaveBeenCalledWith({ hideOverlayOnChatgptExit: false });
  });
});
