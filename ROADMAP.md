# RFWhisper — Roadmap

> *"If you can't demo it to another ham and have them say 'that's clearly better,' it doesn't ship."*

This roadmap is **phased, explicit, and testable**. Every milestone lists:

- **Scope** — what's in, what's out
- **Deliverables** — concrete artifacts, files, binaries, docs
- **Acceptance criteria** — measurable pass/fail gates (automated where possible)
- **Testable success examples** — real-world demos a human operator can verify
- **Stretch goals** — nice-to-haves if we have bandwidth
- **Exit criteria** — the one-line "we can tag this release when..." check

If a proposed PR does not move at least one of these forward, it probably belongs on a branch, not in main.

---

## Guiding Principles

1. **Ship something useful every release.** Even v0.1 should be genuinely better for hams than nothing.
2. **Testable > theoretical.** If there is no acceptance criterion, there is no feature.
3. **Preserve the signal.** We will refuse to ship any denoiser that measurably hurts CW transients or FT8/FT4 decode rates.
4. **Local-first, always.** Zero cloud dependencies in core. No telemetry without explicit opt-in.
5. **Cross-platform from day one.** Linux, macOS, Windows, Raspberry Pi 5.
6. **Latency is a feature.** v0.1 < 100 ms, v0.3 target < 50 ms, v1.0 documented budget per mode.
7. **Community ≥ code.** Docs, samples, and onboarding are first-class deliverables, not afterthoughts.

---

## Latency Budget (reference)


| Stage                             | v0.1 target  | v0.3 target | Notes                                 |
| --------------------------------- | ------------ | ----------- | ------------------------------------- |
| Audio capture (frame)             | 10–20 ms     | 5–10 ms     | PortAudio / ALSA / JACK / WASAPI      |
| Pre-emphasis + feature extraction | ≤ 5 ms       | ≤ 2 ms      | VOLK + liquid-dsp SIMD                |
| ONNX inference (DFN3)             | ≤ 30 ms      | ≤ 15 ms     | XNNPACK/CoreML/DirectML; INT8 stretch |
| Post-processing + overlap-add     | ≤ 5 ms       | ≤ 2 ms      | —                                     |
| Output buffering                  | 10–30 ms     | 5–15 ms     | —                                     |
| **End-to-end (p99)**              | **< 100 ms** | **< 50 ms** | Measured on M1 / i5-8xxx / RPi 5      |


RPi Zero 2 W ships with RNNoise (not DFN3) and has a separate, looser budget.

---

# v0.1 — Audio-Only Denoiser → Virtual Cable

**Theme:** Prove the core value proposition with the smallest possible surface area. One executable, one model, one virtual cable, demonstrably better copy.

## Scope

**In:**

- Offline WAV denoising (CLI)
- Real-time audio-in → audio-out (virtual cable friendly)
- DeepFilterNet3 primary model (ONNX, CPU)
- RNNoise fallback model (ONNX, CPU)
- Minimal Tk/Qt GUI: A/B toggle, device picker, record before+after, live telemetry
- Cross-platform: Linux / macOS / Windows / Raspberry Pi 5

**Explicitly out:**

