<#
.SYNOPSIS
  Produce a self-contained before/after denoise report with the real
  DeepFilterNet3 backend. The one command that turns a recording into a demo.

.EXAMPLE
  demo\make-report.ps1                        # try it now with a real-speech sample
  demo\make-report.ps1 noisy.wav              # your clip, no clean reference
  demo\make-report.ps1 noisy.wav clean.wav    # with a reference => measured SNR gain

  Output lands in demo\out\: cleaned.wav, report.json, report.html
  Open report.html in any browser — it is fully offline.
#>
param(
  [string]$Noisy,
  [string]$Clean
)
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Root = Split-Path -Parent $ScriptDir
$Out = Join-Path $ScriptDir "out"
$Assets = Join-Path $ScriptDir "assets"
$Bin = Join-Path $Root "target\release\rfwhisper.exe"
New-Item -ItemType Directory -Force -Path $Out, $Assets | Out-Null

# DeepFilterNet's own freely-licensed demo clip, pinned to the tag we build against.
$DfnTag = "v0.5.6"
$DfnRaw = "https://raw.githubusercontent.com/Rikorose/DeepFilterNet/$DfnTag/assets"

Write-Host "==> Building the real backend (cargo build --release --features dfn)"
Write-Host "    First run pulls the tract inference tree (~4 min); cached afterwards."
Push-Location $Root
cargo build --release --features dfn
Pop-Location

if (-not $Noisy) {
  Write-Host "==> No input given; fetching a real-speech sample to demonstrate on."
  $Noisy = Join-Path $Assets "noisy_snr0.wav"
  $Clean = Join-Path $Assets "clean_reference.wav"
  if (-not (Test-Path $Noisy)) { Invoke-WebRequest "$DfnRaw/noisy_snr0.wav" -OutFile $Noisy }
  if (-not (Test-Path $Clean)) { Invoke-WebRequest "$DfnRaw/clean_freesound_33711.wav" -OutFile $Clean }
  Write-Host "    Using DeepFilterNet's demo clip (real speech). Swap in your own"
  Write-Host "    recording once you have one — see samples/README.md."
}

$refArgs = @()
if ($Clean -and (Test-Path $Clean)) {
  $refArgs = @("--reference", $Clean)
  Write-Host "==> Reference provided: report will include a measured SNR gain."
} else {
  Write-Host "==> No reference: report shows before/after spectrograms (snr_gain null)."
}

Write-Host "==> Denoising with DeepFilterNet3"
& $Bin denoise `
  --input       $Noisy `
  --output      (Join-Path $Out "cleaned.wav") `
  --model       deepfilternet3 `
  --report      (Join-Path $Out "report.json") `
  --spectrogram (Join-Path $Out "report.html") `
  @refArgs

Write-Host ""
Write-Host "Done. Artifacts in demo\out\:"
Write-Host "  report.html  <- open this in a browser (self-contained before/after)"
Write-Host "  cleaned.wav  <- the denoised audio"
Write-Host "  report.json  <- machine-readable summary"
