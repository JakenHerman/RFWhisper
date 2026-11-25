---
id: virtual-cables
title: Virtual Audio Cables
sidebar_position: 4
description: Route denoised audio into WSJT-X, fldigi, or JS8Call with VB-Cable, BlackHole, or a PipeWire / ALSA loopback.
---

# Virtual Audio Cables

A *virtual audio cable* is a fake sound card: whatever one program plays into it,
another program can record from it. RFWhisper plays denoised audio into the cable,
and WSJT-X, fldigi, JS8Call, or your logger record from it as if it were your rig.

```text
rig USB codec ──▶ rfwhisper denoise-live ──▶ virtual cable ──▶ WSJT-X / fldigi / JS8Call
   (--in)                                      (--out)          (their audio input)
```

If you only want to *listen* through headphones, you don't need a cable at all:
leave `--out` off and RFWhisper plays to your default output.

:::caution Digital modes and CW in the beta
The beta's DeepFilterNet3 model is a speech model and removes FT8/FT4/RTTY/PSK
tones and CW along with the noise. When feeding a decoder, run with
`--model passthrough` for now. The routing below is the same either way. Mode
profiles that are safe for digital modes are on the v0.3 roadmap.
:::

## Picking one

| OS | Use | Cost |
|---|---|---|
| **Windows** | [VB-Cable](https://vb-audio.com/Cable/) | Free (donationware) |
| **macOS** | [BlackHole 2ch](https://existential.audio/blackhole/) | Free |
| **Linux (PipeWire, most modern distros)** | A null sink via `pactl` | Built in |
| **Linux (plain ALSA)** | `snd-aloop` kernel module | Built in |

RFWhisper resamples automatically, so the cable's sample rate doesn't have to
match your rig's. 48 kHz is still the best choice for the cable.

## Windows · VB-Cable

1. Download VB-Cable from <https://vb-audio.com/Cable/>, unzip, and run
   `VBCABLE_Setup_x64.exe` **as Administrator**. Reboot when it asks.
2. Find the names RFWhisper sees:

   ```powershell
   rfwhisper audio list
   ```

   You'll see `CABLE Input (VB-Audio Virtual Cable)` (the side you *play into*)
   and `CABLE Output (VB-Audio Virtual Cable)` (the side apps *record from*).
3. Start RFWhisper, playing into the cable:

   ```powershell
   rfwhisper denoise-live --in "USB Audio CODEC" --out "CABLE Input"
   ```

4. In WSJT-X: **File → Settings → Audio → Input** = `CABLE Output`. In fldigi:
   **Configure → Sound Card → Audio → Capture** = `CABLE Output`.
5. To hear it yourself as well: **Sound settings → More sound settings →
   Recording → CABLE Output → Properties → Listen → Listen to this device →**
   your speakers.

**More than one app?** VB-Cable is one cable. For WSJT-X and JS8Call at the same
time, either let both record from `CABLE Output` (Windows allows shared
capture) or install [VoiceMeeter](https://vb-audio.com/Voicemeeter/) for more
virtual buses.

## macOS · BlackHole

1. Install: `brew install blackhole-2ch` (or the installer from the site). No
   reboot needed.
2. Start RFWhisper, playing into BlackHole:

   ```bash
   rfwhisper denoise-live --in "USB Audio CODEC" --out "BlackHole"
   ```

3. In WSJT-X / fldigi / JS8Call, set the audio **input** to `BlackHole 2ch`.
4. The first time, macOS asks whether your terminal may use the microphone.
   Allow it, or the input is silent (System Settings → Privacy & Security →
   Microphone).

**To hear it yourself as well:** open **Audio MIDI Setup**, click **+ → Create
Multi-Output Device**, tick **BlackHole 2ch** and your speakers or headphones,
and rename it `RFW Listen`. Then run with `--out "RFW Listen"`: the decoder
records from BlackHole and you hear the same audio.

## Linux · PipeWire (Ubuntu 22.10+, Fedora 34+, most current distros)

1. Create a virtual sink named `rfwhisper`:

   ```bash
   pactl load-module module-null-sink sink_name=rfwhisper \
     sink_properties=device.description=RFWhisper
   ```

2. RFWhisper talks to ALSA, which PipeWire routes. Run it on the default
   device and send its output to the sink:

   ```bash
   rfwhisper audio list
   rfwhisper denoise-live --in "USB Audio CODEC" --out pipewire
   ```

   Then, in `pavucontrol` → **Playback**, move RFWhisper's stream to
   **RFWhisper**. PipeWire remembers the choice next time.
3. In WSJT-X / fldigi, choose the input **Monitor of RFWhisper** (in
   `pavucontrol` → **Recording**, if the app only offers "default").

To make the sink permanent, put the `load-module` line in
`~/.config/pipewire/pipewire-pulse.conf.d/rfwhisper.conf` (PipeWire) or
`~/.config/pulse/default.pa` (PulseAudio).

## Linux · snd-aloop (plain ALSA, no sound server)

Lightweight and dependency-free; good for a headless Raspberry Pi.

```bash
sudo modprobe snd-aloop
rfwhisper audio list      # look for devices with "Loopback" in the name
```

`snd-aloop` creates a card called `Loopback` with two devices that are wired to
each other: what is played to device 0 can be recorded from device 1. Pick the
device-0 entry as RFWhisper's output (by its number, or by a name fragment such
as `--out "CARD=Loopback,DEV=0"`), and set the decoder's input to the matching
device-1 entry (`hw:Loopback,1` in WSJT-X).

Load it on every boot:

```bash
echo snd-aloop | sudo tee /etc/modules-load.d/snd-aloop.conf
```

JACK users: the beta's audio layer talks to ALSA (and to JACK through PipeWire's
JACK/ALSA bridges), not to a JACK server directly.

## Check the route in two minutes

1. Start RFWhisper with your rig as input and the cable as output.
2. Tune the rig to a busy SSB frequency (or play a voice recording into the
   rig's input with a cable).
3. Watch the status line: `in-drops`, `underruns`, and `drift-drops` should
   stay at 0.
4. In the decoder app, the audio level meter should move. In WSJT-X, aim for
   roughly 30 dB on the meter by adjusting the rig's USB audio level, not
   RFWhisper.
5. Type `b` + Enter: the level and sound should change between raw and
   denoised, with no gap.

## Why not just use the system default?

1. Decoder apps need a specific input device, and the "default" input is your
   microphone.
2. The system mixer can resample, duck, or add effects behind your back.
3. You lose the ability to listen to raw and denoised audio independently.

## Troubleshooting

| Symptom | Fix |
|---|---|
| Decoder sees no signal | Make sure RFWhisper plays into the cable's *input/playback* side (`CABLE Input`, `BlackHole 2ch`, the sink) and the app records from the *output/monitor* side |
| Silent input on macOS | Grant microphone access to your terminal (Privacy & Security → Microphone) |
| `could not open output device …` (Windows) | Another app holds the cable in exclusive mode: Sound settings → device → Properties → Advanced → untick both "exclusive mode" boxes |
| Crackles, `underruns` climbing | Close CPU-heavy apps and try `--blocksize 960` |
| Hear yourself twice / echo | Turn off "Listen to this device" or the multi-output device when you don't need it |
| FT8 decodes drop with RFWhisper on | Expected with the beta speech model; use `--model passthrough` for digital modes |
