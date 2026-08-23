# Feature Specification: Banshee Local Voice Transcription App

**Feature Branch**: `001-banshee-voice`

**Created**: 2026-08-22

**Status**: Draft

**Input**: User description: "Build Banshee, a polished 100% local desktop voice transcription application purpose-built for agentic software development workflows."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Fast Push-to-Talk Dictation (Priority: P1)

As a developer working in an editor, terminal, coding agent, or browser text field, I hold a global push-to-talk hotkey, speak a multi-sentence instruction, release the hotkey, and have the cleaned transcript inserted back into the currently focused application without the Banshee app stealing focus.

**Why this priority**: This is the core product promise and the fastest path to a useful MVP. If this flow is not reliable and low-latency, the product does not deliver value.

**Independent Test**: Can be fully tested by launching the desktop app, selecting a microphone and local speech model, focusing a normal text field, holding the shortcut, speaking, releasing, and verifying a local transcript is inserted or copied to the clipboard with a clear fallback notice.

**Acceptance Scenarios**:

1. **Given** the app is running with a configured microphone, speech model, and push-to-talk shortcut, **When** the user holds the hotkey while focused in a supported text field and speaks, then releases it, **Then** the HUD appears immediately, audio is captured locally, the speech is transcribed locally, deterministic cleanup is applied, and the final text is inserted into the focused application without requiring network access.
2. **Given** automatic insertion is restricted or fails on the platform, **When** transcription completes, **Then** the final text is preserved locally, copied to the clipboard, and the HUD or app clearly reports that paste failed and clipboard fallback was used.
3. **Given** no speech is detected or the microphone is unavailable, **When** the user attempts a transcription, **Then** the HUD reports the failure succinctly and no transcript is silently discarded.

---

### User Story 2 - Review and Recover Previous Transcripts (Priority: P1)

As a developer using dictation throughout the day, I can open the desktop app to view local history entries, inspect raw and final text, copy or re-paste a previous result, edit it, re-run processing, or delete it.

**Why this priority**: Recovery and confidence are essential for a voice input tool. Users must be able to correct mistakes and reuse previous dictations without fear of losing work.

**Independent Test**: Can be fully tested by completing one or more transcriptions and verifying that the app stores local history with searchable records and supports copy, re-paste, edit, re-run, and delete actions.

**Acceptance Scenarios**:

1. **Given** one or more successful transcriptions exist, **When** the user opens History, **Then** they can see timestamp, source application, project, profile, preview text, and word count for each local transcription.
2. **Given** a history entry is selected, **When** the details view opens, **Then** the user can inspect raw transcript, final transcript, profile, model metadata, latency, duration, and application context.
3. **Given** a history entry is deleted, **When** the delete action is confirmed, **Then** the history record is removed and any associated stored audio is also removed.

---

### User Story 3 - Tune Output for Developer Workflows (Priority: P2)

As a developer working across coding agents, terminals, documentation, and commit flows, I can choose transcription profiles, maintain global or project-specific dictionary entries, and configure application rules so output formatting matches the destination.

**Why this priority**: Product differentiation depends on the app understanding developer dictation better than generic speech tools, especially for code terms, filenames, punctuation, and agent prompts.

**Independent Test**: Can be fully tested by creating profiles and dictionary entries, assigning them manually or via application rules, dictating technical phrases, and verifying the final output preserves intended syntax and vocabulary.

**Acceptance Scenarios**:

1. **Given** the user defines a dictionary mapping such as `tail wind -> Tailwind`, **When** speech recognition produces the spoken phrase, **Then** deterministic cleanup can replace it with the configured output.
2. **Given** the active app rule maps Terminal to the Terminal profile, **When** the user dictates a command containing flags and paths, **Then** the output preserves command syntax rather than converting it to prose.
3. **Given** the user selects the Commit profile, **When** they dictate a natural-language change summary, **Then** the final output is transformed into concise commit-message style text.

---

### User Story 4 - Use Repository Context to Improve Transcripts (Priority: P2)

As a developer working within a source repository, I can associate a project with Banshee, index repository vocabulary locally, and have filenames, symbols, and dependency names improve future dictation and file references.

**Why this priority**: Repository-aware transcription is the main feature that makes Banshee purpose-built for agentic development rather than a generic Whisper wrapper.

**Independent Test**: Can be fully tested by adding a repository, indexing it, dictating references to files or symbols, and verifying that high-confidence matches are reflected in the cleaned transcript while low-confidence matches remain unchanged.

**Acceptance Scenarios**:

1. **Given** a repository containing `src/components/Toast.tsx`, **When** the user dictates a recognizable spoken file reference to that file, **Then** the cleaned result can emit a formatted file reference such as `@src/components/Toast.tsx` when confidence is high.
2. **Given** repository indexing produces a symbol such as `UserAuthProvider`, **When** the raw transcript contains a close but imperfect phrase, **Then** the cleanup pipeline may prefer the repository symbol only when confidence exceeds the replacement threshold.
3. **Given** a repository contains ignored dependency or build directories, **When** indexing runs, **Then** the indexer respects `.gitignore` and configured exclusions to avoid unnecessary scanning.

