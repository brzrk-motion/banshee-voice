export type PageId = "transcribe" | "history" | "plugins" | "settings";

export type Settings = {
  launchAtLogin: boolean;
  startMinimized: boolean;
  playStartSound: boolean;
  playCompletionSound: boolean;
  microphoneDeviceId?: string | null;
  vadSensitivity: number;
  pushToTalkShortcut: string;
  accelerationPreference: "auto" | "cpu" | "gpu";
  historyEnabled: boolean;
  audioRetentionPolicy: "never" | "24_hours" | "forever";
  preserveClipboard: boolean;
  pasteDelayMs: number;
};

export type PluginSummary = {
  manifest: {
    id: string;
    name: string;
    description: string;
    version: string;
    author: string;
    stage: string;
    settings: PluginSettingDefinition[];
  };
  settings: Record<string, string>;
  enabled: boolean;
  runtimeState: "missing" | "downloading" | "loading" | "ready" | "error";
  downloadedBytes: number;
  totalBytes?: number | null;
  message?: string | null;
};

export type PluginSettingDefinition = {
  key: string;
  label: string;
  description?: string | null;
  kind: "select";
  defaultValue: string;
  options: Array<{ value: string; label: string }>;
};

export type AudioInputDevice = {
  id: string;
  name: string;
  isDefault: boolean;
  channels?: number | null;
  sampleRateHz?: number | null;
};

export type RecordingResult = {
  sessionId: string;
  finalText: string;
  origin: "scratch" | "push_to_talk";
  sttBackend: string;
  cleanupBackend: string;
  sttLatencyMs: number;
  cleanupLatencyMs: number;
  cleanupFallbackReason?: string | null;
};

export type ModelStatus = {
  capability: "speech" | "cleanup";
  state: "missing" | "downloading" | "loading" | "ready" | "error";
  modelName: string;
  downloadedBytes: number;
  totalBytes?: number | null;
  message?: string | null;
};

export type ModelsStatus = {
  speech: ModelStatus;
  cleanup: ModelStatus;
};

export type DictionaryEntry = {
  spokenForm: string;
  outputForm: string;
};

export type RecordingSnapshot = {
  state: RecordingStateChanged["state"];
  lastTranscript?: string | null;
};

export type RecordingStateChanged = {
  state: "idle" | "recording" | "stopping" | "transcribing" | "inserting" | "error";
  transcriptionId?: string | null;
};

export type HistoryItem = {
  id: string;
  createdAt: string;
  finalText: string;
};

export type HistoryPageResult = {
  items: HistoryItem[];
  nextCursor?: string | null;
};
