# Changelog

All notable changes to RFWhisper. Versions follow [SemVer](https://semver.org/);
pre-1.0, minor versions may break things. Roadmap criteria (A1–A8, …) refer to
[ROADMAP.md](./ROADMAP.md).

## [0.1.0-beta.1] — 2026-11-25

First beta. The neural denoiser ships and runs in real time.

### Added

- **Real DeepFilterNet3 denoiser**, on by default. It runs upstream's Rust
  inference engine (`deep_filter` on `tract`) with the stock model bundled in
  the binary: no model download, no network at runtime (#10, #117).
- **Truly streaming `denoise-live`** (#13, #14, #17):
  - Stateful streaming engines (`DenoiseEngine::process_stream`) and a
    stateful polyphase resampler, so audio arriving in any block size is
    processed with no seams between blocks.
  - Lock-free single-producer/single-consumer rings between the audio
    callbacks and the worker thread. The callbacks never allocate or lock.
  - `--in` / `--out` accept a device number *or* part of its name;
    `--in-channel` picks the receive channel on stereo codecs; any sample
    format (f32 / i16 / u16 / i32 / …) and any rate pair.
  - **A/B bypass**: `--bypass` to start raw, `b` + Enter to toggle. The
    engine keeps running, the raw side is time-aligned with the denoised side,
    and a 5 ms crossfade keeps the switch click-free.
  - **Before/after recording** (#77): `--record-raw` / `--record-denoised`
    write a sample-aligned 48 kHz float WAV pair.
  - Once-per-second telemetry: latency estimate with a devices / buffers / DSP
    breakdown, processing p50/p99, CPU load, input drops, underruns, and
    drift drops. Session summary on exit.
  - Clock-drift guard so latency cannot creep up over a long session.
  - Clean stop on Ctrl-C, SIGTERM, `q` + Enter, or `--duration`.
  - Error messages that name the likely cause (device busy, microphone
    permission, wrong channel, unknown device).
- `passthrough` model (alias `none`): no processing. Use it for CW and digital
  modes, and as an A/B reference.
- `rfwhisper bench latency` now runs the same streaming pipeline as
  `denoise-live` and reports the algorithmic latency alongside per-block
  processing time (#15).
- CI job that clippies and tests the default (`dfn`) build, and runs the A5
  RTF gate (#117).
- `CHANGELOG.md`.

### Changed

- `cargo build --release` now builds the real denoiser. `--no-default-features`
  gives the lean stub-only build that CI's fast lanes use (#91).
- The spectral stub's realtime path is a proper streaming WOLA implementation
  (sqrt-Hann windows that sum to 1.0) with a running noise estimate.
- `rfwhisper samples synth --preset …` no longer also requires `--kind`.
- `bench latency` / `bench rtf` default to `deepfilternet3`.
- CI: macOS jobs moved off the retired `macos-13` image; the nightly gate lane
  no longer swallows gate failures.

### Fixed

- `denoise-live` re-flushed DeepFilterNet3 with silence and trimmed its delay on
  every 10 ms block, corrupting the model's state and quadrupling the compute.
- Per-block resampling in `denoise-live` zero-padded every block edge (audible
  100 Hz clicking whenever device rate ≠ 48 kHz).
- The DeepFilterNet3 build failed on rustc 1.94 (`kstring` 2.0.4 needs 1.96);
  pinned `kstring` 2.0.2.

### Known limitations

- Stock DeepFilterNet3 is a speech model. It removes CW tones and
  FT8/RTTY/PSK carriers along with the noise, so use `--model passthrough` for
  those modes until the v0.3 profiles ship.
- Not yet validated on real on-air recordings (A1/A2/A3). Hardware loopback
  latency (A4) is not yet measured; the live estimate is about 75 ms on an
  Apple Silicon laptop.
- No GUI yet (#18). No RNNoise fallback yet (#11). Raspberry Pi 5 not yet
  verified.

## [0.1.0-alpha.1]

Rust port of the runtime (#109): CLI, DSP core, fixture synthesis, metrics
harness, model registry, offline denoise with JSON and spectrogram reports.
