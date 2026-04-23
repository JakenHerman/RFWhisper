---
id: why-rfwhisper
title: Why RFWhisper?
sidebar_position: 2
description: Modern RFI has outpaced classical DSP. RFWhisper brings ham-tuned deep learning denoising to amateur radio without giving up locality, latency, or signal preservation.
---

# Why RFWhisper?

## The noise floor isn't what it used to be

If you've tried to work a POTA station on 40m from a townhouse, chased DX through a neighbour's plasma TV, or decoded FT8 next to an EV charger, you know the modern noise floor is **unrecognizable** compared to a decade ago. The culprits:

| Noise source | Character | Typical footprint |
|---|---|---|
| Switch-mode / LED drivers | Broadband hash, 20–150 kHz spurs | HF wide-band raised floor |
| Solar inverters (MPPT) | Rhythmic buzz | 40m–10m, drifts with sun |
| VDSL / PLC / powerline ethernet | Wideband raised floor, data combs | 1.8–30 MHz |
| Plasma TVs, touch lamps, grow lights | Narrowband carriers | Random on HF |
| Neighbor electric fences, thermostats | Regular click trains | Rural/suburban |
| Atmospheric QRN | Impulsive crashes, seasonal | LF/MF/low-HF |

Traditional DSP tools (analog NB/NR, classical Wiener filters, simple spectral subtraction) either:

1. **Miss** modern complex impulsive + stationary mixtures.
2. **Wreck** the signal — they smear CW keying transients, introduce "underwater" artifacts that confuse FT8/WSJT-X decoders, or chew the atmospheric hiss so aggressively that weak signals disappear under a gated floor.

## The RFWhisper approach

Use a deep neural network *designed* for speech denoising (DeepFilterNet3) and *fine-tune it on real amateur-radio noise* — recorded powerline buzz, inverter hash, static crashes — so it learns to preserve SSB phonemes, CW keying transients, and FT8 tones while killing QRM.

Then wrap it in a pipeline that:

- Runs **locally** (CPU or GPU) — no cloud, no telemetry.
- Integrates with **GNU Radio + SoapySDR** so any SDR or rig's audio works.
- Emits to a **virtual cable** so WSJT-X / fldigi / JS8Call / your headphones see clean audio transparently.
- Stays under **100 ms p99 latency** (target &lt; 50 ms by v0.3).
- Is **GPLv3** so nobody can take it closed.

## The non-negotiables

Three things RFWhisper will never do, full stop:

1. **Damage CW keying transients.** We run a CI gate that measures RMS energy in the first 5 ms of each dit. A model that drifts outside ±1 dB of raw does not ship.
2. **Reduce FT8/FT4 decode counts.** We replay a 15-minute FT8 cycle against WSJT-X on every candidate model. If it loses even one decode or fabricates one false decode, it's rejected.
3. **Mandate an internet connection at runtime.** Models are fetched once at install (SHA-256 verified), and the pipeline runs offline forever after.

## Testable value: four concrete examples

These live in [`tests/audio/`](https://github.com/jakenherman/rfwhisper/tree/main/tests/audio) and run in CI. See [ROADMAP.md § Acceptance Criteria](./roadmap) for the full list.

| # | Demo | Pass criterion |
|---|---|---|
| 1 | Weak 40m SSB under S7 powerline buzz | ≥ 4/5 blind listeners prefer denoised AND copy +1 word/sentence |
| 2 | 15-min FT8 cycle through WSJT-X | Denoised decodes ≥ raw decodes, zero false decodes |
| 3 | 25 WPM CW under atmospheric QRN | fldigi copy accuracy unchanged, crashes −6 dB audibly |
| 4 | End-to-end latency probe | &lt; 100 ms p99 on i5-8xxx / M1 / RPi 5 |

## Who this is for

- **Contesters** who want to keep decoding weak stations in a crowded band.
- **POTA / SOTA operators** running from townhouses, condos, or next to a solar farm.
- **Weak-signal specialists** on 630m / 2m EME / microwave where every dB counts.
- **EmComm operators** who need a tool that works when the internet doesn't.
- **DSP and ML hackers** who want a serious, well-documented sandbox for real-time audio neural nets.
- **Any ham** who has ever shrugged at their S9+10 noise floor and said, "there's nothing I can do about it."

There is something you can do about it. <span className="rfw-73">73</span>
