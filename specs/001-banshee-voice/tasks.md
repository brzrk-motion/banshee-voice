---

description: "Task list for implementing Banshee Local Voice Transcription App"
---

# Tasks: Banshee Local Voice Transcription App

**Input**: Design documents from `/specs/001-banshee-voice/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`

**Tests**: Dedicated test tasks are not generated because the feature spec does not require a strict test-first workflow. Validation tasks and runnable verification are included in the final polish phase and within each story checkpoint.

**Organization**: Tasks are grouped by user story so each story can be implemented and validated independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel when dependencies are satisfied
- **[Story]**: Maps the task to a user story from `spec.md`
- Every task includes an exact file path

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the workspace, desktop shell, and shared developer tooling.

- [X] T001 Create the Rust workspace manifest in `/workspace/banshee-voice/Cargo.toml`
- [X] T002 Create the desktop app package, Vite config, and TypeScript config in `/workspace/banshee-voice/apps/desktop/package.json`, `/workspace/banshee-voice/apps/desktop/vite.config.ts`, and `/workspace/banshee-voice/apps/desktop/tsconfig.json`
- [X] T003 [P] Create the desktop Tauri crate manifest and app config in `/workspace/banshee-voice/apps/desktop/src-tauri/Cargo.toml` and `/workspace/banshee-voice/apps/desktop/src-tauri/tauri.conf.json`
- [X] T004 [P] Create the base frontend entrypoints in `/workspace/banshee-voice/apps/desktop/src/main.tsx`, `/workspace/banshee-voice/apps/desktop/src/app/App.tsx`, and `/workspace/banshee-voice/apps/desktop/src/styles/index.css`
- [X] T005 [P] Create shared Rust crate manifests for `/workspace/banshee-voice/crates/audio/Cargo.toml`, `/workspace/banshee-voice/crates/vad/Cargo.toml`, `/workspace/banshee-voice/crates/stt/Cargo.toml`, `/workspace/banshee-voice/crates/transformer/Cargo.toml`, `/workspace/banshee-voice/crates/context/Cargo.toml`, `/workspace/banshee-voice/crates/dictionary/Cargo.toml`, `/workspace/banshee-voice/crates/injector/Cargo.toml`, `/workspace/banshee-voice/crates/history/Cargo.toml`, `/workspace/banshee-voice/crates/models/Cargo.toml`, `/workspace/banshee-voice/crates/platform/Cargo.toml`, `/workspace/banshee-voice/crates/storage/Cargo.toml`, and `/workspace/banshee-voice/crates/core/Cargo.toml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the core architecture that every user story depends on.

**⚠️ CRITICAL**: No user story work should begin until this phase is complete.

- [X] T006 Create the shared domain types and trait definitions in `/workspace/banshee-voice/crates/core/src/domain.rs`
- [X] T007 [P] Create the typed IPC models shared by Tauri commands and events in `/workspace/banshee-voice/apps/desktop/src-tauri/src/app_state/ipc.rs`
- [X] T008 [P] Create the SQLite connection, migration runner, and app-data path helpers in `/workspace/banshee-voice/crates/storage/src/lib.rs` and `/workspace/banshee-voice/crates/storage/src/migrations.rs`
- [X] T009 [P] Create the initial database migration for settings, profiles, models, projects, dictionary entries, and transcriptions in `/workspace/banshee-voice/crates/storage/migrations/0001_initial.sql`
- [X] T010 [P] Create the settings repository and default built-in profile seed logic in `/workspace/banshee-voice/crates/storage/src/settings_repo.rs` and `/workspace/banshee-voice/crates/storage/src/profile_repo.rs`
- [X] T011 [P] Create the platform capability probe and session-type detection in `/workspace/banshee-voice/crates/platform/src/capabilities.rs`
- [X] T012 Create the core application state container and service wiring in `/workspace/banshee-voice/apps/desktop/src-tauri/src/app_state/mod.rs` and `/workspace/banshee-voice/crates/core/src/lib.rs`
- [X] T013 Create the Tauri bootstrap, window registration, tray bootstrap, and command registration in `/workspace/banshee-voice/apps/desktop/src-tauri/src/main.rs`, `/workspace/banshee-voice/apps/desktop/src-tauri/src/windows/mod.rs`, and `/workspace/banshee-voice/apps/desktop/src-tauri/src/tray/mod.rs`

**Checkpoint**: The workspace builds, the database initializes, typed IPC exists, and the app can launch a main window and HUD shell.

---

## Phase 3: User Story 1 - Fast Push-to-Talk Dictation (Priority: P1) 🎯 MVP

**Goal**: Deliver the core hold-to-talk workflow with instant HUD feedback, local transcription, deterministic cleanup, and direct insert or clipboard fallback.

**Independent Test**: Launch the app, import/select a Whisper model and microphone, focus a normal text field, hold the shortcut, speak, release, and verify the cleaned transcript is inserted or copied with a clear fallback notice.

### Implementation for User Story 1

- [X] T014 [P] [US1] Implement microphone capture session types and device enumeration in `/workspace/banshee-voice/crates/audio/src/lib.rs`
- [X] T015 [P] [US1] Implement the VAD engine and trim result types in `/workspace/banshee-voice/crates/vad/src/lib.rs`
- [X] T016 [P] [US1] Implement the Whisper transcription engine adapter and request/result types in `/workspace/banshee-voice/crates/stt/src/lib.rs`
- [X] T017 [P] [US1] Implement deterministic cleanup rules for filler removal, spoken punctuation, correction handling, and profile flags in `/workspace/banshee-voice/crates/transformer/src/lib.rs`
- [X] T018 [P] [US1] Implement output injection backends and clipboard-preserving fallback in `/workspace/banshee-voice/crates/injector/src/lib.rs`
- [X] T019 [P] [US1] Implement active application and window detection adapters in `/workspace/banshee-voice/crates/platform/src/active_window.rs`
- [X] T020 [US1] Implement the end-to-end recording and transcription orchestrator in `/workspace/banshee-voice/crates/core/src/pipeline.rs`
- [X] T021 [US1] Implement hotkey registration, manual start/stop/cancel commands, and HUD state event emission in `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/recording.rs` and `/workspace/banshee-voice/apps/desktop/src-tauri/src/events/hud.rs`
- [X] T022 [US1] Build the transparent HUD window UI with listening, processing, complete, and error states in `/workspace/banshee-voice/apps/desktop/src/hud/HudApp.tsx`, `/workspace/banshee-voice/apps/desktop/src/hud/HudState.ts`, and `/workspace/banshee-voice/apps/desktop/src/styles/hud.css`
- [X] T023 [US1] Build the dashboard and input settings needed to select microphone, shortcuts, and speech model in `/workspace/banshee-voice/apps/desktop/src/features/dashboard/DashboardPage.tsx` and `/workspace/banshee-voice/apps/desktop/src/features/settings/InputSettings.tsx`

**Checkpoint**: User Story 1 is complete when local push-to-talk dictation works end-to-end without opening the main app during normal use.

---

## Phase 4: User Story 2 - Review and Recover Previous Transcripts (Priority: P1)

**Goal**: Persist transcription history locally and provide recovery actions so failed output or mistakes are non-catastrophic.

**Independent Test**: Complete one or more transcriptions, open History, inspect raw and final text, re-paste or copy an entry, edit or re-run processing, and delete it with retained audio cleanup.

### Implementation for User Story 2

- [ ] T024 [P] [US2] Implement the transcription repository, search query support, and retention helpers in `/workspace/banshee-voice/crates/history/src/lib.rs`
- [ ] T025 [P] [US2] Extend the transcriptions schema with FTS and indexes in `/workspace/banshee-voice/crates/storage/migrations/0002_history_indexes.sql`
- [ ] T026 [P] [US2] Implement history list, detail, delete, re-paste, and re-run Tauri commands in `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/history.rs`
- [ ] T027 [US2] Persist raw, deterministic, and final transcript records from the pipeline in `/workspace/banshee-voice/crates/core/src/persistence.rs`
- [ ] T028 [US2] Build the History list screen with search and preview rows in `/workspace/banshee-voice/apps/desktop/src/features/history/HistoryPage.tsx`
- [ ] T029 [US2] Build the History detail panel and recovery actions in `/workspace/banshee-voice/apps/desktop/src/features/history/HistoryDetail.tsx`
- [ ] T030 [US2] Implement local edit and re-run processing UX wiring in `/workspace/banshee-voice/apps/desktop/src/features/history/HistoryActions.tsx`

**Checkpoint**: User Story 2 is complete when every usable transcript is locally recoverable and re-usable from History.

---

## Phase 5: User Story 3 - Tune Output for Developer Workflows (Priority: P2)

**Goal**: Let users shape output using profiles, dictionary entries, and app-based profile routing.

**Independent Test**: Create or edit dictionary entries and profile settings, assign an app rule, dictate technical phrases or commands, and verify the final transcript preserves the intended syntax and vocabulary.

### Implementation for User Story 3

- [ ] T031 [P] [US3] Implement dictionary repositories and scoped lookup logic in `/workspace/banshee-voice/crates/dictionary/src/lib.rs` and `/workspace/banshee-voice/crates/storage/src/dictionary_repo.rs`
- [ ] T032 [P] [US3] Implement profile repositories and built-in profile transformation rules in `/workspace/banshee-voice/crates/storage/src/profile_repo.rs` and `/workspace/banshee-voice/crates/transformer/src/profiles.rs`
- [ ] T033 [P] [US3] Implement application rule repositories and active-profile resolution in `/workspace/banshee-voice/crates/storage/src/app_rule_repo.rs` and `/workspace/banshee-voice/crates/core/src/profile_resolution.rs`
- [ ] T034 [US3] Integrate dictionary, profile, and app-rule evaluation into deterministic cleanup in `/workspace/banshee-voice/crates/transformer/src/lib.rs`
- [ ] T035 [US3] Implement profile, dictionary, and app-rule Tauri commands in `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/profiles.rs` and `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/dictionary.rs`
- [ ] T036 [US3] Build the Profiles management UI in `/workspace/banshee-voice/apps/desktop/src/features/profiles/ProfilesPage.tsx`
- [ ] T037 [US3] Build the Dictionary editor and application-rules settings UI in `/workspace/banshee-voice/apps/desktop/src/features/dictionary/DictionaryPage.tsx` and `/workspace/banshee-voice/apps/desktop/src/features/settings/ApplicationRulesSettings.tsx`

**Checkpoint**: User Story 3 is complete when developer vocabulary and destination-specific formatting are user-configurable and affect future transcriptions.

---

## Phase 6: User Story 4 - Use Repository Context to Improve Transcripts (Priority: P2)

**Goal**: Add project registration, repository indexing, and high-confidence file/symbol corrections for developer dictation.

**Independent Test**: Add a repository, run indexing, dictate a known file or symbol reference, and verify high-confidence replacements occur while ambiguous phrases remain unchanged.

### Implementation for User Story 4

- [ ] T038 [P] [US4] Implement the project repository and index snapshot persistence in `/workspace/banshee-voice/crates/storage/src/project_repo.rs`
- [ ] T039 [P] [US4] Implement repository discovery, ignore handling, and git metadata extraction in `/workspace/banshee-voice/crates/context/src/repository_scan.rs`
- [ ] T040 [P] [US4] Implement Tree-sitter symbol extraction and dependency/config term extraction in `/workspace/banshee-voice/crates/context/src/symbols.rs`
- [ ] T041 [P] [US4] Implement weighted lexical indexing, fuzzy matching, and confidence scoring in `/workspace/banshee-voice/crates/context/src/index.rs`
- [ ] T042 [US4] Implement project registration, reindex, and index progress orchestration in `/workspace/banshee-voice/crates/core/src/projects.rs`
- [ ] T043 [US4] Integrate repository-context substitutions and file-reference formatting into transcript cleanup in `/workspace/banshee-voice/crates/transformer/src/contextual.rs`
- [ ] T044 [US4] Implement project and indexing Tauri commands plus progress events in `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/projects.rs` and `/workspace/banshee-voice/apps/desktop/src-tauri/src/events/projects.rs`
- [ ] T045 [US4] Build the Projects screen with add, reindex, exclusion, and status UI in `/workspace/banshee-voice/apps/desktop/src/features/projects/ProjectsPage.tsx`

**Checkpoint**: User Story 4 is complete when local repository context improves file and symbol references without low-confidence silent rewrites.

---

## Phase 7: User Story 5 - Optional Tiny Local LLM Cleanup (Priority: P3)

**Goal**: Add an optional, bounded local cleanup LLM that refines transcripts without becoming required for normal operation.

**Independent Test**: Enable a local cleanup model, dictate a messy developer instruction, compare deterministic and final output, disable the model, and verify the app still produces useful deterministic output with no regression.

### Implementation for User Story 5

- [ ] T046 [P] [US5] Implement the model registry repository and verification logic for speech and cleanup models in `/workspace/banshee-voice/crates/models/src/lib.rs` and `/workspace/banshee-voice/crates/storage/src/model_repo.rs`
- [ ] T047 [P] [US5] Implement the `llama.cpp` cleanup engine adapter and bounded prompt execution in `/workspace/banshee-voice/crates/transformer/src/llm.rs`
- [ ] T048 [US5] Integrate optional cleanup-model loading, timeout fallback, and telemetry-free error reporting into the pipeline in `/workspace/banshee-voice/crates/core/src/pipeline.rs`
- [ ] T049 [US5] Implement model import/list commands and cleanup-model settings commands in `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/models.rs` and `/workspace/banshee-voice/apps/desktop/src-tauri/src/commands/settings.rs`
- [ ] T050 [US5] Build the Models screen and intelligence settings UI for enabling cleanup LLM behavior in `/workspace/banshee-voice/apps/desktop/src/features/models/ModelsPage.tsx` and `/workspace/banshee-voice/apps/desktop/src/features/settings/IntelligenceSettings.tsx`

**Checkpoint**: User Story 5 is complete when cleanup LLM refinement is optional, bounded, and safely falls back to deterministic output.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Finish system tray behavior, privacy UX, packaging guidance, and end-to-end validation across stories.

- [ ] T051 [P] Implement the global navigation shell and section routing for Dashboard, History, Profiles, Dictionary, Projects, Models, and Settings in `/workspace/banshee-voice/apps/desktop/src/app/App.tsx` and `/workspace/banshee-voice/apps/desktop/src/app/Sidebar.tsx`
- [ ] T052 [P] Implement tray actions, launch-at-login, start-minimized, and previous-transcription shortcuts in `/workspace/banshee-voice/apps/desktop/src-tauri/src/tray/mod.rs` and `/workspace/banshee-voice/crates/platform/src/autostart.rs`
- [ ] T053 [P] Add privacy messaging, Linux platform notes, and manual validation guidance to `/workspace/banshee-voice/README.md` and `/workspace/banshee-voice/docs/platform/linux.md`
- [ ] T054 Run the quickstart validation scenarios and record any implementation follow-ups in `/workspace/banshee-voice/specs/001-banshee-voice/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1: Setup** has no dependencies and starts immediately.
- **Phase 2: Foundational** depends on Setup and blocks all story work.
- **Phase 3: US1** depends on Foundational and is the MVP.
- **Phase 4: US2** depends on Foundational and on the persisted transcription outputs created in US1.
- **Phase 5: US3** depends on Foundational and can start after the transcript pipeline interfaces exist.
- **Phase 6: US4** depends on Foundational and builds on the cleanup/profile pipeline from US1 and US3.
- **Phase 7: US5** depends on Foundational and on the deterministic pipeline from US1.
- **Phase 8: Polish** depends on the desired stories being complete.

