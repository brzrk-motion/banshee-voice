#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
MODELS_DIR="${ROOT_DIR}/models"
WHISPER_DIR="${MODELS_DIR}/whisper"
LLAMA_DIR="${MODELS_DIR}/llama"

WHISPER_MODEL="tiny.en-q5_1"
FETCH_CLEANUP_MODEL=0
CLEANUP_URL=""

usage() {
  cat <<'EOF'
Usage: scripts/fetch-models.sh [options]

Downloads local model files into ./models, which is gitignored.

Options:
  --whisper-model <name>   Whisper preset: tiny.en-q5_1, tiny.en, base.en, small.en, medium.en
                           Default: tiny.en-q5_1
  --with-cleanup-model     Also download a default llama.cpp-compatible cleanup model
  --cleanup-url <url>      Download a cleanup model from a custom URL instead
  -h, --help               Show this help text

Examples:
  scripts/fetch-models.sh
  scripts/fetch-models.sh --whisper-model base.en
  scripts/fetch-models.sh --with-cleanup-model
  scripts/fetch-models.sh --cleanup-url https://example.com/model.gguf
EOF
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

download_file() {
  local url="$1"
  local output_path="$2"

  if [[ -f "$output_path" ]]; then
    printf 'Skipping existing file: %s\n' "$output_path"
    return
  fi

  mkdir -p "$(dirname "$output_path")"
  printf 'Downloading %s\n' "$url"
  curl -L --fail --progress-bar "$url" -o "$output_path"
}

whisper_url_for() {
  case "$1" in
    tiny.en-q5_1) printf 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin' ;;
    tiny.en) printf 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin' ;;
    base.en) printf 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin' ;;
    small.en) printf 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin' ;;
    medium.en) printf 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin' ;;
    *)
      printf 'Unsupported whisper model preset: %s\n' "$1" >&2
      exit 1
      ;;
  esac
}

default_cleanup_url() {
  printf 'https://huggingface.co/bartowski/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/Qwen2.5-0.5B-Instruct-Q4_K_M.gguf'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --whisper-model)
      WHISPER_MODEL="${2:-}"
      shift 2
      ;;
    --with-cleanup-model)
      FETCH_CLEANUP_MODEL=1
      shift
      ;;
    --cleanup-url)
      CLEANUP_URL="${2:-}"
      FETCH_CLEANUP_MODEL=1
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n\n' "$1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_command git
require_command curl

mkdir -p "$WHISPER_DIR" "$LLAMA_DIR"

WHISPER_URL="$(whisper_url_for "$WHISPER_MODEL")"
WHISPER_FILE="${WHISPER_URL##*/}"
download_file "$WHISPER_URL" "$WHISPER_DIR/$WHISPER_FILE"

if [[ "$FETCH_CLEANUP_MODEL" -eq 1 ]]; then
  if [[ -z "$CLEANUP_URL" ]]; then
    CLEANUP_URL="$(default_cleanup_url)"
  fi

  CLEANUP_FILE="${CLEANUP_URL##*/}"
  download_file "$CLEANUP_URL" "$LLAMA_DIR/$CLEANUP_FILE"
fi

printf '\nModels directory: %s\n' "$MODELS_DIR"
printf 'Whisper model: %s\n' "$WHISPER_DIR/$WHISPER_FILE"

if [[ "$FETCH_CLEANUP_MODEL" -eq 1 ]]; then
  printf 'Cleanup model: %s\n' "$LLAMA_DIR/$CLEANUP_FILE"
fi
