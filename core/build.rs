//! Fetch the ocrs OCR models at build time so they can be embedded into the
//! binary via `include_bytes!` (see src/ocr.rs). Downloading here keeps the
//! ~12 MB model blobs out of git while still producing a self-contained exe.
//! Models are cached in OUT_DIR, so this only hits the network on a clean build.

use std::io::Read;
use std::path::Path;

const MODELS: [(&str, &str); 2] = [
    (
        "text-detection.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten",
    ),
    (
        "text-recognition.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten",
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    for (name, url) in MODELS {
        let dest = Path::new(&out_dir).join(name);
        if dest.exists() {
            continue;
        }
        let resp = ureq::get(url)
            .call()
            .unwrap_or_else(|e| panic!("failed to download {name} from {url}: {e}"));
        let mut bytes = Vec::new();
        resp.into_reader()
            .read_to_end(&mut bytes)
            .unwrap_or_else(|e| panic!("failed to read {name}: {e}"));
        std::fs::write(&dest, bytes)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", dest.display()));
    }
}