### User Story Dependencies

- **US1**: No dependency on other user stories.
- **US2**: Depends on US1 because history is populated by completed transcriptions.
- **US3**: Can begin after Phase 2 but integrates best after US1 pipeline wiring exists.
- **US4**: Depends on US3 cleanup hooks for repository-aware substitutions.
- **US5**: Depends on US1 deterministic transcript pipeline and model management foundations.

### Recommended Completion Order

1. Phase 1 Setup
2. Phase 2 Foundational
3. Phase 3 US1
4. Phase 4 US2
5. Phase 5 US3
6. Phase 6 US4
7. Phase 7 US5
8. Phase 8 Polish

---

## Parallel Opportunities

### Setup

- T003, T004, and T005 can run in parallel after T001 starts the workspace layout.

### Foundational

- T007 through T011 can run in parallel once the workspace exists.
- T012 depends on T006 through T011.
- T013 depends on T007 and T012.

### User Story 1

- T014 through T019 can run in parallel as subsystem implementations.
- T020 depends on T014 through T019.
- T021 and T022 can proceed in parallel after T020 defines pipeline state/events.
- T023 depends on T021 for command availability.

### User Story 2

- T024, T025, and T026 can run in parallel.
- T028 and T029 can run in parallel after T026 and T027 exist.

