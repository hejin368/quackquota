import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import SettingsSaveFeedback from "./SettingsSaveFeedback";

const labels = {
  savingLabel: "保存中…",
  savedLabel: "已保存",
  failedLabel: "保存失败，请重试。",
};

describe("SettingsSaveFeedback", () => {
  it("keeps one stable live-region structure for every feedback state", () => {
    const { rerender } = render(
      <SettingsSaveFeedback feedback={{ state: "idle" }} {...labels} />,
    );
    const region = screen.getByRole("status");
    expect(region).toHaveClass("settings-save-feedback");
    expect(region).toHaveAttribute("aria-live", "polite");
    expect(region).toHaveAttribute("data-state", "idle");

    rerender(
      <SettingsSaveFeedback feedback={{ state: "saving" }} {...labels} />,
    );
    expect(screen.getByRole("status")).toBe(region);
    expect(region).toHaveTextContent("保存中…");

    rerender(
      <SettingsSaveFeedback feedback={{ state: "saved" }} {...labels} />,
    );
    expect(region).toHaveTextContent("已保存");

    rerender(
      <SettingsSaveFeedback feedback={{ state: "error" }} {...labels} />,
    );
    expect(region).toHaveTextContent("保存失败，请重试。");
  });

  it("accepts complete English feedback copy", () => {
    render(
      <SettingsSaveFeedback
        feedback={{ state: "error" }}
        savingLabel="Saving…"
        savedLabel="Saved"
        failedLabel="Could not save this setting. Try again."
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent(
      "Could not save this setting. Try again.",
    );
  });
});
