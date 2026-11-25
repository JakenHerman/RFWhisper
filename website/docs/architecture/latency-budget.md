---
id: latency-budget
title: Latency Budget
sidebar_position: 3
description: Per-stage latency targets for RFWhisper v0.1 and v0.3, with measurement methodology.
---

# Latency Budget

RFWhisper treats latency as a hard budget, not a goal.

:::info Measured in v0.1.0-beta.1
On an Apple Silicon laptop (built-in mic → speakers): DeepFilterNet3 processing
p50 0.4 ms / p99 0.9 ms per block, a fixed **30 ms** of model look-ahead,
**~40 ms** for the whole pipeline before sound-card buffers, and **~75 ms**
estimated end to end by `denoise-live` (devices ~21 + buffers ~22 + DSP ~31).
That's inside the v0.1 budget. The A4 number of record still needs a hardware
loopback measurement ([#15](https://github.com/JakenHerman/RFWhisper/issues/15)).
:::

## End-to-end targets

| Hardware | v0.1 p99 | v0.3 p99 |
|---|---|---|
| Intel i5-8xxx (laptop) | &lt; 100 ms | &lt; 50 ms |
| Apple M1 | &lt; 100 ms | &lt; 45 ms |
| Raspberry Pi 5 (8 GB) | &lt; 100 ms | &lt; 70 ms |

Anything worse than the v0.1 column is a release blocker (criterion A4).

## Per-stage budget

| Stage | v0.1 | v0.3 | Notes |
|---|---|---|---|
| Audio capture (frame) | 10–20 ms | 5–10 ms | PortAudio / ALSA / JACK / WASAPI callback |
| Pre-emphasis + feature extraction | ≤ 5 ms | ≤ 2 ms | VOLK windowing + liquid FFT with cached plans |
| ONNX inference (DFN3) | ≤ 30 ms | ≤ 15 ms | XNNPACK/CoreML/DirectML; INT8 stretch |
| Post-processing + overlap-add | ≤ 5 ms | ≤ 2 ms | — |
| Output buffering | 10–30 ms | 5–15 ms | Matches input blocksize |
| **Total (p99)** | **&lt; 100 ms** | **&lt; 50 ms** | — |

RPi Zero 2 W ships with RNNoise (not DFN3) and has a separate, looser budget.

## Measurement methodology

Three layers, from cheapest to the number of record:

```bash
# 1. Pipeline probe: the same streaming chain denoise-live uses, on a file
rfwhisper samples synth --preset ssb_powerline_s3_s7 --out probe.wav --seconds 30
rfwhisper bench latency probe.wav --block 480

# 2. Live estimate: printed once a second while running
rfwhisper denoise-live --in <device> --out <device>
#   latency ~73 ms (devices 21 + buffers 22 + dsp 31)
```

The live estimate adds the sound card's own reported buffer timing (cpal
callback timestamps), the queue fill on both sides, one processing block, and
the pipeline's algorithmic latency.

**3. Hardware loopback** *(planned, [#15](https://github.com/JakenHerman/RFWhisper/issues/15))*:

1. Generate an **impulse train** at known intervals.
2. Play impulses out → loopback cable → back into RFWhisper's input.
3. RFWhisper processes them as normal audio.
4. Align reference and captured impulses, logging the round-trip delta to an **HDR histogram**.
5. Publish p50 / p95 / p99 / max.

**We report p99, not mean.** A pipeline that averages 30 ms but stalls at 180 ms twice per minute is unusable; one that's 85 ms tight is fine.

## Hard rules

- **No allocations in the audio callback.** The beta's callbacks only move samples through preallocated lock-free rings (`rtrb`) and bump atomics. (An automated allocation check in CI is planned.)
- **No locks or blocking in the inference handoff.** Lock-free SPSC rings between callbacks and worker; drop and count on overflow, never block.
- **Thread affinity is intentional**, never left to the scheduler when it matters.
- **Frame sizes** are 10–30 ms — shorter for CW (transient fidelity), longer for FT8 (decoder tolerates, NN gets more context).

## Real-time scheduling per OS *(planned)*

The beta does not raise thread priority yet; the load is light enough (~5 % of a
core) that it hasn't been needed on test machines. The plan:

| OS | What we will request |
|---|---|
| Linux | `SCHED_FIFO` at rtprio 95 if the user is in a `realtime` group |
| macOS | `thread_time_constraint_policy` with appropriate period/constraint/deadline |
| Windows | MMCSS `AvSetMmThreadCharacteristics` with *Pro Audio* characteristic |

If RT priority is unavailable (rootless container, no `realtime` group), RFWhisper will fall back gracefully and log a one-time warning.

## When you miss the budget

1. Read the breakdown in the `denoise-live` status line: `devices` is your sound
   card and OS, `buffers` is queueing, `dsp` is the resamplers and model.
2. Check CPU governor (Linux) and power mode (macOS / Windows).
3. Lower device buffering in your OS audio settings (e.g. PipeWire quantum,
   WASAPI shared-mode period) — that's usually the biggest chunk.
4. If `underruns` climb, raise `--blocksize` (e.g. 960); smaller blocks lower
   latency but raise dropout risk.

If none of that helps, file an issue with `rfwhisper --version`, `rfwhisper audio
list`, the `bench latency` output, and a few status lines.
