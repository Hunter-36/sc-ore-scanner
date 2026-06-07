//! Core detection logic for SC Ore Scanner — the all-Rust pipeline: screen crop ->
//! preprocess -> OCR (ocrs) -> RS-number extraction -> debounce -> resolve to ore.
//! A faithful port of the v1 Python detection pipeline.

pub mod config;
pub mod debounce;
pub mod mineables;
pub mod ocr;
pub mod pipeline;
pub mod preprocess;
pub mod prices;
pub mod resolver;
pub mod signatures;

pub use resolver::{OreMatch, Resolver, SignatureConfig};
pub use signatures::{Location, OreSignature};
