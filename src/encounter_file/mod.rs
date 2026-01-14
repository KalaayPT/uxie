//! Wild Pokemon encounter table parsing
//!
//! Encounter files define which Pokemon appear in different encounter types
//! (grass, water, fishing, etc.) for each map.

pub mod binary;
pub mod json;
pub mod tests;

pub use binary::BinaryEncounterFile;
pub use json::JsonEncounterFile;
