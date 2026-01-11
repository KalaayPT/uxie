//! Uxie - Data fetching library for Pokemon Gen 4 Romhacking
//!
//! Named after the legendary Pokemon Uxie, the "Being of Knowledge",
//! this library provides data fetching utilities for:
//! - Reading and writing map headers (binary and C format)
//! - Parsing C header files (enums, defines, includes)
//! - Querying relationships between game data (e.g., script -> text archive)
//! - Reading ds-rom extracted project files
//!
//! Designed to be used by DSPRE, Rotom, and the pokeplatinum/pokeheartgold decomps.

pub mod c_parser;
pub mod ds_rom;
pub mod encounter_file;
pub mod event_file;
pub mod game;
pub mod map_header;
pub mod narc;
pub mod provider;
pub mod rom_header;
pub mod script_file;
pub mod text_bank;
pub mod workspace;

pub use c_parser::SymbolTable;
pub use ds_rom::{DsRomArm9Config, DsRomToolProject, DspreProject};
pub use encounter_file::{BinaryEncounterFile, JsonEncounterFile};
pub use event_file::{
    BgEventBinary, BgEventJson, BinaryEventFile, CoordEventBinary, CoordEventJson, JsonEventFile,
    ObjectEventBinary, ObjectEventJson, WarpEventBinary, WarpEventJson,
};
pub use game::{Game, GameFamily};
pub use map_header::{MapHeader, MapHeaderJson};
pub use provider::{Arm9Provider, DataProvider};
pub use rom_header::RomHeader;
pub use script_file::ScriptTable;
pub use text_bank::TextBankTable;
pub use workspace::{ProjectType, Workspace};
