# Quickstart Validation: Banshee Local Voice Transcription App

## Purpose

Validate the Banshee MVP end-to-end after implementation, focusing on the core push-to-talk workflow first and then the developer-intelligence features.

## Prerequisites

- Rust toolchain installed
- Node.js and package manager installed
- Tauri 2 development prerequisites for the target platform
- Linux desktop dependencies installed if validating on Linux
- At least one compatible local `whisper.cpp` model available for import
- Optional compatible local `llama.cpp` cleanup model available for import
- Microphone permission granted to the app where required

## Setup

1. Install frontend dependencies in `apps/desktop`.
2. Build or fetch native dependencies required by Tauri and the selected inference bindings.
3. Start the app in development mode.
4. Import a local Whisper model through the Models screen.
5. Select a microphone and configure a push-to-talk shortcut.

## Validation Scenario 1: Core Push-to-Talk Flow

1. Launch the desktop app.
2. Verify the dashboard shows local-only status, current microphone, selected speech model, and push-to-talk shortcut.
3. Focus an external text field such as Cursor, VS Code, Zed, a terminal input, or a browser textarea.
4. Hold the configured push-to-talk shortcut.
5. Verify the HUD appears immediately in `Listening` state and does not steal focus.
6. Speak a multi-sentence developer instruction.
7. Release the shortcut.
8. Verify the HUD enters `Processing`, then `Complete` or an explicit fallback/error state.
9. Verify the final text appears in the focused application or, if automatic insertion is unavailable, verify the text is on the clipboard with a clear notification.

**Expected outcome**:

- The transcription is produced fully locally.
- Deterministic cleanup runs.
- The transcript is preserved in history.
- No network access is required.

## Validation Scenario 2: History and Recovery

1. Open the History screen.
2. Verify the new transcription row includes timestamp, app, project, profile, preview, and word count.
3. Open the transcription detail view.
4. Verify raw transcript, final transcript, speech model, cleanup model, latency, duration, and output result are visible.
5. Trigger `copy`, `re-paste`, and `delete` actions.

**Expected outcome**:

- Recovery actions work without re-recording.
- Delete also removes retained audio if it exists.

## Validation Scenario 3: Dictionary-Based Cleanup

1. Add a global dictionary entry such as `tail wind -> Tailwind`.
2. Dictate a phrase containing the spoken form.
3. Inspect the final transcript in the destination app and in history.

**Expected outcome**:

- Deterministic cleanup replaces the spoken phrase with the configured output.

## Validation Scenario 4: Project Indexing and File References

1. Add a repository through the Projects screen.
2. Run indexing and wait for `ready` status.
3. Dictate a phrase referencing a known file or symbol in that repository.
4. Inspect the final transcript.

**Expected outcome**:

- High-confidence file references or symbol corrections are applied.
- Low-confidence ambiguous phrases remain unchanged.

## Validation Scenario 5: Optional Cleanup LLM

1. Import and enable a compatible cleanup model.
2. Dictate an intentionally messy but valid developer instruction.
3. Compare the deterministic transcript and final transcript.
4. Disable cleanup LLM and repeat.

**Expected outcome**:

- With the cleanup model enabled, the final transcript becomes cleaner without changing technical meaning.
- With the model disabled, the deterministic pipeline still produces usable output.

## Validation Scenario 6: Failure Recovery

1. Disconnect or disable the microphone, then attempt recording.
2. Remove or unselect the speech model, then attempt recording.
3. Validate on a Linux Wayland session where direct insertion is restricted.

**Expected outcome**:

- Errors are surfaced clearly.
- The app never silently loses the transcript when a usable result exists.
- Clipboard fallback is treated as a valid recovery path.

## Contract References

- IPC surface: [`contracts/tauri-ipc.md`](contracts/tauri-ipc.md)
- Pipeline behavior: [`contracts/transcript-pipeline.md`](contracts/transcript-pipeline.md)
- Data entities and retention: [`data-model.md`](data-model.md)

## Suggested Verification Commands

Run the exact commands appropriate to the implementation once source code exists. At minimum the implementation should support equivalents of:

```bash
cargo test --workspace
pnpm --dir apps/desktop test
pnpm --dir apps/desktop build
cargo build --workspace
pnpm --dir apps/desktop tauri build
```

## Manual Platform Matrix

Validate at least:

- Linux X11: direct insertion plus tray and HUD behavior
- Linux Wayland: clipboard-first fallback, tray/menu behavior, and HUD behavior
- macOS: accessibility permission flow and direct insertion
- Windows: direct insertion and global shortcut behavior
