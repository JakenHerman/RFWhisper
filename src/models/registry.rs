//! Model registry: artifact paths, execution-provider auto-select, thread knobs.
//!
//! `ARTIFACTS` and `model_path` describe on-disk locations for the fetch command.
//! `resolve_providers` keeps the per-OS ONNX Runtime execution-provider priority
//! logic (CoreML on macOS, DirectML on Windows, CUDA → XNNPACK → CPU fallback) so
//! the DFN3 backend can wire straight into it when it lands.
//!
//! Licensing: DeepFilterNet upstream is MIT or Apache-2.0; ONNX conversions
//! inherit that. See models/README.md.

use std::path::{Path, PathBuf};

use crate::models::{Model, NullModel};

/// Default per AGENTS §Real-Time Constraints; override via `RFWHISPER_ORT_INTRA_OP`.
const DEFAULT_INTRA_OP_THREADS: u32 = 2;
const INTRA_OP_ENV_VAR: &str = "RFWHISPER_ORT_INTRA_OP";

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Unknown model: {0:?}. Known: [\"deepfilternet3\", \"null\", \"rnnoise\"]")]
    UnknownModel(String),
    #[error("Weights for {name:?} not found at {path}")]
    WeightsNotFound { name: String, path: PathBuf },
    #[error(
        "{0:?} backend is not yet available in the Rust port (tracked in the DFN3 / RNNoise \
         backend issues)"
    )]
    BackendUnavailable(String),
}

/// On-disk descriptor for a downloadable model artifact.
///
/// Consumed by [`crate::models::fetch`]. `sha256` is the literal
/// `"VERIFY_ON_FIRST_RUN"` until a release pins a hash — see `models/README.md`
/// for the bless-on-first-run flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelArtifact {
    pub name: &'static str,
    pub relpath: &'static str,
    pub url: &'static str,
    /// Set to `"VERIFY_ON_FIRST_RUN"` until release pins a hash.
    pub sha256: &'static str,
    pub license_note: &'static str,
}

pub const ARTIFACTS: [ModelArtifact; 2] = [
    ModelArtifact {
        name: "DeepFilterNet3 ONNX (community export — verify before production)",
        relpath: "models/deepfilternet3/dfn3.onnx",
        url: "https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter.onnx",
        sha256: "VERIFY_ON_FIRST_RUN",
        license_note: "Upstream DeepFilterNet: MIT OR Apache-2.0. Third-party export; test parity.",
    },
    ModelArtifact {
        name: "ERB + window auxiliary (if required by your ONNX pack)",
        relpath: "models/deepfilternet3/deepfilter-auxiliary.bin",
        url: "https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter-auxiliary.bin",
        sha256: "VERIFY_ON_FIRST_RUN",
        license_note: "See HuggingFace model card; must match the ONNX you run.",
    },
];

/// Repository root: `RFWHISPER_ROOT` when set, else the crate's source checkout.
///
/// The Python package derived this from `__file__`; the env override is the
/// escape hatch for installed binaries that keep their models elsewhere.
pub fn repo_root() -> PathBuf {
    if let Ok(root) = std::env::var("RFWHISPER_ROOT") {
        return PathBuf::from(root);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute on-disk directory for a given model name (e.g. `deepfilternet3`).
pub fn model_path(name: &str) -> PathBuf {
    repo_root().join("models").join(name)
}

// --- Execution-provider auto-select ---------------------------------------------------

/// Per-OS EP priority. CUDA / XNNPACK / CPU are appended on every OS so the resolver
/// falls through cleanly when the preferred EP isn't installed.
fn os_preferred(system: &str) -> &'static [&'static str] {
    match system {
        "Darwin" | "macos" => &["CoreMLExecutionProvider"],
        "Windows" | "windows" => &["DmlExecutionProvider"],
        _ => &[],
    }
}

const COMMON_FALLBACK: [&str; 3] = [
    "CUDAExecutionProvider",
    "XnnpackExecutionProvider",
    "CPUExecutionProvider",
];

fn current_system() -> &'static str {
    match std::env::consts::OS {
        "macos" => "Darwin",
        "windows" => "Windows",
        _ => "Linux",
    }
}

