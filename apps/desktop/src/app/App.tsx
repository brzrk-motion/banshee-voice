import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Toaster, toast } from "sonner";
import { AppShell } from "./AppShell";
import { HistoryPage } from "@/features/history/HistoryPage";
import { PluginsPage } from "@/features/plugins/PluginsPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { TranscribePage } from "@/features/transcribe/TranscribePage";
import { errorMessage, run } from "@/lib/tauri";
import type { AudioInputDevice, DictionaryEntry, ModelStatus, ModelsStatus, PageId, PluginSummary, RecordingResult, RecordingSnapshot, RecordingStateChanged, Settings } from "@/lib/types";

const missingCleanupStatus: ModelStatus = {
  capability: "cleanup",
  state: "missing",
  modelName: "NVIDIA-Nemotron3-Nano-4B-Q4_K_M",
  downloadedBytes: 0,
};

function normalizeModelsStatus(status: ModelsStatus | ModelStatus): ModelsStatus {
  if ("speech" in status) return status;
  return { speech: status, cleanup: missingCleanupStatus };
}

async function loadModelsStatus() {
  try {
    return normalizeModelsStatus(await run<ModelsStatus | ModelStatus>("models_status_get"));
  } catch {
    return normalizeModelsStatus(await run<ModelsStatus | ModelStatus>("model_status_get"));
  }
}

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
    cleanup: missingCleanupStatus,
  });
  const [vocabulary, setVocabulary] = useState<DictionaryEntry[]>([]);
  const [plugins, setPlugins] = useState<PluginSummary[]>([]);
  const [changingPlugin, setChangingPlugin] = useState<string | null>(null);
  const [savingPluginSettings, setSavingPluginSettings] = useState<string | null>(null);

  useEffect(() => {
    const reportLoadError = (area: string) => (error: unknown) => {
      toast.error(`Banshee could not load ${area}`, { description: errorMessage(error) });
    };

    void run<Settings>("settings_get").then(setSettings).catch(reportLoadError("settings"));
    void run<AudioInputDevice[]>("audio_list_input_devices").then(setDevices).catch(reportLoadError("audio devices"));
    void loadModelsStatus().then(setModelsStatus).catch(reportLoadError("model status"));
    void run<DictionaryEntry[]>("dictionary_entries_get").then(setVocabulary).catch(reportLoadError("custom vocabulary"));
    void run<PluginSummary[]>("plugins_list").then(setPlugins).catch(reportLoadError("plugins"));
    void run<RecordingSnapshot>("recording_snapshot_get").then((snapshot) => {
      setRecordingState(snapshot.state);
      if (snapshot.lastTranscript) setScratchText(snapshot.lastTranscript);
    }).catch(reportLoadError("recording state"));

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
    listen<void>("plugins_changed", () => { void run<PluginSummary[]>("plugins_list").then(setPlugins); })
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

  async function togglePlugin(pluginId: string, enabled: boolean) {
    setChangingPlugin(pluginId);
    try {
      setPlugins(await run<PluginSummary[]>("plugin_set_enabled", { pluginId, enabled }));
    } catch (error) {
      toast.error("Plugin setting was not saved", { description: errorMessage(error) });
    } finally {
      setChangingPlugin(null);
    }
  }

  async function retryPlugin(pluginId: string) {
    try {
      await run("plugin_setup_retry", { pluginId });
    } catch (error) {
      toast.error("Plugin setup could not start", { description: errorMessage(error) });
    }
  }

  async function savePluginSettings(pluginId: string, pluginSettings: Record<string, string>) {
    setSavingPluginSettings(pluginId);
    try {
      setPlugins(await run<PluginSummary[]>("plugin_settings_update", { pluginId, settings: pluginSettings }));
      toast.success("Plugin settings saved");
    } catch (error) {
      toast.error("Plugin settings were not saved", { description: errorMessage(error) });
      throw error;
    } finally {
      setSavingPluginSettings(null);
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
        {page === "plugins" ? <PluginsPage plugins={plugins} changing={changingPlugin} savingSettings={savingPluginSettings} onToggle={togglePlugin} onRetry={retryPlugin} onSaveSettings={savePluginSettings} /> : null}
        {page === "settings" ? <SettingsPage settings={settings} devices={devices} vocabulary={vocabulary} saving={savingSettings} onSave={saveSettings} /> : null}
      </AppShell>
      <Toaster theme="dark" position="bottom-right" richColors closeButton />
    </>
  );
}
