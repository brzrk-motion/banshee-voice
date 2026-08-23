# Implementation Plan: Banshee Local Voice Transcription App

**Branch**: `001-banshee-voice` | **Date**: 2026-08-22 | **Spec**: [`specs/001-banshee-voice/spec.md`](spec.md)

**Input**: Feature specification from `/specs/001-banshee-voice/spec.md`

## Summary

Build Banshee as a Linux-first, fully local Tauri 2 desktop application with a thin React UI and a Rust-owned core that handles hotkeys, microphone capture, VAD, `whisper.cpp` transcription, deterministic developer-aware cleanup, optional `llama.cpp` transcript refinement, repository vocabulary indexing, history persistence, and cross-platform text insertion with clipboard-preserving fallback. Optimize the product around the press-hold-speak-release-insert loop, with a dedicated transparent HUD window and strong fallback behavior on Linux Wayland.

## Technical Context

**Language/Version**: Rust stable (Edition 2024), TypeScript 5.x, React 19, Tauri 2

**Primary Dependencies**: Tauri 2, React, Vite, SQLite, `cpal`, `whisper.cpp` via Rust FFI/bindings, `llama.cpp` via Rust FFI/bindings, Tree-sitter, `git2` or native `git` process invocation, `ignore`, `rusqlite` or `sqlx` with SQLite, `serde`, `tokio`, `tracing`, `specta`/typed IPC support

**Storage**: SQLite for app state and history, filesystem app-data directory for models, optional retained audio, and project indexes/cache artifacts

**Testing**: `cargo test` for Rust crates, integration tests for pipeline/services, UI unit tests with Vitest, desktop end-to-end validation through Tauri dev/build smoke tests and manual platform verification matrix

**Target Platform**: Desktop application for Linux, macOS, and Windows, with Linux first-class and explicit X11 vs Wayland behavioral differences

**Project Type**: Desktop app with Rust core, Tauri shell, and React/TypeScript frontend

**Performance Goals**: HUD visible in under 50 ms from hotkey press on warmed app state; recording starts immediately; deterministic cleanup adds near-zero overhead; balanced local transcription returns quickly enough to preserve conversational flow; optional cleanup LLM remains bounded and cancellable so it never stalls the primary workflow

**Constraints**: 100% local after model installation; no network dependency; no Python runtime; no Docker; no Ollama; no focus stealing; no silent transcript loss; direct text injection must degrade cleanly to clipboard fallback; Wayland restrictions are expected and must be productized rather than ignored

**Scale/Scope**: Single-user desktop utility; thousands of history entries; dozens of tracked projects; per-project lexical indexes derived from medium-size repositories; one active transcription pipeline at a time with background indexing and model management

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The repository constitution at `.specify/memory/constitution.md` is still an unfilled template and does not define enforceable project-specific principles, gates, or governance rules yet.

Pre-Phase 0 gate result:

- No ratified project constitution constraints are currently available to fail.
- This plan therefore proceeds under the user requirements in `spec.md` as the governing source for scope, privacy, quality, and architecture.
- Follow-up requirement: ratify a real constitution before implementation begins so future planning and review can enforce repository-specific rules.

Post-Phase 1 gate result:

- Design artifacts remain aligned with the feature spec's non-negotiables: local-only operation, no cloud/API dependency, Linux-first support, optional LLM cleanup, Rust-owned core, Tauri command/event boundary, and explicit platform fallback behavior.
- No constitution violations are recorded because no concrete constitution rules exist yet.

## Project Structure

### Documentation (this feature)

```text
specs/001-banshee-voice/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── tauri-ipc.md
│   └── transcript-pipeline.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/
└── desktop/
    ├── src/
    │   ├── app/
    │   ├── components/
    │   ├── features/
    │   │   ├── dashboard/
    │   │   ├── history/
    │   │   ├── profiles/
    │   │   ├── dictionary/
    │   │   ├── projects/
    │   │   ├── models/
    │   │   └── settings/
    │   ├── hud/
    │   ├── lib/
    │   └── styles/
    └── src-tauri/
        ├── src/
        │   ├── app_state/
        │   ├── commands/
        │   ├── events/
        │   ├── tray/
        │   ├── windows/
        │   └── main.rs
        ├── capabilities/
        ├── icons/
        └── tauri.conf.json

crates/
├── audio/
├── vad/
├── stt/
├── transformer/
├── context/
├── dictionary/
├── injector/
├── history/
├── models/
├── platform/
├── storage/
└── core/

docs/
└── platform/

tests/
├── integration/
├── contract/
└── fixtures/
```

**Structure Decision**: Use a workspace-oriented desktop architecture with `apps/desktop` as the Tauri plus React shell and a set of Rust crates for independent subsystems. Keep SQLite, inference, indexing, shortcuts, clipboard handling, and orchestration in Rust. Keep the frontend responsible for configuration, history, and HUD presentation only.

## Complexity Tracking

No constitution-driven exceptions are currently required.