/// Return ONNX Runtime execution-provider names in priority order.
///
/// Picks the OS-preferred EP first (CoreML on macOS, DirectML on Windows), then
/// falls back through CUDA → XNNPACK → CPU. EPs not present in `available` are
/// skipped, so a machine without DirectML installed silently falls through to the
/// next option rather than crashing.
///
/// `system` and `available` are injected for tests; production calls pass `None`.
pub fn resolve_providers(system: Option<&str>, available: Option<&[&str]>) -> Vec<String> {
    let sys_name: &str = match system {
        Some(s) => s,
        None => current_system(),
    };
    let queried;
    let avail: &[&str] = match available {
        Some(a) => a,
        None => {
            queried = query_available_providers();
            &queried
        }
    };

    let mut priority: Vec<&str> = Vec::new();
    priority.extend_from_slice(os_preferred(sys_name));
    priority.extend_from_slice(&COMMON_FALLBACK);

    let selected: Vec<String> = priority
        .into_iter()
        .filter(|p| avail.contains(p))
        .map(str::to_string)
        .collect();
    if selected.is_empty() {
        // No EPs matched — ONNX Runtime always advertises CPU, so this is only
        // reachable when a caller injected an empty list. Force CPU so we never
        // hand an inference session an empty provider list.
        eprintln!(
            "warning: no matching ONNX execution providers; defaulting to CPUExecutionProvider"
        );
        return vec!["CPUExecutionProvider".to_string()];
    }
    selected
}

/// Installed ONNX Runtime EPs. Until the ONNX backend lands in the Rust port this
/// is always the CPU provider (the NullModel-only environment).
fn query_available_providers() -> Vec<&'static str> {
    vec!["CPUExecutionProvider"]
}

/// Resolve the intra-op thread count, honoring `RFWHISPER_ORT_INTRA_OP` if set.
///
/// Invalid values warn and fall back to the default rather than crashing the engine.
pub fn resolve_intra_op_threads() -> u32 {
    let raw = match std::env::var(INTRA_OP_ENV_VAR) {
        Ok(v) => v,
        Err(_) => return DEFAULT_INTRA_OP_THREADS,
    };
    match raw.parse::<i64>() {
        Ok(n) if n >= 1 => n as u32,
        Ok(n) => {
            eprintln!(
                "warning: {INTRA_OP_ENV_VAR}={n} must be >= 1; using default \
                 {DEFAULT_INTRA_OP_THREADS}"
            );
            DEFAULT_INTRA_OP_THREADS
        }
        Err(_) => {
            eprintln!(
                "warning: {INTRA_OP_ENV_VAR}={raw:?} is not an int; using default \
                 {DEFAULT_INTRA_OP_THREADS}"
            );
            DEFAULT_INTRA_OP_THREADS
        }
    }
}

// --- Name → Model factory --------------------------------------------------------------

/// Relative path (under the repo root) to the primary weights file for each named
/// model. Presence on disk is what gates "have weights" vs "need fallback".
fn weights_relpath(name: &str) -> Option<&'static str> {
    match name {
        "deepfilternet3" => Some("models/deepfilternet3/dfn3.onnx"),
        "rnnoise" => Some("models/rnnoise/model.onnx"),
        _ => None,
    }
}

/// Resolve `name` to a `Model` instance using the default repo root.
///
/// Names: `null`, `deepfilternet3`, `rnnoise`. When weights or the backend aren't
/// present, `fallback_to_null = true` returns `NullModel` (with a warning);
/// otherwise the error propagates.
pub fn load_model(name: &str, fallback_to_null: bool) -> Result<Box<dyn Model>, RegistryError> {
    load_model_in(&repo_root(), name, fallback_to_null)
}

/// [`load_model`] with an explicit root — injected by tests, mirrored from the
/// Python tests' `REPO_ROOT` monkeypatching.
pub fn load_model_in(
    root: &Path,
    name: &str,
    fallback_to_null: bool,
) -> Result<Box<dyn Model>, RegistryError> {
    if name == "null" {
        return Ok(Box::new(NullModel));
    }
    let Some(rel) = weights_relpath(name) else {
        return Err(RegistryError::UnknownModel(name.to_string()));
    };

    let weights = root.join(rel);
    if !weights.is_file() {
        return fallback_or_raise(
            name,
            RegistryError::WeightsNotFound {
                name: name.to_string(),
                path: weights,
            },
            fallback_to_null,
        );
    }

    // Weights exist but the NN backends have not landed in the Rust port yet —
    // same shape as the Python ImportError path for a missing wrapper module.
    fallback_or_raise(
        name,
        RegistryError::BackendUnavailable(name.to_string()),
        fallback_to_null,
    )
}

/// Return `NullModel` with a warning when fallback is allowed; otherwise the error.
fn fallback_or_raise(
    name: &str,
    err: RegistryError,
    fallback_to_null: bool,
) -> Result<Box<dyn Model>, RegistryError> {
    if fallback_to_null {
        eprintln!("warning: load_model({name:?}) falling back to NullModel: {err}");
        return Ok(Box::new(NullModel));
    }
    Err(err)
}
