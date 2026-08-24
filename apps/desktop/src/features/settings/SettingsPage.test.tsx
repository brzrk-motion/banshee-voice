import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Settings } from "@/lib/types";
import { SettingsPage } from "./SettingsPage";

const settings: Settings = {
  launchAtLogin: false,
  startMinimized: false,
  playStartSound: false,
  playCompletionSound: false,
  microphoneDeviceId: null,
  vadSensitivity: 0.5,
  pushToTalkShortcut: "Ctrl+Shift+Space",
  accelerationPreference: "auto",
  historyEnabled: true,
  audioRetentionPolicy: "never",
  preserveClipboard: true,
  pasteDelayMs: 40,
  cleanupLlmEnabled: false,
};

describe("SettingsPage", () => {
  it("saves a draft instead of persisting each edit", () => {
    const onSave = vi.fn(async () => {});
    render(<SettingsPage settings={settings} devices={[]} saving={false} onSave={onSave} />);

    const save = screen.getByRole("button", { name: "Save changes" });
    expect(save).toBeDisabled();
    fireEvent.click(screen.getByRole("switch", { name: "Save text history" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(save).toBeEnabled();
    fireEvent.click(save);
    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ historyEnabled: false }));
  });
});
