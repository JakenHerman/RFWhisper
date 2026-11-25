---
id: faq
title: FAQ
sidebar_position: 50
description: Frequently asked questions about RFWhisper — technical, operational, and ham-specific.
---

# FAQ

## Does RFWhisper send my audio anywhere?

**No.** Everything runs locally. The DeepFilterNet3 model is built into the binary, so nothing is downloaded at runtime either. The only network code is the optional `rfwhisper models fetch` command for future add-on models, which never runs unless you type it. No telemetry is ever sent. This is a GPLv3 project — if that ever changes, fork it.

## Will it decode encrypted transmissions?

**No.** RFWhisper is a noise-reduction tool applied to already-demodulated audio. It does not decrypt, descramble, or otherwise defeat intentional obfuscation. Doing so would likely violate your country's regulations and the project's [Code of Conduct](./code-of-conduct).

## Is this a replacement for my rig's NR?

Usually yes — turn your rig's NR **off** when using RFWhisper, or they'll fight each other. The one exception is classical **noise blanking** (for keyclicks / impulsive transmitter splatter), which does something different; keep that on if it helps.

## Does it work for digital modes (FT8, FT4, PSK31, RTTY)?

**Not in the beta.** The beta's stock DeepFilterNet3 model is trained on speech, and it treats FT8/FT4/RTTY/PSK tones as noise. Run digital modes with `--model passthrough` for now. The goal for v0.1 final and the v0.3 `ft8` profile is a hard guarantee: a non-regression gate (A2) requires the decoder to see at least as many valid decodes on denoised audio as on raw, with zero false decodes.

## What about CW? Won't it soften my dits?

**In the beta, don't use the speech model on CW:** it can remove the tone. Use `--model passthrough`. CW is the most protected signal type on the roadmap: the v0.3 `cw` profile uses shorter frames and lower aggressiveness, and the A3 gate measures RMS in the first 5 ms of every keying onset and refuses any model that drifts outside ±1 dB.

## Can I use this on a rig that doesn't have a USB CODEC?

Yes — any way you get audio into the computer works: line-out into a USB sound card, a SignaLink, a direct audio cable into mic-in. If your rig is SDR-based (ANAN, Flex, KiwiSDR) use the rig's native audio routing into a virtual cable and have RFWhisper read from that.

## Does latency matter for FT8?

Less than you'd think. FT8 is a 15-second mode with wide synchronization tolerance; a 100 ms denoiser latency is invisible to the decoder. For SSB ragchewing and CW where you're reacting to other operators, the under-100-ms budget matters more.

## CPU-only or GPU?

CPU only. The beta's DeepFilterNet3 runs on the CPU and uses about 5 % of one core on an Apple Silicon laptop (RTF ≈ 0.02), so a GPU isn't needed. GPU execution providers are on the roadmap for custom ONNX models.

## Can I run multiple RFWhisper pipelines in parallel?

Yes — each instance is a separate process. Typical case: HF SSB denoiser on your main rig + a second instance on a 2m FM radio. Just route each to its own virtual cable.

## Does it work with [specific SDR]?

If the SDR has a SoapySDR driver, yes (v0.2+). See [Hardware → SDRs](./hardware/sdrs) for the current matrix. If your SDR isn't listed, please test and file a report.

## Can I train my own model on my shack's noise?

Yes (v0.5). See [Fine-Tuning for Your QTH](./models/fine-tuning) — the recipe is about 30 minutes of work plus training wall time.

## What license are community models under?

Whatever the contributor chooses, as long as it's GPLv3-compatible. Models in the core repo prefer MIT / Apache-2.0 / BSD so downstream users have the maximum freedom.

## Can I use RFWhisper commercially?

Yes — GPLv3 permits commercial use. You must ship source for any modifications you distribute, and you cannot impose additional restrictions downstream. If you sell a product that includes RFWhisper, the user has the right to inspect, modify, and redistribute the version you shipped.

## Why GPLv3 and not MIT / Apache?

Because this is ham-community code and we want it to stay ham-community code. GPLv3 guarantees that any improvements flow back into the public commons. See [README § License](https://github.com/jakenherman/rfwhisper/blob/master/README.md#license) for the full reasoning.

## Will this work on Windows 10 / old Linux / older macOS?

CI builds and tests on Ubuntu 22.04, macOS 14 (Apple Silicon), and Windows
Server 2022 (the Windows 10/11 code base). The beta has been run live on macOS;
Windows and Linux live-use reports are very welcome. Older systems may work, but
we can't promise CI coverage.

## How do I benchmark my hardware?

```bash
rfwhisper samples synth --preset ssb_powerline_s3_s7 --out bench.wav --seconds 30
rfwhisper bench rtf bench.wav        # real-time factor (A5: < 0.5)
rfwhisper bench latency bench.wav    # per-block processing + algorithmic latency
```

Then run `rfwhisper denoise-live` for a minute and copy the last status line.
Post all three, with your CPU and OS, to
[Discussions → Benchmarks](https://github.com/jakenherman/rfwhisper/discussions/categories/benchmarks):
your numbers help us keep the reference targets honest.

## What if I find a bug?

See [Reporting Bugs](https://github.com/jakenherman/rfwhisper/blob/master/CONTRIBUTING.md#reporting-bugs) in CONTRIBUTING.md. Please include `rfwhisper --version`, `rfwhisper audio list`, your OS and CPU, the last `denoise-live` status line, and a short before/after clip (`--record-raw` / `--record-denoised`) if the issue is audio-quality related.

## How do I contribute?

See [CONTRIBUTING.md](./contributing). Code, samples, hardware reports, docs fixes, and community support in Discussions are all first-class contributions.

## Where do I send QSL cards?

Via the bureau (or direct with SASE) to the project's honorary call, once we're assigned one. Until then, keep your QSLs to the hams you worked — that's the real exchange. <span className="rfw-73">73</span>
