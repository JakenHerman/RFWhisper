---
id: virtual-cables
title: Virtual Audio Cables
sidebar_position: 4
description: VB-Cable, BlackHole, JACK, PipeWire, snd-aloop — picking and configuring the right virtual cable.
---

# Virtual Audio Cables

RFWhisper emits clean audio to a *virtual audio cable*. WSJT-X, fldigi, JS8Call, your rig's logging software, or your headphones then read from that cable.

## Picking one

| OS | Recommendation |
|---|---|
| **Windows** | [VB-Cable](https://vb-audio.com/Cable/) (free donationware). Add Voicemeeter if you want multi-app routing. |
| **macOS** | [BlackHole 2ch](https://existential.audio/blackhole/) (`brew install blackhole-2ch`). |
| **Linux (PipeWire)** | Native loopback via `pw-loopback`. |
| **Linux (JACK)** | qjackctl + JACK modules. |
| **Linux (classic ALSA)** | `sudo modprobe snd-aloop`. |

## Windows · VB-Cable

1. Download from <https://vb-audio.com/Cable/>. Run installer **as Administrator**.
2. **Reboot** (VB-Cable installs a kernel audio device).
3. In Sound settings, set **CABLE Input** as RFWhisper's output device.
4. In WSJT-X → Settings → Audio → Input: pick **CABLE Output**.
5. To hear it yourself: in Sound Settings → **CABLE Output → Properties → Listen → Listen to this device → Speakers**.

### Multi-app routing

For more than one listener app (WSJT-X + JS8Call + speakers), install [VB-Voicemeeter Banana](https://vb-audio.com/Voicemeeter/banana.htm) and use its virtual I/O bus. RFWhisper emits to a Voicemeeter input; each app reads from a dedicated Voicemeeter output.

## macOS · BlackHole

1. `brew install blackhole-2ch` (or download from the site).
2. Open **Audio MIDI Setup**.
3. **+ → Create Multi-Output Device**, check **BlackHole 2ch** and **MacBook Pro Speakers** (or your external DAC).
4. Rename to *RFW-Listen*.
5. Select **RFW-Listen** as the system output when you want to hear RFWhisper through your speakers as well.
6. In RFWhisper: `--out "BlackHole 2ch"`.
7. In WSJT-X: Input = **BlackHole 2ch**.

### BlackHole 16ch

Install the 16-channel variant if you want to run multiple independent RFWhisper pipelines simultaneously (e.g. HF SSB and VHF FM denoisers side by side).

## Linux · PipeWire

Modern distros ship PipeWire. Create a virtual sink:

```bash
pw-loopback \
  --capture-props='media.class=Audio/Sink'   \
  --playback-props='media.class=Audio/Source' \
  &
```

Then:

```bash
rfwhisper denoise-live --in hw:0,0 --out "loopback" --profile ssb
```

WSJT-X will see the loopback as a normal input.

## Linux · JACK

1. `sudo apt install qjackctl jackd2 pulseaudio-module-jack`
2. Start QjackCtl → Start.
3. Create a virtual output port (*RFW → WSJT-X*).
4. RFWhisper uses PortAudio's JACK backend automatically if JACK is running.

## Linux · snd-aloop

Lightweight, zero dependencies:

```bash
sudo modprobe snd-aloop enable=1 index=10
# Now two devices: Loopback,0 (playback) and Loopback,1 (capture) — symmetric

rfwhisper denoise-live --in hw:0,0 --out hw:10,0,0 --profile ssb
# WSJT-X input = hw:10,1,0
```

Persist across reboots by adding `snd-aloop` to `/etc/modules-load.d/snd-aloop.conf`.

## Why not just use the system default?

Because:

1. You'd lose the ability to hear raw audio simultaneously (for A/B).
2. The denoise → decoder chain would fight whatever system mixer is doing in the background.
3. Many decoder apps only accept a specific input device, not the system default.

A virtual cable is 2 minutes of setup and you get it back tenfold in reliability.

## Troubleshooting

- **Silent output** — check sample rate of the virtual cable matches RFWhisper (48 kHz).
- **Crackles / dropouts** — raise blocksize (`--blocksize 960`), confirm nothing else on the machine is grabbing real-time scheduler priority.
- **Double audio** — one of the listeners has loopback monitoring enabled; turn it off in that app's settings.
