# Banshee Voice

Banshee is a local Tauri desktop application that records microphone audio and transcribes it with Whisper `tiny.en-q5_1`.

The desktop window includes an editable scratch space. Separately, the global push-to-talk shortcut opens a compact bottom-center HUD: hold the shortcut to record, then release it to transcribe and paste into the text field that was focused when recording began. If that target is unavailable or changed, Banshee leaves the transcript on the clipboard. Closing or minimizing the desktop window keeps Banshee available from the system tray; use the tray's Quit command to stop it.

Target-aware paste uses Windows UI Automation, macOS Accessibility, and AT-SPI with XTest on X11. Wayland uses the safe clipboard fallback because it does not provide general-purpose global paste synthesis.

## Development prerequisites

- Rust and Node.js
- CMake and libclang/LLVM (required to build the `whisper-rs` native dependency; `npm run tauri:dev` bootstraps isolated copies automatically on Windows)
- Windows: Visual Studio C++ build tools
- macOS: Xcode command-line tools
- Linux: a C++ toolchain plus ALSA development headers

Install frontend dependencies with `npm install` in `apps/desktop`, then run `npm run tauri:dev`. On first launch, Banshee downloads the approximately 31 MiB speech model into the platform application-data directory and verifies its published SHA-1 before loading it. After installation, recording and transcription are offline.

For development or model verification, download the same model into the ignored repository `models` directory:

```powershell
./scripts/fetch-models.ps1
```

```bash
./scripts/fetch-models.sh
```
