//! Uxie - Data fetching library for Pokemon Gen 4 Romhacking
//!
//! Named after the legendary Pokemon Uxie, the "Being of Knowledge",
//! this library provides data fetching utilities for:
//! - Reading and writing map headers (binary and C format)
//! - Parsing C header files (enums, defines, includes)
//! - Querying relationships between game data (e.g., script -> text archive)
//!
//! Designed to be used by DSPRE, Rotom, and the pokeplatinum/pokeheartgold decomps.

pub mod c_parser;
pub mod game;
pub mod map_header;
pub mod provider;

pub use game::{Game, GameFamily};
pub use map_header::{MapHeader, MapHeaderPt, MapHeaderDP, MapHeaderHGSS};
pub use provider::{DataProvider, Arm9Provider, DecompProvider};
