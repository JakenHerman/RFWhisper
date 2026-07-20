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

:::info Where things stand
The measurement harness (fixtures, SNR/latency/RTF metrics, the before/after
report) is built and tested. The gates that wrap it are landing one at a time —
**A5 first**. A gate marked "in progress" below does not exist as a runnable test
yet; its issue link is the source of truth.
:::

## Status at a glance

| # | Criterion | Threshold | Status |
|---|---|---|---|
| A1 | Effective SNR gain on speech mix | ≥ +3 dB avg, ≥ +6 dB powerline | 🔬 **needs real speech** — see note ([#20](https://github.com/JakenHerman/RFWhisper/issues/20)) |
| A2 | No FT8 decode regressions | denoised ≥ raw, 0 false | ⏳ in progress ([#21](https://github.com/JakenHerman/RFWhisper/issues/21)) |
| A3 | No CW transient damage | keying-onset RMS within ±1 dB | ⏳ in progress ([#3](https://github.com/JakenHerman/RFWhisper/issues/3)) |
| A4 | End-to-end latency (p99) | &lt; 100 ms | ⏳ needs realtime probe ([#22](https://github.com/JakenHerman/RFWhisper/issues/22), [#15](https://github.com/JakenHerman/RFWhisper/issues/15)) |
| A5 | Real-time factor | &lt; 0.5 on reference CPU | ✅ **implemented + passing** ([#23](https://github.com/JakenHerman/RFWhisper/issues/23)) |
| A6 | No-op sanity (clean → clean) | PESQ drop ≤ 0.3, STOI ≤ 0.02 | ⏳ in progress ([#24](https://github.com/JakenHerman/RFWhisper/issues/24)) |
| A7 | Cross-platform build | green on Linux / macOS / Windows | ✅ CI matrix (ubuntu-22.04, macos-13, windows-2022) |
| A8 | Virtual-cable routing docs | beginner routes in ≤ 10 min | ⏳ docs ([#4](https://github.com/JakenHerman/RFWhisper/issues/4)) |

## How the gates run

Acceptance gates are `#[ignore]`-marked tests named `gate_*`. They measure the
**real** DeepFilterNet3 backend, so they run with the `dfn` feature:

```bash
cargo test --release --features dfn -- --ignored gate_
```

Each writes a JSON report under `build/audio-reports/`. Without `--features dfn`,
a gate skips with a reason rather than passing on the stub (whose numbers are not
meaningful).

## A5 — Real-time factor ✅

The one gate you can run today. It confirms the denoiser uses well under half of
real time, leaving CPU for WSJT-X, fldigi, and logging on the same box.

```bash
cargo test --release --features dfn -- --ignored gate_rtf
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

Every push and PR builds and unit-tests on the CI matrix (ubuntu-22.04, macos-13,
windows-2022). The nightly `audio-quality` job additionally runs the acceptance
gates with `--features dfn`.

## When a gate fails

Gates are precious. **Do not disable one to get CI green.** In priority order:

1. Fix the change — almost always the right answer.
2. If the gate is genuinely wrong (false alarm, flaky fixture), open a separate PR
   fixing the gate with a written justification.
3. Escalate if the trade-off is genuinely in tension.

<span className="rfw-73">73</span>
