import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PluginsPage } from "./PluginsPage";

const plugin = {
  manifest: { id: "banshee.prompt-enhancer", name: "Prompt Enhancer", description: "Enhances prompts", version: "0.1.0", author: "Banshee", stage: "After transcript cleanup" },
  enabled: false,
  runtimeState: "missing" as const,
  downloadedBytes: 0,
};

describe("PluginsPage", () => {
  it("lists registry plugins and toggles them immediately", () => {
    const onToggle = vi.fn(async () => {});
    render(<PluginsPage plugins={[plugin]} changing={null} onToggle={onToggle} onRetry={vi.fn(async () => {})} />);
    fireEvent.click(screen.getByRole("switch", { name: "Enable Prompt Enhancer" }));
    expect(onToggle).toHaveBeenCalledWith("banshee.prompt-enhancer", true);
  });
});
