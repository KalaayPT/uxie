//! Script file handling for Gen 4 Pokémon games.
//!
//! This module provides functionality for working with script files, including:
//!
//! - **Script ID Resolution**: Resolve script IDs to their containing files and associated data
//! - **Global Script Table**: Map common/global script IDs (2000+) to script files
//! - **Script Table**: Map script file names to numeric IDs from `scripts.order` files
//!
//! # Script ID System
//!
//! Gen 4 games use two categories of scripts:
//!
//! - **Local Scripts** (ID 0-1999): Scripts that belong to a specific map's script file
//! - **Common Scripts** (ID 2000+): Global scripts accessible from anywhere
//!
//! # Example
//!
//! ```rust,no_run
//! use uxie::script_file::{
//!     GlobalScriptTable, resolve_script_id, is_common_script_id,
//! };
//!
//! // Check if a script ID is a common/global script
//! assert!(is_common_script_id(2500));
//! assert!(!is_common_script_id(100));
//!
//! // Load the global script table for Platinum
//! let table = GlobalScriptTable::platinum_hardcoded();
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

pub use global_script_table::{GlobalScriptEntry, GlobalScriptTable};
pub use script_resolution::{
    find_maps_for_level_script_file, find_maps_for_script_file, get_common_script_info,
    get_script_file_info_for_map, is_common_script_id, resolve_level_script,
    resolve_level_script_by_file, resolve_script_id, resolve_script_id_by_file,
    resolve_script_id_by_level_script_file, MapScriptInfo, ScriptFileInfo, ScriptResolution,
    COMMON_SCRIPT_THRESHOLD,
};
pub use script_table::ScriptTable;
