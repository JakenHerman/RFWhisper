//! Integration tests for `rfwhisper denoise` (issue #16): report schema, exit codes.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rfwhisper"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("rfwhisper-cli-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn write_wav(path: &PathBuf, channels: u16, sr: u32, secs: f32) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: sr,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    let n = (sr as f32 * secs) as usize;
    for i in 0..n {
        let v = (0.4
            * (2.0 * std::f32::consts::PI * 800.0 * i as f32 / sr as f32).sin()
            * i16::MAX as f32) as i16;
        for _ in 0..channels {
            w.write_sample(v).unwrap();
        }
    }
    w.finalize().unwrap();
}

/// Report JSON carries the issue-#16 schema fields; exit code 0 on success.
#[test]
fn test_denoise_report_schema() {
    let input = scratch("in_mono.wav");
    let output = scratch("out_mono.wav");
    let report = scratch("report.json");
    write_wav(&input, 1, 48_000, 1.0);

    let status = bin()
        .args(["denoise", "-i"])
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .args(["--model", "spectral_stub", "--report"])
        .arg(&report)
        .arg("--reference")
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());

    let rep: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    for key in [
        "model",
        "sr",
        "duration_s",
        "inference_time_ms",
        "rtf",
        "snr_gain_db",
        "spectrogram_path",
    ] {
        assert!(rep.get(key).is_some(), "report missing key {key}");
    }
    assert_eq!(rep["model"], "spectral_stub");
    assert_eq!(rep["sr"], 48_000);
    assert!((rep["duration_s"].as_f64().unwrap() - 1.0).abs() < 1e-6);
    // Output WAV exists and has the same length as the input.
    let out_len = hound::WavReader::open(&output).unwrap().len();
    assert_eq!(out_len, 48_000);
}

/// Multichannel input is unsupported input → exit code 2.
#[test]
fn test_denoise_multichannel_exits_2() {
    let input = scratch("in_stereo.wav");
    write_wav(&input, 2, 48_000, 0.2);
    let status = bin()
        .args(["denoise", "-i"])
        .arg(&input)
        .arg("-o")
        .arg(scratch("out_stereo.wav"))
        .args(["--model", "spectral_stub"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

/// Unknown model is a model load failure → exit code 3.
#[test]
fn test_denoise_bad_model_exits_3() {
    let input = scratch("in_mono2.wav");
    write_wav(&input, 1, 48_000, 0.2);
    let status = bin()
        .args(["denoise", "-i"])
        .arg(&input)
        .arg("-o")
        .arg(scratch("out2.wav"))
        .args(["--model", "not_a_model"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(3));
}

/// Missing input file is unsupported input → exit code 2.
#[test]
fn test_denoise_missing_input_exits_2() {
    let status = bin()
        .args(["denoise", "-i"])
        .arg(scratch("does_not_exist.wav"))
        .arg("-o")
        .arg(scratch("out3.wav"))
        .args(["--model", "spectral_stub"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}
