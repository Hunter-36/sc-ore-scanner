//! Core detection logic for SC Ore Scanner (the v2, all-Rust pipeline).
//!
//! Phase 1: the RS-number -> ore resolver and the embedded signature database.
//! Ported from the Python `backend/src/resolver` + `data/signatures.json`.

pub mod resolver;
pub mod signatures;

pub use resolver::{OreMatch, Resolver, SignatureConfig};
pub use signatures::OreSignature;
