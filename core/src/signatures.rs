//! Ore signature database, embedded into the binary at compile time so the app
//! ships as a single self-contained executable (no external data file).
//!
//! The embedded copy (`core/data/mineables.json`) is generated from the curated
//! `core/data/signatures.json` enriched with Wiki-API harvest data by
//! `scripts/fetch_mineables.py`. At runtime the app prefers the live feed (see
//! `mineables.rs`); this embedded copy is the offline fallback.

use serde::Deserialize;

/// One harvest location for an ore: which body, and the spawn probability — the chance
/// that a deposit of this ore's mining context at that body is this ore (sums to ~100%
/// per body within a context). Sourced from the Wiki API by `scripts/fetch_mineables.py`;
/// empty for asteroid-type / salvage signatures, which aren't Wiki commodities.
#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    pub body: String,
    #[serde(default)]
    pub system: String,
    #[serde(default, rename = "type")]
    pub body_type: Option<String>,
    #[serde(default)]
    pub probability: f64,
    #[serde(default)]
    pub quality_min: Option<i64>,
    #[serde(default)]
    pub quality_max: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OreSignature {
    pub id: String,
    pub name: String,
    pub base_rs: i64,
    pub tier: String,
    pub tier_value: i64,
    pub volatile: bool,
    #[serde(default)]
    pub context: Vec<String>,
    #[serde(default)]
    pub notes: String,
    /// Per-body harvest data (Wiki API). Empty for asteroid/salvage signatures.
    #[serde(default)]
    pub locations: Vec<Location>,
}

#[derive(Deserialize)]
struct MineablesFile {
    ores: Vec<OreSignature>,
}

// The generated mineables dataset (signatures + per-location spawn data), embedded at
// build time as the offline fallback. Regenerate with `uv run scripts/fetch_mineables.py`.
const MINEABLES_JSON: &str = include_str!("../data/mineables.json");

/// Parse a mineables dataset — the published feed or the embedded copy. Extra envelope
/// fields (generated_at, source, …) are ignored.
pub fn parse_signatures(json: &str) -> serde_json::Result<Vec<OreSignature>> {
    Ok(serde_json::from_str::<MineablesFile>(json)?.ores)
}

/// Load the embedded mineables dataset. Panics only if the embedded JSON is invalid,
/// which a unit test guards against.
pub fn load_signatures() -> Vec<OreSignature> {
    parse_signatures(MINEABLES_JSON).expect("embedded mineables.json must parse")
}