- Any SDR / RF / IQ input (that's v0.2)
- GNU Radio flowgraph integration (v0.2)
- Mode profiles, adaptive notch (v0.3)
- Spectrogram UI (v0.4)
- Fine-tuning tooling (v0.5)

## Deliverables

- `rfwhisper` Python package with `pyproject.toml` (installable via `pip install -e .`)
- `rfwhisper denoise` — offline WAV in → WAV out + JSON report
- `rfwhisper denoise-live` — real-time audio device → audio device
- `rfwhisper audio list` — enumerate input/output devices
- `rfwhisper gui` — minimal cross-platform GUI (PySide6 or Tk)
- `rfwhisper models fetch` — downloads pre-converted DFN3 + RNNoise ONNX
- `models/deepfilternet3/model.onnx` (or hosted with SHA-256 pinned in repo)
- `models/rnnoise/model.onnx` (or hosted with SHA-256 pinned)
- `tests/audio/` with acceptance harness (see below)
- `docs/quickstart.md`, `docs/virtual-cable-setup.md` (Win/Mac/Linux)
- At least 5 seed samples in `samples/` covering SSB, CW, FT8, VHF FM, powerline buzz

## Acceptance Criteria (Pass/Fail)


| #   | Criterion                            | Measurement                                                                  | Threshold                                                  |
| --- | ------------------------------------ | ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| A1  | Effective SNR gain on ham speech mix | Matched-filter correlation vs clean reference on `tests/audio/ssb_mix_*.wav` | **≥ +3 dB** avg, ≥ +6 dB on powerline-dominant clips       |
| A2  | No FT8 decode regressions            | WSJT-X decoder run on raw vs denoised 15-min segment                         | Denoised decodes **≥** raw decodes, **zero** false decodes |
| A3  | No CW transient damage               | RMS energy in keying-onset window (first 5 ms of dit)                        | Within **±1 dB** of raw                                    |
| A4  | End-to-end latency (p99)             | `tests/audio/latency_probe.py` with impulse train                            | **< 100 ms** on i5-8xxx / M1 / RPi 5                       |
| A5  | Real-time factor (RTF)               | DFN3 CPU-only on target hardware                                             | **RTF < 0.5** (headroom for other tasks)                   |
| A6  | No-op sanity (clean in → clean out)  | PESQ / STOI on clean speech                                                  | PESQ drop ≤ 0.3, STOI drop ≤ 0.02                          |
| A7  | Cross-platform install               | CI matrix build + smoke test                                                 | Green on ubuntu-22.04, macos-13, windows-2022              |
| A8  | Virtual cable routing docs           | Manual verification checklist                                                | A beginner can get audio routed to WSJT-X in ≤ 10 minutes  |


All CI-enforceable criteria run in `[.github/workflows/basic-ci.yml](./.github/workflows/basic-ci.yml)`.

## Testable Success Examples

1. **"Weak 40m DX" demo** — A 60-second clip of an S3 DX station under S7 powerline buzz is processed and played blind for 5 volunteer hams. ≥ 4/5 prefer the denoised version AND copy ≥ 1 extra word/sentence on average.
2. **"FT8 marginal decode" demo** — A 15-minute FT8 cycle with known weak stations, replayed through RFWhisper → virtual cable → WSJT-X, recovers **at least as many** stations as raw, with zero false decodes. Ideal: +1 to +3 marginals/cycle.
3. **"CW under QRN" demo** — 25 WPM CW with atmospheric crashes. fldigi copy accuracy does not drop; operator reports crashes reduced ≥ 6 dB audibly.
4. **"POTA in a townhouse" demo** — Field recording from a noisy QTH (solar inverter + neighbor VDSL). Operator reports "I can actually work this band now" in a recorded testimonial.
5. **Latency probe** — Impulse-train round trip < 100 ms p99 on reference hardware. Logged in CI.

## Stretch Goals

- INT8 quantized DFN3 variant for RPi 4 support
- CoreML / DirectML / CUDA execution provider auto-select
- Push-to-toggle keyboard shortcut in GUI
- Waveform-only (no spectrogram) before/after viewer
- One-liner install scripts (`install.sh`, `install.ps1`)

## Exit Criteria (Tag v0.1.0)

> All of A1–A8 pass in CI on the reference hardware matrix, at least 3 of the 5 testable-success demos are recorded and linked from the release notes, and the quickstart doc has been validated end-to-end by at least one non-author contributor on each OS.

---

# v0.2 — GNU Radio + SoapySDR Pipeline

**Theme:** Take the audio MVP and wire it into a real SDR flowgraph. Live RF → denoised audio, on-air.

## Scope

**In:**

- GNU Radio 3.10.x flowgraph: SoapySDR source → demod (SSB/AM/FM/NBFM) → resample → gr-dnn ONNX denoiser block → audio sink
- Pre-built flowgraphs for RTL-SDR, Airspy HF+, ADALM-Pluto, SDRplay, HackRF
- `rfwhisper flowgraph run <name>` launcher
- Working on at least **3 real SDRs** with on-air QSOs documented

**Out:** Mode profiles (v0.3), UI overlays (v0.4), training tools (v0.5)

## Deliverables

- `flowgraphs/rtl_ssb_hf.grc` (and generated `.py`)
- `flowgraphs/airspy_hfplus_ssb.grc`
- `flowgraphs/pluto_vhf_fm.grc`
- `flowgraphs/sdrplay_ssb.grc`
- `flowgraphs/hackrf_wbfm.grc`
- `gr-rfwhisper/` — our own gr-dnn wrapper block with model-swap support, or a documented recipe for using stock gr-dnn
- `docs/gnuradio-install.md` (per-OS)
- `docs/soapy-hardware-matrix.md` with tested SDR list
- On-air demo videos / recordings in release notes

## Acceptance Criteria


| #   | Criterion                                  | Threshold                                                       |
| --- | ------------------------------------------ | --------------------------------------------------------------- |
| B1  | Live flowgraph runs end-to-end on ≥ 3 SDRs | RTL-SDR v4, Airspy HF+, 1 of {Pluto, SDRplay, HackRF}           |
| B2  | End-to-end latency (antenna → speaker)     | **< 250 ms** p99 on reference laptop (includes radio DSP)       |
| B3  | CPU at 48 kHz audio + 384 ksps IQ          | **< 60 %** of one core on i5-8xxx                               |
| B4  | Flowgraph restart stability                | 1 hr soak without dropouts; 100 stop/start cycles without leaks |
| B5  | Model hot-swap                             | Switching DFN3 ↔ RNNoise at runtime works without restart       |
| B6  | Retains all v0.1 acceptance                | Re-run A1–A8 on equivalent audio through the flowgraph          |


## Testable Success Examples

- **On-air QSO recording** demonstrating a real contact worked through RFWhisper on at least 3 different SDRs, posted in release notes.
- **Contest replay** — 30-minute HF contest recording through RFWhisper shows improved busy-band copy without destroying CQ decodes.
- **VHF FM weak-signal** — a scratchy 2m simplex signal becomes readable.

## Stretch Goals

- `gr-rfwhisper` OOT module published to PyBOMBS / distro repos
- Windows installer bundle (GR 3.10 + RFWhisper)
- GNU Radio Companion block palette entry with icon + help
- Live waterfall preview in GRC (not full UI yet)

## Exit Criteria (Tag v0.2.0)

> B1–B6 pass. At least 3 on-air recordings from community contributors posted. Hardware matrix doc lists ≥ 5 SDRs with known status (even if "unsupported / needs help").

---

# v0.3 — Mode Profiles + Adaptive Notch

**Theme:** One denoiser is good; a *tuned* denoiser per mode is dramatically better. Add mode awareness and a classical notch that cooperates with the NN.

## Scope

**In:**

- Mode profiles: `ssb`, `cw`, `ft8`, `ft4`, `rtty`, `am`, `fm-narrow`, `fm-wide`, `vhf-ssb`
- Per-mode parameters: attack/release, NN aggressiveness, notch enable, bandwidth, de-emphasis
- Adaptive narrowband notch (classical DSP) **before** the NN — removes carriers / birdies that would otherwise confuse it
- Profile auto-detect (optional, conservative)
- Latency budget **< 50 ms** on reference hardware for SSB/CW

## Deliverables

- `rfwhisper/profiles/*.yaml` — one per mode
- `rfwhisper denoise --profile cw` CLI flag
- Adaptive notch implementation (liquid-dsp `iirfilt` or VOLK-accelerated LMS)
- Profile docs with band/scenario suggestions
- Regression tests per profile

## Acceptance Criteria


| #   | Criterion                                       | Threshold                                       |
| --- | ----------------------------------------------- | ----------------------------------------------- |
| C1  | CW profile preserves keying transients          | RMS window within ±0.5 dB of raw                |
| C2  | FT8 profile: decode count                       | **≥** raw decodes on test cycle; ideal +1 to +3 |
| C3  | SSB profile intelligibility (PESQ)              | ≥ raw PESQ on matched pairs                     |
| C4  | Adaptive notch removes ≥ 30 dB at target birdie | Measured on synthetic carrier                   |
| C5  | Latency p99 on SSB/CW profiles                  | **< 50 ms** on i5-8xxx / M1                     |
| C6  | Auto-detect accuracy (if enabled)               | ≥ 90 % on labeled eval set; **off by default**  |


## Testable Success Examples

- **"2m weak-signal SSB"** — EME-style weak SSB goes from unreadable to copyable.
- **"630m CW"** — slow CW under atmospheric QRN preserves all keying, crashes reduced.
- **"FT8 profile never regresses"** — 10 hours of replayed FT8 across bands shows zero decode loss.

## Stretch Goals

- DIGI profile that preserves generic MFSK/PSK shape
- Per-profile model variants (tiny model for CW, full model for SSB)
- Rig-control (Hamlib) hook to auto-select profile from mode

## Exit Criteria (Tag v0.3.0)

> C1–C5 pass across all profiles. At least one community contributor has validated each profile on real on-air signals.

---

# v0.4 — UI: Before/After Spectrogram + Metrics

**Theme:** Make the improvement **visible**. Hams trust their eyes and ears; give them both.

## Scope

**In:**

- Qt/PySide6 main window with:
  - Dual waterfall (raw vs denoised) with synchronized scroll
  - A/B audio toggle (big, obvious)
  - Live telemetry: effective SNR gain, SINAD, RTF, CPU %, end-to-end latency
  - Model selector + profile selector
  - Before/after record button
- Optional docked view inside GRC-generated flowgraph
- Dark mode, high-contrast mode (for field ops in sunlight)

## Deliverables

- `rfwhisper-gui` binary (PySide6)
- Waterfall renderer (OpenGL or pyqtgraph)
- Metrics panel
- Screenshots in docs

## Acceptance Criteria


| #   | Criterion          | Threshold                                                       |
| --- | ------------------ | --------------------------------------------------------------- |
| D1  | GUI frame rate     | ≥ 30 fps on reference laptop; ≥ 20 fps on RPi 5                 |
| D2  | GUI CPU overhead   | ≤ +10 % CPU vs headless                                         |
| D3  | Telemetry accuracy | SNR gain estimate within ±1 dB of reference                     |
| D4  | Latency display    | Agrees with A4 probe within 5 ms                                |
| D5  | Accessibility      | All controls keyboard-navigable; WCAG AA contrast in light/dark |


## Testable Success Examples

- Screenshot thread on QRZ / r/amateurradio where hams post "look at this waterfall difference".
- Field-ops screenshot from a POTA activation showing the UI working on a laptop in bright sun (high-contrast mode).

## Stretch Goals

- Web-based UI option (for headless server / remote ops)
- Recording library browser with ABX test mode
- Shareable "clip" export (6-second before/after MP4 with waveform + waterfall)

## Exit Criteria (Tag v0.4.0)

> D1–D5 pass. 3+ screenshots + 1 short demo video in release notes.

---

# v0.5 — Fine-tuning Tools + Dataset Generator

**Theme:** Give operators the power to tune the model to *their* noise environment.

## Scope

**In:**

- Dataset builder: record clean ham speech + isolated noise, synthesize mixed training data with realistic SNRs
- Fine-tuning CLI (`rfwhisper train fine-tune`) wrapping PyTorch + DFN3 training code
- Export to ONNX with validated parity
- Evaluation harness (reuse v0.1 acceptance tests)
- Docs: "how to record your own noise, train, and deploy in 30 minutes"

## Deliverables

- `rfwhisper train record-noise` — captures isolated noise from rig
- `rfwhisper train build-dataset` — synthesizes mixes with SNR control
- `rfwhisper train fine-tune` — LoRA-style or full fine-tune
- `rfwhisper train export` — PyTorch → ONNX with parity check
- `rfwhisper train evaluate` — runs v0.1 acceptance suite on new model
- Tutorial notebook in `notebooks/`
- Optional: Hugging Face / self-hosted model hub structure

## Acceptance Criteria


| #   | Criterion                                           | Threshold                                                                                  |
| --- | --------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| E1  | Fine-tune wall time (30 min noise sample, RTX 3060) | ≤ 2 hours                                                                                  |
| E2  | Fine-tune on RPi 5 CPU (long-running)               | Completes in < 24 hours for tiny model                                                     |
| E3  | ONNX export parity vs PyTorch                       | Output diff ≤ 1e-3 RMS on eval set                                                         |
| E4  | Post-fine-tune eval                                 | Beats baseline on user's noise by ≥ +2 dB SNR gain; no regression on held-out clean speech |
| E5  | Dataset generator realism                           | Blind listeners cannot reliably distinguish real vs synthesized mixes                      |


## Testable Success Examples

- Tutorial: operator records 10 minutes of their solar-inverter noise, fine-tunes for an hour, and demonstrates an additional +2 dB gain over the stock model on their own shack.
- Community-submitted model (e.g., "HF-Contester-v1") beats baseline on contest-week recordings.

## Stretch Goals

- Federated / community averaging of fine-tunes (privacy-preserving; opt-in)
- Quantization-aware fine-tuning
- LoRA adapters that can be hot-swapped per band

## Exit Criteria (Tag v0.5.0)

> E1–E5 pass. At least 2 community-submitted fine-tunes accepted into the model hub.

---

# v1.0 — Polished Release

**Theme:** Boring-in-a-good-way. This is the version we tell non-hacker hams to install.

## Scope

**In:**

- Signed installers (Windows `.msi`, macOS `.dmg` notarized), `.deb`, `.rpm`, AUR
- Raspberry Pi OS image (bootable SD card) with RFWhisper pre-installed
- At least **8 SDRs** tested and officially supported
- Model hub: minimum 4 community-validated models (HF contest, VHF mobile, POTA portable, 630m / LF)
- Extensive docs site (mkdocs or similar)
- Performance benchmarks published for laptop + RPi 5 + mid-range desktop
- Localization scaffolding (en at minimum; community translations welcomed)

## Deliverables

- Signed Windows installer
- Notarized macOS .dmg (universal binary)
- `.deb`, `.rpm`, AUR `PKGBUILD`
- RPi OS image + install doc
- Model hub (static site + CI validator)
- Docs site with tutorials, API reference, architecture
- Published benchmark report (reproducible, with hardware + methodology)

## Acceptance Criteria


| #   | Criterion                          | Threshold                                                        |
| --- | ---------------------------------- | ---------------------------------------------------------------- |
| F1  | 8 supported SDRs tested on-air     | Documented in hardware matrix with logs                          |
| F2  | Installer UX                       | Non-technical ham installs and gets audio routed in ≤ 15 minutes |
| F3  | Benchmark repro                    | Any contributor can replicate published numbers ±10 %            |
| F4  | All previous acceptance tests pass | A1–A8, B1–B6, C1–C6, D1–D5, E1–E5 green                          |
| F5  | Zero critical bugs open > 30 days  | Tracked in GitHub Projects                                       |
| F6  | Docs completeness                  | Every CLI flag, every GUI control, every profile documented      |


## Testable Success Examples

- A new ham with zero DSP / ML background installs v1.0 on Windows, routes it to WSJT-X, and logs their first FT8 DX through RFWhisper within 1 hour of download.
- Contest club publishes testimonial: "RFWhisper made the difference on 80m Saturday night."
- At least one magazine / podcast review (QST, SDR-Academy, Ham Radio Crash Course, etc.).

## Stretch Goals

- Pre-installed on a popular SDR distro (DragonOS, etc.)
- Chrome/Firefox extension for browser-based SDR (KiwiSDR, WebSDR)

## Exit Criteria (Tag v1.0.0)

> F1–F6 pass. Release notes include 8 SDR on-air logs, 4 community models, a benchmark report, and at least one external review.

---

# v1.1+ — Plugins, GR4 Native, Propagation Extras

**Theme:** Expand the platform. Keep v1.0 stable on a long-term support branch; iterate on main.

## Candidate Work (prioritized at v1.0 retro)

- **Plugin architecture**: drop-in user models, custom DSP blocks, alternative notchers
- **GNU Radio 4.0 native** block (`gr4-rfwhisper`) with DAG-native perf wins; keep 3.10 LTS branch
- **Propagation extras**: VOACAP / ionosonde hints to bias the denoiser on expected signal shape
- **RX-specific modes**: digital voice (DMR/D-STAR/YSF audio denoise), EMCOMM voice modes
- **TX assist**: compander / de-noise of your own mic (with heavy ethical guardrails and clear labeling)
- **Remote shack** integration: WFView, FLrig, OpenWebRX pass-through
- **Federated model improvements** with cryptographic verification
- **Rust rewrite of hot paths** where profiling justifies it

## Ground Rules for Post-1.0

1. **Main stays green.** v1.0 users on an LTS branch never get surprised.
2. **Every new feature needs a testable success metric**, same bar as v0.1–v1.0.
3. **Telemetry remains opt-in.** Always. No exceptions.
4. **If it doesn't help an actual ham on the air, it waits.**

---

## How to Move a Version Forward

1. Pick an unchecked deliverable above.
2. Open an issue linking to the relevant acceptance criterion (e.g., "A2: FT8 regression harness").
3. Propose a design in GitHub Discussions if it's non-trivial.
4. Submit a PR referencing the issue. CI enforces the applicable acceptance tests.
5. Request review from the relevant area owner (see [AGENTS.md](./AGENTS.md) role mapping).
6. When the last box for a version is checked and all its acceptance tests pass, we cut the tag.

**The roadmap is a living document.** If you think a milestone is wrong, open an RFC issue. We'd rather change the plan than ship the wrong thing.

73 and keep the signal clean.