---
id: linux
title: Linux
sidebar_position: 2
description: Install RFWhisper on Ubuntu, Debian, Fedora, Arch, or any modern Linux distro.
---

# Install on Linux

Tested on Ubuntu 22.04 / 24.04, Debian 12, Fedora 39, and Arch (rolling).

## 1. System dependencies

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
  python3 python3-venv python3-pip \
  git git-lfs \
  libportaudio2 libsndfile1 libasound2-dev \
  ffmpeg build-essential cmake pkg-config
```

### Fedora

```bash
sudo dnf install -y python3 python3-virtualenv git git-lfs \
  portaudio-devel libsndfile alsa-lib-devel ffmpeg \
  gcc gcc-c++ cmake pkgconfig
```

### Arch

```bash
sudo pacman -S --needed python python-virtualenv git git-lfs \
  portaudio libsndfile alsa-lib ffmpeg base-devel cmake
```

## 2. (Optional) GNU Radio for v0.2+ flowgraphs

```bash
# Ubuntu 24.04
sudo apt install -y gnuradio gr-soapy soapysdr-module-all soapysdr-tools
# gr-dnn is not yet packaged on all distros; build from source or use our
# prebuilt wheels (ships with v0.2 release). See docs/architecture/flowgraphs.
```

## 3. Audio routing (virtual cable)

Pick one:

- **PipeWire loopback** (modern desktop — Fedora 36+, Ubuntu 22.10+):
  ```bash
  pw-loopback --capture-props='media.class=Audio/Sink' \
              --playback-props='media.class=Audio/Source' &
  ```
- **JACK** — run `qjackctl` and create a virtual input/output pair.
- **`snd-aloop`** — `sudo modprobe snd-aloop` gives you two loopback cards.

WSJT-X / fldigi will see the loopback as a regular input device.

## 4. RFWhisper itself

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
python3 -m venv .venv && source .venv/bin/activate
pip install -U pip wheel
pip install -e ".[audio]"
python -m rfwhisper.models.fetch
rfwhisper doctor
```

## 5. Realtime scheduling (recommended)

For best latency, give the audio/inference threads realtime priority:

```bash
sudo groupadd -r realtime
sudo usermod -aG realtime "$USER"
sudo tee /etc/security/limits.d/99-realtime.conf <<'EOF'
@realtime   -   rtprio   95
@realtime   -   memlock  unlimited
@realtime   -   nice     -19
EOF
```

Log out and back in. RFWhisper will auto-detect and use `SCHED_FIFO` where available.

## 6. SDR USB permissions

Most SDR dongles need a `udev` rule so you don't have to run as root:

```bash
# RTL-SDR example
sudo tee /etc/udev/rules.d/20-rtlsdr.rules <<'EOF'
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2838", GROUP="plugdev", MODE="0660"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="0bda", ATTRS{idProduct}=="2832", GROUP="plugdev", MODE="0660"
EOF
sudo udevadm control --reload-rules
sudo udevadm trigger
```

See [Hardware → SDRs](../hardware/sdrs) for other devices.

## Troubleshooting

- **`PortAudioError: no default output device`** — install `pipewire-pulse` or `pulseaudio-utils`, then reboot.
- **Underruns / xruns** — confirm you're in the `realtime` group (`id | grep realtime`) and that `cpufreq` governor isn't `powersave` during use.
- **`onnxruntime` missing CUDA provider** — install `onnxruntime-gpu` instead: `pip install onnxruntime-gpu`.
