//! # Uxie - Data fetching library for Pokemon Gen 4 Romhacking
//!
//! Named after the legendary Pokémon [Uxie](https://bulbapedia.bulbagarden.net/wiki/Uxie_(Pok%C3%A9mon)),
//! the "Being of Knowledge", this library provides unified access to ROM data from multiple sources:
//! DSPRE projects, decompilation sources (pokeplatinum/pokeheartgold), and raw binary files.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use uxie::{Workspace, RomHeader};
//!
//! // Auto-detect and open any supported project
//! let workspace = Workspace::open("path/to/project")?;
//!
//! // Resolve constants from any loaded header file
//! if let Some(value) = workspace.resolve_constant("ITEM_POKE_BALL") {
//!     println!("Poké Ball ID: {}", value);
//! }
//!
//! // Read ROM header information
//! let header = RomHeader::open("path/to/header.bin")?;
//! println!("Game: {:?}", header.detect_game());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ## Core Features
//!
//! - **Smart Project Detection**: Automatically detects DSPRE projects, decompilation sources, and raw binaries
//! - **Unified Symbol Resolution**: Parse C headers, enums, and defines with full expression evaluation
//! - **High Performance**: ~200ms to load 50,000+ symbols with parallel parsing and caching
//! - **Map Header Access**: Read map data across all Gen 4 games (Diamond, Pearl, Platinum, HeartGold, SoulSilver)
//! - **Script & Text Banks**: Bidirectional symbol resolution for game scripts and text archives
//! - **Format Agnostic**: Seamlessly handle binary formats (NARC, arm9.bin) and modern JSON/YAML sources
//!
//! ## Module Organization
//!
//! - [`workspace`]: High-level API for managing projects, symbols, scripts, and text banks
//! - [`rom_header`]: ROM header reading with auto-detection of game version and region
//! - [`c_parser`]: C header file parsing with full expression evaluation (enums, defines, includes)
//! - [`map_header`]: Map header structures for all Gen 4 games
//! - [`provider`]: Trait-based data access for different project types
//! - [`event_file`] / [`encounter_file`]: Event and encounter data structures
//! - [`ds_rom`]: DSPRE and ds-rom-tool project structures
//! - [`narc`]: Nintendo Archive (NARC) file format reader
//! - [`game`]: Game version detection and family classification
//!
//! ## Examples
//!
//! ### Working with Decompilation Projects
//!
//! ```rust,no_run
//! use uxie::Workspace;
//!
//! let workspace = Workspace::open("path/to/pokeplatinum")?;
//!
//! // Access the symbol table
//! if let Some(val) = workspace.resolve_constant("FLAG_HIDE_RIVAL") {
//!     println!("Flag value: {}", val);
//! }
//!
//! // Bidirectional script resolution
//! let script = "SetFlag FLAG_HIDE_RIVAL";
//! let resolved = workspace.resolve_script_symbols(script);
//! println!("{}", resolved); // "SetFlag 1234"
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ### Parsing C Headers
//!
//! ```rust
//! use uxie::SymbolTable;
//!
//! let mut symbols = SymbolTable::new();
//! # let temp_dir = std::env::temp_dir();
//! # std::fs::create_dir_all(temp_dir.join("include")).ok();
//! # std::fs::write(temp_dir.join("include/constants.h"),
//! #     "#define MAX_PARTY_SIZE 6\n#define ITEM_POKE_BALL 1\n")?;
//!
//! // Load all headers from a directory
//! symbols.load_headers_from_dir(temp_dir.join("include"))?;
//!
//! // Evaluate complex expressions
//! let value = symbols.evaluate_expression("(1 << 8) | 0xFF");
//! assert_eq!(value, Some(0x1FF));
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ### Reading Map Headers
//!
//! ```rust,no_run
//! use uxie::{Arm9Provider, GameFamily, DataProvider};
//!
//! let provider = Arm9Provider::new(
//!     "path/to/arm9.bin",
//!     0xE601C,  // Platinum table offset
//!     559,      // Header count
//!     GameFamily::Platinum,
//! );
//!
//! let header = provider.get_map_header(0)?;
//! println!("Map 0 script file ID: {}", header.script_file_id());
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ## Performance
//!
//! Uxie is optimized for high-throughput symbol resolution:
//!
//! - **~200ms**: Load 50,000+ symbols from pokeplatinum
//! - **< 1 microsecond**: Cached constant resolution (O(1) lookup)
//! - **< 5 microseconds**: Complex C expression evaluation
//! - **Parallel loading**: Multi-threaded header parsing via `rayon`
//! - **Smart caching**: Global canonical path cache and expression evaluation cache
//!
//! ## Supported Games
//!
//! | Game | Region Codes | Family |
//! |------|--------------|--------|
//! | Diamond | ADAE (US), ADAJ (JP), ADAP (EU) | DP |
//! | Pearl | APAE (US), APAJ (JP), APAP (EU) | DP |
//! | Platinum | CPUE (US), CPUJ (JP), CPUP (EU) | Platinum |
//! | HeartGold | IPKE (US), IPKJ (JP), IPKP (EU) | HGSS |
//! | SoulSilver | IPGE (US), IPGJ (JP), IPGP (EU) | HGSS |
//!
//! ## Design Philosophy
//!
//! Uxie bridges the gap between legacy binary ROM hacking tools and modern decompilation workflows:
//!
//! - **Universal Symbols**: Single namespace for C headers, assembly constants, and binary offsets
//! - **Toolchain Agnostic**: Works with DSPRE, ds-rom-tool, and decompilation projects
//! - **Format Fluidity**: Unified API regardless of source format (binary, YAML, JSON, C)
//! - **Zero Configuration**: Smart auto-detection for standard project structures

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
