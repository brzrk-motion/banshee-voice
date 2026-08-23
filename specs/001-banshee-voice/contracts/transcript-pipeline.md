# Contract: Transcript Pipeline

## Purpose

Defines the behavior contract for Banshee's end-to-end dictation pipeline so Rust subsystems remain swappable behind stable interfaces.

## Pipeline Stages

1. `ShortcutBackend` or manual UI action starts recording.
2. `HudController` immediately switches to `listening`.
3. `AudioCapture` begins buffering microphone input.
4. `VadEngine` tracks speech activity, trims silence, and emits envelope/state hints for HUD updates.
5. On stop, trimmed audio is handed to `TranscriptionEngine`.
6. Raw transcript is produced.
7. `DeterministicCleanup` applies fillers, punctuation, corrections, dictionary rules, profile behavior, and repository-aware high-confidence substitutions.
8. Optional `CleanupEngine` refines the deterministic transcript within a bounded deadline.
9. Final transcript is persisted to history.
10. `OutputBackend` attempts direct insertion, paste fallback, or clipboard-only fallback.
11. HUD transitions to `complete` or `error` and fades.

## Core Traits

### `AudioCapture`

```rust
trait AudioCapture {
    fn start(&self, request: AudioCaptureRequest) -> Result<CaptureSession, AudioError>;
}
```

**Guarantees**:

- Capture starts without waiting for STT initialization.
- Session exposes a stream or buffer handle consumable by VAD and pipeline orchestration.

### `VadEngine`

```rust
trait VadEngine {
    fn analyze_chunk(&mut self, chunk: AudioChunk) -> VadUpdate;
    fn finalize(&mut self) -> VadResult;
}
```

**Guarantees**:

- Provides speech start/end detection and trim ranges.
- Never mutates original audio buffers in place.

### `TranscriptionEngine`

```rust
trait TranscriptionEngine {
    fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionOutput, SttError>;
}
```

**Request requirements**:

- Includes audio path or in-memory PCM reference, selected model, language, acceleration preference, and latency profile.

**Output requirements**:

- Returns raw text, token or segment metadata if available, actual backend used, and latency metrics.

### `CleanupEngine`

```rust
trait CleanupEngine {
    fn refine(&self, request: CleanupRequest) -> Result<CleanupOutput, CleanupError>;
}
```

**Guarantees**:

- Returns only cleaned transcript text.
- Must honor max token and timeout constraints.
- Must not execute actions, answer the instruction, or add unrelated content.

### `ContextProvider`

```rust
trait ContextProvider {
    fn resolve(&self, request: ContextRequest) -> ContextSnapshot;
}
```

**Responsibilities**:

- Resolve active app/window info.
- Resolve candidate project.
- Provide repository lexical matches with confidence scores.

### `OutputBackend`

```rust
trait OutputBackend {
    fn insert_text(&self, request: OutputRequest) -> OutputResult;
}
```

**Guarantees**:

- Returns structured result: direct insert, pasted via clipboard, copied only, or failure.
- Preserves or restores clipboard according to settings when feasible.

## Pipeline Rules

- A deterministic cleaned transcript must exist before optional LLM refinement begins.
- If optional cleanup exceeds the deadline or fails, deterministic output becomes the final transcript.
- Final transcript persistence occurs before output injection so recovery is always possible.
- If output injection fails, the final transcript must remain available through clipboard fallback and history.
- Low-confidence repository matches must not replace user text silently.

## Confidence Rules

- `high`: may auto-apply repository term or file reference.
- `medium`: may surface as suggestion metadata for future UI, but MVP should not auto-rewrite.
- `low`: do not rewrite.

## Timeout and Fallback Rules

- Hotkey press must never block on model loading if the model is already warmed.
- Cleanup LLM deadline is profile-aware but bounded.
- Failure in any downstream stage must degrade to the best available transcript rather than aborting the whole flow.
