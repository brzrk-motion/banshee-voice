import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { HudStateChanged, hudStateLabel, initialHudState, isHudVisible } from "./HudState";

export function HudApp() {
  const [hud, setHud] = useState<HudStateChanged>(initialHudState);

  useEffect(() => {
    let dispose = () => {};

    listen<HudStateChanged>("hud_state_changed", (event) => {
      setHud(event.payload);
    })
      .then((unlisten) => {
        dispose = unlisten;
      })
      .catch(() => {});

    return () => dispose();
  }, []);

  return (
    <main className={`hud-shell hud-shell--${hud.state}`}>
      <section className="hud-card" aria-hidden={!isHudVisible(hud)}>
        <div className="hud-badge">Banshee HUD</div>
        <div className="hud-state">{hudStateLabel(hud)}</div>
        <p className="hud-message">{hud.message ?? "Waiting for the recording pipeline."}</p>
        {typeof hud.level === "number" ? (
          <div className="hud-meter" aria-label="speech level">
            <span style={{ width: `${Math.round(Math.max(0, Math.min(1, hud.level)) * 100)}%` }} />
          </div>
        ) : null}
        {hud.liveTranscript ? <p className="hud-transcript">{hud.liveTranscript}</p> : null}
      </section>
    </main>
  );
}
