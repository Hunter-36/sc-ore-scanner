//! Mineables dataset loader — the ore signatures enriched with per-location spawn data,
//! fetched from the published feed at startup. Mirrors `prices.rs`, but because the dataset
//! changes per game patch (not per scan) it's loaded once: live feed -> on-disk cache ->
//! embedded copy. The embedded copy is always a usable floor, so the app works offline.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use crate::signatures::{load_signatures, parse_signatures, OreSignature};

pub const DEFAULT_MINEABLES_URL: &str = "https://hunter-36.github.io/sc-ore-scanner/mineables.json";

/// Connect/read timeout for the feed fetch (runs once at startup; bound it so a hung
/// request can't stall the scan thread).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on the response body. The real dataset is a few hundred KB; this guards against a
/// runaway/hostile response. An oversized body is truncated and then fails to parse.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Where the loaded dataset came from (for logging).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Feed,
    Cache,
    Embedded,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Feed => "live feed",
            Source::Cache => "on-disk cache",
            Source::Embedded => "embedded",
        }
    }
}

/// Load the mineables dataset, preferring fresh data: live feed -> on-disk cache ->
/// embedded copy. On a successful feed fetch the on-disk cache is refreshed. Always
/// returns a usable list (the embedded copy is the floor).
pub fn load(url: &str, cache_path: Option<&Path>) -> (Vec<OreSignature>, Source) {
    if let Some(body) = fetch(url) {
        if let Ok(ores) = parse_signatures(&body) {
            if let Some(path) = cache_path {
                write_cache(path, &body);
            }
            return (ores, Source::Feed);
        }
    }
    if let Some(path) = cache_path {
        if let Ok(body) = std::fs::read_to_string(path) {
            if let Ok(ores) = parse_signatures(&body) {
                return (ores, Source::Cache);
            }
        }
    }
    (load_signatures(), Source::Embedded)
}

fn fetch(url: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(DEFAULT_TIMEOUT)
        .timeout_read(DEFAULT_TIMEOUT)
        .build();
    let resp = agent.get(url).call().ok()?;
    let mut body = String::new();
    resp.into_reader()
        .take(MAX_BYTES)
        .read_to_string(&mut body)
        .ok()?;
    Some(body)
}

/// Best-effort atomic write of the cache (temp + rename), mirroring `Config::save`.
fn write_cache(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}
