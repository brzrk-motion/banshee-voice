import { listen } from "@tauri-apps/api/event";
import { Check, Clipboard, LoaderCircle, Mic, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { run } from "@/lib/tauri";
import {
  type AudioLevelChanged,
  type HudState,
  type HudStateChanged,
  hudStateLabel,
  initialHudState,
  isHudVisible,
} from "./HudState";

const BAR_COUNT = 28;

function Waveform({ level }: { level: number }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const historyRef = useRef<number[]>(Array.from({ length: BAR_COUNT }, () => 0.04));

  useEffect(() => {
    const history = historyRef.current;
    history.push(Math.max(0.04, Math.min(1, Math.sqrt(level))));
    history.shift();
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.round(rect.width * ratio);
    canvas.height = Math.round(rect.height * ratio);
    const context = canvas.getContext("2d");
    if (!context) return;
    context.scale(ratio, ratio);
    context.clearRect(0, 0, rect.width, rect.height);
    const gap = 3;
    const barWidth = Math.max(2, (rect.width - gap * (BAR_COUNT - 1)) / BAR_COUNT);
    context.fillStyle = "rgba(244, 244, 245, 0.86)";
    history.forEach((sample, index) => {
      const height = Math.max(3, sample * (rect.height - 4));
      const x = index * (barWidth + gap);
      const y = (rect.height - height) / 2;
      context.beginPath();
      context.roundRect(x, y, barWidth, height, barWidth / 2);
      context.fill();
    });
  }, [level]);

  return <canvas ref={canvasRef} className="hud-waveform" aria-label="Live microphone waveform" />;
}

function StateIcon({ state }: { state: HudState }) {
  if (state === "processing") return <LoaderCircle className="hud-spinner" />;
  if (state === "inserted") return <Check />;
  if (state === "clipboard") return <Clipboard />;
  if (state === "error") return <TriangleAlert />;
  return <Mic />;
}

export function HudApp() {
  const [hud, setHud] = useState<HudStateChanged>(initialHudState);
  const [level, setLevel] = useState(0);
  const hudRef = useRef(hud);

  useEffect(() => {
    const disposers: Array<() => void> = [];
    let receivedStateEvent = false;
    listen<HudStateChanged>("hud_state_changed", (event) => {
      receivedStateEvent = true;
      hudRef.current = event.payload;
      setHud(event.payload);
      if (event.payload.state !== "recording") setLevel(0);
    }).then((unlisten) => disposers.push(unlisten)).catch(() => {});
    listen<AudioLevelChanged>("audio_level_changed", (event) => {
      const current = hudRef.current;
      if (current.state === "recording" && current.sessionId === event.payload.sessionId) {
        setLevel(event.payload.level);
      }
    }).then((unlisten) => disposers.push(unlisten)).catch(() => {});
    void run<{ hud: HudStateChanged }>("recording_snapshot_get").then((snapshot) => {
      if (!receivedStateEvent) {
        hudRef.current = snapshot.hud;
        setHud(snapshot.hud);
      }
    }).catch(() => {});
    return () => disposers.forEach((dispose) => dispose());
  }, []);

  const recording = hud.state === "recording";
  return (
    <main className={`hud-shell hud-shell--${hud.state}`} aria-hidden={!isHudVisible(hud)}>
      <section className="hud-card" role="status" aria-live="polite">
        <div className={`hud-icon hud-icon--${hud.state}`}>
          <StateIcon state={hud.state} />
          {recording ? <span className="hud-recording-dot" /> : null}
        </div>
        {recording ? (
          <Waveform level={level} />
        ) : (
          <div className="hud-copy">
            <span className="hud-label">{hudStateLabel(hud)}</span>
            {hud.message && hud.state !== "error" ? <span className="hud-detail">{hud.message}</span> : null}
          </div>
        )}
      </section>
    </main>
  );
}
