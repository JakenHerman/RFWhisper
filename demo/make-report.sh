#!/usr/bin/env sh
# Produce a self-contained before/after denoise report with the REAL
# DeepFilterNet3 backend. This is the one command that turns a recording into a
# demo you can screen-record or embed in a post.
#
# Usage:
#   demo/make-report.sh                      # try it now: fetches a real-speech
#                                            # sample and denoises that
#   demo/make-report.sh noisy.wav           # your clip, no clean reference
#   demo/make-report.sh noisy.wav clean.wav # with a reference => measured SNR gain
#
# Output lands in demo/out/: cleaned.wav, report.json, report.html
# Open report.html in any browser — it is fully offline (inline everything).

set -eu

# Repo root = parent of this script's directory.
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
OUT="$SCRIPT_DIR/out"
ASSETS="$SCRIPT_DIR/assets"
BIN="$ROOT/target/release/rfwhisper"
mkdir -p "$OUT" "$ASSETS"

# DeepFilterNet's own freely-licensed demo clip, pinned to the tag we build
# against. Real speech + noise — the honest way to show a speech denoiser working
# before a ham SSB recording lands (see samples/README.md).
DFN_TAG="v0.5.6"
DFN_RAW="https://raw.githubusercontent.com/Rikorose/DeepFilterNet/$DFN_TAG/assets"

echo "==> Building the real backend (cargo build --release --features dfn)"
echo "    First run pulls the tract inference tree (~4 min); cached afterwards."
( cd "$ROOT" && cargo build --release --features dfn )

NOISY="${1:-}"
CLEAN="${2:-}"

if [ -z "$NOISY" ]; then
  echo "==> No input given; fetching a real-speech sample to demonstrate on."
  NOISY="$ASSETS/noisy_snr0.wav"
  CLEAN="$ASSETS/clean_reference.wav"
  [ -f "$NOISY" ] || curl -fsSL "$DFN_RAW/noisy_snr0.wav" -o "$NOISY"
  [ -f "$CLEAN" ] || curl -fsSL "$DFN_RAW/clean_freesound_33711.wav" -o "$CLEAN"
  echo "    Using DeepFilterNet's demo clip (real speech). Swap in your own"
  echo "    recording once you have one — see samples/README.md."
fi

REF_ARGS=""
if [ -n "$CLEAN" ] && [ -f "$CLEAN" ]; then
  REF_ARGS="--reference $CLEAN"
  echo "==> Reference provided: report will include a measured SNR gain."
else
  echo "==> No reference: report shows the before/after spectrograms (snr_gain null)."
fi

echo "==> Denoising with DeepFilterNet3"
# shellcheck disable=SC2086
"$BIN" denoise \
  --input       "$NOISY" \
  --output      "$OUT/cleaned.wav" \
  --model       deepfilternet3 \
  --report      "$OUT/report.json" \
  --spectrogram "$OUT/report.html" \
  $REF_ARGS

echo
echo "Done. Artifacts in demo/out/:"
echo "  report.html  <- open this in a browser (self-contained before/after)"
echo "  cleaned.wav  <- the denoised audio"
echo "  report.json  <- machine-readable summary"
echo
echo "Listen to the input vs cleaned.wav, screen-record report.html, and you have"
echo "your before/after. RTF and the SNR gain (if a reference was given) are in"
echo "report.json."
