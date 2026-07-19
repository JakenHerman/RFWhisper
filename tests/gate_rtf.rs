//! A5 acceptance gate (#23): real-time factor `< 0.5` on the reference CPU.
//!
//! RTF is wall-seconds per second of audio; `< 0.5` means the denoiser leaves at
//! least half the CPU free for WSJT-X, fldigi, and logging on the same shack box.
//!
//! Unlike the quality gates (A1/A6), this is a *timing* gate, so a synthetic clip
//! is legitimate — the denoiser does the same work per frame regardless of what
//! the audio contains. It does, however, need the **real** backend: RTF on the
//! spectral stub is trivially tiny and proves nothing, so without the `dfn`
//! feature this skips with a reason rather than passing vacuously.
//!
//! Run: `cargo test --release --features dfn -- --ignored gate_rtf`

/// ROADMAP A5 threshold, to the decimal.
const RTF_THRESHOLD: f64 = 0.5;
const MODEL: &str = "deepfilternet3";

#[test]
#[ignore = "acceptance gate; run with `cargo test --release --features dfn -- --ignored gate_`"]
fn gate_rtf() {
    #[cfg(not(feature = "dfn"))]
    {
        // Skip-with-reason: the stub's RTF (~0.001) is not a meaningful A5 check.
        eprintln!(
            "SKIP gate_rtf: built without the `dfn` feature, so `{MODEL}` falls back to the \
             spectral stub, whose RTF is trivially tiny. Rebuild with `--features dfn` to \
             measure the real DeepFilterNet3 backend against A5."
        );
        write_report(serde_json::json!({
            "gate": "rtf",
            "model": MODEL,
            "skipped": true,
            "reason": "dfn feature disabled; stub RTF is not a meaningful A5 check",
            "threshold": RTF_THRESHOLD,
        }));
    }

    #[cfg(feature = "dfn")]
    {
        // A5 specifies a 30 s synthetic clip at the model's native 48 kHz.
        const SR: u32 = 48_000;
        const CLIP_SECONDS: f64 = 30.0;

        let clip: Vec<f32> = rfwhisper::samples::SignalKind::Ssb
            .render(SR, CLIP_SECONDS, 7)
            .expect("render synthetic clip")
            .iter()
            .map(|v| *v as f32)
            .collect();

        let mut engine =
            rfwhisper::denoise::select_engine(MODEL).expect("DeepFilterNet3 engine loads");
        let (_out, stats) = engine.process_file(&clip, SR);
        let rtf = stats.rtf();
        let realtime_x = if rtf > 0.0 { 1.0 / rtf } else { f64::INFINITY };

        write_report(serde_json::json!({
            "gate": "rtf",
            "model": MODEL,
            "skipped": false,
            "backend": "tract (CPU)", // libDF runs on tract; no ONNX EP breakdown applies
            "rtf": rtf,
            "realtime_factor_x": realtime_x,
            "threshold": RTF_THRESHOLD,
            "seconds_audio": stats.seconds_audio,
            "pass": rtf < RTF_THRESHOLD,
        }));

        eprintln!(
            "gate_rtf: {MODEL} RTF {rtf:.4} ({realtime_x:.0}x realtime) vs threshold {RTF_THRESHOLD}"
        );
        assert!(
            rtf < RTF_THRESHOLD,
            "A5 FAIL: {MODEL} RTF {rtf:.4} is not < {RTF_THRESHOLD} on the reference CPU"
        );
    }
}

/// Write a gate report under `build/audio-reports/` (uploaded as a CI artifact).
fn write_report(report: serde_json::Value) {
    let dir = std::path::Path::new("build").join("audio-reports");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("warning: could not create {}: {e}", dir.display());
        return;
    }
    let path = dir.join(format!("rtf_{MODEL}.json"));
    let body = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Err(e) = std::fs::write(&path, body) {
        eprintln!("warning: could not write {}: {e}", path.display());
    }
}
