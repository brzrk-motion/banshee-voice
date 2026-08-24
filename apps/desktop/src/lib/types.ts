export type PageId = "transcribe" | "history" | "settings";

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
  cleanupLlmEnabled: boolean;
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
};

export type ModelStatus = {
  state: "missing" | "downloading" | "loading" | "ready" | "error";
  modelName: string;
  downloadedBytes: number;
  totalBytes?: number | null;
  message?: string | null;
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
