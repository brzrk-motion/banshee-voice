export type HudState = "hidden" | "recording" | "processing" | "inserted" | "clipboard" | "error";

export type HudStateChanged = {
  sessionId?: string | null;
  state: HudState;
  message?: string | null;
};

export type AudioLevelChanged = {
  sessionId: string;
  level: number;
};

export const initialHudState: HudStateChanged = { state: "hidden", sessionId: null };

export function isHudVisible(hud: HudStateChanged) {
  return hud.state !== "hidden";
}

export function hudStateLabel(hud: HudStateChanged) {
  switch (hud.state) {
    case "recording": return "Recording";
    case "processing": return "Transcribing…";
    case "inserted": return "Inserted";
    case "clipboard": return "Copied to clipboard";
    case "error": return hud.message ?? "Transcription failed";
    default: return "";
  }
}
