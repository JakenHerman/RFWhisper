---
id: faq
title: FAQ
sidebar_position: 50
description: Frequently asked questions about RFWhisper — technical, operational, and ham-specific.
---

# FAQ

## Does RFWhisper send my audio anywhere?

**No.** Core runs 100% locally. Models are fetched once (SHA-256 verified) and cached. No telemetry is ever sent without explicit, disclosed opt-in. This is a GPLv3 project — if that ever changes, fork it.

## Will it decode encrypted transmissions?

**No.** RFWhisper is a noise-reduction tool applied to already-demodulated audio. It does not decrypt, descramble, or otherwise defeat intentional obfuscation. Doing so would likely violate your country's regulations and the project's [Code of Conduct](./code-of-conduct).

## Is this a replacement for my rig's NR?

Usually yes — turn your rig's NR **off** when using RFWhisper, or they'll fight each other. The one exception is classical **noise blanking** (for keyclicks / impulsive transmitter splatter), which does something different; keep that on if it helps.

## Does it work for digital modes (FT8, FT4, PSK31, RTTY)?

Yes, with the **non-regression gates** guaranteeing the decoder sees at least as many valid decodes on denoised audio as on raw. Start with the `ft8` profile and measure.

## What about CW? Won't it soften my dits?

**No.** This is the most protected signal type in the project. The `cw` profile uses shorter frames and lower NN aggressiveness; a CI gate (`cw_transient_test`) measures RMS in the first 5 ms of every keying onset and refuses to merge any model that drifts outside ±1 dB.

## Can I use this on a rig that doesn't have a USB CODEC?

Yes — any way you get audio into the computer works: line-out into a USB sound card, a SignaLink, a direct audio cable into mic-in. If your rig is SDR-based (ANAN, Flex, KiwiSDR) use the rig's native audio routing into a virtual cable and have RFWhisper read from that.

## Does latency matter for FT8?

Less than you'd think. FT8 is a 15-second mode with wide synchronization tolerance; a 100 ms denoiser latency is invisible to the decoder. For SSB ragchewing and CW where you're reacting to other operators, the under-100-ms budget matters more.

## CPU-only or GPU?

CPU-only is the default and works great on Apple Silicon, any Ryzen 5000+, Intel 8th gen+, and Pi 5. GPU (CUDA / DirectML / CoreML via ONNX Runtime) helps on weaker hardware or when you're running several RFWhisper pipelines in parallel. Select providers with `rfwhisper info providers`.

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

Because this is ham-community code and we want it to stay ham-community code. GPLv3 guarantees that any improvements flow back into the public commons. See [README § License](https://github.com/jakenherman/rfwhisper/blob/main/README.md#license) for the full reasoning.

## Will this work on Windows 10 / old Linux / older macOS?

- **Windows 10 22H2** — officially supported.
- **macOS 13+** — officially supported.
- **Ubuntu 22.04+ / Debian 12 / Fedora 38+** — officially supported.
- Older systems may work; we can't promise CI coverage.

## How do I benchmark my hardware?

```bash
rfwhisper bench report --profile ssb --out report.html
```

Upload `report.html` to [Discussions → Benchmarks](https://github.com/jakenherman/rfwhisper/discussions/categories/benchmarks) — your numbers help us keep the reference targets honest.

## What if I find a bug?

See [Reporting Bugs](https://github.com/jakenherman/rfwhisper/blob/main/CONTRIBUTING.md#reporting-bugs) in CONTRIBUTING.md. Please include `rfwhisper doctor` output and a short audio clip if the issue is audio-quality related.

## How do I contribute?

See [CONTRIBUTING.md](./contributing). Code, samples, hardware reports, docs fixes, and community support in Discussions are all first-class contributions.

## Where do I send QSL cards?

Via the bureau (or direct with SASE) to the project's honorary call, once we're assigned one. Until then, keep your QSLs to the hams you worked — that's the real exchange. <span className="rfw-73">73</span>
