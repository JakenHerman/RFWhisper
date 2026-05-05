# RFWhisper — Cursor Rules for AI

> Condensed slice of [AGENTS.md](../AGENTS.md), the source of truth — read it for role prompts, the full latency table, the ham-radio domain primer, and the PR rubric. **If you change AGENTS.md and it touches anything below, update this file in the same PR.**

## What we're building

Real-time ML noise reduction for amateur radio. Local-first. **GPLv3-or-later.**

## Primary stack

- **Primary denoiser:** DeepFilterNet3 (DFN3) — ham-fine-tuned **ONNX**, ONNX Runtime 1.17+, opset ≥ 17.
- **Fallback denoiser:** RNNoise (Valin / Xiph) — ham-retuned ONNX, for the RPi Zero / low-power tier.
- **DSP / RF:** GNU Radio 3.10.x (LTS), SoapySDR, liquid-dsp, VOLK.
- **Languages:** Python 3.10+ (glue, CLI, GUI), C++17 (GR blocks + hot paths), Rust optional.

## Hard constraints

1. **Latency:** v0.1 end-to-end **< 100 ms p99** on i5-8xxx / M1 / RPi 5; v0.3 target < 50 ms. If you can't measure your change's latency impact, you haven't finished it.
2. **Non-regression gates — never bypass:**
   - **A3 — CW keying transient:** RMS in the first 5 ms of a dit stays within **±1 dB** of raw (`tests/audio/cw_transient_test.py`).
   - **A2 — FT8 decode count:** denoised decodes **≥** raw, **zero** false decodes (`tests/audio/ft8_regression_test.py`).
3. **GPLv3-compatible deps only.** MIT / Apache / BSD: fine. Proprietary or GPL-incompatible: never.
4. **Local-first.** No network calls in runtime paths. Models fetched once, SHA-256 pinned in source.
5. **No allocations in the audio callback.** Preallocate buffers, reuse plans, release the GIL around inference.
6. **Every PR moves one roadmap criterion forward** (A*/B*/C*/D*/E*/F* in [ROADMAP.md](../ROADMAP.md)) and reports p50/p99 latency on the changed path.

## Conventions

- Conventional Commits (`feat:`, `fix:`, `perf:`, `test:`, …) with roadmap refs in the body (e.g. `refs A2`).
- Python: `ruff format`, `ruff check`, `mypy --strict` on new code. C++17 with repo `.clang-format`. Rust: `rustfmt`, `clippy -D warnings`.
