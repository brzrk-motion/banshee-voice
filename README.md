# Banshee Voice

Banshee is a local Tauri desktop application that records microphone audio and transcribes it into an editable scratch space with Whisper `tiny.en-q5_1`.

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
