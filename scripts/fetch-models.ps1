param(
    [ValidateSet('tiny.en-q5_1', 'tiny.en', 'base.en', 'small.en', 'medium.en')]
    [string]$WhisperModel = 'tiny.en-q5_1'
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
    exit 0
}

Write-Host "Downloading $WhisperModel to $outputPath"
Invoke-WebRequest -Uri $url -OutFile $outputPath
Write-Host "Whisper model: $outputPath"
