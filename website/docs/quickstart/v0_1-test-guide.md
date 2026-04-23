---
id: v0_1-test-guide
title: v0.1 Test Guide
sidebar_position: 2
description: Step-by-step acceptance harness for RFWhisper v0.1. Exact pass criteria, reference data, and an end-to-end test script.
---

# v0.1 Test Guide

This is the **formal acceptance harness** for RFWhisper v0.1. Every criterion below is pinned to an `A*` item in [ROADMAP.md](../roadmap). If you can complete this guide end-to-end on your hardware, you have validated v0.1 in your environment and we'd love your results in the [release discussion](https://github.com/jakenherman/rfwhisper/discussions).

## Checklist (TL;DR)

| # | Criterion | Automated | Pass when |
|---|---|---|---|
| A1 | Effective SNR gain on ham speech mix | ✅ | ≥ +3 dB avg, ≥ +6 dB on powerline-dominant |
| A2 | No FT8 decode regressions | ✅ | Denoised decodes ≥ raw, 0 false decodes |
| A3 | No CW transient damage | ✅ | RMS in keying-onset window within ±1 dB |
| A4 | End-to-end latency (p99) | ✅ | &lt; 100 ms on reference hardware |
| A5 | Real-time factor (RTF) | ✅ | &lt; 0.5 on reference CPU |
| A6 | No-op sanity (clean → clean) | ✅ | PESQ drop ≤ 0.3, STOI drop ≤ 0.02 |
| A7 | Cross-platform install | ✅ | Green on Ubuntu 22.04, macOS 13, Windows 11 |
| A8 | Virtual cable routing docs | manual | A beginner can route in ≤ 10 min |

## Reference hardware (CI runners)

We normalize numbers against these so you can compare apples to apples:

- **Laptop Linux** — Intel i5-8350U, 16 GB, Ubuntu 22.04
- **Laptop macOS** — Apple M1, 16 GB, macOS 13
- **Laptop Windows** — AMD Ryzen 5500U, 16 GB, Windows 11
- **SBC** — Raspberry Pi 5 (8 GB), active cooling, Raspberry Pi OS Bookworm

## Run the full suite

```bash
# From a clean checkout (audio quality tests need --runslow)
pytest -q tests/audio/ --runslow --junitxml=build/junit-audio.xml

# Generates JSON reports + spectrograms under build/audio-reports/
rfwhisper bench report --out build/audio-reports/report.html
open build/audio-reports/report.html   # or xdg-open / start
```

The HTML report has before/after spectrograms, per-criterion pass/fail, and latency histograms.

## Criterion-by-criterion

### A1 — Effective SNR gain

**Intent:** measurable, not just audible, improvement on a ham-speech-plus-noise mix.

```bash
pytest -q tests/audio/snr_gain_test.py --runslow
```

What it does: takes reference clean speech convolved with measured room IR, mixes with a catalog of real ham noise (powerline buzz, inverter rasp, PLC combs) at SNRs spanning −10 to +20 dB, runs RFWhisper, and computes effective SNR gain via matched-filter correlation against the clean reference.

**Pass:** average gain ≥ +3 dB across the full catalog; powerline-dominant clips must reach ≥ +6 dB.

### A2 — FT8 decode non-regression

**Intent:** a denoiser that increases SNR but breaks decoders is worse than useless.

```bash
pytest -q tests/audio/ft8_regression_test.py --runslow
```

What it does: replays a 15-minute FT8 cycle (multi-band, curated) through `jt9` (WSJT-X's decoder) twice — once raw, once denoised — and compares decode lists.

**Pass:** `len(denoised_decodes) ≥ len(raw_decodes)` **AND** no decoded message appears in `denoised_decodes` that isn't verifiable against the ground-truth transmit list. Zero false decodes.

### A3 — CW keying transient preservation

**Intent:** never soften the operator's fist.

```bash
pytest -q tests/audio/cw_transient_test.py --runslow
```

What it does: feeds a 25 WPM CW recording with synthetic QRN crashes; measures RMS energy in the first 5 ms of every dit onset; compares raw vs denoised.

**Pass:** difference within **±1 dB** on every transient in the window (no averaging — a single broken transient fails the gate).

### A4 — End-to-end latency

**Intent:** real-time use in WSJT-X / fldigi / headphones.

```bash
python -m rfwhisper.bench latency --duration 120 --out build/latency/
```

What it does: injects an impulse train into the input device, records round-trip, measures p50 / p95 / p99 latency as an HDR histogram.

**Pass:** p99 &lt; 100 ms on reference laptop / M1 / RPi 5.

### A5 — Real-time factor

```bash
python -m rfwhisper.bench rtf --iters 5000 --profile ssb
```

**Pass:** RTF &lt; 0.5 (i.e., headroom for other tasks) on reference CPU.

### A6 — No-op sanity

**Intent:** clean audio in, clean audio out. Don't damage high-SNR signals.

```bash
pytest -q tests/audio/noop_quality_test.py --runslow
```

**Pass:** PESQ drop ≤ 0.3, STOI drop ≤ 0.02 on a curated clean-speech set.

### A7 — Cross-platform install

Green CI run on the matrix (ubuntu-22.04 × {3.10, 3.11, 3.12}, macos-13 × {3.11, 3.12}, windows-2022 × {3.11, 3.12}).

### A8 — Virtual cable routing docs

Manual: verify a non-author contributor can route rig audio → RFWhisper → WSJT-X in ≤ 10 minutes following the installation guide for their OS.

## Repro script (single entry-point)

All of the above in one command:

```bash
./scripts/ci_acceptance.sh --hardware-profile auto
```

:::tip What to submit if you're helping validate

Run `rfwhisper doctor && ./scripts/ci_acceptance.sh --hardware-profile auto`, gzip `build/audio-reports/`, and post the link + hardware spec in [Discussions](https://github.com/jakenherman/rfwhisper/discussions/categories/acceptance-reports).

:::

## When a gate fails

Gates are precious. **Do not disable one to get CI green.** Options in priority order:

1. Fix the change — this is almost always the right answer.
2. If the gate is genuinely wrong (false alarm, flaky fixture), open a separate PR fixing the gate with a paragraph of justification.
3. Escalate to a [Ham Domain Expert reviewer](https://github.com/jakenherman/rfwhisper/blob/main/AGENTS.md#6-ham-domain-expert-subject-matter-reviewer) if the tradeoff is genuinely in tension.

<span className="rfw-73">73</span>
