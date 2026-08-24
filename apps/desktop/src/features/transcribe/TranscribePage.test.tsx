import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TranscribePage } from "./TranscribePage";

const baseProps = {
  text: "",
  onTextChange: vi.fn(),
  recordingState: "idle",
  completedSessionId: null,
  modelStatus: { capability: "speech" as const, state: "ready" as const, modelName: "base.en", downloadedBytes: 148_000_000, totalBytes: 148_000_000 },
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
    render(<TranscribePage {...baseProps} modelStatus={{ capability: "speech", state: "downloading", modelName: "base.en", downloadedBytes: 50, totalBytes: 100 }} />);
    expect(screen.getByText(/Downloading base.en: 50%/)).toBeVisible();
    expect(screen.getByRole("button", { name: /Start recording/ })).toBeDisabled();
  });
});
