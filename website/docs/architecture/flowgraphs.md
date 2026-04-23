---
id: flowgraphs
title: GNU Radio Flowgraphs
sidebar_position: 4
description: One GNU Radio flowgraph per supported SDR, rendered as Mermaid diagrams auto-extracted from .grc files.
---

import FlowgraphRenderer from '@site/src/components/FlowgraphRenderer';

# GNU Radio Flowgraphs

Starting in v0.2 RFWhisper ships a curated set of GNU Radio 3.10 flowgraphs — one per commonly used SDR. Each `.grc` file lives under [`flowgraphs/`](https://github.com/jakenherman/rfwhisper/tree/main/flowgraphs); the diagrams below are **auto-generated** from the `.grc` at build time by [`scripts/grc-to-mermaid.mjs`](https://github.com/jakenherman/rfwhisper/tree/main/website/scripts/grc-to-mermaid.mjs) so they never drift from the canonical flowgraph.

:::info Live-extracted content

If you're reading this in the docs and a flowgraph doesn't match the repo, it's a bug — our build broke. Please [open an issue](https://github.com/jakenherman/rfwhisper/issues/new). The sync script runs on every docs build.

:::

## RTL-SDR v4 · HF SSB

<FlowgraphRenderer
  name="rtl_ssb_hf"
  sdr="RTL-SDR v4"
  mode="USB / LSB (HF)"
  source="flowgraphs/rtl_ssb_hf.grc"
  description="Reference v0.2 flowgraph. Works at 14 MHz with a direct-sampling-mod dongle or with an upconverter.">
{`
flowchart LR
  A([RTL-SDR v4<br/>2.048 Msps @ 14.200 MHz]) --> B[SoapySDR Source]
  B --> C[Freq Xlating FIR<br/>tune ±BFO, decimate 64]
  C --> D[SSB Demod<br/>USB]
  D --> E[Polyphase Resampler<br/>→ 48 kHz]
  E --> F["gr-dnn · DeepFilterNet3"]
  F --> G[Soft Limiter + AGC]
  G --> H[/Virtual Audio Cable/]
  F -.-> T{{SNR · RTF · latency<br/>→ telemetry}}
`}
</FlowgraphRenderer>

## Airspy HF+ · HF SSB

<FlowgraphRenderer
  name="airspy_hfplus_ssb"
  sdr="Airspy HF+ Discovery"
  mode="USB / LSB (HF)"
  source="flowgraphs/airspy_hfplus_ssb.grc"
  description="Native 768 kHz IQ from the HF+ Discovery means less decimation and cleaner dynamic range than the RTL-SDR path.">
{`
flowchart LR
  A([Airspy HF+<br/>768 ksps]) --> B[SoapySDR Source]
  B --> C[Freq Xlating FIR<br/>decimate 16]
  C --> D[SSB Demod]
  D --> E[Polyphase Resampler → 48 kHz]
  E --> F["gr-dnn · DeepFilterNet3"]
  F --> G[/Virtual Audio Cable/]
`}
</FlowgraphRenderer>

## ADALM-Pluto · VHF NBFM

<FlowgraphRenderer
  name="pluto_vhf_fm"
  sdr="ADALM-Pluto"
  mode="Narrowband FM (VHF)"
  source="flowgraphs/pluto_vhf_fm.grc"
  description="2m simplex with de-emphasis before the NN so hiss doesn't get amplified into the denoiser's features.">
{`
flowchart LR
  A([ADALM-Pluto<br/>2.4 Msps]) --> B[SoapySDR Source]
  B --> C[Freq Xlating FIR<br/>decimate 50]
  C --> D[Quadrature Demod<br/>NBFM]
  D --> E[De-emphasis<br/>75 µs]
  E --> F[Polyphase Resampler → 48 kHz]
  F --> G["gr-dnn · DeepFilterNet3"]
  G --> H[/Virtual Audio Cable/]
`}
</FlowgraphRenderer>

## SDRplay RSP1A · HF SSB

<FlowgraphRenderer
  name="sdrplay_ssb"
  sdr="SDRplay RSP1A"
  mode="USB / LSB (HF)"
  source="flowgraphs/sdrplay_ssb.grc"
  description="Via SoapySDRPlay3. Remember to enable broadcast notches and the bias-T stays off on RX-only setups.">
{`
flowchart LR
  A([SDRplay RSP1A<br/>2 Msps via SoapySDRPlay3]) --> B[SoapySDR Source]
  B --> C[Freq Xlating FIR<br/>decimate 42]
  C --> D[SSB Demod]
  D --> E[Polyphase Resampler → 48 kHz]
  E --> F["gr-dnn · DeepFilterNet3"]
  F --> G[/Virtual Audio Cable/]
`}
</FlowgraphRenderer>

## HackRF One · Wideband FM

<FlowgraphRenderer
  name="hackrf_wbfm"
  sdr="HackRF One"
  mode="Wideband FM (BCB)"
  source="flowgraphs/hackrf_wbfm.grc"
  description="Stress test: 2 Msps input, wideband FM demod, denoise. Useful as a demo but ham use is typically narrowband.">
{`
flowchart LR
  A([HackRF One<br/>2 Msps]) --> B[SoapySDR Source]
  B --> C[WBFM Demod]
  C --> D[Polyphase Resampler → 48 kHz]
  D --> E["gr-dnn · DeepFilterNet3"]
  E --> F[/Virtual Audio Cable/]
`}
</FlowgraphRenderer>

## Authoring your own flowgraph

1. Open **GNU Radio Companion** and build your flowgraph.
2. Insert a **gr-dnn** block with the RFWhisper ONNX model path and the model's native input/output contract.
3. Save as `flowgraphs/<short_name>.grc`.
4. Run `npm run sync:grc` in `website/` — this re-extracts all flowgraphs into Mermaid diagrams.
5. Open a PR; CI will re-publish the docs automatically.

## Hardware matrix

See [Hardware → SDRs](../hardware/sdrs) for the per-device compatibility table and on-air notes.
