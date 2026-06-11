//! Script file handling for Gen 4 Pokémon games.
//!
//! This module provides functionality for working with script files, including:
//!
//! - **Script ID Resolution**: Resolve script IDs to their containing files and associated data
//! - **Global Script Table**: Map global script IDs to script files
//!
//! ## Script ID System
//!
//! Gen 4 games use two categories of scripts:
//!
//! - **Local Scripts**: Scripts that belong to a specific map's script file
//! - **Global Scripts**: Shared scripts looked up via [`GlobalScriptTable`]
//!
//! # Example
//!
//! ```rust,no_run
//! use uxie::script_file::{GlobalScriptTable, resolve_script_id};
//!
//! let table = GlobalScriptTable::platinum_hardcoded();
//! assert!(table.is_global_script(2500));
//! assert!(!table.is_global_script(100));
//!
//! // Look up which file contains script ID 2018
//! if let Some(entry) = table.lookup(2018) {
//!     println!("Script file: {}", entry.script_file_id);
//!     println!("Text archive: {}", entry.text_archive_id);
//! }
//! ```

pub mod global_script_table;
pub mod script_resolution;
pub mod script_table;

pub use global_script_table::{GlobalScriptEntry, GlobalScriptRange, GlobalScriptTable};
pub use script_resolution::{
    MapScriptInfo, ScriptFileInfo, ScriptResolution, find_maps_for_level_script_file,
    find_maps_for_script_file, get_common_script_info, get_script_file_info_for_map,
    resolve_level_script, resolve_level_script_by_file, resolve_script_id,
    resolve_script_id_by_file, resolve_script_id_by_level_script_file,
};
pub use script_table::ScriptTable;
