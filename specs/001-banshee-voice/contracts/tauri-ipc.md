# Contract: Tauri IPC Surface

## Purpose

Defines the initial typed command and event contract between the React/Tauri UI and the Rust core. The UI must never call platform APIs, model runtimes, or SQLite directly.

## Commands

### `app_get_dashboard()`

**Returns**:

```ts
type DashboardSnapshot = {
  privacyMode: "local_only";
  transcriptionsToday: number;
  wordsToday: number;
  speechMinutesToday: number;
  microphoneName: string | null;
  speechModelName: string | null;
  cleanupModelName: string | null;
  activeProfileName: string | null;
  pushToTalkShortcut: string;
  sessionType: "x11" | "wayland" | "windows" | "macos" | "unknown";
};
```

### `settings_get()`

**Returns**: current `Settings` view model.

### `settings_update(payload)`

**Input**:

```ts
type SettingsUpdate = Partial<{
  launchAtLogin: boolean;
  startMinimized: boolean;
  minimizeToTray: boolean;
  showHud: boolean;
  microphoneDeviceId: string | null;
  vadSensitivity: number;
  pushToTalkShortcut: string;
  toggleRecordingShortcut: string;
  cancelShortcut: string;
  repastePreviousShortcut: string;
  accelerationPreference: "auto" | "cpu" | "gpu";
  autoPasteEnabled: boolean;
  preserveClipboard: boolean;
  pasteDelayMs: number;
  historyEnabled: boolean;
  audioRetentionPolicy: "never" | "24_hours" | "forever";
  cleanupLlmEnabled: boolean;
}>;
```

**Behavior**: validates and persists settings, returning the updated snapshot.

### `audio_list_input_devices()`

**Returns**:

```ts
type AudioInputDevice = {
  id: string;
  name: string;
  isDefault: boolean;
  channels: number | null;
  sampleRateHz: number | null;
};
```

### `models_list()`

**Returns**:

```ts
type ModelRecord = {
  id: string;
  name: string;
  engine: "whisper_cpp" | "llama_cpp";
  capability: "speech_to_text" | "cleanup_llm";
  path: string;
  sizeBytes: number;
  quantization: string | null;
  recommendedUsage: "fast" | "balanced" | "accurate" | "custom";
  defaultSelected: boolean;
  lastVerifiedAt: string | null;
};
```

### `models_import(payload)`

**Input**:

```ts
type ImportModelRequest = {
  path: string;
  engine: "whisper_cpp" | "llama_cpp";
  capability: "speech_to_text" | "cleanup_llm";
  copyIntoManagedStorage: boolean;
};
```

**Behavior**: verifies compatibility, optionally copies into app storage, registers the model, and returns the new `ModelRecord`.

### `history_list(query)`

**Input**:

```ts
type HistoryQuery = {
  search?: string;
  projectId?: string;
  profileId?: string;
  limit: number;
  cursor?: string;
};
```

**Returns**: paginated history rows plus `nextCursor`.

### `history_get(id)`

**Returns**: a full transcription detail object with raw and final text, timing, output method, model data, and error details.

### `history_repaste(id)`

**Behavior**: re-runs only the output stage against the stored final text, preserving history.

### `history_rerun_processing(id)`

**Behavior**: re-runs deterministic cleanup and optional cleanup LLM from the stored raw transcript and current rules, creating a new revisioned result or updating the entry according to implementation policy.

### `history_delete(id)`

**Behavior**: deletes the transcription and any retained audio.

### `dictionary_list(query)`

**Returns**: global and project-scoped dictionary entries.

### `dictionary_upsert(payload)`

**Input**:

```ts
type DictionaryEntryInput = {
  id?: string;
  scope: "global" | "project";
  projectId?: string | null;
  spokenForm: string;
  outputForm: string;
  enabled: boolean;
  priority?: number;
};
```

### `projects_list()`

**Returns**: registered projects with index status and counts.

### `projects_add(payload)`

**Input**:

```ts
type AddProjectRequest = {
  rootPath: string;
  name?: string;
  excludePatterns?: string[];
};
```

### `projects_reindex(projectId)`

**Behavior**: schedules or starts a full local reindex.

### `profiles_list()`

**Returns**: built-in and user-defined profiles.

### `profiles_update(payload)`

**Behavior**: persists editable profile behavior.

### `recording_start_manual()`

**Behavior**: starts recording from the main app UI or tray without requiring a global hotkey.

### `recording_stop_manual()`

**Behavior**: stops recording and runs the pipeline.

### `recording_cancel()`

**Behavior**: aborts the current capture/transcription pipeline if possible.

### `transcription_repaste_previous()`

**Behavior**: re-inserts the most recent successful transcription using the configured output backend.

## Events

### `hud_state_changed`

```ts
type HudStateChanged = {
  state: "hidden" | "listening" | "processing" | "complete" | "error";
  message?: string;
  level?: number;
  liveTranscript?: string;
};
```

### `recording_state_changed`

```ts
type RecordingStateChanged = {
  state: "idle" | "recording" | "stopping" | "transcribing" | "inserting" | "error";
  transcriptionId?: string;
};
```

### `project_index_progress`

```ts
type ProjectIndexProgress = {
  projectId: string;
  phase: "discover" | "scan" | "symbols" | "finalize";
  completed: number;
  total?: number;
  message?: string;
};
```

### `history_changed`

Signals that history list caches should refresh.

### `settings_changed`

Signals that shortcut bindings, selected models, or output behavior changed and any dependent UI should refresh.

## Error Contract

Commands return structured application errors:

```ts
type AppError = {
  code:
    | "microphone_unavailable"
    | "microphone_permission_denied"
    | "model_missing"
    | "inference_failed"
    | "no_speech_detected"
    | "paste_unavailable"
    | "clipboard_failed"
    | "project_index_failed"
    | "settings_invalid"
    | "unknown";
  message: string;
  recoverable: boolean;
  fallbackUsed?: "clipboard" | "history" | "none";
};
```

## Contract Rules

- Commands mutate durable state; events report state changes.
- Large blobs should remain on disk or be referenced by ID/path rather than pushed repeatedly over IPC.
- The Rust core owns validation and normalization.
