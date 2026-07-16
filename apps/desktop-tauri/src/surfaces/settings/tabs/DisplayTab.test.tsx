import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("../../../hooks/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key, language: "english" }),
}));
// The FloatBar section pulls in its own bridge dependencies; it is irrelevant
// to the window-scale control under test.
vi.mock("../../../floatbar", () => ({
  FloatBarSettingsSection: () => null,
}));

import DisplayTab from "./DisplayTab";
import type { SettingsSnapshot } from "../../../types/bridge";

const baseSettings = {
  trayIconMode: "single",
  switcherShowsIcons: false,
  menuBarShowsHighestUsage: false,
  menuBarShowsPercent: false,
  menuBarDisplayMode: "detailed",
  windowScalePercent: 100,
  showAsUsed: false,
  showAllTokenAccountsInMenu: false,
  resetTimeRelative: false,
  showResetWhenExhausted: false,
} as unknown as SettingsSnapshot;

function renderTab(set: (patch: Record<string, unknown>) => void) {
  return render(
    <DisplayTab settings={baseSettings} set={set as never} saving={false} />,
  );
}

describe("DisplayTab window scale", () => {
  it("commits the new window scale on blur", () => {
    const set = vi.fn();
    renderTab(set);
    const slider = screen.getByRole("slider", { name: "WindowScaleAriaLabel" });

    fireEvent.change(slider, { target: { value: "175" } });
    fireEvent.blur(slider);

    expect(set).toHaveBeenCalledWith({ windowScalePercent: 175 });
  });

  it("previews slider movement locally and persists only when the interaction ends", () => {
    const set = vi.fn();
    renderTab(set);
    const slider = screen.getByRole("slider", { name: "WindowScaleAriaLabel" });

    act(() => {
      fireEvent.change(slider, { target: { value: "125" } });
      fireEvent.change(slider, { target: { value: "150" } });
    });

    expect(set).not.toHaveBeenCalled();

    fireEvent.pointerUp(slider);
    expect(set).toHaveBeenCalledTimes(1);
    expect(set).toHaveBeenCalledWith({ windowScalePercent: 150 });
  });

  it("does not commit when the value is unchanged", () => {
    const set = vi.fn();
    renderTab(set);
    const slider = screen.getByRole("slider", { name: "WindowScaleAriaLabel" });

    fireEvent.change(slider, { target: { value: "100" } });
    fireEvent.blur(slider);

    expect(set).not.toHaveBeenCalled();
  });

  it("updates the exhausted reset display preference", () => {
    const set = vi.fn();
    renderTab(set);

    fireEvent.click(
      screen.getByRole("checkbox", { name: "ShowResetWhenExhausted" }),
    );

    expect(set).toHaveBeenCalledWith({ showResetWhenExhausted: true });
  });
});
