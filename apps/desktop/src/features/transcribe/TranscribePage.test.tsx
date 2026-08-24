import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TranscribePage } from "./TranscribePage";

const baseProps = {
  text: "",
  onTextChange: vi.fn(),
  recordingState: "idle",
  completedSessionId: null,
  modelStatus: { state: "ready" as const, modelName: "tiny.en-q5_1", downloadedBytes: 32_000_000, totalBytes: 32_000_000 },
  onStart: vi.fn(async () => {}),
  onStop: vi.fn(async () => {}),
  onCancel: vi.fn(async () => {}),
  onCopy: vi.fn(async () => {}),
  onRetryModel: vi.fn(async () => {}),
};

afterEach(cleanup);

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

  it("gates recording while the model downloads", () => {
    render(<TranscribePage {...baseProps} modelStatus={{ state: "downloading", modelName: "tiny.en-q5_1", downloadedBytes: 50, totalBytes: 100 }} />);
    expect(screen.getByText(/Downloading tiny.en-q5_1: 50%/)).toBeVisible();
    expect(screen.getByRole("button", { name: /Start recording/ })).toBeDisabled();
  });
});
