# Agent Guide

This repo is Banshee Voice, a local Tauri desktop app. The backend is Rust, the frontend is Vite/React under `apps/desktop/src`, and the app records microphone audio, trims it with VAD, transcribes it with whisper.cpp, then runs enabled text-transform plugins before inserting or copying the result.

## Architecture

- `apps/desktop/src-tauri` owns Tauri setup, app state, IPC commands, hotkeys, tray, and windows.
- `apps/desktop/src` owns the UI.
- `packages/core` is the orchestration facade used by the desktop app; it re-exports the implementation crates and contains the recording pipeline.
- `crates/contracts` defines the shared domain types and traits.
- `crates/audio`, `crates/vad`, `crates/stt`, `crates/injector`, `crates/models`, `crates/platform`, `crates/history`, `crates/dictionary`, `crates/storage`, and `crates/plugins` are private Rust implementation crates.
- `plugins/*` contains built-in plugin crates.
- `scripts/*` contains build helpers, model fetchers, and the prompt-worker sidecar build.
- `models/` is a gitignored local cache for downloaded models.

## Runtime Flow

1. Audio capture starts from the selected microphone.
2. `crates/vad` trims silence and detects speech.
3. `crates/stt::WhisperCppEngine` loads the speech model and transcribes the trimmed audio.
4. `crates/plugins::PluginRegistry` runs enabled plugins in registry order.
5. `crates/injector` pastes into the captured target or falls back to clipboard-only delivery.
6. Results and status updates are surfaced through Tauri commands and events.

## Plugin System

- Plugins are compiled in. There is no dynamic plugin discovery or loading.
- The shared trait is `TextTransformPlugin` in `crates/contracts/src/domain.rs`.
- A plugin provides a `PluginManifest`, `runtime_status`, and `transform(context, settings)`.
- Plugin settings are schema-driven. Right now the only supported control is `PluginSettingControl::Select`.
- The registry validates settings against the manifest and resolves defaults before execution.
- Plugin order matters. The registry runs plugins in the order passed to `PluginRegistry::new(...)`.
- Non-ready plugins are skipped and recorded as skipped or failed in `PluginRunRecord`.
- The desktop app state emits `plugins_changed` after enablement or settings changes.

### Existing Plugins

- `plugins/transcript-cleanup`: deterministic cleanup, no model, enabled by default.
- `plugins/prompt-enhancer`: LLM-backed prompt rewriting, disabled by default, uses a worker sidecar and a local cleanup model.

### Adding a Plugin

1. Create `plugins/<name>/Cargo.toml` and `src/lib.rs`.
2. Add the crate to the root workspace `Cargo.toml`.
3. Export it from `packages/core/src/lib.rs` and add the dependency in `packages/core/Cargo.toml`.
4. Instantiate it in `apps/desktop/src-tauri/src/app_state/mod.rs` and add it to `PluginRegistry::new(...)`.
5. If it needs persistent defaults or seeded rows, add or update a migration in `crates/storage/migrations/` and register it in `crates/storage/src/migrations.rs`.
6. Add frontend and IPC updates if users need to view or edit its settings.
7. If it needs a worker binary, follow the prompt-enhancer pattern below.

### Prompt Enhancer Sidecar Pattern

- Shared plugin and protocol code live in `plugins/prompt-enhancer/src/lib.rs`.
- The worker entrypoint lives in `plugins/prompt-enhancer/src/main.rs`.
- The worker binary is built with `--features worker`.
- `scripts/build-sidecar.mjs` builds the worker and copies it to `apps/desktop/src-tauri/binaries/`.
- `scripts/tauri.mjs` registers `binaries/banshee-prompt-worker` as an external binary for Tauri.
- `app_state::prompt_worker_path()` expects `banshee-prompt-worker` or `banshee-prompt-worker.exe` next to the desktop executable.

## Models

- `crates/models::ModelInstaller` downloads a model to `<app data>/models/<capability>/<file>`.
- It verifies the SHA-256 digest before marking the model ready.
- `crates/storage::resolve_data_dir()` chooses the app data directory; `BANSHEE_APP_DATA_DIR` overrides the OS default.
- `ModelCapability` currently has `Speech` and `Cleanup`.
- The speech model is `base.en` from `whisper.cpp`.
- The prompt enhancer model is `Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` from Hugging Face.
- `scripts/fetch-models.sh` and `scripts/fetch-models.ps1` download verification copies into the gitignored repo-local `models/` directory.
- The desktop app exposes model status through Tauri commands and retries installation with `model_download_retry`.

### Adding a Model

1. Define a `ModelDescriptor` with capability, name, directory, file, URL, and SHA-256.
2. Wire a `ModelInstaller::from_descriptor(...)` into app state.
3. Update any commands or UI that should show status or retry the download.
4. Add a fetch helper or docs if developers need a repo-local copy for verification.

## Persistence

- `crates/storage` owns SQLite setup, migrations, and repositories.
- `plugin_states` stores plugin enablement and serialized settings JSON.
- Migrations seed default plugin state.
- `banshee.prompt-enhancer` starts disabled.
- `banshee.transcript-cleanup` starts enabled.
- Profiles, settings, history, and plugin state all live in the same SQLite database under the app data directory.

## Conventions

- Keep changes minimal and consistent with existing patterns.
- Prefer compiled-in plugins over new abstraction layers.
- Use `.opencode/skills/plugin-development/SKILL.md` as the canonical walkthrough for new plugin work.
- Do not commit build outputs or downloaded models.
- Useful workspace commands: `npm run dev`, `npm run test`, `npm run check`, `npm run build`, `npm run desktop:build`.

## Useful Paths

- `packages/core/src/pipeline.rs`
- `crates/contracts/src/domain.rs`
- `crates/plugins/src/lib.rs`
- `crates/models/src/lib.rs`
- `crates/storage/src/lib.rs`
- `apps/desktop/src-tauri/src/app_state/mod.rs`
- `apps/desktop/src-tauri/src/commands/*.rs`
- `plugins/transcript-cleanup/src/lib.rs`
- `plugins/prompt-enhancer/src/lib.rs`
- `plugins/prompt-enhancer/src/main.rs`
- `.opencode/skills/plugin-development/SKILL.md`
