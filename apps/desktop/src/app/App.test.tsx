import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("sonner", () => ({
  Toaster: () => null,
  toast: { error: vi.fn(), success: vi.fn() },
}));

const settings = {
  launchAtLogin: false,
  startMinimized: false,
  playStartSound: true,
  playCompletionSound: true,
  microphoneDeviceId: null,
  vadSensitivity: 0.5,
  pushToTalkShortcut: "Ctrl+Shift+Space",
  accelerationPreference: "auto",
  historyEnabled: true,
  audioRetentionPolicy: "never",
  preserveClipboard: true,
  pasteDelayMs: 120,
};

describe("App startup", () => {
  beforeEach(() => {
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "settings_get") return Promise.resolve(settings);
      if (command === "acceleration_status_get") return Promise.resolve({ gpuAvailable: false });
      if (command === "audio_list_input_devices") return Promise.resolve([]);
      if (command === "recording_snapshot_get") return Promise.resolve({ state: "idle" });
      return Promise.reject(new Error(`missing ${command}`));
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("still renders Settings when an optional startup command is unavailable", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(await screen.findByText("Microphone")).toBeInTheDocument();
    expect(screen.queryByRole("switch", { name: "Cleanup model" })).not.toBeInTheDocument();
  });
});
