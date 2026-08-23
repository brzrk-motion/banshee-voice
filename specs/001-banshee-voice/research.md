# Research: Banshee Local Voice Transcription App

## Decision 1: Use a thin Tauri shell with a Rust-owned application core

**Decision**: Structure the product as a Tauri 2 desktop shell plus a Rust core split into focused crates, with React/TypeScript used only for the main app UI and HUD rendering.

**Rationale**: The product's value is in hotkeys, audio, inference, indexing, persistence, and platform integration. Those responsibilities need native performance, strong typing, and cross-platform isolation. A Rust-owned core also keeps SQLite, privacy-sensitive history, and ML orchestration out of the frontend.

**Alternatives considered**:

- Put more logic in the frontend with Tauri plugins. Rejected because it weakens typed boundaries and pushes native concerns into UI code.
- Build a browser/server architecture. Rejected because it conflicts with local-only, no-background-service goals.

## Decision 2: Keep the hot path deterministic and reserve the LLM for bounded post-processing

**Decision**: Make `whisper.cpp` plus deterministic cleanup the primary success path. Run optional `llama.cpp` refinement only after deterministic output exists and only within a strict time budget.

**Rationale**: The app must remain useful with no cleanup LLM installed. Deterministic cleanup is fast, explainable, and reliable for developer vocabulary, punctuation, correction handling, and file references. A bounded optional LLM can improve readability without making the core experience fragile.

**Alternatives considered**:

- Use the LLM as the main cleanup and instruction-shaping engine. Rejected because it adds latency, variability, and failure risk.
- Skip the LLM entirely. Rejected for architecture because optional refinement is a stated product requirement, though the implementation will keep it out of the main critical path.

## Decision 3: Use a persistent-loaded model strategy with dedicated inference workers

**Decision**: Load the selected Whisper model once and keep it resident, run transcription and cleanup inference on dedicated blocking threads, and persist deterministic results before optional refinement completes.

**Rationale**: Model load time is one of the biggest sources of perceived latency. Persistent loaded models and dedicated workers prevent the UI thread from blocking and keep the app responsive during transcription.

**Alternatives considered**:

- Load models per transcription. Rejected because it is too slow for push-to-talk workflows.
- Run inference directly on the async runtime. Rejected because native FFI work can block cooperative executors and degrade app responsiveness.

## Decision 4: Treat Linux X11 and Linux Wayland as different operating environments

**Decision**: Design platform services with explicit support tiers: strong direct insertion and active-window handling on X11, opportunistic direct insertion on macOS/Windows, and clipboard-first fallback behavior on Wayland when direct injection is unavailable.

**Rationale**: Wayland intentionally restricts global input synthesis and portable active-window inspection. Productizing clipboard fallback as a first-class success path is more robust than promising a capability the session may not allow.

**Alternatives considered**:

- Assume generic text injection works everywhere. Rejected because it would produce unreliable behavior and poor UX on Wayland.
- Depend on privileged tools such as `/dev/uinput`-driven injectors. Rejected as a default because it raises security and packaging complexity.

## Decision 5: Use repository-derived lexical indexing instead of embeddings

**Decision**: Build repository context from filenames, directories, git-tracked paths, dependencies, branch names, symbols, exports, environment variables, and config names using local filesystem scanning, git metadata, Tree-sitter, and weighted lexical fuzzy matching.

**Rationale**: The feature requires repository-aware improvements without cloud calls, vector databases, or over-heavy infrastructure. A weighted lexical index is deterministic, fast to update, explainable, and aligned with file/symbol resolution needs.

**Alternatives considered**:

- Use embeddings or semantic search. Rejected because it adds complexity, model cost, and maintenance without being necessary for filename and symbol resolution.
- Limit context to filenames only. Rejected because symbols, dependencies, and env/config names are important developer vocabulary.

## Decision 6: Use Rust-owned SQLite with migrations and WAL mode

**Decision**: Keep persistence in Rust through a dedicated storage layer backed by SQLite, with explicit migrations, indexes, retention jobs, and write-ahead logging.

**Rationale**: History, settings, dictionary entries, model metadata, and project indexes are central local data. Rust-owned storage gives better control over migrations, privacy behavior, retention, and typed access than frontend-managed SQL.

**Alternatives considered**:

- Store settings in local frontend storage and history in ad hoc files. Rejected because it fragments data and weakens reliability.
- Use a Tauri frontend SQL plugin as the primary persistence surface. Rejected because persistence belongs in the Rust core.

## Decision 7: Model profiles should change processing rules before changing models

**Decision**: Represent profiles as configurable transformation behavior, syntax preservation, and output formatting rules, while keeping model selection mostly independent from profile choice.

**Rationale**: Switching models per profile would increase memory pressure and latency. Most profile differences are about punctuation, cleanup strictness, file-reference formatting, and how much explanatory context to preserve.

**Alternatives considered**:

- Hard-code profile behavior only in the frontend. Rejected because profile semantics belong in the transcript pipeline.
- Bind specific models to every profile by default. Rejected because the product needs stable latency and simpler model management.

## Decision 8: Use typed command and event contracts between UI and core

**Decision**: Expose user actions and data fetches as typed Tauri commands and pipeline/state updates as typed Tauri events, with high-frequency signals such as waveform/VAD updates throttled for HUD rendering.

**Rationale**: The HUD and main app need live state without coupling the frontend directly to inference or native platform code. Typed IPC preserves maintainability across many app sections.

**Alternatives considered**:

- Use unstructured event payloads and ad hoc command schemas. Rejected because the product contains many settings and state transitions.
- Poll from the frontend. Rejected because it increases latency and complexity for recording-state updates.
