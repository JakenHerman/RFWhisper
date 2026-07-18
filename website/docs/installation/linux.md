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
  git git-lfs \
  libasound2-dev \
  ffmpeg build-essential pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust toolchain
```

### Fedora

```bash
sudo dnf install -y git git-lfs alsa-lib-devel ffmpeg gcc pkgconfig
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Arch

```bash
sudo pacman -S --needed rustup git git-lfs alsa-lib ffmpeg base-devel
rustup default stable
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
cargo build --release
sudo install -m755 target/release/rfwhisper /usr/local/bin/
rfwhisper models fetch
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

- **`no default device` from `rfwhisper audio list`** — install `pipewire-pulse` or `pulseaudio-utils`, then reboot.
- **Underruns / xruns** — confirm you're in the `realtime` group (`id | grep realtime`) and that `cpufreq` governor isn't `powersave` during use.
- **Build fails on `alsa-sys`** — install the ALSA headers: `sudo apt install libasound2-dev` (or your distro's equivalent).
