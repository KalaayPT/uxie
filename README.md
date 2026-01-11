<img src="docs/uxie.gif" align="right" width="120" alt="Animated sprite of Uxie from Pokemon Black and White"/>

# `uxie`

A data fetching library for Pokemon Gen 4 romhacking. Provides unified access to
ROM data from DSPRE projects and decompilation sources.

## Table of Contents

<!--toc:start-->
- [Background](#background)
  - [Etymology](#etymology)
- [Features](#features)
- [Install](#install)
  - [Build from Source](#build-from-source)
  - [As a Library](#as-a-library)
- [CLI Usage](#cli-usage)
  - [header](#header)
  - [map](#map)
  - [event](#event)
  - [symbols](#symbols)
  - [resolve-script](#resolve-script)
- [Integration](#integration)
- [Library Usage](#library-usage)
  - [High-Level Workspace](#high-level-workspace)
  - [Reading ROM Headers](#reading-rom-headers)
  - [Reading Map Headers](#reading-map-headers)
  - [Working with DSPRE Projects](#working-with-dspre-projects)
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

## Features

- **Unified ROM Access**: Auto-detects and reads data from DSPRE projects and decompilation sources.
- **High-Level Workspace**: Unified API for managing symbols, script mappings, and text banks across a project. Automatically detects project types.
- **Complex Expression Resolution**: Evaluates C expressions in `#define` and `enum` blocks, including bitwise OR (`|`), left shifts (`<<`), and nested parentheses.
- **Enhanced Symbol Table**: Automatically resolves cross-references between constants and supports `.txt` files with incremental indexing (e.g., `VAR_0 = 0`, `VAR_1`).
- **Map Header Parsing**: Unified access to area data, scripts, and events (so far) across all Gen 4 games.
- **Format Agnostic**: Seamlessly bridge legacy binary formats and modern JSON/YAML source data for both input and output.
- **Bidirectional Script Resolution**: Resolve script constants from names to values AND values to names using a shortest-name heuristic.

## Install

### Build from Source

```shell
git clone https://github.com/KalaayPT/uxie.git
cd uxie
cargo build --release
```

The binary will be at `target/release/uxie`. Verify installation:

```shell
./target/release/uxie --version
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
uxie = { git = "https://github.com/KalaayPT/uxie.git", branch = "mother" }
```

## CLI Usage

```shell
uxie <COMMAND> [OPTIONS]
```

### header

Read ROM header information. Auto-detects format from a file path or project directory:

```shell
# From current directory (auto-detects project type)
uxie header

# From a specific ROM file or header.bin
uxie header path/to/header.bin

# From a DSPRE project directory
uxie header path/to/dspre-project/

# Output as JSON
uxie header --json
```

### map

Read map header data. Automatically detects game version and table offsets from the project:

```shell
# Read map header ID 0 from current project
uxie map 0

# Read from a specific project path
uxie map 100 -p /path/to/project/

# Read from DSPRE project but resolve symbols using a decomp root
uxie map 3 -p /path/to/dspre/ --decomp /path/to/decomp/
```

### event

Load and resolve event file data:

```shell
# Resolve events for map 3 using current workspace
uxie event 3

# Resolve using a specific decomp root for symbols
uxie event 3 --decomp /path/to/pokeplatinum/
```

### symbols

Parse C header files or symbol list files:

```shell
# Parse and resolve complex expressions in a header
uxie symbols constants.h

# Parse a .txt list with incremental indexing
uxie symbols variables.txt

# Output only enums as JSON
uxie symbols constants.h --only-enums --json
```

### resolve-script (untested)

Bidirectionally resolve constants within a script file (Names -> Values AND Values -> Names):

```shell
# Resolve symbols in a script using current directory as workspace
uxie resolve-script game_script.s

# Resolve using a specific decomp root
uxie resolve-script game_script.s --decomp /path/to/pokeplatinum/
```

## Integration

`uxie` is designed to bridge the gap between different toolchains in the Gen 4
romhacking ecosystem:

- **Decompilation Projects**: Fully compatible with the formats used by
  `pokeplatinum` and `pokeheartgold`. Provides a seamless bridge between raw
  binary and modern source-controlled data.
- **DSPRE / Binary Tools**: Ensures 1:1 binary round-tripping for map headers
  and event files, maintaining compatibility with standard ROM editing tools.
- **Unified Workspace**: Transparently handles both legacy binary projects and
  modern source trees, allowing tools to be built once and run anywhere.

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

// Parse and resolve defines
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
