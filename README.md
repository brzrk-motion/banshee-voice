# Banshee Voice

Banshee is a local Tauri desktop application that records microphone audio and transcribes it with Whisper `base.en`. Transcripts pass through enabled text transformation plugins before output. The built-in Transcript Cleanup plugin performs deterministic cleanup and can be turned off to preserve the raw transcription. Tauri dev/build goes through `scripts/tauri.mjs`, which also builds and bundles the Prompt Enhancer sidecar worker.

The desktop window includes an editable scratch space. Separately, the global push-to-talk shortcut opens a compact bottom-center HUD: hold the shortcut to record, then release it to transcribe and paste into the text field that was focused when recording began. If that target is unavailable or changed, Banshee leaves the transcript on the clipboard. Closing or minimizing the desktop window keeps Banshee available from the system tray; use the tray's Quit command to stop it.

Target-aware paste uses Windows UI Automation, macOS Accessibility, and AT-SPI with XTest on X11. Wayland uses the safe clipboard fallback because it does not provide general-purpose global paste synthesis.

## Development prerequisites

- Rust and Node.js
- Python and libclang/LLVM (required for native dependencies; `npm run dev` bootstraps an isolated CMake when none is installed and also bootstraps libclang on Windows)
- Windows: Visual Studio C++ build tools and the LunarG Vulkan SDK with `VULKAN_SDK` set
- macOS: Xcode command-line tools
- Linux: a C++ toolchain, ALSA development headers, `glslc`, Vulkan headers/loader, and the `SPIRV-Headers` CMake package

Linux package examples for the Vulkan dependencies:

```bash
# Arch Linux
sudo pacman -S shaderc vulkan-headers vulkan-icd-loader spirv-headers

# Ubuntu
sudo apt install glslc libvulkan-dev spirv-headers

# Fedora
sudo dnf install glslc vulkan-headers vulkan-loader-devel spirv-headers
```

Install all workspace dependencies from the repository root, then launch the desktop app:

```bash
npm install
npm run dev
```

The root Turbo workspace also provides `npm run build`, `npm run test`, `npm run check`, and `npm run desktop:build`. On first launch, Banshee downloads the approximately 141 MiB speech model into the platform application-data directory and verifies its published SHA-256 before loading it. Enabling Prompt Enhancer on the Plugins page downloads an additional approximately 2.64 GiB NVIDIA Nemotron 3 Nano 4B cleanup model and starts the bundled `banshee-prompt-worker` sidecar. Its Settings dialog selects the coding model that will receive the enhanced prompt. After installation, transcription and plugin processing are offline.

Linux and Windows builds include Vulkan acceleration for both Whisper and Prompt Enhancer. The Settings acceleration control supports `Auto` (prefer GPU and fall back to CPU), `CPU`, and `GPU` (require GPU and report initialization errors without silently falling back). History records the backend actually used for each transcription.

## Repository structure

- `apps/desktop` contains the Tauri desktop application.
- `packages/core` is the public Rust facade imported by Banshee applications.
- `crates` contains private Rust implementation crates used by the core facade.
- `crates/plugins` contains the generic ordered plugin registry, schema-driven settings validation, and failure-isolated execution. Plugins are compiled into Banshee in this release; external plugin discovery and loading are not yet supported.
- `plugins/transcript-cleanup` contains the built-in deterministic Transcript Cleanup plugin. It runs first when enabled and requires no model download.
- `plugins/prompt-enhancer` contains the Prompt Enhancer manifest, model metadata, host integration, and bundled inference sidecar, keeping its llama.cpp runtime isolated from Whisper's native runtime.
- `.opencode/skills/plugin-development` contains the repo-local plugin-development skill for end-to-end plugin work.

For development or model verification, download the same models into the ignored repo-local `models/` directory:

```powershell
./scripts/fetch-models.ps1
./scripts/fetch-models.ps1 -WhisperModel base.en -WithCleanupModel
```

```bash
./scripts/fetch-models.sh
./scripts/fetch-models.sh --whisper-model base.en --with-cleanup-model
./scripts/fetch-models.sh --cleanup-url https://example.com/model.gguf
```

The bash helper also accepts `--whisper-model <preset>` for `tiny.en-q5_1`, `tiny.en`, `base.en`, `small.en`, or `medium.en`.
