import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppShell } from "./AppShell";

describe("AppShell", () => {
  it("places primary pages in the sidebar and settings in the footer", () => {
    const onNavigate = vi.fn();
    render(<AppShell page="transcribe" onNavigate={onNavigate}><div>Page content</div></AppShell>);

    expect(screen.getByRole("button", { name: "Transcribe" })).toHaveAttribute("data-active", "true");
    expect(screen.getByRole("button", { name: "Settings" }).parentElement).toHaveClass("border-t");
    fireEvent.click(screen.getByRole("button", { name: "History" }));
    expect(onNavigate).toHaveBeenCalledWith("history");
    fireEvent.click(screen.getByRole("button", { name: "Plugins" }));
    expect(onNavigate).toHaveBeenCalledWith("plugins");
  });
});
