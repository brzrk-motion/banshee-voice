import { describe, expect, it } from "vitest";
import { shortcutFromKeyboardEvent } from "./shortcut";

describe("shortcutFromKeyboardEvent", () => {
  it("captures a complete push-to-talk chord", () => {
    expect(shortcutFromKeyboardEvent({ key: " ", ctrlKey: true, shiftKey: true, altKey: false, metaKey: false })).toBe("Ctrl+Shift+Space");
  });

  it("waits until a non-modifier key is pressed", () => {
    expect(shortcutFromKeyboardEvent({ key: "Control", ctrlKey: true, shiftKey: false, altKey: false, metaKey: false })).toBeNull();
  });
});
