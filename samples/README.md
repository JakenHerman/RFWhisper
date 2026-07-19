# RFWhisper audio samples

Real recordings that the denoiser is demonstrated and measured against. The
**synthetic** fixtures (`rfwhisper samples synth`) live in code and need no files;
this directory is for **real audio** — the clips a human listens to, and the clean
references the acceptance gates measure against.

> **Why this exists.** DeepFilterNet3 is a *speech* model. It correctly deletes
> our synthetic tone-stack "speech" as non-speech (measured −5.5 dB on a synthetic
> mix vs +0.86 dB on real speech). So a convincing before/after demo — and the A1
> SNR-gain gate — **cannot** run on `samples synth` output. They need real audio.
> See [#117](https://github.com/JakenHerman/RFWhisper/issues/117) and the A1 note
> on [#20](https://github.com/JakenHerman/RFWhisper/issues/20).

## Layout

| Directory | What goes here | Has a clean reference? | Used for |
|---|---|---|---|
| `clean/`  | Dry, quiet source speech/audio | is the reference | mixing into `mixed/`, A1 |
| `noise/`  | Real recorded RFI / band noise (powerline, inverter, static) | n/a | mixing, realism |
| `mixed/`  | A noisy clip **plus** its aligned clean reference | yes (`*.clean.wav`) | A1 gate (measurable gain) |
| `onair/`  | Real off-air captures of live QSOs | no | the "listen to this" demo, ABX |

The distinction that matters:

- **`onair/` clips** are the demo. Real signal, real noise, no clean reference —
  you judge them by ear and by the before/after spectrogram. `snr_gain_db` is
  `null` for these because there is nothing to correlate against.
- **`mixed/` pairs** are the gate. Because the clean source is known and sample-
  aligned, `rfwhisper denoise --reference` can report an exact SNR gain, which is
  what A1 asserts on. The realistic way to build one: take a `clean/` speech clip
  and a `noise/` capture and mix them at a known SNR (see "Making a mixed pair").

## File format

- **Container:** WAV (`.wav`).
- **Channels:** mono. Split a stereo capture to one channel first — `rfwhisper
  denoise` rejects multi-channel input by design.
- **Sample rate:** 48 kHz preferred (DeepFilterNet3's native rate; anything else is
  resampled internally, so avoid it for reference material). Never below 8 kHz.
- **Bit depth:** 16-bit PCM or 32-bit float. Peak below full scale (leave ~1 dB of
  headroom); clipped input confuses the denoiser.
- **Length:** 6–30 s is plenty for a demo clip. Longer is fine for `onair/`.
- **Loudness:** don't normalize a `mixed/` pair after mixing — it changes the SNR
  you set. `clean/` and `noise/` source material should be peak-normalized.

## Naming

```
<mode>_<band-or-tag>_<callsign-or-source>_<seq>.wav
```

- `mode`: `ssb` | `cw` | `ft8` | `am` | `fm` | `voice`
- Examples:
  - `onair/ssb_40m_k0abc_01.wav`
  - `clean/voice_studio_n0xyz_01.wav`
  - `noise/powerline_shack_n0xyz_01.wav`
  - `mixed/ssb_powerline_s3_s7_01.wav` + `mixed/ssb_powerline_s3_s7_01.clean.wav`

A `mixed/` noisy file `foo.wav` pairs with its reference `foo.clean.wav` — the same
convention `samples synth --clean-out` writes, so the tooling already understands it.

## Metadata (required)

Every committed clip needs a sidecar `<clip>.meta.json` next to it **and** an entry
in [`ATTRIBUTION.md`](./ATTRIBUTION.md). The sidecar:

```json
{
  "file": "onair/ssb_40m_k0abc_01.wav",
  "mode": "ssb",
  "band": "40m",
  "frequency_khz": 7185,
  "captured_utc": "2026-07-19T14:32:00Z",
  "receiver": "RTL-SDR v4 + SDR++",
  "antenna": "40m EFHW at 9 m",
  "rfi_environment": "suburban, VDSL + LED lighting hash, ~S6 floor",
  "source": "own-transmission | consented | licensed",
  "license": "CC-BY-4.0",
  "contributor": "N0XYZ",
  "notes": "weak DX under powerline buzz; good A/B candidate"
}
```

`frequency_khz`, `antenna`, and `notes` are optional; everything else is required.

## Licensing and consent — read before committing a QSO

Recording radio is easy; **publishing** someone else's transmission is not always
yours to do. Rules vary by country, and this project ships worldwide. To keep every
clip unambiguously OK to redistribute under the repo's license:

1. **Prefer your own transmissions.** A clip of *your* station, or a QSO where the
   other operator gave consent, is the safe default. Record `source: own-transmission`.
2. **Get consent for identifiable third parties.** If another operator's callsign or
   voice is present, get their OK and note it (`source: consented`). When in doubt,
   ask on the QSO or bleep/trim the callsign.
3. **Some jurisdictions restrict divulging the *content* of communications.** If
   your country does, don't publish intelligible third-party content without consent
   — use your own transmission or a `clean/` + `noise/` mix instead.
4. **Each clip must carry a redistributable license** (`CC-BY-4.0` or `CC0-1.0`
   recommended) recorded in the sidecar and ATTRIBUTION. No license → not merged.

Noise-only captures (`noise/`) carry no communication content and are the easiest
to contribute — a recording of your own shack's RFI floor with no signal present.

## Making a `mixed/` pair (for the A1 gate)

A1 needs a clean reference, which on-air audio doesn't have. Build one from real
parts:

```bash
# 1. A dry, quiet speech clip -> clean/voice_studio_n0xyz_01.wav
# 2. A real shack-noise capture -> noise/powerline_shack_n0xyz_01.wav
# (Same length, same 48 kHz. Trim to match.)

# Mixing at a known SNR with real components is a planned `samples synth` mode
# (--clean-file / --noise-file); until it lands, mix in your DAW/sox at a measured
# SNR and write the pair as foo.wav + foo.clean.wav.
```

Then it runs like any fixture:

```bash
rfwhisper denoise \
  --input     mixed/ssb_powerline_s3_s7_01.wav \
  --output    /tmp/out.wav \
  --reference mixed/ssb_powerline_s3_s7_01.clean.wav \
  --model     deepfilternet3 \
  --spectrogram /tmp/report.html   # (build with --features dfn for the real model)
```

`report.html` is the shareable before/after ([#115](https://github.com/JakenHerman/RFWhisper/issues/115)).

## Running a demo clip (no reference)

```bash
rfwhisper denoise -i onair/ssb_40m_k0abc_01.wav -o /tmp/clean.wav \
  --model deepfilternet3 --spectrogram /tmp/report.html
# snr_gain_db is null (no reference); judge by ear + the spectrogram panels.
```

## Git LFS

Audio is tracked with [Git LFS](https://git-lfs.com) via `.gitattributes`
(`samples/**/*.wav`). Install LFS once (`git lfs install`) before adding clips.
The synthetic generators are code, not files, and are **not** tracked here.

Keep the directory lean: a few representative clips per mode, not a corpus. The
large-corpus dataset generator is a later milestone (v0.5).
