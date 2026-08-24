# Banshee Voice

Banshee is a local Tauri desktop application that records microphone audio and transcribes it with Whisper `base.en`. An optional local Qwen2.5 0.5B model can conservatively refine the transcript before output.

The desktop window includes an editable scratch space. Separately, the global push-to-talk shortcut opens a compact bottom-center HUD: hold the shortcut to record, then release it to transcribe and paste into the text field that was focused when recording began. If that target is unavailable or changed, Banshee leaves the transcript on the clipboard. Closing or minimizing the desktop window keeps Banshee available from the system tray; use the tray's Quit command to stop it.

Target-aware paste uses Windows UI Automation, macOS Accessibility, and AT-SPI with XTest on X11. Wayland uses the safe clipboard fallback because it does not provide general-purpose global paste synthesis.

## Development prerequisites

- Rust and Node.js
- CMake and libclang/LLVM (required to build the `whisper-rs` native dependency; `npm run dev` bootstraps isolated copies automatically on Windows)
- Windows: Visual Studio C++ build tools
- macOS: Xcode command-line tools
- Linux: a C++ toolchain plus ALSA development headers

Install all workspace dependencies from the repository root, then launch the desktop app:

```bash
npm install
npm run dev
```

The root Turbo workspace also provides `npm run build`, `npm run test`, `npm run check`, and `npm run desktop:build`. On first launch, Banshee downloads the approximately 141 MiB speech model into the platform application-data directory and verifies its published SHA-256 before loading it. Enabling the cleanup model in Settings downloads an additional approximately 379 MiB model. After installation, recording and cleanup are offline.

## Repository structure

- `apps/desktop` contains the Tauri desktop application.
- `packages/core` is the public Rust facade imported by Banshee applications.
- `crates` contains private Rust implementation crates used by the core facade.
- `plugins` is reserved for future independent plugin repositories. It is intentionally not an npm, Cargo, or Turbo workspace and contains no plugin architecture today.

For development or model verification, download the same model into the ignored repository `models` directory:

```powershell
./scripts/fetch-models.ps1
# Include the optional cleanup model:
./scripts/fetch-models.ps1 -WithCleanupModel
```

```bash
./scripts/fetch-models.sh
```
