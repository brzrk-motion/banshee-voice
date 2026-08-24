import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TranscribePage } from "./TranscribePage";

const baseProps = {
  text: "",
  onTextChange: vi.fn(),
  recordingState: "idle",
  completedSessionId: null,
  onStart: vi.fn(async () => {}),
  onStop: vi.fn(async () => {}),
  onCancel: vi.fn(async () => {}),
  onCopy: vi.fn(async () => {}),
};

describe("TranscribePage", () => {
  it("keeps copy disabled until scratch text exists", () => {
    const { rerender } = render(<TranscribePage {...baseProps} />);
    expect(screen.getByRole("button", { name: "Copy text" })).toBeDisabled();
    rerender(<TranscribePage {...baseProps} text="Ready to copy" />);
    fireEvent.click(screen.getByRole("button", { name: "Copy text" }));
    expect(baseProps.onCopy).toHaveBeenCalledWith("Ready to copy");
  });

  it("shows stop and cancel while recording", () => {
    render(<TranscribePage {...baseProps} recordingState="recording" />);
    expect(screen.getByRole("button", { name: /Stop and transcribe/ })).toBeVisible();
    expect(screen.getByRole("button", { name: /Cancel/ })).toBeVisible();
  });
});
