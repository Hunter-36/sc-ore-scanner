//! Ore signature database, embedded into the binary at compile time so the app
//! ships as a single self-contained executable (no external data file).

use serde::Deserialize;

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
}

#[derive(Deserialize)]
struct SignaturesFile {
    ores: Vec<OreSignature>,
}

// Embed the same signatures.json the Python backend uses (single source of truth
// for now; it moves into the crate at cutover).
const SIGNATURES_JSON: &str = include_str!("../../backend/data/signatures.json");

/// Load the embedded ore signatures. Panics only if the embedded JSON is invalid,
/// which a unit test guards against.
pub fn load_signatures() -> Vec<OreSignature> {
    let file: SignaturesFile =
        serde_json::from_str(SIGNATURES_JSON).expect("embedded signatures.json must parse");
    file.ores
}
