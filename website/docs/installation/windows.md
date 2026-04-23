---
id: windows
title: Windows
sidebar_position: 4
description: Install RFWhisper on Windows 10/11 with VB-Cable and DirectML acceleration.
---

# Install on Windows

Tested on Windows 10 22H2 and Windows 11.

## 1. Install Python 3.12

Grab the official installer: <https://www.python.org/downloads/windows/>. **Check "Add Python to PATH"** during setup.

Verify:

```powershell
python --version
# Python 3.12.x
```

## 2. Install Git + Git LFS

<https://git-scm.com/download/win> and <https://git-lfs.github.com/>.

## 3. Install VB-Cable (virtual audio cable)

1. Download VB-Cable from <https://vb-audio.com/Cable/>.
2. Run as administrator. **Reboot** after install (required).
3. You will now see **CABLE Input / Output** devices in the Sound panel.

For multi-application routing, [VB-Audio Voicemeeter](https://vb-audio.com/Voicemeeter/) is recommended but optional.

## 4. (Optional) GNU Radio

The official [GNU Radio Windows installer](https://www.gnuradio.org/download/) (3.10.x) is the simplest route. Make sure to add GR's Python to your PATH if you want to run flowgraphs from the same shell as RFWhisper.

## 5. RFWhisper itself

```powershell
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -U pip wheel
pip install -e ".[audio]"
python -m rfwhisper.models.fetch
rfwhisper doctor
```

## 6. DirectML acceleration (recommended)

Most modern Windows machines have a GPU that DirectML can use. Install the DirectML-enabled ONNX Runtime:

```powershell
pip install onnxruntime-directml
```

Then:

```powershell
rfwhisper info providers
# DmlExecutionProvider    (preferred)
# CPUExecutionProvider    (fallback)
```

For NVIDIA GPUs with CUDA already installed:

```powershell
pip install onnxruntime-gpu
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

- **`error: Microsoft Visual C++ 14.0 or greater is required`** — install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload.
- **`ImportError: DLL load failed while importing onnxruntime_pybind11_state`** — install [Visual C++ Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist).
- **VB-Cable audio crackling** — set VB-Cable's internal rate to **48000 Hz** in its control panel.
- **PowerShell script-execution policy blocks `Activate.ps1`** — `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`.
