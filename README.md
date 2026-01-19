<img src="docs/uxie.gif" align="right" width="120" alt="Animated sprite of Uxie from Pokemon Black and White"/>

# `uxie`

A data fetching library for Pokemon Gen 4 romhacking. Provides unified access to
ROM data from DSPRE projects and decompilation sources.

## Table of Contents

<!--toc:start-->
- [Background](#background)
  - [Etymology](#etymology)
- [Performance](#performance)
- [Features](#features)
- [Install](#install)
  - [CLI (Command Line Interface)](#cli-command-line-interface)
  - [Library Usage](#library-usage)
  - [Build from Source](#build-from-source)
- [CLI Usage](#cli-usage)
  - [Common Flags](#common-flags)
  - [header](#header)
  - [map](#map)
  - [event & encounter](#event--encounter)
  - [symbols](#symbols)
  - [resolve-script](#resolve-script)
  - [personal](#personal)
  - [move](#move)
  - [item](#item)
  - [trainer](#trainer)
  - [evolution](#evolution)
  - [learnset](#learnset)
  - [egg-moves](#egg-moves)
  - [Working Without Decompilation Sources](#working-without-decompilation-sources)
- [Integration](#integration)
- [Library Usage](#library-usage-1)
  - [High-Level Workspace](#high-level-workspace)
  - [SymbolTable API](#symboltable-api)
  - [Reading ROM Headers](#reading-rom-headers)
  - [Reading Map Headers](#reading-map-headers)
  - [Working with DSPRE Projects](#working-with-dspre-projects)
  - [GameStrings API](#gamestrings-api)
  - [Parsing C Headers](#parsing-c-headers)
- [Supported Games](#supported-games)
- [License](#license)
<!--toc:end-->

## Background

Working with Pokemon Gen 4 ROMs involves multiple data sources: binary files
extracted via `ndstool` or `ds-rom`, `DSPRE` projects (folders containing `arm9.bin` and
`unpacked/` filesystem), and modern JSON/C source structures from
decompilation projects (pokeplatinum/pokeheartgold). Each source has
different formats and conventions.

`uxie` provides a unified interface for reading ROM data regardless of source.
Its `RomHeader::open()` function auto-detects the format and returns a
consistent struct. The library also includes utilities for reading map headers,
parsing C enums and defines, and querying relationships between game data.

### Etymology

`uxie` takes its name from [Uxie][uxie-bulbapedia], the legendary Pokemon known
as the "Being of Knowledge."

## Performance

`uxie` is optimized for high-performance symbol resolution and project loading:

- **~200ms loading time** for full pokeplatinum decompilation projects (50,000+ symbols)
- **O(1) Resolution**: Project-wide symbols are pre-resolved and cached, making lookups nearly instantaneous.
- **Lightweight Inheritance**: Collecting constants for individual script files uses a parent-pointer overlay instead of cloning the global symbol table, reducing per-file overhead by 99%.
- **Parallel Source Loading**: All headers, JSON text banks, event files, and build artifacts are loaded in parallel via `rayon`.
- **Minimized Syscall Overhead**: Canonical paths are cached globally to avoid redundant filesystem lookups.

Performance benchmarks on a typical development machine:
- Loading 50,000+ constants from pokeplatinum: ~200ms
- Single constant resolution (cached): < 1 microsecond
- Complex expression evaluation (e.g., `RGB(r,g,b)`, bitwise operations): < 5 microseconds

## Features

- **Smart Discovery**: Automatically detects game version, internal project names, and table offsets. No manual configuration required for standard projects.
- **Unified ROM Access**: Auto-detects and reads data from DSPRE projects and decompilation sources.
- **High-Level Workspace**: Unified API for managing symbols, script mappings, and text banks across a project
- **GameStrings Support**: Automatic name resolution for species, items, moves, abilities, and types from DSPRE text archives when decompilation sources aren't available
- **Full C Expression Evaluation**: Pratt parser implementation with correct operator precedence for all C operators (`+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, `>>`, `~`, `!`, parentheses)
- **Parallel Loading**: Multi-threaded header file loading via rayon for significantly faster project initialization
- **Standard HashMap API**: Public getters return `std::collections::HashMap` for easy interoperability with Rust's standard library
- **Enhanced Symbol Table**: Automatically resolves cross-references between constants, supports `.txt` files with incremental indexing, and parses event JSON files for object IDs.
- **Built-in Constants**: `TRUE` and `FALSE` are pre-defined, ensuring compatibility with C-style boolean expressions.
- **Map Header Parsing**: Unified access to area data, scripts, and events across all Gen 4 games.
- **Format Agnostic**: Seamlessly bridge legacy binary formats and modern JSON/YAML source data.
- **Bidirectional Script Resolution**: Resolve script constants from names to values AND values to names using a shortest-name heuristic.

## Install

### CLI (Command Line Interface)

The easiest way to install the `uxie` CLI is via `cargo`:

```shell
cargo install uxie
```

Verify installation:

```shell
uxie --version
```

### Library Usage

Add `uxie` to your project's dependencies. Using `cargo add`:

```shell
cargo add uxie
```

Or manually add to your `Cargo.toml`:

```toml
[dependencies]
uxie = "0.3.0"
```

For full API documentation, visit [docs.rs/uxie](https://docs.rs/uxie).

### Build from Source

To build from the latest source code:

```shell
git clone https://github.com/KalaayPT/uxie.git
cd uxie
cargo build --release
```

The binary will be at `target/release/uxie`. Verify installation:

```shell
./target/release/uxie --version
```

## CLI Usage

The Being of Knowledge offers several ways to query the secrets of the Sinnoh and Johto regions.

### Common Flags

Most commands (`map`, `event`, `encounter`, `resolve-script`) share these common options:

- `--project, -p`: Path to the project root (defaults to current directory).
- `--decomp, -d`: Optional local path OR GitHub raw URL (e.g., `https://raw.githubusercontent.com/pret/pokeplatinum/master/`) for symbol resolution.
- `--json`: Output the result as a structured JSON object.

### header

Read ROM header information. Auto-detects format from a file path or project directory:

```shell
# From current directory
uxie header

# From a specific ROM file or header.bin
uxie header path/to/header.bin

# Output as JSON
uxie header --json
```

### map

Read map header data, including internal and pretty names.

```shell
# Read map header ID 0 from current project
uxie map 0

# Read from DSPRE project but resolve symbols using a remote decomp
uxie map 3 -p /path/to/dspre/ -d https://raw.githubusercontent.com/pret/pokeplatinum/master/
```

**Example Output:**
```text
Map Header 0 (Platinum)
Internal Name:   D21R0101
Pretty Name:     Twinleaf Town
=========================
Area Data ID:    0
Matrix ID:       0
Script File ID:  0
...
```

### event & encounter

Load and resolve event or encounter data:

```shell
# Resolve events for map 3
uxie event 3

# Fetch encounters for map 100 in JSON format
uxie encounter 100 --json
```

### symbols

Parse C header files or symbol list files:

```shell
# Parse and resolve complex expressions
uxie symbols constants.h

# Parse a .txt list with incremental indexing
uxie symbols variables.txt

# Output only enums as JSON
uxie symbols constants.h --only-enums --json
```

### resolve-script (untested)

Bidirectionally resolve constants within a script file (Names -> Values AND Values -> Names):

```shell
# Resolve using a specific decomp root (optional)
uxie resolve-script game_script.s --decomp /path/to/pokeplatinum/
```

### personal

Query Pokémon personal/base stats data by species ID or name.

```shell
# Query by species ID
uxie personal 25

# Query by species name (works without decomp flag)
uxie personal pikachu

# Query by species name from specific DSPRE project
uxie personal charizard -p /path/to/my-dspre-project/

# Output as JSON
uxie personal bulbasaur --json
```

**Example Output:**
```text
Personal Data 25 (Platinum)
========================
Species:         PIKACHU
HP:              35
Attack:          55
Defense:         30
Speed:           90
Sp. Attack:      50
Sp. Defense:     40
Type 1:          ELECTRIC
Type 2:          ELECTRIC
Catch Rate:      190
Base Exp:        82
Ability 1:       Static
... more fields ...
```

### move

Query move data by ID or name.

```shell
uxie move thunderbolt
uxie move 85 -p /path/to/dspre-project/
uxie move flamethrower --json
```

**Example Output:**
```text
Move Data 85 (Platinum)
===================
Move:            Thunderbolt
Effect:          6
Split:           Special
Power:           95
Type:            ELECTRIC
Accuracy:        100
PP:              15
Effect Chance:   10
... more fields ...
```

### item

Query item data by ID or name.

```shell
uxie item "master ball"
uxie item 1 --json
uxie item 300 -p /path/to/dspre-project/
```

**Example Output:**
```text
Item Data 1 (Platinum)
===================
Item:            Master Ball
Price:           0
Hold Effect:     0
Hold Param:      0
Natural Gift Pow: 0
Fling Effect:    0
Fling Power:     0
Natural Gift Ty: 31
Prevent Toss:    false
... more fields ...
```

### trainer

Query trainer data by ID.

```shell
uxie trainer 1
uxie trainer 50 -p /path/to/dspre-project/ --json
```

**Example Output:**
```text
Trainer Data 1 (Platinum)
======================
Flags:           TrainerFlags(0x0)
Trainer Class:   2
Double Battle:   0
Party Size:      1
AI Mask:         0x00000001

Party:
  1. Lv5 STARLY (Diff=0)
```

### evolution

Query evolution data by species name or ID.

```shell
uxie evolution eevee
uxie evolution 133 --json
```

**Example Output:**
```text
Evolution Data for EEVEE (Platinum)
================================
  LevelUpNearMossRock (0) -> LEAFEON
  LevelUpNearIceRock (0) -> GLACEON
  UseItem (Thunderstone) -> JOLTEON
  UseItem (Water Stone) -> VAPOREON
  UseItem (Fire Stone) -> FLAREON
  HappinessDay (0) -> ESPEON
  HappinessNight (0) -> UMBREON
```

### learnset

Query level-up learnset data by species.

```shell
uxie learnset pikachu
uxie learnset 25 --json
```

**Example Output:**
```text
Learnset for PIKACHU (Platinum)
==========================
  Lv   1: ThunderShock
  Lv   1: Growl
  Lv   5: Tail Whip
  Lv  10: Thunder Wave
  Lv  13: Quick Attack
  Lv  18: Double Team
  Lv  21: Slam
  ... more moves ...
```

### egg-moves

Query egg move data by species (or list all species with egg moves).

```shell
# Query specific species
uxie egg-moves pikachu

# List all species with egg moves
uxie egg-moves --json
```

**Note**: HGSS uses NARC format for egg moves (not yet supported). Platinum and DP use overlay data.

### Working Without Decompilation Sources

Most data commands (`personal`, `move`, `item`, `trainer`, `evolution`, `learnset`, `egg-moves`) now work **without** requiring the `-d/--decomp` flag. `uxie` automatically uses DSPRE's `expanded/textArchives/` folder to resolve names when decompilation sources aren't available.

**Example workflow using only DSPRE project:**

```bash
# Works without any dspre source - uses text archives
cd /path/to/my-dspre-project
uxie personal pikachu
uxie move thunderbolt --json
uxie evolution eevee
uxie learnset charizard
```

**When to use `-d/--decomp`:**

- You have access to a decompilation project (e.g., pokeplatinum)
- You need to resolve custom symbols not present in vanilla text archives
- You want the most accurate symbolic names (decomp sources are authoritative)

**Fallback behavior:**

When both decomp and text archives are available, `uxie` tries decomp first, then falls back to text archives:

```bash
# Uses decomp symbols first, falls back to text archives if needed
uxie personal CustomMon -p . -d /path/to/pokeplatinum/
```

**Note**: Type names, species names, move names, item names, and ability names are all resolved from text archives when decomp is not provided.

## Integration

`uxie` is designed to bridge the gap between different toolchains in the Gen 4
romhacking ecosystem:

- **Universal Symbols**: Bridges C headers, assembly constants, and binary offsets into a single, searchable namespace.
- **Text & Archives**: Seamlessly maps text archive IDs to their symbolic names and content, linking binary data to human-readable strings.
- **Format Fluidity**: Provides a unified bridge between legacy binary formats (NARC, arm9.bin) and modern source-controlled data (JSON, YAML, C).
- **Toolchain Agnostic**: Handles both legacy binary projects and modern decompilation trees transparently, allowing your tools to work anywhere knowledge is stored.

## Library Usage

### High-Level Workspace

The `Workspace` struct is the primary entry point. It automatically detects the project type (DSPRE or Decompilation) and sets up the environment.

```rust
use uxie::Workspace;

// Auto-detect and load from any supported project path
let workspace = Workspace::open("path/to/project")?;

// Resolve a constant name to its value
let val = workspace.resolve_constant("VARS_START"); // Some(16384)

// Bidirectional resolution: Names -> Values AND Values -> Names
let script = "SetFlag FLAG_UNK_0x000A";
let resolved = workspace.resolve_script_symbols(script);
println!("{}", resolved); // "SetFlag 10"

let binary_script = "SetFlag 10";
let symbolic = workspace.resolve_script_symbols(binary_script);
println!("{}", symbolic); // "SetFlag FLAG_UNK_0x000A" (shortest name heuristic)
```

### SymbolTable API

The `SymbolTable` provides a powerful API for parsing C headers and evaluating expressions with correct C operator precedence.

```rust
use uxie::SymbolTable;
use std::collections::HashMap;

let mut symbols = SymbolTable::new();

// Load all headers from a directory in parallel (handles .h, .hpp, .txt, .py, .json)
symbols.load_headers_from_dir("include/constants")?;

// Resolve a constant from any of the loaded files
if let Some(val) = symbols.resolve_constant("ITEM_POKE_BALL") {
    println!("ID: {}", val);
}

// Evaluate arbitrary C expressions with correct precedence
// Handles bitwise OR, shifts, arithmetic, and nested parentheses
let expr_val = symbols.evaluate_expression("(1 << 8) | (2 << 4) | 3");
println!("Expression value: {:?}", expr_val); // Some(275)

// Access all constants using standard HashMap for easy interoperability
let all_defines: HashMap<String, i64> = symbols.get_all_defines();
for (name, value) in &all_defines {
    println!("{} = {}", name, value);
}

// Access enum data using standard HashMap
let enums: HashMap<String, Vec<(String, Option<i64>)>> = symbols.get_enums_std();
for (enum_name, variants) in &enums {
    println!("enum {}:", enum_name);
    for (variant, value) in variants {
        println!("  {} = {:?}", variant, value);
    }
}
```

The Pratt parser implementation ensures correct operator precedence matching C standard:
- Bitwise OR (`|`) has lower precedence than shifts
- Arithmetic operators (`*`, `/`, `%`) have higher precedence than (`+`, `-`)
- Parentheses and unary operators (`~`, `!`, `-`, `+`) are handled correctly

### Reading ROM Headers

```rust
use uxie::RomHeader;

// Auto-detect format from path
let header = RomHeader::open("path/to/header.bin")?;
let header = RomHeader::open("path/to/dspre-project/")?;

// Access header data
println!("Game: {:?}", header.detect_game());       // Some(Platinum)
println!("Family: {:?}", header.detect_game_family()); // Some(Platinum)
println!("Region: {:?}", header.region());          // Some("USA")
```

### Reading Map Headers

```rust
use uxie::{Arm9Provider, GameFamily};

// Low-level provider access
let provider = Arm9Provider::new(
    "path/to/arm9.bin",
    0xE601C,  // Table offset
    559,      // Count
    GameFamily::Platinum,
);

let header = provider.get_map_header(0)?;
println!("Script file: {}", header.script_file_id());
```

### Working with DSPRE Projects

```rust
use uxie::Workspace;

// Workspace handles DSPRE projects transparently
let workspace = Workspace::open("path/to/dspre-project")?;

println!("Project Type: {:?}", workspace.project_type); // Dspre
println!("Detected Game: {:?}", workspace.game);
```

### GameStrings API

The `GameStrings` struct provides name resolution for game data without requiring decompilation sources. It automatically loads text archives from DSPRE's `expanded/textArchives/` folder (Chatot JSON format).

```rust
use uxie::{Workspace, GameStrings};

// GameStrings is automatically loaded when opening a DSPRE workspace
let workspace = Workspace::open("path/to/dspre-project")?;

// Get species name by ID (supports all games: Platinum, DP, HGSS)
if let Some(name) = workspace.game_strings.get_species_name(25) {
    println!("Species #25: {}", name); // "PIKACHU"
}

// Get species ID by name (case-insensitive)
if let Some(id) = workspace.game_strings.get_species_id("Pikachu") {
    println!("Pikachu ID: {}", id); // 25
}

// All lookup methods support:
// - get_species_name/id()
// - get_move_name/id()
// - get_item_name/id()
// - get_ability_name/id()
// - get_type_name/id()
```

**Fallback behavior**: When both a `SymbolTable` (from decomp) and `GameStrings` (from text archives) are available, the CLI automatically tries the SymbolTable first, then falls back to GameStrings. This allows name resolution to work even without decompilation sources.

### Parsing C Headers

For complex projects, use `SymbolTable` to load multiple headers and resolve cross-references:

```rust
use uxie::SymbolTable;

let mut symbols = SymbolTable::new();

// Load all headers from a directory (handles .h, .hpp, and .txt)
symbols.load_headers_from_dir("include/constants")?;

// Resolve a constant from any of the loaded files
if let Some(val) = symbols.resolve_constant("ITEM_POKE_BALL") {
    println!("ID: {}", val);
}
```

You can also use the low-level parsing functions for single files:

```rust
use uxie::c_parser::{parse_enum, parse_and_resolve_defines};

let content = std::fs::read_to_string("header.h")?;

// Parse enums
if let Some(e) = parse_enum(&content) {
    for variant in &e.variants {
        println!("{} = {:?}", variant.name, variant.value);
    }
}

// Parse and resolve defines (with correct C precedence)
let defines = parse_and_resolve_defines(&content);
for d in &defines {
    if let Some(resolved) = d.resolved {
        println!("#define {} {} (= {})", d.name, d.value, resolved);
    }
}
```

## Supported Games

| Game | Code | Family |
|------|------|--------|
| Diamond | ADAE (US), ADAJ (JP), ADAP (EU) | DP |
| Pearl | APAE (US), APAJ (JP), APAP (EU) | DP |
| Platinum | CPUE (US), CPUJ (JP), CPUP (EU) | Platinum |
| HeartGold | IPKE (US), IPKJ (JP), IPKP (EU) | HGSS |
| SoulSilver | IPGE (US), IPGJ (JP), IPGP (EU) | HGSS |

## License

`uxie` is free software licensed under the MIT License. See [LICENSE](./LICENSE)
for details.

[uxie-bulbapedia]: https://bulbapedia.bulbagarden.net/wiki/Uxie_(Pok%C3%A9mon)
