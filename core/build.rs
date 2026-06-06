//! Fetch the ocrs OCR models at build time so they can be embedded into the
//! binary via `include_bytes!` (see src/ocr.rs). Downloading here keeps the
//! ~12 MB model blobs out of git while still producing a self-contained exe.
//!
//! Integrity + offline behaviour:
//! - Each model is pinned to a **SHA-256**; any copy (downloaded, cached, or
//!   vendored) is verified before it's embedded, and a mismatch fails the build
//!   loudly — a poisoned/MITM'd model never makes it into a release.
//! - Verified downloads are cached in a stable per-user dir (surviving
//!   `cargo clean`), so a later build with no network reuses them instead of
//!   hard-failing. Set `OCRS_MODEL_DIR` to a directory of pre-provided `.rten`
//!   files for fully offline / air-gapped builds.
//!
//! The hashes below are the current models from the official ocrs S3 bucket.

use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// (filename, download URL, expected SHA-256 hex).
const MODELS: [(&str, &str, &str); 2] = [
    (
        "text-detection.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten",
        "f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca",
    ),
    (
        "text-recognition.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten",
        "e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e",
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=OCRS_MODEL_DIR");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    for (name, url, sha) in MODELS {
        let dest = Path::new(&out_dir).join(name);

        // 1. Valid copy already in OUT_DIR (incremental build) — nothing to do.
        if read_if_valid(&dest, sha).is_some() {
            continue;
        }

        // 2. Try local sources before the network: the stable cache and an
        //    explicitly vendored dir (OCRS_MODEL_DIR). A present-but-wrong file
        //    fails loudly rather than being silently re-fetched.
        let bytes = local_copy(name, sha).unwrap_or_else(|| {
            // 3. Fall back to downloading, then verify and cache for next time.
            let downloaded = download(url).unwrap_or_else(|e| {
                panic!(
                    "could not fetch {name} from {url}: {e}\n\
                     No verified local copy is available either. For an offline \
                     build, set OCRS_MODEL_DIR to a directory containing {name} \
                     (SHA-256 {sha})."
                )
            });
            let got = sha256_hex(&downloaded);
            if got != sha {
                panic!(
                    "downloaded {name} failed its SHA-256 check (possible tampering \
                     or corruption): expected {sha}, got {got}"
                );
            }
            if let Some(cache) = stable_cache_dir() {
                let _ = std::fs::create_dir_all(&cache);
                let _ = std::fs::write(cache.join(name), &downloaded);
            }
            downloaded
        });

        std::fs::write(&dest, &bytes)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", dest.display()));
    }
}

/// Read `path` and return its bytes only if they match `sha`; otherwise `None`.
fn read_if_valid(path: &Path, sha: &str) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    (sha256_hex(&bytes) == sha).then_some(bytes)
}

/// Look for the model in the stable cache, then in `OCRS_MODEL_DIR`. A file that
/// exists but fails its checksum aborts the build (don't embed a bad model).
fn local_copy(name: &str, sha: &str) -> Option<Vec<u8>> {
    for dir in cache_candidates() {
        let path = dir.join(name);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let got = sha256_hex(&bytes);
        if got == sha {
            return Some(bytes);
        }
        panic!(
            "local model {} at {} failed its SHA-256 check: expected {sha}, got {got}. \
             Refusing to embed it — delete the file to re-fetch a clean copy.",
            name,
            path.display()
        );
    }
    None
}

/// Directories to search for a pre-existing model, in priority order.
fn cache_candidates() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(cache) = stable_cache_dir() {
        dirs.push(cache);
    }
    if let Ok(vendor) = std::env::var("OCRS_MODEL_DIR") {
        if !vendor.is_empty() {
            dirs.push(PathBuf::from(vendor));
        }
    }
    dirs
}

/// A stable per-user cache dir that survives `cargo clean`:
/// `%LOCALAPPDATA%` (Windows) or `$XDG_CACHE_HOME` / `$HOME/.cache` (Unix),
/// under `sc-ore-scanner/models`.
fn stable_cache_dir() -> Option<PathBuf> {
    let root = if let Ok(local) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local)
    } else if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(std::env::var("HOME").ok()?).join(".cache")
    };
    Some(root.join("sc-ore-scanner").join("models"))
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}
