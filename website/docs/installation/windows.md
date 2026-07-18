---
id: windows
title: Windows
sidebar_position: 4
description: Install RFWhisper on Windows 10/11 with VB-Cable and DirectML acceleration.
---

# Install on Windows

Tested on Windows 10 22H2 and Windows 11.

## 1. Install Rust

Grab rustup from <https://rustup.rs/> and run the installer (accept the defaults; it pulls the MSVC toolchain — if prompted, let it install the Visual Studio C++ Build Tools).

Verify:

```powershell
cargo --version
# cargo 1.7x.x
```

## 2. Install Git + Git LFS

<https://git-scm.com/download/win> and <https://git-lfs.github.com/>.

## 3. Install VB-Cable (virtual audio cable)

1. Download VB-Cable from <https://vb-audio.com/Cable/>.
2. Run as administrator. **Reboot** after install (required).
3. You will now see **CABLE Input / Output** devices in the Sound panel.

For multi-application routing, [VB-Audio Voicemeeter](https://vb-audio.com/Voicemeeter/) is recommended but optional.

## 4. (Optional) GNU Radio

The official [GNU Radio Windows installer](https://www.gnuradio.org/download/) (3.10.x) is the simplest route (only needed for v0.2+ SDR flowgraph work).

## 5. RFWhisper itself

```powershell
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release
.\target\release\rfwhisper.exe models fetch
.\target\release\rfwhisper.exe doctor
```

Copy `target\release\rfwhisper.exe` anywhere on your `PATH` — it's a single self-contained binary.

## 6. DirectML acceleration (recommended)

When the ONNX (DFN3) backend is enabled, RFWhisper prefers **DirectML** on Windows automatically and falls back through CUDA → CPU:

```powershell
rfwhisper info providers
# DmlExecutionProvider    (preferred)
# CPUExecutionProvider    (fallback)
```

## 7. Routing audio

| From | To | How |
|---|---|---|
| Rig USB CODEC (IC-7300, etc.) | RFWhisper input | Choose **USB Audio CODEC** in `rfwhisper audio list` |
| RFWhisper output | WSJT-X / fldigi input | Choose **CABLE Input** as RFWhisper output, **CABLE Output** as WSJT-X input |
| Also hear it yourself | Speakers | In Sound Settings, enable *Listen to this device* on **CABLE Output**, forward to Speakers |

## 8. Realtime / MMCSS

RFWhisper automatically requests the "Pro Audio" MMCSS characteristic on the audio thread on Windows. No action needed.

## Troubleshooting

- **`error: linker 'link.exe' not found`** — install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload (rustup offers this during install).
- **VB-Cable audio crackling** — set VB-Cable's internal rate to **48000 Hz** in its control panel.
- **No audio devices listed** — check that the app has microphone permission (Settings → Privacy & security → Microphone).
