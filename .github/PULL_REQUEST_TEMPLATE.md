<!-- Every section is required. PRs without concrete, runnable testing criteria
     will not be merged — "tests pass" is not verification on a project whose
     product is an audio path. -->

## Summary

<!-- What changed and why. Reference the roadmap criterion (e.g. "refs A4"). -->

## Testing criteria (required)

<!-- Exact commands a reviewer runs to verify this change, and the observable
     result that counts as a pass. If the change needs signal/noise audio,
     generate it deterministically — do not assume sample files exist:

       cargo run --release -- samples synth --kind mix --clean speech \
         --noise powerline --snr-db 0 --out noisy.wav --clean-out clean.wav

     Then state the expected outcome, e.g.:

       cargo run --release -- denoise -i noisy.wav -o out.wav \
         --model spectral_stub --reference clean.wav
       # PASS if: exit 0, report snr_gain_db is a number, out.wav opens in Audacity
-->

- [ ] Command(s):
- [ ] Expected observable result:

## Physical-device verification (required for any realtime / audio-path change)

<!-- Which physical devices this was run against (from `rfwhisper audio list`),
     the exact denoise-live invocation, and what was heard/observed.
     If you could not run on hardware, say so explicitly and state what a
     reviewer with hardware must do before merge. N/A only for pure docs/CI. -->

- Hardware used:
- Command(s) run:
- Observed:

## Measurements

<!-- Numbers before/after where relevant: latency p50/p99, RTF, SNR gain, CPU.
     Include hardware + OS + commit SHA for any benchmark figure. -->

## Regression evidence

<!-- Output of the gate_cw_transient / gate_ft8_regression acceptance tests,
     or an explicit note why they are N/A for this change. -->

## Checklist

- [ ] `cargo test` green
- [ ] `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` clean
- [ ] Testing criteria above are runnable by a reviewer verbatim
- [ ] Docs updated (or N/A)
