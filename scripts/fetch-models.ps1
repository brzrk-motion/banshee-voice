param(
    [ValidateSet('tiny.en-q5_1', 'tiny.en', 'base.en', 'small.en', 'medium.en')]
    [string]$WhisperModel = 'base.en',
    [switch]$WithCleanupModel
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$modelsDirectory = Join-Path $repositoryRoot 'models\whisper'
$files = @{
    'tiny.en-q5_1' = 'ggml-tiny.en-q5_1.bin'
    'tiny.en' = 'ggml-tiny.en.bin'
    'base.en' = 'ggml-base.en.bin'
    'small.en' = 'ggml-small.en.bin'
    'medium.en' = 'ggml-medium.en.bin'
}
$fileName = $files[$WhisperModel]
$outputPath = Join-Path $modelsDirectory $fileName
$url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$fileName"

New-Item -ItemType Directory -Force -Path $modelsDirectory | Out-Null
if (Test-Path -LiteralPath $outputPath) {
    Write-Host "Skipping existing file: $outputPath"
} else {
    Write-Host "Downloading $WhisperModel to $outputPath"
    Invoke-WebRequest -Uri $url -OutFile $outputPath
}
Write-Host "Whisper model: $outputPath"

if ($WithCleanupModel) {
    $cleanupDirectory = Join-Path $repositoryRoot 'models\llama'
    $cleanupFile = 'Qwen2.5-0.5B-Instruct-Q4_K_M.gguf'
    $cleanupPath = Join-Path $cleanupDirectory $cleanupFile
    $cleanupUrl = "https://huggingface.co/bartowski/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/$cleanupFile"
    New-Item -ItemType Directory -Force -Path $cleanupDirectory | Out-Null
    if (-not (Test-Path -LiteralPath $cleanupPath)) {
        Write-Host "Downloading cleanup model to $cleanupPath"
        Invoke-WebRequest -Uri $cleanupUrl -OutFile $cleanupPath
    }
    Write-Host "Cleanup model: $cleanupPath"
}
