---
id: windows
title: Windows
sidebar_position: 4
description: Install RFWhisper on Windows 10/11 with VB-Cable.
---

# Install on Windows

Built and unit-tested in CI on Windows Server 2022 (the Windows 10/11 code base). Live use on Windows has not been verified for the beta yet; reports welcome.

## 1. Install Rust

Grab rustup from <https://rustup.rs/> and run the installer (accept the defaults; it pulls the MSVC toolchain — if prompted, let it install the Visual Studio C++ Build Tools).

Verify:

```powershell
cargo --version
# cargo 1.7x.x
```

## 2. Install Git

<https://git-scm.com/download/win>.

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
cargo build --release        # a few minutes the first time: builds DeepFilterNet3 in
.\target\release\rfwhisper.exe --version
.\target\release\rfwhisper.exe audio list
```

Copy `target\release\rfwhisper.exe` anywhere on your `PATH`. It's a single
self-contained binary with the DeepFilterNet3 model inside: nothing to download,
no network at runtime.

## 6. Performance

The beta runs DeepFilterNet3 on the CPU (no GPU or DirectML needed); any
quad-core laptop from the last several years has ample headroom.

## 7. Routing audio

| From | To | How |
|---|---|---|
| Rig USB CODEC (IC-7300, etc.) | RFWhisper input | Choose **USB Audio CODEC** in `rfwhisper audio list` |
| RFWhisper output | WSJT-X / fldigi input | `--out "CABLE Input"`; set **CABLE Output** as WSJT-X's input |
| Also hear it yourself | Speakers | In Sound Settings, enable *Listen to this device* on **CABLE Output**, forward to Speakers |

Example:

```powershell
rfwhisper denoise-live --in "USB Audio CODEC" --out "CABLE Input"
```

Full walkthrough: [Virtual audio cables → Windows](../hardware/virtual-cables#windows--vb-cable).

## Troubleshooting

- **`error: linker 'link.exe' not found`** — install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload (rustup offers this during install).
- **VB-Cable audio crackling** — set VB-Cable's internal rate to **48000 Hz** in its control panel.
- **No audio devices listed / silent input**: allow desktop apps to use the microphone (Settings → Privacy & security → Microphone → *Let desktop apps access your microphone*).
- **`could not open … device`**: another app holds it in exclusive mode. Sound settings → device → Properties → Advanced → untick the two *exclusive mode* boxes.
