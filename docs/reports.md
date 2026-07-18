# Report JSON schema

`rfwhisper denoise` writes a JSON report to stdout, and to `--report <path>` if given.
The schema is **stable for all of v0.1** — the acceptance gates and the docs parse it, so
adding, removing, or renaming a key is a breaking change that needs its own PR.

## Keys

| Key                 | Type            | Notes                                                        |
| ------------------- | --------------- | ------------------------------------------------------------ |
| `model`             | string          | Resolved model name, after `RFWHISPER_FORCE_STUB` is applied |
| `input`             | string          | Source path as given on the command line                     |
| `output`            | string          | Destination path                                             |
| `sr`                | int             | Sample rate in Hz of both input and output                   |
| `duration_s`        | float           | Input duration in seconds                                    |
| `inference_time_ms` | float           | Wall-clock time in the model, milliseconds                   |
| `rtf`               | float           | Real-time factor — `inference_time / duration` (A5 wants < 0.5) |
| `snr_gain_db`       | float \| null   | Effective SNR gain (A1); null unless `--reference` was passed |
| `spectrogram_path`  | string \| null  | Reserved; always null in v0.1 (`--spectrogram` is v0.4)      |

Keys are always present. A value that could not be computed is `null` rather than absent,
so consumers can index without guarding.

## Example

```console
$ rfwhisper denoise -i noisy.wav -o clean.wav --reference truth.wav
```

```json
{
  "model": "deepfilternet3",
  "input": "noisy.wav",
  "output": "clean.wav",
  "sr": 48000,
  "duration_s": 60.0,
  "inference_time_ms": 8421.5,
  "rtf": 0.140358,
  "snr_gain_db": 4.812,
  "spectrogram_path": null
}
```

## When `snr_gain_db` stays null

- `--reference` was not passed.
- The reference is at a different sample rate than the input.
- The reference and input do not overlap enough to align.

Each case prints a message to **stderr** and leaves the run successful — a bad reference
costs you the metric, not the denoised audio. stdout stays pure JSON so it can be piped.

## Exit codes

| Code | Meaning                                            |
| ---- | -------------------------------------------------- |
| `0`  | Success                                            |
| `2`  | Unsupported input (multi-channel, empty, unreadable) |
| `3`  | Model load failure                                 |

## Related

- ROADMAP A1 (SNR gain), A5 (RTF)
- `rfwhisper/dsp/metrics.py` — where `snr_gain_db` and `rtf` come from
