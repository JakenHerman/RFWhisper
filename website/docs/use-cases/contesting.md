---
id: contesting
title: Contesting
sidebar_position: 2
description: Using RFWhisper in CQ WW, ARRL Sweepstakes, RTTY Roundup, and general contest ops.
---

# Contesting

Contest weekends are where QRM piles up fast: dozens of stations packed into a few kHz, overloaded front-ends, exchanges barely above the noise floor. RFWhisper is tuned to **preserve decode quality** under exactly this condition — never at the cost of busting a call.

## Hard requirements for contest use

1. **Zero decode regressions.** If a raw pass decodes `K3XYZ` and the denoised pass decodes `KJXYZ`, that's a bug in the model. Run the [FT8 regression gate](../quickstart/v0_1-test-guide#a2--ft8-decode-non-regression) on your chosen model before the contest.
2. **Low latency.** No operator wants to hit the TX button two beats behind the other station. Stay under 100 ms p99.
3. **Bypass toggle on one key.** You'll want to A/B instantly when a station comes up.
4. **Predictable level.** Soft limiter catches overshoots so your AGC / noise blanker upstream stays happy.

## Recommended profiles

| Contest type | Profile | Notes |
|---|---|---|
| SSB phone contest (CQWW SSB, Sweepstakes Phone) | `ssb` | Full aggression OK — speech-centric |
| CW contest (CQWW CW, ARRL DX CW) | `cw` | Transient preservation is non-negotiable |
| RTTY (RTTY Roundup) | `rtty` (v0.3) | Careful — don't filter the 170 Hz shift |
| FT contests (FT Roundup) | `ft8` / `ft4` | Gentler; let the decoder do its job |

## Running vs S&P

- **Running (CQ):** keep the denoiser on. You're looking for the faintest caller at the bottom of the pile.
- **S&P:** consider running the denoiser for searching, but when you hear a clear station you want to work, the raw signal already has what you need — the denoiser is break-even to mildly positive here.

## Sample contest scenario: 40m SSB at 0200 UTC

A typical late-night 40m SSB contest:

- Band crowded: 7.150–7.200 wall-to-wall.
- Your neighbour's inverter adds an S5 rhythmic buzz.
- You're trying to copy an exchange at S3.

```bash
rfwhisper denoise-live \
  --in "USB Audio CODEC" --out "CABLE Input" \
  --model deepfilternet3 --profile ssb \
  --telemetry ./contest-log.jsonl
```

`contest-log.jsonl` records rolling SNR gain, RTF, and latency per minute so you can analyze the session later.

## Spotting weak multipliers

Anecdotally: the biggest single-operator benefit is hearing the **call sign prefix** under splatter from an adjacent strong station. The NN preserves consonant structure that classical NR tends to smear.

## What RFWhisper is *not*

- A **noise blanker** — we don't cancel impulsive transmitter artifacts (keyclicks, splatter). Use your rig's NB for those.
- A **passband filter** — use your rig's PBT / DSP bandwidth to set the crop; RFWhisper works inside whatever you hand it.
- A **replacement for a good antenna.** +3–6 dB of SNR gain on a compromise antenna is still less than what a real antenna change does.

## After-action reports welcome

Post your contest results with RFWhisper in the [Contesting Discussion](https://github.com/jakenherman/rfwhisper/discussions/categories/contesting). Include:

- Contest + your category
- Band + mode
- Model used + profile + hardware
- Rate improvements / subjective observations
- Any decodes you're confident the raw audio would not have produced

Your real-world data is how we decide what to improve next.
