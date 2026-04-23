---
id: latency-budget
title: Latency Budget
sidebar_position: 3
description: Per-stage latency targets for RFWhisper v0.1 and v0.3, with measurement methodology.
---

# Latency Budget

RFWhisper treats latency as a hard budget, not a goal.

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

```bash
python -m rfwhisper.bench latency \
  --backend portaudio \
  --in default --out default \
  --blocksize 480 \
  --duration 120 \
  --out build/latency/
```

What it does:

1. Generates an **impulse train** at known intervals.
2. Plays impulses out → routes through the virtual cable → back into RFWhisper's input.
3. RFWhisper processes them as normal audio.
4. The probe aligns the reference and captured impulses, logging the round-trip delta to an **HDR histogram**.
5. Publishes p50 / p95 / p99 / max; renders a histogram image.

**We report p99, not mean.** A pipeline that averages 30 ms but stalls at 180 ms twice per minute is unusable; one that's 85 ms tight is fine.

## Hard rules

- **No allocations in the audio callback.** Verified in CI with an ASan build that flags `malloc` from RT threads.
- **GIL released in the inference path.** Python code that doesn't respect this fails profiler checks.
- **Thread affinity is intentional**, never left to the scheduler when it matters.
- **Frame sizes** are 10–30 ms — shorter for CW (transient fidelity), longer for FT8 (decoder tolerates, NN gets more context).

## Real-time scheduling per OS

| OS | What we request |
|---|---|
| Linux | `SCHED_FIFO` at rtprio 95 if the user is in a `realtime` group |
| macOS | `thread_time_constraint_policy` with appropriate period/constraint/deadline |
| Windows | MMCSS `AvSetMmThreadCharacteristics` with *Pro Audio* characteristic |

If RT priority is unavailable (rootless container, no `realtime` group), RFWhisper falls back gracefully and logs a one-time warning.

## When you miss the budget

1. Run `python -m rfwhisper.bench latency --trace` to get per-stage histograms.
2. Check CPU governor (Linux) and power mode (macOS / Windows).
3. Check virtual cable internal rate (48 kHz) matches RFWhisper's.
4. Reduce blocksize carefully: smaller blocks lower latency but raise xrun risk.
5. Switch to INT8 model variant (`--model-variant int8`) if you're CPU-bound.

If none of that helps, file an issue with the full `rfwhisper doctor` output and the trace HDR histogram.
