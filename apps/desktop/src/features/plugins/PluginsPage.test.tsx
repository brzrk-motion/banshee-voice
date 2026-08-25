import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PluginsPage } from "./PluginsPage";

const plugin = {
  manifest: {
    id: "banshee.prompt-enhancer",
    name: "Prompt Enhancer",
    description: "Enhances prompts",
    version: "0.1.0",
    author: "Banshee",
    stage: "After transcript cleanup",
    settings: [{
      key: "targetModel",
      label: "Target coding model",
      kind: "select" as const,
      defaultValue: "gpt-5.3-codex",
      options: [
        { value: "gpt-5.3-codex", label: "GPT-5.3-Codex" },
        { value: "claude-opus-5", label: "Claude Opus 5" },
      ],
    }],
  },
  settings: { targetModel: "gpt-5.3-codex" },
  enabled: false,
  runtimeState: "missing" as const,
  downloadedBytes: 0,
};

describe("PluginsPage", () => {
  afterEach(cleanup);

  it("lists registry plugins and toggles them immediately", () => {
    const onToggle = vi.fn(async () => {});
    render(<PluginsPage plugins={[plugin]} changing={null} savingSettings={null} onToggle={onToggle} onRetry={vi.fn(async () => {})} onSaveSettings={vi.fn(async () => {})} />);
    fireEvent.click(screen.getByRole("switch", { name: "Enable Prompt Enhancer" }));
    expect(onToggle).toHaveBeenCalledWith("banshee.prompt-enhancer", true);
  });

  it("opens the shared settings modal and saves the selected target", async () => {
    const onSaveSettings = vi.fn(async () => {});
    render(<PluginsPage plugins={[plugin]} changing={null} savingSettings={null} onToggle={vi.fn(async () => {})} onRetry={vi.fn(async () => {})} onSaveSettings={onSaveSettings} />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.change(screen.getByLabelText("Target coding model"), { target: { value: "claude-opus-5" } });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    expect(onSaveSettings).toHaveBeenCalledWith("banshee.prompt-enhancer", { targetModel: "claude-opus-5" });
  });

  it("discards modal edits when canceled", () => {
    const onSaveSettings = vi.fn(async () => {});
    render(<PluginsPage plugins={[plugin]} changing={null} savingSettings={null} onToggle={vi.fn(async () => {})} onRetry={vi.fn(async () => {})} onSaveSettings={onSaveSettings} />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    fireEvent.change(screen.getByLabelText("Target coding model"), { target: { value: "claude-opus-5" } });
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByLabelText("Target coding model")).toHaveValue("gpt-5.3-codex");
    expect(onSaveSettings).not.toHaveBeenCalled();
  });
});
