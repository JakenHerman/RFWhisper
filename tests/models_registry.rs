//! Unit tests for the EP resolver and the name → Model factory
//! (port of `tests/models/test_registry.py`).

use std::path::Path;

use rfwhisper::models::load;
use rfwhisper::models::registry::{
    load_model_in, repo_root, resolve_intra_op_threads, resolve_providers, RegistryError, ARTIFACTS,
};

const ALL_PROVIDERS: [&str; 5] = [
    "CoreMLExecutionProvider",
    "DmlExecutionProvider",
    "CUDAExecutionProvider",
    "XnnpackExecutionProvider",
    "CPUExecutionProvider",
];

// --- EP resolver -----------------------------------------------------------------------

/// macOS runners must put CoreML at the head of the EP priority list.
#[test]
fn test_resolve_providers_macos_prefers_coreml() {
    let got = resolve_providers(Some("Darwin"), Some(&ALL_PROVIDERS));
    assert_eq!(got.first().unwrap(), "CoreMLExecutionProvider");
    assert_eq!(got.last().unwrap(), "CPUExecutionProvider");
}

/// Windows runners must prefer DirectML and never expose CoreML.
#[test]
fn test_resolve_providers_windows_prefers_directml() {
    let got = resolve_providers(Some("Windows"), Some(&ALL_PROVIDERS));
    assert_eq!(got.first().unwrap(), "DmlExecutionProvider");
    assert!(!got.iter().any(|p| p == "CoreMLExecutionProvider"));
}

/// Linux has no OS-specific accelerator EP; prefer CUDA, then XNNPACK, then CPU.
#[test]
fn test_resolve_providers_linux_prefers_cuda_then_xnnpack() {
    let got = resolve_providers(Some("Linux"), Some(&ALL_PROVIDERS));
    assert_eq!(got[0], "CUDAExecutionProvider");
    assert_eq!(got[1], "XnnpackExecutionProvider");
    assert_eq!(got.last().unwrap(), "CPUExecutionProvider");
}

/// An EP that isn't in `available` is dropped silently — no crash.
#[test]
fn test_resolve_providers_skips_missing_eps() {
    let got = resolve_providers(Some("Linux"), Some(&["CPUExecutionProvider"]));
    assert_eq!(got, vec!["CPUExecutionProvider"]);
}

/// macOS without CoreML EP installed must still produce a usable provider list.
#[test]
fn test_resolve_providers_macos_without_coreml_falls_through() {
    let got = resolve_providers(
        Some("Darwin"),
        Some(&["XnnpackExecutionProvider", "CPUExecutionProvider"]),
    );
    assert_eq!(
        got,
        vec!["XnnpackExecutionProvider", "CPUExecutionProvider"]
    );
}

/// Pathological empty `available` warns and returns CPU rather than an empty list.
#[test]
fn test_resolve_providers_empty_available_returns_cpu() {
    let got = resolve_providers(Some("Linux"), Some(&[]));
    assert_eq!(got, vec!["CPUExecutionProvider"]);
}

// --- Name → Model factory --------------------------------------------------------------

/// `load("null")` must succeed with no weights on disk and no ONNX backend.
#[test]
fn test_load_null_no_weights_needed() {
    let m = load("null", false).unwrap();
    assert_eq!(m.sample_rate(), 48_000);
}

/// An unknown model name must error, not silently fall back.
#[test]
fn test_load_unknown_name_raises() {
    match load("not_a_real_model", false) {
        Err(RegistryError::UnknownModel(_)) => {}
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("expected an error"),
    }
}

/// `fallback_to_null = true` returns NullModel when weights are missing.
#[test]
fn test_load_missing_weights_falls_back_when_allowed() {
    let tmp = std::env::temp_dir().join("rfwhisper-test-empty-root");
    std::fs::create_dir_all(&tmp).unwrap();
    let m = load_model_in(&tmp, "deepfilternet3", true).unwrap();
    assert_eq!(m.hop(), 480); // NullModel contract
}

/// Without `fallback_to_null` the missing-weights case must error.
#[test]
fn test_load_missing_weights_raises_without_fallback() {
    let tmp = std::env::temp_dir().join("rfwhisper-test-empty-root");
    std::fs::create_dir_all(&tmp).unwrap();
    match load_model_in(&tmp, "deepfilternet3", false) {
        Err(RegistryError::WeightsNotFound { .. }) => {}
        Err(other) => panic!("wrong error: {other}"),
        Ok(_) => panic!("expected an error"),
    }
}

/// RNNoise also gets the missing-weights fallback path (mirrors DFN3).
#[test]
fn test_load_rnnoise_missing_weights_falls_back() {
    let tmp = std::env::temp_dir().join("rfwhisper-test-empty-root");
    std::fs::create_dir_all(&tmp).unwrap();
    let m = load_model_in(&tmp, "rnnoise", true).unwrap();
    assert_eq!(m.hop(), 480);
}

/// Env-var cases run in ONE test because env vars are process-global and the
/// test harness runs tests in parallel.
#[test]
fn test_resolve_intra_op_threads_env_behaviour() {
    // Default when unset.
    std::env::remove_var("RFWHISPER_ORT_INTRA_OP");
    assert_eq!(resolve_intra_op_threads(), 2);
    // Valid override.
    std::env::set_var("RFWHISPER_ORT_INTRA_OP", "4");
    assert_eq!(resolve_intra_op_threads(), 4);
    // Non-integer falls back with a warning.
    std::env::set_var("RFWHISPER_ORT_INTRA_OP", "not-an-int");
    assert_eq!(resolve_intra_op_threads(), 2);
    // Non-positive falls back with a warning.
    std::env::set_var("RFWHISPER_ORT_INTRA_OP", "0");
    assert_eq!(resolve_intra_op_threads(), 2);
    std::env::remove_var("RFWHISPER_ORT_INTRA_OP");
}

/// ARTIFACTS entries must use relative paths so fetch joins them under the root.
#[test]
fn test_artifact_table_paths_are_relative() {
    for art in ARTIFACTS {
        assert!(!Path::new(art.relpath).is_absolute());
        assert!(art.relpath.starts_with("models/"));
    }
    // Sanity: repo_root resolves to the crate checkout, not somewhere random.
    assert!(repo_root().join("Cargo.toml").is_file());
}
