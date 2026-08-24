import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Toaster, toast } from "sonner";
import { AppShell } from "./AppShell";
import { HistoryPage } from "@/features/history/HistoryPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { TranscribePage } from "@/features/transcribe/TranscribePage";
import { errorMessage, run } from "@/lib/tauri";
import type { AudioInputDevice, DictionaryEntry, ModelStatus, ModelsStatus, PageId, RecordingResult, RecordingSnapshot, RecordingStateChanged, Settings } from "@/lib/types";

export default function App() {
  const [page, setPage] = useState<PageId>("transcribe");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [recordingState, setRecordingState] = useState("idle");
  const [scratchText, setScratchText] = useState("");
  const [completedSessionId, setCompletedSessionId] = useState<string | null>(null);
  const [savingSettings, setSavingSettings] = useState(false);
  const [modelsStatus, setModelsStatus] = useState<ModelsStatus>({
    speech: { capability: "speech", state: "missing", modelName: "base.en", downloadedBytes: 0 },
    cleanup: { capability: "cleanup", state: "missing", modelName: "Qwen2.5-0.5B-Instruct-Q4_K_M", downloadedBytes: 0 },
  });
  const [vocabulary, setVocabulary] = useState<DictionaryEntry[]>([]);

  useEffect(() => {
    void Promise.all([
      run<Settings>("settings_get"),
      run<AudioInputDevice[]>("audio_list_input_devices"),
      run<ModelsStatus>("models_status_get"),
      run<DictionaryEntry[]>("dictionary_entries_get"),
      run<RecordingSnapshot>("recording_snapshot_get"),
    ]).then(([loadedSettings, loadedDevices, loadedModelsStatus, loadedVocabulary, snapshot]) => {
      setSettings(loadedSettings);
      setDevices(loadedDevices);
      setModelsStatus(loadedModelsStatus);
      setVocabulary(loadedVocabulary);
      setRecordingState(snapshot.state);
      if (snapshot.lastTranscript) setScratchText(snapshot.lastTranscript);
    }).catch((error) => toast.error("Banshee could not load", { description: errorMessage(error) }));

    const disposers: Array<() => void> = [];
    listen<RecordingStateChanged>("recording_state_changed", (event) => setRecordingState(event.payload.state))
      .then((unlisten) => { disposers.push(unlisten); })
      .catch(() => {});
    listen<ModelStatus>("model_status_changed", (event) => setModelsStatus((current) => ({ ...current, [event.payload.capability]: event.payload })))
      .then((unlisten) => { disposers.push(unlisten); })
      .catch(() => {});
    listen<RecordingResult>("transcription_completed", (event) => {
      if (event.payload.origin === "scratch") {
        setScratchText(event.payload.finalText);
        setCompletedSessionId(event.payload.sessionId);
      }
      setRecordingState("idle");
    }).then((unlisten) => { disposers.push(unlisten); }).catch(() => {});
    listen<PageId>("navigate_to_page", (event) => setPage(event.payload))
      .then((unlisten) => { disposers.push(unlisten); })
      .catch(() => {});
    return () => disposers.forEach((dispose) => dispose());
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

  async function retryModelDownload(capability: ModelStatus["capability"]) {
    setModelsStatus((current) => ({ ...current, [capability]: { ...current[capability], state: "missing", message: null } }));
    try {
      await run("model_download_retry", { capability });
    } catch (error) {
      toast.error("Model download could not start", { description: errorMessage(error) });
    }
  }

  async function saveSettings(next: Settings, nextVocabulary: DictionaryEntry[]) {
    setSavingSettings(true);
    try {
      const savedVocabulary = await run<DictionaryEntry[]>("dictionary_entries_replace", { entries: nextVocabulary });
      const saved = await run<Settings>("settings_update", { payload: next });
      setSettings(saved);
      setVocabulary(savedVocabulary);
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
            modelStatus={modelsStatus.speech}
            onStart={startRecording}
            onStop={stopRecording}
            onCancel={cancelRecording}
            onCopy={copyText}
            onRetryModel={() => retryModelDownload("speech")}
          />
        ) : null}
        {page === "history" ? <HistoryPage onCopy={copyText} /> : null}
        {page === "settings" ? <SettingsPage settings={settings} devices={devices} vocabulary={vocabulary} cleanupStatus={modelsStatus.cleanup} saving={savingSettings} onSave={saveSettings} onRetryCleanup={() => retryModelDownload("cleanup")} /> : null}
      </AppShell>
      <Toaster theme="dark" position="bottom-right" richColors closeButton />
    </>
  );
}
