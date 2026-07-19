# Demo: make a before/after in one command

This directory turns a recording into a shareable before/after report — the raw
material for a YouTube video or a blog post — using the **real** DeepFilterNet3
backend.

## TL;DR

```bash
# Linux / macOS
demo/make-report.sh

# Windows (PowerShell)
demo\make-report.ps1
```

With no arguments it fetches a real-speech sample and denoises it, so you can see
the whole thing work before you have your own recording. Output lands in
`demo/out/`:

- **`report.html`** — a self-contained before/after page (SNR tiles, side-by-side
  spectrograms on a shared scale, median-spectrum chart). Opens offline in any
  browser. **This is your hero visual.**
- **`cleaned.wav`** — the denoised audio, for the A/B listen.
- **`report.json`** — the numbers (RTF, and SNR gain if you passed a reference).

## Using your own recording

```bash
demo/make-report.sh path/to/noisy.wav                    # A/B by ear + spectrogram
demo/make-report.sh path/to/noisy.wav path/to/clean.wav  # + a measured SNR gain
```

The second form needs a **clean reference** aligned with the noisy clip — see
[`samples/README.md`](../samples/README.md) for how to make one. On-air ham audio
usually has no reference, so it demos by ear and by the spectrograms; a measured
+dB number needs a `clean/` + `noise/` mix.

## What makes a good demo clip

For a "listen to this" post, record **two** things at your station:

1. A short QSO or off-air capture — real signal under your real noise floor.
2. ~10 seconds of that noise floor **with no signal** — this becomes the `noise/`
   component you can mix with clean speech for a *measured* before/after.

Mono WAV, 48 kHz, a little headroom below full scale. Details and the
licensing/consent rules are in [`samples/README.md`](../samples/README.md).

## The stats worth quoting

`report.json` and the A5 gate give you real, honest numbers:

- **RTF ≈ 0.017 — about 60× faster than real time** on a laptop CPU
  (`cargo test --release --features dfn -- --ignored gate_rtf`).
- **SNR gain** vs the clean reference, when you provide one.

## Honest caveats (say them in the post)

- DeepFilterNet3 is a **speech** model. It shines on SSB/voice; it is *not* tuned
  for CW or FT8, where it can suppress the very tones you want (mode profiles are
  v0.3). Demo it on voice.
- Without `--features dfn`, `deepfilternet3` falls back to a ~+1 dB spectral stub.
  The script always builds the real backend.

## Note on the bundled sample

The no-argument path downloads DeepFilterNet's own demo clip
(`assets/noisy_snr0.wav` + a freesound-sourced clean reference) from their
repository at tag `v0.5.6`. It is real speech, and a fine stand-in for proving the
pipeline — but it is not ham radio. Your own on-air recording is the real demo.
Generated files under `demo/out/` and `demo/assets/` are git-ignored.
