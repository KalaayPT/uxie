//! Event file parsing for Pokemon Gen 4 maps
//!
//! Event files contain map-specific data like NPCs, warps, triggers, and signs.
//! This module supports both binary and JSON formats.

pub mod binary;
pub mod json;
#[cfg(test)]
mod tests;

pub use binary::{
    BgEventBinary, BinaryEventFile, CoordEventBinary, ObjectEventBinary, WarpEventBinary,
};
pub use json::{BgEventJson, CoordEventJson, JsonEventFile, ObjectEventJson, WarpEventJson};
