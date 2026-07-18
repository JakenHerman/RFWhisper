# Virtual cable setup

Getting RFWhisper's output into WSJT-X is the single biggest friction point for new users.
A ham who can't route audio can't evaluate the denoiser at all, so this page is held to a
hard target — **criterion A8: a beginner gets audio into WSJT-X in ten minutes or less.**

If it takes you longer than that, that's a bug in this page. Please
[open an issue](https://github.com/JakenHerman/RFWhisper/issues/new) and say where you got
stuck.

## What you're building

```
   radio / SDR                RFWhisper                virtual cable            WSJT-X
   ───────────  ──audio in──▶ denoise-live ──audio out──▶ CABLE / ──input──▶  decodes
                                                          BlackHole /
                                                          loopback
```

Three device selections, and that's the whole job:

1. RFWhisper **reads** from your radio's audio interface.
2. RFWhisper **writes** to the virtual cable.
3. WSJT-X **reads** from the virtual cable.

## Step 0 — find your device numbers (all platforms)

```console
$ rfwhisper audio list
```

Every device gets an index. **`--in` and `--out` take those integers**, not device names:

```console
$ rfwhisper denoise-live --in 3 --out 7
```

Write down the index of your radio's input and of the virtual cable's *input* side. You'll
need both.

:::note
v0.1 has no `--profile` flag. Mode profiles (`ssb`, `cw`, `ft8`, …) arrive in v0.3 — see
the [roadmap](../ROADMAP.md). If you find a command with `--profile` in it anywhere, it's
stale; please report it.
:::

---

## Windows — VB-Cable

**Time: about 5 minutes, plus a reboot.**

1. Download [VB-Cable](https://vb-audio.com/Cable/) (free donationware) and run the
   installer **as Administrator**.
2. **Reboot.** VB-Cable installs a kernel audio device and will not appear until you do.
3. Find your indices:
   ```console
   $ rfwhisper audio list
   ```
   You're looking for **CABLE Input (VB-Audio Virtual Cable)** — that's what RFWhisper
   writes *to*.
4. Start RFWhisper:
   ```console
   $ rfwhisper denoise-live --in <radio index> --out <CABLE Input index>
   ```
5. In **WSJT-X → File → Settings → Audio**, set **Input** to
   **CABLE Output (VB-Audio Virtual Cable)**.

   The naming trips everyone up once: RFWhisper writes to CABLE **Input**, WSJT-X reads
   from CABLE **Output**. They're two ends of the same cable.
6. To hear it yourself as well: **Sound Settings → More sound settings → Recording →
   CABLE Output → Properties → Listen → Listen to this device →** pick your speakers.

### Multi-app routing

For more than one listener at once (WSJT-X *and* JS8Call *and* speakers), install
[Voicemeeter Banana](https://vb-audio.com/Voicemeeter/banana.htm). RFWhisper writes to a
Voicemeeter virtual input; each app reads from its own Voicemeeter output.

---

## macOS — BlackHole

**Time: about 5 minutes.**

1. ```console
   $ brew install blackhole-2ch
   ```
   Or download from [existential.audio/blackhole](https://existential.audio/blackhole/).
2. Find your indices:
   ```console
   $ rfwhisper audio list
   ```
3. Start RFWhisper:
   ```console
   $ rfwhisper denoise-live --in <radio index> --out <BlackHole 2ch index>
   ```
4. In **WSJT-X → Settings → Audio**, set **Input** to **BlackHole 2ch**.

### Hearing it while WSJT-X decodes

BlackHole is a *sink* — audio going into it doesn't reach your speakers. To monitor:

1. Open **Audio MIDI Setup** (Applications → Utilities).
2. **+ → Create Multi-Output Device.**
3. Tick both **BlackHole 2ch** and your speakers or DAC.
4. Rename it *RFW-Listen*, and select it as RFWhisper's output device.

Put your real output device **first** in the multi-output list — macOS uses the first
device as the clock master, and a virtual device as master is a common source of drift.

### Running two pipelines

Install the 16-channel variant if you want independent denoisers running side by side
(HF SSB and VHF FM, say).

---

## Linux — PipeWire (recommended)

**Time: about 3 minutes on any modern distro.**

Create a virtual sink whose output shows up as a capture source:

```bash
pw-loopback \
  --capture-props='media.class=Audio/Sink' \
  --playback-props='media.class=Audio/Source' &
```

Then:

```console
$ rfwhisper audio list
$ rfwhisper denoise-live --in <radio index> --out <loopback index>
```

WSJT-X sees the loopback as an ordinary input device.

To make it permanent, add a `pw-loopback` stanza under
`~/.config/pipewire/pipewire.conf.d/` rather than running it by hand each boot.

## Linux — ALSA `snd-aloop`

No PipeWire, or you want the lightest possible option:

```bash
sudo modprobe snd-aloop enable=1 index=10
```

This creates a symmetric pair: whatever is played to `Loopback,0` is captured from
`Loopback,1`. Point RFWhisper at the playback half and WSJT-X at the capture half — use
`rfwhisper audio list` to get the indices.

Persist it with:

```bash
echo snd-aloop | sudo tee /etc/modules-load.d/snd-aloop.conf
```

## Linux — JACK (appendix)

If you're already running a JACK realtime setup:

```bash
sudo apt install qjackctl jackd2 pulseaudio-module-jack
```

Start QjackCtl, then wire RFWhisper's output port to your decoder's input port in the
Connections pane. PortAudio picks up the JACK backend automatically when JACK is running.

---

## Troubleshooting

### WSJT-X shows no audio at all

Almost always a **sample-rate mismatch**. RFWhisper runs at 48 kHz and does not negotiate.

- **Windows:** Sound Settings → CABLE Output → Properties → Advanced → set
  **2 channel, 16 bit, 48000 Hz**. Do the same on CABLE Input.
- **macOS:** Audio MIDI Setup → BlackHole 2ch → Format → **48 000 Hz**.
- **Linux:** `pw-metadata -n settings 0 clock.force-rate 48000` for PipeWire.

Also confirm you picked the *other* end of the cable in WSJT-X — CABLE **Output**, not
CABLE Input.

### Crackling, stuttering, or dropouts

The block size is too small for your OS audio stack. Raise it:

```console
$ rfwhisper denoise-live --in 3 --out 7 --blocksize 960
```

`--blocksize` is in samples at 48 kHz, so 480 is 10 ms and 960 is 20 ms. Larger is more
robust and adds latency in exact proportion — 960 costs you an extra 10 ms against the
100 ms budget. If 960 doesn't settle it, something else on the machine is competing for
realtime priority; a browser with hardware acceleration is the usual suspect.

### You hear both the raw and the denoised audio at once

Something is monitoring the *source* as well as the cable. Check, in order:

1. Windows "Listen to this device" enabled on the **radio's** input rather than on CABLE
   Output.
2. Your radio's own USB audio playthrough or sidetone monitor.
3. A DAW or mixer app left running with the input armed.

### Windows: the device disappears or RFWhisper errors on start

Another application has the device in **exclusive mode**. Sound Settings → the device →
Properties → Advanced → untick **Allow applications to take exclusive control of this
device**. WSJT-X, SDR console software, and some VoIP clients all do this.

### `rfwhisper audio list` shows nothing

The `[audio]` extra isn't installed, or PortAudio isn't present on the system:

```console
$ pip install -e '.[audio]'
```

On bare Linux containers you also need the system library:
`sudo apt install libportaudio2`.

### Index numbers changed after a reboot

They can — PortAudio indices aren't stable across device plug/unplug or driver updates.
Re-run `rfwhisper audio list` and use the new numbers.

---

## Related

- [Virtual cable comparison](../website/docs/hardware/virtual-cables.md) — picking between
  VB-Cable, BlackHole, PipeWire, JACK, and `snd-aloop`
- [Report JSON schema](./reports.md) — what `rfwhisper denoise` writes
- [Models](./models.md) — fetching and swapping model artifacts
- ROADMAP criterion A8

## Not covered here

- ASIO setup for broadcast-grade interfaces — see your card's own documentation
- Full WSJT-X / fldigi configuration — see the
  [WSJT-X user guide](https://wsjt.sourceforge.io/wsjtx-doc/wsjtx-main-2.6.1.html)
