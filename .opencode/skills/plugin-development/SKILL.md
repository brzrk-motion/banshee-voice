---
name: plugin-development
description: Use when adding or integrating a new Banshee plugin, plugin settings, plugin storage seeds, plugin model support, or a sidecar worker.
---

# Plugin Development

Use this as the end-to-end playbook for a new compiled-in Banshee plugin.

## What To Know First

- Plugins are compiled in. There is no dynamic discovery or loading.
- The shared trait is `TextTransformPlugin` in `crates/contracts/src/domain.rs`.
- The registry lives in `crates/plugins` and runs plugins in order.
- Plugin settings are schema-driven and currently only support `PluginSettingControl::Select`.
- Non-ready plugins are skipped, so `runtime_status()` must be honest.
- Existing examples:
  - `plugins/transcript-cleanup` for a deterministic plugin.
  - `plugins/prompt-enhancer` for a model-backed plugin with a worker sidecar.

## Start Here

Before coding, answer these questions:

1. What is the plugin supposed to transform?
2. Is it deterministic or model-backed?
3. Does it need settings?
4. Should it be enabled by default?
5. Does it need storage seeds, a model download, or a sidecar worker?
6. Does the UI need to show status, settings, or a retry action?

## Build It

### 1. Define the plugin contract

- Pick a stable ID like `banshee.<kebab-case-name>`.
- Write a short manifest with `name`, `description`, `version`, `author`, `stage`, and `settings`.
- Keep settings small unless the settings system itself must change.
- Return `PluginExecutionOutput { text, backend }` from `transform()`.

### 2. Create the plugin crate

- Add `plugins/<name>/Cargo.toml`.
- Add `plugins/<name>/src/lib.rs`.
- Add the crate to the root workspace `Cargo.toml`.
- Export it from `packages/core/src/lib.rs`.
- Add the dependency in `packages/core/Cargo.toml`.
- If the desktop app needs direct access, instantiate it in `apps/desktop/src-tauri/src/app_state/mod.rs`.
- Add it to `PluginRegistry::new(...)` in the correct execution order.

### 3. Seed storage if needed

- If the plugin needs a default enabled state or default settings, add a migration under `crates/storage/migrations/`.
- Register that migration in `crates/storage/src/migrations.rs`.
- Seed `plugin_states` with the desired default row.
- Keep in mind that settings are stored as JSON in `plugin_states.settings_json`.

### 4. Implement the transform

- `manifest()` should describe the plugin accurately.
- `runtime_status()` should return `Missing`, `Downloading`, `Loading`, `Ready`, or `Error` as appropriate.
- `transform()` should be deterministic for the same context and settings.
- Reject empty or invalid output by design; the registry will skip bad results.
- If the plugin should run before or after another plugin, put it in the registry order that way.

### 5. Wire the UI and IPC

- Add Tauri commands if users need to list, enable, disable, configure, or retry the plugin.
- Update the desktop plugin page and any settings dialog.
- Emit `plugins_changed` after enablement or settings changes.
- If the plugin introduces a new model capability, extend `ModelCapability`, `ModelsStatus`, the app state, and the retry path.

### 6. Add model support if needed

- Define a `ModelDescriptor` with capability, name, directory, file, URL, and SHA-256.
- Wire `ModelInstaller::from_descriptor(...)` into app state.
- Models download to `<app data>/models/<capability>/<file>`.
- Update `scripts/fetch-models.sh` and `scripts/fetch-models.ps1` for repo-local verification downloads.
- Surface the model through commands/UI if the user needs to see status or retry installation.

### 7. Add a sidecar worker if needed

- Put shared protocol and host code in `plugins/<name>/src/lib.rs`.
- Put the worker entrypoint in `plugins/<name>/src/main.rs`.
- Gate the worker build behind a `worker` feature.
- Build it with `scripts/build-sidecar.mjs`.
- Register the external binary in `scripts/tauri.mjs`.
- Make the host resolve the binary next to the desktop executable.

## Finish Strong

- Add unit tests for the plugin logic.
- Add tests for registry ordering, settings validation, and runtime readiness when relevant.
- Add worker/model tests if the plugin uses either.
- Verify with `npm run test`, `npm run check`, `npm run build`, and `npm run desktop:build` when the change crosses crates.

## Practical Rules

- Prefer the smallest plugin that solves the problem.
- Keep plugin logic compiled in; do not invent dynamic loading.
- Use only the existing `Select` setting control unless the broader settings system is changing.
- Do not commit downloaded models or build outputs.
