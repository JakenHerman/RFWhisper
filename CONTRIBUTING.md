# Contributing to RFWhisper

First: **thank you.** RFWhisper exists because hams, DSP engineers, ML folks, and open-source contributors are willing to pour time into a project that helps operators pull weak signals out of an ever-noisier RF environment. Whether you're fixing a typo, recording some powerline buzz, or porting the whole pipeline to GNU Radio 4.0 — you are welcome here.

This document covers:

- [Code of Conduct](#code-of-conduct)
- [Ways to Contribute](#ways-to-contribute)
- [Getting Started (Dev Setup)](#getting-started-dev-setup)
- [Project Structure](#project-structure)
- [Workflow (Branching, Commits, PRs)](#workflow-branching-commits-prs)
- [Testing & Acceptance Criteria](#testing--acceptance-criteria)
- [Style Guides](#style-guides)
- [Submitting RF/Audio Samples](#submitting-rfaudio-samples)
- [Submitting Trained Models](#submitting-trained-models)
- [Using AI Coding Assistants](#using-ai-coding-assistants)
- [Reporting Bugs](#reporting-bugs)
- [Requesting Features](#requesting-features)
- [Hardware Testing](#hardware-testing)
- [Security](#security)
- [Licensing & Copyright](#licensing--copyright)
- [Questions](#questions)

---

## Code of Conduct

By participating in this project you agree to uphold the [Code of Conduct](./CODE_OF_CONDUCT.md). TL;DR: be kind, be patient, assume good intent, leave your ego at the door, and remember there are new hams reading every thread.

---

## Ways to Contribute

Not just code. Every one of these is gold:

- **RF noise recordings** from your QTH (powerline, solar, VDSL, inverters, plasma, neighbour's electric fence) — the more labeled, the better. See [Submitting RF/Audio Samples](#submitting-rfaudio-samples).
- **On-air recordings** of weak signals we can use for evaluation (FT8 marginal decodes, DX SSB under QRM, CW under QRN).
- **Hardware testing** with any SoapySDR-supported SDR. Report back against the matrix.
- **Model training / fine-tuning** — ham-specific DeepFilterNet3 variants, RNNoise retunes, new architectures.
- **DSP engineering** — VOLK kernels, liquid-dsp integration, GR blocks.
- **GNU Radio flowgraphs** for new SDRs or modes.
- **UI / UX** — the v0.4 spectrogram UI has lots of room to help.
- **Documentation** — tutorials, quickstarts, per-OS install guides, translations.
- **Packaging** — `.deb` / `.rpm` / AUR / Homebrew / Windows installers / RPi OS images.
- **Bug reports** and **feature requests** with reproducible steps.
- **Community building** — answering discussion questions, welcoming new hams, writing about RFWhisper on your blog / podcast / QRZ page.

If you want to do something big, file an issue or start a GitHub Discussion first so we can help scope it and avoid duplicate work.

---

## Getting Started (Dev Setup)

### Prerequisites

- **Git** (and `git-lfs` for samples / models)
- **Python 3.10, 3.11, or 3.12**
- A working audio stack:
  - **Linux**: ALSA + PipeWire or JACK
  - **macOS**: CoreAudio (built in); BlackHole for virtual cable
  - **Windows**: WASAPI (built in); VB-Cable for virtual cable
- **For v0.2+ work:** GNU Radio 3.10.x with `gr-soapy`, `gr-dnn`, and SoapySDR modules for any SDR you own
- **C++ toolchain** if you're touching native blocks (`gcc` / `clang` / MSVC, CMake ≥ 3.20)
- Optional: a GPU (CUDA / CoreML / DirectML) for faster inference and training

### Clone & Install (dev)

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
git lfs install    # required for samples/ and models/

python -m venv .venv
source .venv/bin/activate          # Windows: .venv\Scripts\activate
pip install -U pip wheel

# Install with all dev + audio dependencies
pip install -e ".[dev,audio]"

# Pre-commit hooks (formatting, linting, basic checks)
pre-commit install

# Pull pre-converted ONNX models
python -m rfwhisper.models.fetch
```

### Run the test suite

```bash
# Fast unit tests
pytest -q

# Full acceptance harness (audio quality tests - slow)
pytest -q tests/audio/ --runslow
```

If you don't have the audio dev dependencies set up yet, `pip install -e ".[audio,dev]"` pulls `sounddevice`, `soundfile`, `numpy`, `scipy`, `onnxruntime`, `pytest`, `ruff`, `mypy`, and friends.

### Smoke test

```bash
rfwhisper denoise \
  --input  samples/noisy_40m_ssb.wav \
  --output /tmp/cleaned.wav \
  --model  deepfilternet3 \
  --report /tmp/report.json

cat /tmp/report.json   # effective SNR gain, RTF, latency, etc.
```

---

## Project Structure

See [AGENTS.md § Repo Layout](./AGENTS.md#shared-context-tech-stack--constraints) for the canonical tree. The short version:

```
rfwhisper/         Python package (CLI, DSP, models, realtime, GUI, training)
gr-rfwhisper/      GNU Radio OOT module (C++ blocks; v0.2+)
flowgraphs/        .grc + generated .py (v0.2+)
tests/audio/       Acceptance harness tied to ROADMAP criteria
samples/           Seed audio (Git LFS)
models/            ONNX models (Git LFS or fetched at runtime)
docs/              Tutorials, architecture, hardware matrix
notebooks/         Training + analysis notebooks
```

---

## Workflow (Branching, Commits, PRs)

1. **Fork** the repo (or get write access for trusted maintainers).
2. **Branch** off `main`. Naming: `<type>/<short-slug>`, e.g. `feat/rnnoise-int8`, `fix/macos-audio-device-picker`, `docs/rpi5-quickstart`.
3. **Commit** using [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat: ` new user-facing functionality
   - `fix: ` bug fix
   - `perf: ` performance improvement with numbers
   - `docs: ` documentation only
   - `test: ` adding / improving tests
   - `refactor: ` code structure, no behavior change
   - `build: ` packaging, CI, deps
   - `chore: ` everything else
4. **Reference the roadmap criterion** in the commit body, e.g. `refs A4` or `closes #123 (criterion B3)`.
5. **Sign off** your commits with `git commit -s` (DCO). See [Licensing & Copyright](#licensing--copyright).
6. **Push** and open a PR against `main`.

### Pull Request Template (summary)

Every PR description must include:

- **Which roadmap criterion** this moves forward (e.g., "Refs A2, C1").
- **What you measured** — hardware, numbers before/after, p50/p99 latency if realtime.
- **Regression evidence** — paste the output of `tests/audio/cw_transient_test.py` and `tests/audio/ft8_regression_test.py` (or note why N/A).
- **Platforms tested** — Linux / macOS / Windows / RPi 5 / other SBC.
- **Follow-ups** — issues you opened for work you deliberately deferred.
- **Checklist**: tests added, docs updated, CI green, signed off.

**No PR is merged without green CI.** Never disable a test to make CI pass. If a test is wrong, fix the test in a separate PR and justify it.

### Review Expectations

- Expect a first-pass response within a few days. RFWhisper is an all-volunteer project — maintainers have day jobs and radios to operate.
- Reviewers will focus on: correctness, latency impact, signal preservation (CW / FT8 / SSB), cross-platform parity, tests, and docs.
- We review **kindly and directly**. If feedback feels harsh, please assume tiredness not hostility and push back. See [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).

---

## Testing & Acceptance Criteria

RFWhisper is a **testable-success** project. Every feature is pinned to a criterion in [ROADMAP.md](./ROADMAP.md). Your PR should:

1. Declare which criterion/criteria it touches.
2. Pass existing criteria (no regressions).
3. Add tests for any new criterion coverage.

### Test tiers

| Tier | Command | When it runs |
|---|---|---|
| Unit | `pytest -q` | Every push + PR |
| Lint / format | `ruff check && ruff format --check` | Every push + PR |
| Type check | `mypy rfwhisper` | Every push + PR |
| Audio quality (slow) | `pytest tests/audio/ --runslow` | Nightly + PRs touching `rfwhisper/{dsp,models,realtime}/` |
| Latency probe | `python -m rfwhisper.bench latency` | Nightly on self-hosted runners (reference hardware) |
| Integration (SDR) | Manual, documented in PR | When flowgraphs change |

### The two non-regression gates (always run)

1. **CW keying transients** (`tests/audio/cw_transient_test.py`) — RMS in the keying-onset window must be within ±1 dB of raw.
2. **FT8 decodes** (`tests/audio/ft8_regression_test.py`) — decoder must recover ≥ the number of stations that the raw audio gives, with zero false decodes.

If you break either, your PR will be blocked. These gates are why we exist.

---

## Style Guides

### Python

- Formatter: `ruff format`
- Linter: `ruff check`
- Type checker: `mypy --strict` on new code in `rfwhisper/` (existing gaps are tracked)
- Docstrings: Google style
- Import order: stdlib / third-party / local, separated by blank lines (ruff handles this)
- No `from foo import *`

### C++

- Standard: C++17 minimum
- Formatter: `clang-format` using the project `.clang-format` (LLVM base, 4-space indent)
- Linter: `clang-tidy` with project checks
- RAII everything. No raw owning pointers.
- No exceptions in the audio callback path.

### Rust (if used)

- Edition 2021
- `rustfmt` and `clippy -D warnings`

### Commit messages

- Conventional Commits + roadmap criterion refs in body.
- Imperative mood: "add RNNoise fallback" not "added" / "adds".
- Wrap body at 72 cols.
- Sign off with `-s`.

### Docs

- Markdown with the same tone as [README.md](./README.md) — accurate, welcoming, ham-friendly.
- Use headings, tables, and code blocks liberally. Assume a smart reader who is new to the specific topic.
- No emoji in technical docs unless a Ham Domain Expert review approves it.

---

## Submitting RF/Audio Samples

High-quality labeled recordings are genuinely one of the most valuable contributions to this project.

### What we want

- **Noisy recordings** of real ham operations (HF / VHF / UHF, any mode).
- **Isolated noise** (powerline, solar, plasma, PLC, thermostat clicks, atmospheric QRN) — unmodulated, no signal, just the noise.
- **Clean reference audio** where available (dummy load, studio ham speech, high-SNR CW / FT8).

### Format

- **WAV** (PCM 16-bit or 32-bit float), 48 kHz preferred, 16 kHz acceptable
- Mono unless stereo is meaningful (I/Q pair → document it)
- Keep clips ≤ 3 minutes (longer is fine; we'll split)

### Metadata (required)

Every sample needs a sidecar `.yaml` or a row in `samples/catalog.yaml`:

```yaml
filename: noisy_40m_ssb_koXYZ_2025-04-20.wav
description: 40m SSB DX station under powerline noise, suburban QTH
band: 40m
frequency_khz: 7185
mode: USB
rig: Icom IC-7300
antenna: EFHW-8010 at 8m
sdr_or_audio_source: rig USB CODEC
noise_sources_known: ["powerline", "VDSL"]
sample_rate_hz: 48000
duration_s: 62.0
recorded_at: "2025-04-20T19:30:00Z"
operator_callsign_or_anon: "anon"
license: "CC-BY-4.0"   # or your choice; GPLv3-compatible required
notes: "S7 noise floor on raw, weak DX station at ~S3"
```

### License for samples

We strongly prefer **CC-BY-4.0** or **CC0** for samples to maximize reuse. Any license that is GPLv3-compatible and allows redistribution is acceptable. You retain copyright.

### Privacy

- Do not submit samples containing third-party callsigns or identifying information without the other operator's consent.
- Anonymize / redact personal info where present.

### How to submit

1. Put the file in `samples/<category>/` (e.g. `samples/ssb/`, `samples/noise/`).
2. Add the metadata entry to `samples/catalog.yaml`.
3. `git lfs track "*.wav"` is already configured — just `git add` normally.
4. Open a PR tagged `samples`.

---

## Submitting Trained Models

See [AGENTS.md § ML Training Engineer](./AGENTS.md#2-ml-training-engineer). In short: every ONNX you submit must ship with:

- SHA-256 pinned in source
- PyTorch↔ONNX Runtime parity check (RMS diff ≤ 1e-3)
- Eval table (PESQ, STOI, SI-SDR, ham SNR gain, RTF, latency p50/p99) on reference hardware
- **Green CW + FT8 regression tests**
- A model card in `docs/models/<your-model-name>.md` covering: architecture, training data, training recipe, license, known failure modes, maintainer contact

---

## Using AI Coding Assistants

AI coding tools (Claude, Cursor, Copilot, Aider, Codex CLI, etc.) are welcome here. Please:

1. **Point them at [AGENTS.md](./AGENTS.md).** Load the role-specific system prompt that matches the area you're working in.
2. **Read the output.** You are the contributor of record. Review every line.
3. **Run the tests locally.** Don't rely on CI to catch regressions.
4. **Disclose significant AI assistance** in the PR description. Example: "Drafted with Claude via Cursor; DSP review was human; tests manually verified on M1 + RPi 5." This helps reviewers and builds community trust.
5. **Never let an agent merge its own PR.** A human must approve.

---

## Reporting Bugs

Open a GitHub Issue with the **Bug report** template. Include:

- RFWhisper version / commit SHA
- OS + version
- CPU / GPU / SDR model (if relevant)
- Minimal steps to reproduce
- Expected vs actual behavior
- Relevant logs (attach; do not paste multi-kilobyte dumps)
- A short recording or screenshot if it's about audio quality or UI

Security bugs: see [Security](#security) below.

---

## Requesting Features

Open an Issue with the **Feature request** template. Include:

- What problem are you trying to solve? (the real-world ham scenario)
- Why the existing tooling / a different workflow isn't enough
- Proposed solution sketch (if any)
- Which roadmap version this might fit into (your best guess is fine)

For anything non-trivial, start a GitHub Discussion first so we can scope collaboratively.

---

## Hardware Testing

If you own a SoapySDR-supported SDR not yet in the matrix, we'd love a test report.

1. Run the nearest matching flowgraph (see `flowgraphs/`).
2. File an Issue using the **Hardware test** template with:
   - SDR make/model/firmware
   - Sample rate + mode tested
   - On-air audio file (before + after, trimmed to ≤ 30 s)
   - Latency / CPU numbers from the telemetry panel (or CLI report)
   - Any gotchas (driver install quirks, permissions, etc.)
3. We'll update `docs/soapy-hardware-matrix.md` and credit you.

---

## Security

If you discover a security issue — privacy leak, RCE, supply-chain concern, signed-model tampering — **do not open a public issue.** Instead email the maintainers at `security@rfwhisper.org` (placeholder — will publish a real address on first release) with:

- Description of the issue
- Reproduction steps
- Potential impact
- Your preferred disclosure timeline

We aim to acknowledge within 72 hours and propose a coordinated disclosure timeline within 7 days. We credit reporters (or keep them anonymous if preferred).

---

## Licensing & Copyright

- RFWhisper is licensed under **GPL-3.0-or-later** ([LICENSE](./LICENSE)).
- By contributing, you agree your contributions will be licensed under the same terms.
- We use the **Developer Certificate of Origin (DCO) 1.1** — please sign off every commit with `git commit -s`. This means you wrote the code (or have rights to submit it) and you agree to license it under GPLv3-or-later. Full DCO text: <https://developercertificate.org/>.
- Third-party code brought into the project must be GPLv3-compatible. When in doubt, ask before opening the PR.

---

## Questions

- **GitHub Discussions** for open-ended design / "how do I?" questions.
- **Issues** for bugs and feature requests.
- **Matrix / Discord** (*link TBD*) for real-time chat.
- **On the air** — if RFWhisper helps you pull a signal, tell us in Discussions. That's the best feedback we get.

**Thank you. 73.**

*— the RFWhisper maintainers*
