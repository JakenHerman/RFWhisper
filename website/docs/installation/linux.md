---
id: linux
title: Linux
sidebar_position: 2
description: Install RFWhisper on Ubuntu, Debian, Fedora, Arch, or any modern Linux distro.
---

# Install on Linux

Built and unit-tested in CI on Ubuntu 22.04. Other modern distros should work the same way; reports welcome.

## 1. System dependencies

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
  git libasound2-dev build-essential pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust toolchain
```

### Fedora

```bash
sudo dnf install -y git alsa-lib-devel gcc pkgconfig
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Arch

```bash
sudo pacman -S --needed rustup git alsa-lib base-devel
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

To feed WSJT-X / fldigi, create a PipeWire null sink or load `snd-aloop`. Step by
step: [Virtual audio cables → Linux](../hardware/virtual-cables#linux--pipewire-ubuntu-2210-fedora-34-most-current-distros).

## 4. RFWhisper itself

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release        # a few minutes the first time: builds DeepFilterNet3 in
sudo install -m755 target/release/rfwhisper /usr/local/bin/
rfwhisper --version          # rfwhisper 0.1.0-beta.1
rfwhisper audio list         # your sound devices
```

The DeepFilterNet3 model is bundled in the binary: there is nothing to download
and no network access at runtime.

## 5. Lower latency (optional)

The beta does not request realtime scheduling itself. On a busy machine you can
still help it:

- Use the `performance` CPU governor while operating.
- With PipeWire, a smaller quantum lowers device latency:
  `pw-metadata -n settings 0 clock.force-quantum 256` (reset with `0`).
- If `underruns` climb in the status line, try `--blocksize 960`.

Realtime thread priority (SCHED_FIFO) is planned; see the
[latency budget](../architecture/latency-budget).

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

- **`no default input device`**: pick one explicitly with `--in` (number or name from `rfwhisper audio list`), or install `pipewire-alsa` so ALSA's `default` routes through PipeWire.
- **Underruns / xruns**: check the CPU governor isn't `powersave`, close heavy apps, and try `--blocksize 960`.
- **Build fails on `alsa-sys`** — install the ALSA headers: `sudo apt install libasound2-dev` (or your distro's equivalent).
