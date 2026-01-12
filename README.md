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
- [Integration](#integration)
- [Library Usage](#library-usage-1)
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

- **Smart Discovery**: Automatically detects game version, internal project names, and table offsets. No manual configuration required for standard projects.
- **Unified ROM Access**: Auto-detects and reads data from DSPRE projects and decompilation sources.
- **High-Level Workspace**: Unified API for managing symbols, script mappings, and text banks across a project
- **Complex Expression Resolution**: Evaluates C expressions in `#define` and `enum` blocks, including bitwise OR (`|`), left shifts (`<<`), and nested parentheses.
- **Enhanced Symbol Table**: Automatically resolves cross-references between constants and supports `.txt` files with incremental indexing.
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
uxie = "0.2.0"
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
