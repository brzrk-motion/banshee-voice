export type HudState = "hidden" | "listening" | "processing" | "complete" | "error";

export type HudStateChanged = {
  state: HudState;
  message?: string;
  level?: number;
  liveTranscript?: string;
};

export const initialHudState: HudStateChanged = {
  state: "hidden",
};

export function isHudVisible(hud: HudStateChanged) {
  return hud.state !== "hidden";
}

export function hudStateLabel(hud: HudStateChanged) {
  return hud.state.replace(/_/g, " ");
}
