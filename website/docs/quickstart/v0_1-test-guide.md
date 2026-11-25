---
id: v0_1-test-guide
title: v0.1 Test Guide
sidebar_position: 2
description: The v0.1 acceptance criteria, which are implemented today, and how to run the ones that are.
---

# v0.1 Test Guide

The v0.1 acceptance criteria (`A1`–`A8`) each pin a measurable claim to a line in
[ROADMAP.md](../roadmap). This page is honest about **which are implemented today**
and how to run them — several are still in progress, and this guide tracks reality,
not the target.

:::info Where things stand (v0.1.0-beta.1)
The measurement harness (fixtures, SNR/latency/RTF metrics, the before/after
report) is built and tested, and the real DeepFilterNet3 denoiser now runs in it.
**A5 passes.** A4 has a pipeline probe and a live estimate, but still needs a
hardware loopback measurement. The quality gates (A1/A2/A3/A6) need real on-air
recordings. A gate marked "in progress" below does not exist as a runnable test
yet; its issue link is the source of truth.
:::

## Status at a glance

| # | Criterion | Threshold | Status |
|---|---|---|---|
| A1 | Effective SNR gain on speech mix | ≥ +3 dB avg, ≥ +6 dB powerline | 🔬 **needs real speech** — see note ([#20](https://github.com/JakenHerman/RFWhisper/issues/20)) |
| A2 | No FT8 decode regressions | denoised ≥ raw, 0 false | ⏳ in progress ([#21](https://github.com/JakenHerman/RFWhisper/issues/21)) |
| A3 | No CW transient damage | keying-onset RMS within ±1 dB | ⏳ in progress ([#3](https://github.com/JakenHerman/RFWhisper/issues/3)) |
| A4 | End-to-end latency (p99) | &lt; 100 ms | 🟡 pipeline ≈ 40 ms; live estimate ≈ 75 ms; hardware loopback pending ([#22](https://github.com/JakenHerman/RFWhisper/issues/22), [#15](https://github.com/JakenHerman/RFWhisper/issues/15)) |
| A5 | Real-time factor | &lt; 0.5 on reference CPU | ✅ **implemented + passing** ([#23](https://github.com/JakenHerman/RFWhisper/issues/23)) |
| A6 | No-op sanity (clean → clean) | PESQ drop ≤ 0.3, STOI ≤ 0.02 | ⏳ in progress ([#24](https://github.com/JakenHerman/RFWhisper/issues/24)) |
| A7 | Cross-platform build | green on Linux / macOS / Windows | ✅ CI matrix (ubuntu-22.04, macos-14, windows-2022) |
| A8 | Virtual-cable routing docs | beginner routes in ≤ 10 min | 🟡 [guide written](../hardware/virtual-cables); needs a timed beginner walkthrough ([#4](https://github.com/JakenHerman/RFWhisper/issues/4)) |

## How the gates run

Acceptance gates are `#[ignore]`-marked tests named `gate_*`. They measure the
**real** DeepFilterNet3 backend, which is in the default build:

```bash
cargo test --release -- --ignored gate_
```

Each writes a JSON report under `build/audio-reports/`. In a
`--no-default-features` build, a gate skips with a reason rather than passing on
the stub (whose numbers are not meaningful).

## A5 — Real-time factor ✅

The one gate you can run today. It confirms the denoiser uses well under half of
real time, leaving CPU for WSJT-X, fldigi, and logging on the same box.

```bash
cargo test --release -- --ignored gate_rtf
```

On a modern laptop CPU, DeepFilterNet3 measures:

```json
{
  "gate": "rtf",
  "model": "deepfilternet3",
  "backend": "tract (CPU)",
  "rtf": 0.0165,
  "realtime_factor_x": 60.6,
  "threshold": 0.5,
  "pass": true
}
```

**RTF 0.0165 — about 60× faster than real time.** A5 passes with large headroom.
Per-block processing latency is p99 ≈ 0.5 ms, far inside the 10 ms hop budget.

## A4 — latency 🟡

Two measurements exist today; the third needs your hardware.

1. **Pipeline probe** (no sound card). Runs a WAV through the exact streaming
   chain `denoise-live` uses:

   ```bash
   rfwhisper samples synth --preset ssb_powerline_s3_s7 --out ssb.wav --seconds 10
   rfwhisper bench latency ssb.wav
   ```

   ```text
   model=deepfilternet3 block=480 n=999
   processing per block: p50=0.09 ms p99=0.11 ms max=0.12 ms
   algorithmic latency (resample + model): 30.0 ms
   block buffering: 10.0 ms
   pipeline total (excl. sound-card buffers): ~40.1 ms p99
   ```

   The 30 ms is DeepFilterNet3's own look-ahead (STFT overlap plus two frames of
   model look-ahead). It is fixed and does not depend on CPU speed.

2. **Live estimate.** `denoise-live` prints `latency ~N ms (devices + buffers +
   dsp)` every second, using the sound card's own reported buffer timing. An
   Apple Silicon laptop (built-in mic → speakers) shows about 75 ms.

3. **Hardware loopback** (pending, [#15](https://github.com/JakenHerman/RFWhisper/issues/15)).
   The A4 number of record is an impulse sent out of the sound card, through a
   loopback cable, through RFWhisper, and timed on return. If you have a loopback
   cable and want to help, see the issue.

## A1 — why it needs real speech 🔬

A1 measures effective SNR gain against a clean reference. The natural instinct is
to run it on a `samples synth` mix — but **DeepFilterNet3 is a speech model**, and
it correctly removes the synthetic tone-stack signal as non-speech. On a synthetic
fixture its measured gain is *negative*; on real speech it is positive.

So A1 has to run on real speech-plus-noise pairs, not synthetic fixtures. That work
is tracked in [#20](https://github.com/JakenHerman/RFWhisper/issues/20) and depends
on real sample contributions ([#47](https://github.com/JakenHerman/RFWhisper/issues/47) —
see [`samples/README.md`](https://github.com/JakenHerman/RFWhisper/tree/master/samples)).
The synthetic fixtures remain correct for the stub and for pipeline/latency/RTF
wiring — they just can't judge a neural speech model.

## A7 — cross-platform build ✅

Every push and PR builds and unit-tests on the CI matrix (ubuntu-22.04, macos-14,
windows-2022), and a separate job clippies and tests the full DeepFilterNet3 build
and runs the A5 gate. The nightly `audio-quality` job additionally runs every
acceptance gate.

## When a gate fails

Gates are precious. **Do not disable one to get CI green.** In priority order:

1. Fix the change — almost always the right answer.
2. If the gate is genuinely wrong (false alarm, flaky fixture), open a separate PR
   fixing the gate with a written justification.
3. Escalate if the trade-off is genuinely in tension.

<span className="rfw-73">73</span>
