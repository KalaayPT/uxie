//! Decomp source data loaders
//!
//! This module provides parsers for reading game data from decompilation source files
//! (JSON, CSV) as an alternative to binary NARC files. Used when running in a decomp
//! folder without a build directory, or when the build directory is incomplete.

pub mod items;
pub mod moves;
pub mod paths;
pub mod pokemon;
pub mod trainers;

pub use items::{DecompItemData, load_item_data_from_csv};
pub use moves::{DecompMoveData, load_move_data_from_json};
pub use paths::DecompPaths;
pub use pokemon::{DecompPokemonData, load_all_pokemon_data, load_pokemon_data_from_json};
pub use trainers::{DecompTrainerData, load_trainer_data_from_json};