---

### User Story 5 - Optional Tiny Local LLM Cleanup (Priority: P3)

As a developer who wants slightly cleaner prompts, I can enable a tiny local cleanup model that refines transcripts after deterministic processing while preserving identifiers, filenames, paths, and technical terms.

**Why this priority**: This improves quality for agent prompts, but the application must remain fully useful without it. It should be optional and never block the core experience.

**Independent Test**: Can be fully tested by enabling or disabling the local cleanup model, dictating the same prompt twice, and comparing deterministic-only output to optionally refined output while ensuring the LLM never answers the prompt or invents content.

**Acceptance Scenarios**:

1. **Given** the cleanup LLM is disabled or no cleanup model is installed, **When** a transcription completes, **Then** the deterministic pipeline still produces a usable final transcript and normal operation continues.
2. **Given** the cleanup LLM is enabled with an installed compatible model, **When** a transcript completes deterministic processing, **Then** the cleanup model refines the text locally using a constrained prompt and returns only the cleaned transcript.
3. **Given** the cleanup model fails or exceeds its time budget, **When** refinement cannot complete, **Then** the system falls back to deterministic output without losing the transcript.

### Edge Cases

- Microphone permissions are denied or revoked after the app launches.
- A required local speech model is not installed.
- Hardware acceleration is requested but unavailable on the current device.
- The active platform does not permit reliable direct text injection.
- Wayland prevents direct key event injection and clipboard-based paste is the only safe fallback.
- The user releases the hotkey immediately, producing a very short or empty utterance.
- VAD trims too aggressively or too conservatively for the current environment.
- The same dictated phrase could refer to normal prose or spoken punctuation and confidence is ambiguous.
- Repository indexing encounters malformed files, very large repositories, or unsupported symbol grammars.
- History retention is disabled or old data must be pruned according to privacy settings.
- The user requests audio retention settings that conflict with a later delete action.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST run as a desktop application using Tauri 2 with a Rust native core and React plus TypeScript UI.
- **FR-002**: The system MUST operate 100% locally during normal usage after required models are installed and MUST not require cloud inference, accounts, telemetry, or external API calls.
- **FR-003**: The system MUST support a configurable global push-to-talk shortcut whose default interaction is press-and-hold to record and release to stop and transcribe.
- **FR-004**: The system MUST also support configurable shortcuts for toggle recording, cancel current transcription, and re-paste previous transcription.
- **FR-005**: The system MUST show a transparent, borderless, always-on-top, non-focus-stealing floating HUD with distinct hidden, listening, processing, complete, and error states.
- **FR-006**: The system MUST capture microphone audio locally using a native Rust audio stack and begin capture immediately on recording start.
- **FR-007**: The system MUST apply local voice activity detection to remove leading and trailing silence, expose configurable sensitivity, and surface VAD state to the HUD.
- **FR-008**: The system MUST perform local speech recognition through a generic `TranscriptionEngine` abstraction with `whisper.cpp` as the initial production backend.
- **FR-009**: The system MUST support speech model selection, engine profiles such as Fast, Balanced, Accurate, and Custom, and acceleration modes Auto, CPU, and GPU.
- **FR-010**: The system MUST function when GPU acceleration is unavailable by falling back to CPU inference.
- **FR-011**: The system MUST function without the optional cleanup LLM enabled.
- **FR-012**: The system MUST perform deterministic transcript cleanup before any optional LLM refinement.
- **FR-013**: Deterministic cleanup MUST support configurable filler removal, spoken punctuation and developer syntax handling, natural correction resolution where clear, and dictionary-based vocabulary substitution.
- **FR-014**: The system MUST support editable global and per-project dictionary entries with enable and disable controls.
- **FR-015**: The system MUST support first-class transcription profiles including Raw, Agent, Codex, Claude Code, Terminal, Commit, and Documentation, with behavior encoded as configurable processing rules rather than UI labels alone.
- **FR-016**: The system MUST support editable application rules that map active applications or windows to transcription profiles.
- **FR-017**: The system MUST detect the active application and window context where practical without stealing focus from the user’s current application.
- **FR-018**: The system MUST support automatic text insertion through an `OutputBackend` abstraction with platform-specific strategies and a reliable clipboard-paste fallback.
- **FR-019**: Clipboard fallback MUST preserve the user’s prior clipboard contents when that setting is enabled, attempt paste if allowed, and restore the previous clipboard after an appropriate delay.
- **FR-020**: If direct insertion is unavailable, the system MUST clearly notify the user and MUST leave the resulting text available on the clipboard.
- **FR-021**: The system MUST persist local history in SQLite, including raw and final transcript content, context metadata, model metadata, latency, duration, word count, and optional audio path.
- **FR-022**: The system MUST provide searchable history with actions to copy, re-paste, edit, re-run processing, and delete transcriptions.
- **FR-023**: Audio recording persistence MUST default to never and the system MUST support retention options Never, 24 hours, and Forever.
- **FR-024**: Deleting a transcription MUST also delete any associated stored audio.
- **FR-025**: The system MUST support a local model registry that tracks installed models, engine, path, approximate size, quantization, capability, and default selection.
- **FR-026**: The system MUST allow users to manually import compatible local models and MUST clearly indicate when a required speech model is missing.
- **FR-027**: The system MUST support repository/project registration, indexing, reindexing, exclusion configuration, status reporting, and project-specific dictionary entries.
- **FR-028**: Repository indexing MUST build a weighted lexical index from local repository sources such as filenames, directories, git-tracked files, dependency names, branch names, symbols, exported types, environment variables, and configuration names, while respecting `.gitignore` and configured excludes.
- **FR-029**: Repository-aware cleanup MUST use fuzzy matching and confidence thresholds to improve likely technical terms and spoken file references but MUST NOT silently replace low-confidence text.
- **FR-030**: The system MUST support developer-friendly file-reference formatting that can emit agent-oriented references such as `@src/components/Toast.tsx` when a high-confidence match exists.
- **FR-031**: The system MUST support an optional local cleanup model through a generic `CleanupEngine` abstraction with `llama.cpp` as the initial backend.
- **FR-032**: Cleanup model prompting MUST be tightly constrained to transcript refinement only, preserve technical identifiers, and return only the cleaned transcript.
- **FR-033**: Cleanup inference MUST be bounded so that deterministic output can be used as a fallback if the optional cleanup stage fails or takes too long.
- **FR-034**: The system MUST provide a polished desktop UI with primary sections Dashboard, History, Profiles, Dictionary, Projects, Models, and Settings.
- **FR-035**: The system MUST provide dashboard visibility into local-only status, transcription activity, selected microphone, selected models, active profile, and current shortcut.
- **FR-036**: The system MUST provide a system tray with actions for Open, Start or Stop Listening where appropriate, current profile visibility, Settings, and Quit.
- **FR-037**: The system MUST run unobtrusively in the background and support launch-at-login, start minimized, and minimize-to-tray behavior where supported.
- **FR-038**: The system MUST keep platform-dependent functionality behind interfaces including audio capture, VAD, transcription, cleanup, context, output, and active-window providers.
- **FR-039**: The React UI MUST communicate with the Rust core through typed Tauri commands and events rather than direct coupling to inference or platform APIs.
- **FR-040**: The system MUST preserve user text whenever possible on failures by falling back to clipboard or stored history rather than silently dropping a transcript.
- **FR-041**: The system MUST make the local-only privacy model visible in the UI without requiring network access for normal operation after model installation.
- **FR-042**: The system MUST document prerequisites, platform permissions, model installation, keyboard shortcut behavior, Linux and Wayland considerations, project architecture, and known limitations in the README.