### User Story 3

- T031, T032, and T033 can run in parallel.
- T036 and T037 can run in parallel after T035.

### User Story 4

- T038 through T041 can run in parallel.
- T044 and T045 can run in parallel after T042.

### User Story 5

- T046 and T047 can run in parallel.
- T049 and T050 can run in parallel after T048.

---

## Parallel Example: User Story 1

```bash
Task: "Implement microphone capture session types and device enumeration in /workspace/banshee-voice/crates/audio/src/lib.rs"
Task: "Implement the VAD engine and trim result types in /workspace/banshee-voice/crates/vad/src/lib.rs"
Task: "Implement the Whisper transcription engine adapter and request/result types in /workspace/banshee-voice/crates/stt/src/lib.rs"
Task: "Implement deterministic cleanup rules for filler removal, spoken punctuation, correction handling, and profile flags in /workspace/banshee-voice/crates/transformer/src/lib.rs"
Task: "Implement output injection backends and clipboard-preserving fallback in /workspace/banshee-voice/crates/injector/src/lib.rs"
Task: "Implement active application and window detection adapters in /workspace/banshee-voice/crates/platform/src/active_window.rs"
```

## Parallel Example: User Story 4

```bash
Task: "Implement the project repository and index snapshot persistence in /workspace/banshee-voice/crates/storage/src/project_repo.rs"
Task: "Implement repository discovery, ignore handling, and git metadata extraction in /workspace/banshee-voice/crates/context/src/repository_scan.rs"
Task: "Implement Tree-sitter symbol extraction and dependency/config term extraction in /workspace/banshee-voice/crates/context/src/symbols.rs"
Task: "Implement weighted lexical indexing, fuzzy matching, and confidence scoring in /workspace/banshee-voice/crates/context/src/index.rs"
```

