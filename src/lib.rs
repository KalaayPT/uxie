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
pub mod event_file;
pub mod game;
pub mod map_header;
pub mod provider;
pub mod rom_header;
pub mod script_file;
pub mod text_bank;
pub mod workspace;
pub mod encounter_file;
pub mod narc;

pub use game::{Game, GameFamily};
pub use map_header::{MapHeader, MapHeaderJson};
pub use provider::{DataProvider, Arm9Provider};
pub use rom_header::RomHeader;
pub use ds_rom::{DsRomToolProject, DsRomArm9Config, DspreProject};
pub use event_file::{BinaryEventFile, JsonEventFile, BgEventBinary, ObjectEventBinary, WarpEventBinary, CoordEventBinary, BgEventJson, ObjectEventJson, WarpEventJson, CoordEventJson};
pub use c_parser::SymbolTable;
pub use script_file::ScriptTable;
pub use text_bank::TextBankTable;
pub use workspace::{Workspace, ProjectType};
pub use encounter_file::{BinaryEncounterFile, JsonEncounterFile};
