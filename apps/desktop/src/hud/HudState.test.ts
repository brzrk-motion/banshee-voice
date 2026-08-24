import { describe, expect, it } from "vitest";
import { hudStateLabel, isHudVisible } from "./HudState";

describe("HudState helpers", () => {
  it("treats hidden as not visible", () => {
    expect(isHudVisible({ state: "hidden" })).toBe(false);
    expect(isHudVisible({ state: "recording" })).toBe(true);
  });

  it("formats state labels for display", () => {
    expect(hudStateLabel({ state: "processing" })).toBe("Transcribing…");
  });
});