---

## Implementation Strategy

### MVP First

1. Complete Phase 1 Setup.
2. Complete Phase 2 Foundational.
3. Complete Phase 3 US1.
4. Validate the quickstart core push-to-talk scenario before expanding scope.

### Incremental Delivery

1. Add US2 once the pipeline is stable so recovery is always available.
2. Add US3 to make output useful across agent, terminal, and documentation contexts.
3. Add US4 for repository-aware differentiation.
4. Add US5 last so optional cleanup never destabilizes the deterministic path.

### Suggested MVP Scope

- Phase 1 Setup
- Phase 2 Foundational
- Phase 3 US1
- Minimum tray/dashboard pieces from T023 and T052 only if required to support the core workflow

---

## Independent Test Criteria by Story

- **US1**: Hold shortcut, speak, release, and observe local transcript insertion or clipboard fallback without focus stealing.
- **US2**: Open History and recover, inspect, re-run, or delete a previously recorded transcription.
- **US3**: Apply dictionary entries, profiles, and app rules to meaningfully change future transcript formatting.
- **US4**: Index a repository and verify high-confidence file or symbol references are resolved while ambiguous phrases are not silently changed.
- **US5**: Enable and disable the cleanup LLM and confirm bounded refinement with deterministic fallback.

---

## Notes

- The repository constitution file is still a placeholder template, so no extra constitution-specific task gates were added.
- All checklist items use the required `- [ ] T###` format with explicit file paths.
- The user input context `Ke` does not add any additional scoping constraints beyond the existing feature specification.