### Key Entities *(include if feature involves data)*

- **Transcription**: A persisted record of one dictation event including raw text, final text, timing, application context, project context, profile choice, model metadata, and optional audio file path.
- **Project**: A local repository association including root path, indexing status, exclusion rules, vocabulary statistics, and optional app associations.
- **Project Index**: Derived lexical and symbol metadata extracted from a project for repository-aware transcript cleanup.
- **Dictionary Entry**: A spoken phrase to output mapping that can be scoped globally or to a project and can be enabled or disabled.
- **Profile**: A named transformation and output policy controlling cleanup behavior, syntax preservation, file-reference formatting, and optional cleanup-model usage.
- **Application Rule**: A mapping between an active application or window pattern and a selected transcription profile.
- **Model Record**: Metadata for an installed speech or cleanup model including engine, path, capabilities, size, quantization, and selection state.
- **Settings**: User preferences covering launch behavior, audio input, VAD, shortcuts, transcription behavior, intelligence features, output behavior, and privacy.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can complete the primary flow from hotkey press to final text insertion into a focused text field using only local processing and without opening the main window.
- **SC-002**: The HUD becomes visible quickly enough that users perceive feedback as immediate when recording starts.
- **SC-003**: When a valid speech model is installed, the app can transcribe a multi-sentence developer prompt, apply deterministic cleanup, and preserve the result in local history with raw and final text.
- **SC-004**: The application remains fully usable with the optional cleanup LLM disabled.
- **SC-005**: Users can create dictionary entries and project associations that measurably improve at least one subsequent transcription involving technical vocabulary or file references.
- **SC-006**: The product can be operated for normal daily use without requiring internet access after models are installed.
- **SC-007**: Failure modes such as missing model, missing microphone, no speech, paste restriction, or cleanup-model timeout preserve the user’s best available transcript rather than losing it.

## Assumptions

- Linux is treated as a first-class platform for architecture and UX, but some text-injection capabilities may vary between X11 and Wayland and require clipboard fallback.
- Users will source compatible local `whisper.cpp` and optional `llama.cpp` model files manually if automatic distribution is impractical.
- Initial language support can default to English-first behavior while still allowing a configurable language setting for future expansion.
- Tree-sitter grammar coverage may begin with common development languages and expand over time; unsupported languages still contribute filenames and other lexical context.
- The MVP prioritizes a rock-solid push-to-talk flow, local history, tray behavior, and deterministic cleanup over advanced hands-free interaction.
