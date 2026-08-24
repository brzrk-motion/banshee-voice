import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Toaster, toast } from "sonner";
import { AppShell } from "./AppShell";
import { HistoryPage } from "@/features/history/HistoryPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { TranscribePage } from "@/features/transcribe/TranscribePage";
import { errorMessage, run } from "@/lib/tauri";
import type { AudioInputDevice, PageId, RecordingResult, RecordingStateChanged, Settings } from "@/lib/types";

export default function App() {
  const [page, setPage] = useState<PageId>("transcribe");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [recordingState, setRecordingState] = useState("idle");
  const [scratchText, setScratchText] = useState("");
  const [completedSessionId, setCompletedSessionId] = useState<string | null>(null);
  const [savingSettings, setSavingSettings] = useState(false);

  useEffect(() => {
    void Promise.all([
      run<Settings>("settings_get"),
      run<AudioInputDevice[]>("audio_list_input_devices"),
    ]).then(([loadedSettings, loadedDevices]) => {
      setSettings(loadedSettings);
      setDevices(loadedDevices);
    }).catch((error) => toast.error("Banshee could not load", { description: errorMessage(error) }));

    let dispose = () => {};
    listen<RecordingStateChanged>("recording_state_changed", (event) => setRecordingState(event.payload.state))
      .then((unlisten) => { dispose = unlisten; })
      .catch(() => {});
    return () => dispose();
  }, []);

  async function startRecording() {
    try {
      setCompletedSessionId(null);
      await run("recording_start_manual");
      setRecordingState("recording");
    } catch (error) {
      setRecordingState("error");
      toast.error("Recording could not start", { description: errorMessage(error) });
    }
  }

  async function stopRecording() {
    try {
      setRecordingState("transcribing");
      const result = await run<RecordingResult>("recording_stop_manual");
      setScratchText(result.finalText);
      setCompletedSessionId(result.sessionId);
      setRecordingState("idle");
      toast.success("Transcription ready");
    } catch (error) {
      setRecordingState("error");
      toast.error("Transcription failed", { description: errorMessage(error) });
    }
  }

  async function cancelRecording() {
    try {
      await run("recording_cancel");
      setRecordingState("idle");
    } catch (error) {
      toast.error("Recording could not be canceled", { description: errorMessage(error) });
    }
  }

  async function copyText(text: string) {
    try {
      await run("clipboard_write_text", { text });
      toast.success("Copied to clipboard");
    } catch (error) {
      toast.error("Could not copy text", { description: errorMessage(error) });
    }
  }

  async function saveSettings(next: Settings) {
    setSavingSettings(true);
    try {
      const saved = await run<Settings>("settings_update", { payload: next });
      setSettings(saved);
      toast.success("Settings saved");
    } catch (error) {
      toast.error("Settings were not saved", { description: errorMessage(error) });
    } finally {
      setSavingSettings(false);
    }
  }

  return (
    <>
      <AppShell page={page} onNavigate={setPage}>
        {page === "transcribe" ? (
          <TranscribePage
            text={scratchText}
            onTextChange={setScratchText}
            recordingState={recordingState}
            completedSessionId={completedSessionId}
            onStart={startRecording}
            onStop={stopRecording}
            onCancel={cancelRecording}
            onCopy={copyText}
          />
        ) : null}
        {page === "history" ? <HistoryPage onCopy={copyText} /> : null}
        {page === "settings" ? <SettingsPage settings={settings} devices={devices} saving={savingSettings} onSave={saveSettings} /> : null}
      </AppShell>
      <Toaster theme="dark" position="bottom-right" richColors closeButton />
    </>
  );
}
