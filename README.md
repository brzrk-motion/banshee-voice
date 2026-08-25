# Banshee Voice

Banshee is a local Tauri desktop application that records microphone audio and transcribes it with Whisper `base.en`. Transcripts pass through deterministic cleanup and then through any enabled text transformation plugins before output.

The desktop window includes an editable scratch space. Separately, the global push-to-talk shortcut opens a compact bottom-center HUD: hold the shortcut to record, then release it to transcribe and paste into the text field that was focused when recording began. If that target is unavailable or changed, Banshee leaves the transcript on the clipboard. Closing or minimizing the desktop window keeps Banshee available from the system tray; use the tray's Quit command to stop it.

Target-aware paste uses Windows UI Automation, macOS Accessibility, and AT-SPI with XTest on X11. Wayland uses the safe clipboard fallback because it does not provide general-purpose global paste synthesis.

## Development prerequisites

- Rust and Node.js
- Python and libclang/LLVM (required for native dependencies; `npm run dev` bootstraps an isolated CMake when none is installed and also bootstraps libclang on Windows)
- Windows: Visual Studio C++ build tools
- macOS: Xcode command-line tools
- Linux: a C++ toolchain plus ALSA development headers

Install all workspace dependencies from the repository root, then launch the desktop app:

```bash
npm install
npm run dev
```

The root Turbo workspace also provides `npm run build`, `npm run test`, `npm run check`, and `npm run desktop:build`. On first launch, Banshee downloads the approximately 141 MiB speech model into the platform application-data directory and verifies its published SHA-256 before loading it. Enabling Prompt Enhancer on the Plugins page downloads an additional approximately 379 MiB Qwen2.5 0.5B model. After installation, transcription and plugin processing are offline.

## Repository structure

- `apps/desktop` contains the Tauri desktop application.
- `packages/core` is the public Rust facade imported by Banshee applications.
- `crates` contains private Rust implementation crates used by the core facade.
- `crates/plugins` contains the ordered plugin registry and built-in Prompt Enhancer host integration. Plugins are compiled into Banshee in this release; external plugin discovery and loading are not yet supported.
- `crates/prompt-worker` runs Prompt Enhancer inference in a bundled sidecar process, keeping its llama.cpp runtime isolated from Whisper's native runtime.

For development or model verification, download the same model into the ignored repository `models` directory:

```powershell
./scripts/fetch-models.ps1
# Include the optional cleanup model:
./scripts/fetch-models.ps1 -WithCleanupModel
```

```bash
./scripts/fetch-models.sh
```
