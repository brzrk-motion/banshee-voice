import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { DashboardPage } from "../features/dashboard/DashboardPage";
import { InputSettings } from "../features/settings/InputSettings";

type Dashboard = {
  privacyMode: string;
  transcriptionsToday: number;
  wordsToday: number;
  speechMinutesToday: number;
  microphoneName?: string | null;
  speechModelName?: string | null;
  activeProfileName?: string | null;
  pushToTalkShortcut: string;
  sessionType: string;
};

type Settings = {
  microphoneDeviceId?: string | null;
  vadSensitivity: number;
  pushToTalkShortcut: string;
  toggleRecordingShortcut: string;
  cancelShortcut: string;
  autoPasteEnabled: boolean;
  preserveClipboard: boolean;
};

type AudioInputDevice = {
  id: string;
  name: string;
  isDefault: boolean;
  channels?: number | null;
  sampleRateHz?: number | null;
};

type RecordingStateChanged = {
  state: string;
};

type RecordingResult = {
  sessionId: string;
  rawText: string;
  deterministicText: string;
  finalText: string;
  sttBackend: string;
  peakLevel: number;
  status: string;
  outputMethod: string;
  outputResult: string;
  outputMessage: string;
  applicationName: string;
  windowTitle: string;
  durationMs: number;
};

async function run<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args);
}

export default function App() {
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [devices, setDevices] = useState<AudioInputDevice[]>([]);
  const [recordingState, setRecordingState] = useState("idle");
  const [latestResult, setLatestResult] = useState<RecordingResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void Promise.all([
      run<Dashboard>("app_get_dashboard"),
      run<Settings>("settings_get"),
      run<AudioInputDevice[]>("audio_list_input_devices"),
    ])
      .then(([dashboardData, settingsData, devicesData]) => {
        setDashboard(dashboardData);
        setSettings(settingsData);
        setDevices(devicesData);
      })
      .catch((loadError) => {
        setError(String(loadError));
      });

    let dispose = () => {};
    listen<RecordingStateChanged>("recording_state_changed", (event) => {
      setRecordingState(event.payload.state);
    })
      .then((unlisten) => {
        dispose = unlisten;
      })
      .catch(() => {});

    return () => dispose();
  }, []);

  async function refreshSnapshot() {
    const [dashboardData, settingsData, devicesData] = await Promise.all([
      run<Dashboard>("app_get_dashboard"),
      run<Settings>("settings_get"),
      run<AudioInputDevice[]>("audio_list_input_devices"),
    ]);
    setDashboard(dashboardData);
    setSettings(settingsData);
    setDevices(devicesData);
  }

  async function updateSettings(payload: Record<string, unknown>) {
    setError(null);
    try {
      const nextSettings = await run<Settings>("settings_update", { payload });
      setSettings(nextSettings);
      setDashboard(await run<Dashboard>("app_get_dashboard"));
    } catch (updateError) {
      setError(String(updateError));
    }
  }

  async function startRecording() {
    setError(null);
    try {
      await run("recording_start_manual");
      setRecordingState("recording");
    } catch (startError) {
      setError(String(startError));
    }
  }

  async function stopRecording() {
    setError(null);
    try {
      const result = await run<RecordingResult>("recording_stop_manual");
      setLatestResult(result);
      setRecordingState("idle");
      await refreshSnapshot();
    } catch (stopError) {
      setRecordingState("error");
      setError(String(stopError));
    }
  }

  async function cancelRecording() {
    setError(null);
    try {
      await run("recording_cancel");
      setRecordingState("idle");
    } catch (cancelError) {
      setError(String(cancelError));
    }
  }

  return (
    <main className="shell shell--dashboard">
      <section className="panel panel--wide">
        <div className="eyebrow">Banshee</div>
        <div className="app-grid">
          <DashboardPage
            dashboard={dashboard}
            recordingState={recordingState}
            latestResult={latestResult}
            onStart={startRecording}
            onStop={stopRecording}
            onCancel={cancelRecording}
          />
          <InputSettings devices={devices} settings={settings} onUpdate={updateSettings} />
        </div>
        {error ? <p className="error-banner">{error}</p> : null}
      </section>
    </main>
  );
}
