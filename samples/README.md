# Samples

Audio fixtures the acceptance gates run against.

Two kinds live here, and they are not interchangeable:

| Kind          | Where it comes from                  | Committed? | Used by                        |
| ------------- | ------------------------------------ | ---------- | ------------------------------ |
| **Synthetic** | `rfwhisper/samples/synth.py`, on demand | No      | CI gates (A1, A3, A6)          |
| **On-air**    | Real receivers, contributed by hams  | Yes, via Git LFS | Human testable-success demos |

CI runs on synthetic noise only. That is deliberate: the generators are pure functions of
their seed, so a gate failure always means the *denoiser* changed, never that a fixture
drifted or a download flaked. Real clips are what convince a human the thing works on the
air — they are for the demos in the release notes, not for threshold assertions.

## Synthetic fixtures

Nothing to download. Generate what you need:

```python
from rfwhisper.samples.synth import mix, powerline_buzz

noise = powerline_buzz(sr=48_000, duration_s=10.0, seed=42)
noisy = mix(clean, noise, snr_db=-6.0)
```

Available generators — all deterministic on `seed`, all returning peak-normalised float64:

- `powerline_buzz(sr, duration_s, fundamental_hz=60, n_harmonics=30, seed=0)`
- `solar_inverter(sr, duration_s, tick_rate_hz=120, ringing_q=50, seed=0)`
- `vdsl_hash(sr, duration_s, seed=0)`
- `atmospheric_qrn(sr, duration_s, crash_rate_hz=2, seed=0)`
- `mix(clean, noise, snr_db)` — rescales the *noise* to hit an exact SNR, leaving your
  clean reference sample-aligned so `dsp.metrics.effective_snr_gain` can measure against it

For 50 Hz mains regions, pass `fundamental_hz=50.0`.

## Directory layout

```
samples/
  clean/    ssb_*.wav  cw_*.wav  ft8_*.wav  vhf_fm_*.wav
  noise/    powerline_*.wav  inverter_*.wav  vdsl_*.wav  qrn_*.wav
  mixed/    ssb_powerline_s3_s7.wav  ...
```

`clean/` holds signal with no meaningful noise; `noise/` holds noise with no signal;
`mixed/` holds pre-rendered combinations for demos where the exact clip matters.

## Contributing a clip

Real recordings are welcome and genuinely useful. Requirements:

1. **You must have the right to license it.** Off-air recordings of amateur
   transmissions: get the other operator's OK before contributing a QSO. Do not
   contribute recordings of non-amateur services.
2. **Licensed CC BY 4.0 or CC0.** Noted per clip in `ATTRIBUTION.md`.
3. **48 kHz mono WAV**, 16- or 24-bit, no processing — no AGC ride, no noise reduction,
   no normalisation. We need the raw article.
4. **60 seconds or less** unless there is a reason; keep the repo small.
5. **Add an `ATTRIBUTION.md` entry** with every field filled in.
6. **Commit through Git LFS** — `.gitattributes` already tracks `samples/**/*.wav`.

Naming: `<mode>_<band>_<descriptor>_s<signal>_s<noise>.wav`, e.g.
`ssb_40m_powerline_s3_s7.wav` — S-meter readings for signal and noise respectively.

### Required metadata

Per clip, in `ATTRIBUTION.md`:

- Callsign of the contributor (or "anonymous" if preferred)
- Date, approximate time (UTC), and band/frequency
- Receiver and antenna
- Mode, and what the dominant noise source is believed to be
- Licence

## Non-goals

This directory is not a training corpus. A few hundred MB total is the ceiling; the
dataset generator in v0.5 is what handles scale.
