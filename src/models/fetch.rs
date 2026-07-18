//! Download pre-converted models with optional SHA-256 verification.
//!
//! **No network in realtime paths** — this is a one-time pull + verify (AGENTS.md).

use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::models::registry::{repo_root, ARTIFACTS};

pub const LOCAL_SHA_SUFFIX: &str = ".sha256.local";

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension(format!(
        "{}part",
        dest.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default()
    ));
    eprintln!("Downloading {url} -> {}", dest.display());
    let resp = ureq::get(url)
        .set("User-Agent", "rfwhisper-model-fetch/0.1")
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|e| e.to_string())?;
    let mut bytes: Vec<u8> = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn verify_or_bless(path: &Path, expected: &str) -> bool {
    let got = match sha256_file(path) {
        Ok(h) => h,
        Err(_) => return false,
    };
    if expected == "VERIFY_ON_FIRST_RUN" {
        let local = path.with_file_name(format!(
            "{}{LOCAL_SHA_SUFFIX}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        if !local.is_file() {
            let _ = fs::write(&local, format!("{got}\n"));
            eprintln!(
                "Wrote {} with SHA256 {got}. Pin this in src/models/registry.rs for \
                 reproducible builds.",
                local.display()
            );
        }
        return true;
    }
    got == expected
}

fn allow_missing_model() -> bool {
    std::env::var("RFWHISPER_ALLOW_MISSING_MODEL").is_ok_and(|v| !v.is_empty())
}

/// Fetch / verify all registered artifacts. Returns a process exit code.
pub fn run(no_network: bool, verify_only: bool) -> i32 {
    let allow_network = !no_network;
    for art in ARTIFACTS {
        let dest = repo_root().join(art.relpath);
        if verify_only {
            if !dest.is_file() {
                eprintln!("Missing {} (cannot verify)", dest.display());
                return 1;
            }
            if art.sha256 == "VERIFY_ON_FIRST_RUN" {
                match sha256_file(&dest) {
                    Ok(h) => eprintln!("{}: {h} (no expected hash pinned yet)", dest.display()),
                    Err(e) => {
                        eprintln!("Cannot hash {}: {e}", dest.display());
                        return 1;
                    }
                }
            } else if sha256_file(&dest).ok().as_deref() != Some(art.sha256) {
                eprintln!("SHA256 mismatch for {}", dest.display());
                return 1;
            }
            continue;
        }
        let on_disk_ok = dest.is_file()
            && (art.sha256 == "VERIFY_ON_FIRST_RUN"
                || sha256_file(&dest).ok().as_deref() == Some(art.sha256));
        if on_disk_ok {
            if art.sha256 == "VERIFY_ON_FIRST_RUN" {
                verify_or_bless(&dest, art.sha256);
            }
            eprintln!("OK: {}", dest.display());
            continue;
        }
        if !allow_network {
            eprintln!(
                "Missing {} (use --no-network only if files are already present).",
                dest.display()
            );
            return if allow_missing_model() { 0 } else { 1 };
        }
        if let Err(e) = download(art.url, &dest) {
            eprintln!("Fetch failed: {e} — place artefact manually or try later");
            return if allow_missing_model() { 0 } else { 1 };
        }
        if !verify_or_bless(&dest, art.sha256) {
            eprintln!("Hash mismatch {}", dest.display());
            return 1;
        }
        eprintln!("License: {}", art.license_note);
    }
    0
}
