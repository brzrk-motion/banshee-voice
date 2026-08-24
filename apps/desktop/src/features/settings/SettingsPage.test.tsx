import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

afterEach(cleanup);

describe("SettingsPage", () => {
  it("saves a draft instead of persisting each edit", () => {
    const onSave = vi.fn(async () => {});
    render(<SettingsPage settings={settings} devices={[]} vocabulary={[]} cleanupStatus={{ capability: "cleanup", state: "missing", modelName: "cleanup", downloadedBytes: 0 }} saving={false} onSave={onSave} onRetryCleanup={vi.fn(async () => {})} />);

    const save = screen.getByRole("button", { name: "Save changes" });
    expect(save).toBeDisabled();
    fireEvent.click(screen.getByRole("switch", { name: "Save text history" }));
    expect(onSave).not.toHaveBeenCalled();
    expect(save).toBeEnabled();
    fireEvent.click(save);
    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ historyEnabled: false }), []);
  });

  it("parses canonical terms and spoken aliases", () => {
    const onSave = vi.fn(async () => {});
    render(<SettingsPage settings={settings} devices={[]} vocabulary={[]} cleanupStatus={{ capability: "cleanup", state: "missing", modelName: "cleanup", downloadedBytes: 0 }} saving={false} onSave={onSave} onRetryCleanup={vi.fn(async () => {})} />);
    fireEvent.change(screen.getByRole("textbox", { name: "Custom vocabulary" }), { target: { value: "HUD\nbanci => Banshee" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(onSave).toHaveBeenCalledWith(settings, [
      { spokenForm: "HUD", outputForm: "HUD" },
      { spokenForm: "banci", outputForm: "Banshee" },
    ]);
  });
});
