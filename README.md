<img src="docs/uxie.gif" align="right" width="120" alt="Animated sprite of Uxie from Pokemon Black and White"/>

# `uxie`

A data fetching library for Pokemon Gen 4 romhacking. Provides unified access to
ROM data from DSPRE projects and decompilation sources.

## Table of Contents

<!--toc:start-->
- [Background](#background)
- [Features](#features)
- [Install](#install)
- [CLI Usage](#cli-usage)
- [Library Usage](#library-usage)
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
Its `Workspace::open()` function auto-detects the format and returns a
consistent API. The library also includes utilities for reading map headers,
parsing C enums and defines, and querying relationships between game data.

The name comes from [Uxie](https://bulbapedia.bulbagarden.net/wiki/Uxie_(Pok%C3%A9mon)), the legendary Pokemon known as the "Being of Knowledge."

## Features

- **Smart Discovery**: Automatically detects game version, internal project names, and table offsets
- **Unified ROM Access**: Auto-detects and reads data from DSPRE projects and decompilation sources
- **High-Level Workspace**: Unified API for managing symbols, script mappings, text banks, and global script tables
- **Global Script Table**: Automatic resolution of `CallCommonScript` IDs to their script files and text banks (supports HGSS binary reading and Platinum hardcoded tables)
- **Full C Expression Evaluation**: Pratt parser with correct operator precedence for all C operators
- **Parallel Loading**: Multi-threaded header file loading via rayon (~200ms for 50,000+ symbols)
- **Bidirectional Script Resolution**: Resolve constants from names to values AND values to names

## Install

### CLI

```shell
cargo install uxie
```

### Library

```shell
cargo add uxie
```

Or add to `Cargo.toml`:

```toml
[dependencies]
uxie = "0.3.0"
```

For full API documentation, visit [docs.rs/uxie](https://docs.rs/uxie).

## CLI Usage

All commands support `--json` for structured output and `-p/--project` to specify the project path.

| Command | Description | Example |
|---------|-------------|---------|
| `header` | Read ROM header info | `uxie header` |
| `map <id>` | Read map header data | `uxie map 0` |
| `event <id>` | Load event data for a map | `uxie event 3` |
| `encounter <id>` | Load encounter data | `uxie encounter 100 --json` |
| `symbols <file>` | Parse C headers or symbol files | `uxie symbols constants.h` |
| `personal <id>` | Query Pokemon base stats | `uxie personal 25` |
| `move <id>` | Query move data | `uxie move 85` |
| `item <id>` | Query item data | `uxie item 1` |
| `trainer <id>` | Query trainer data | `uxie trainer 1` |
| `evolution <id>` | Query evolution data | `uxie evolution 133` |
| `learnset <id>` | Query level-up moves | `uxie learnset 25` |
| `egg-moves [species]` | Query egg moves | `uxie egg-moves 25` |

Use `-d/--decomp` to specify a decompilation project path for symbol resolution.

## Library Usage

### Workspace API

The `Workspace` struct is the primary entry point:

```rust
use uxie::Workspace;

let workspace = Workspace::open("path/to/project")?;

// Resolve constants
let val = workspace.resolve_constant("VARS_START"); // Some(16384)

// Bidirectional symbol resolution
let resolved = workspace.resolve_script_symbols("SetFlag FLAG_UNK_0x000A");
println!("{}", resolved); // "SetFlag 10"

// Access global script table for CallCommonScript resolution
if let Some(entry) = workspace.global_script_table.lookup(2050) {
    println!("Script file: {}, Text bank: {}", entry.script_file_id, entry.text_bank_id);
}
```

### SymbolTable API

For parsing C headers and evaluating expressions:

```rust
use uxie::SymbolTable;

let mut symbols = SymbolTable::new();
symbols.load_headers_from_dir("include/constants")?;

// Resolve constants
if let Some(val) = symbols.resolve_constant("ITEM_POKE_BALL") {
    println!("ID: {}", val);
}

// Evaluate C expressions with correct precedence
let expr_val = symbols.evaluate_expression("(1 << 8) | (2 << 4) | 3");
println!("Value: {:?}", expr_val); // Some(275)
```

### Reading ROM Headers

```rust
use uxie::RomHeader;

let header = RomHeader::open("path/to/header.bin")?;
println!("Game: {:?}", header.detect_game());   // Some(Platinum)
println!("Region: {:?}", header.region());      // Some("USA")
```

## Supported Games

> **Note**: This library is a work in progress. Platinum is the primary target and has the most complete support. HeartGold/SoulSilver support is partial—some NARC paths and data structures differ and are not yet fully implemented.

| Game | Code | Family |
|------|------|--------|
| Diamond | ADAE (US), ADAJ (JP), ADAP (EU) | DP |
| Pearl | APAE (US), APAJ (JP), APAP (EU) | DP |
| Platinum | CPUE (US), CPUJ (JP), CPUP (EU) | Platinum |
| HeartGold | IPKE (US), IPKJ (JP), IPKP (EU) | HGSS |
| SoulSilver | IPGE (US), IPGJ (JP), IPGP (EU) | HGSS |

## License

MIT License. See [LICENSE](./LICENSE) for details.

--- 

*"It is said that its emergence gave humans the intelligence to improve their quality of life."* -- Pokédex entry from Pokémon Pearl
