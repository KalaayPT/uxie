<img src="docs/uxie.gif" align="right" width="120" alt="Animated sprite of Uxie from Pokemon Black and White"/>

# `uxie`

A data fetching library for Pokemon Gen 4 romhacking. Provides unified access to
ROM data from NdsTool extractions, ds-rom projects, and decompilation sources.

## Table of Contents

<!--toc:start-->
- [Background](#background)
  - [Etymology](#etymology)
- [Features](#features)
- [Install](#install)
  - [Build from Source](#build-from-source)
  - [As a Library](#as-a-library)
- [CLI Usage](#cli-usage)
  - [rom-header](#rom-header)
  - [map-header](#map-header)
  - [script-text](#script-text)
  - [ds-rom](#ds-rom)
  - [parse-enum](#parse-enum)
  - [parse-defines](#parse-defines)
- [Integration](#integration)
- [Library Usage](#library-usage)
- [Supported Games](#supported-games)
- [License](#license)
<!--toc:end-->

## Background

Working with Pokemon Gen 4 ROMs involves multiple data sources: binary files
extracted via NdsTool, YAML-based ds-rom projects, and modern JSON/C source
structures from decompilation projects like pokeplatinum/pokeheartgold. Each
source has different formats and conventions.

`uxie` provides a unified interface for reading ROM data regardless of source.
Its `RomHeader::open()` function auto-detects the format and returns a
consistent struct. The library also includes utilities for reading map headers,
parsing C enums and defines, and querying relationships between game data.

### Etymology

`uxie` takes its name from [Uxie][uxie-bulbapedia], the legendary Pokemon known
as the "Being of Knowledge." Just as Uxie embodies wisdom in the Pokemon world,
this library aims to provide knowledge about ROM data structures.

## Features

- **Unified ROM Access**: Auto-detects and reads data from NdsTool extractions, `ds-rom` projects, and decompilation sources.
- **Complex Expression Resolution**: Evaluates C expressions in `#define` and `enum` blocks, including bitwise OR (`|`), left shifts (`<<`), and nested parentheses.
- **Symbolic Resolution**: Automatically resolves cross-references between constants (e.g., `(A | B)` where A and B are other defines).
- **Map Header Parsing**: Unified access to area data, music, scripts, and worldmap coordinates across all Gen 4 games.
- **Format Agnostic**: Seamlessly bridge legacy binary formats and modern JSON/YAML source data for both input and output.

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

### rom-header

Read ROM header information from any supported format:

```shell
# From NdsTool header.bin
uxie rom-header -p /path/to/header.bin

# From ds-rom header.yaml
uxie rom-header -p /path/to/header.yaml

# From ds-rom project directory
uxie rom-header -p /path/to/ds-rom-project/

# Output as JSON
uxie rom-header -p /path/to/header.bin --json
```

Example output:

```
ROM Header Information
======================
Source:      NdsTool
Title:       POKEMON PL
Game Code:   CPUE
Maker Code:  01
ROM Version: 0
Game:        Platinum
Family:      Platinum
Region:      USA

ARM9 Offset: 0x00004000
ARM9 Size:   0xB52A0 (742048 bytes)
```

### map-header

Read map header data from ARM9 binary:

```shell
# Read map header ID 0 from Platinum
uxie map-header 0 -a /path/to/arm9.bin -g pt

# Read map header ID 100 from HeartGold/SoulSilver
uxie map-header 100 -a /path/to/arm9.bin -g hgss

# Output as JSON
uxie map-header 0 -a /path/to/arm9.bin -g pt --json
```

Game family options: `dp`, `pt`, `hgss`

### script-text

Find which text archive a script file uses:

```shell
uxie script-text 42 -a /path/to/arm9.bin -g pt
# Output: Script 42 uses text archive 123
```

### ds-rom

Read ds-rom project information:

```shell
# From project directory
uxie ds-rom /path/to/ds-rom-project/

# From config.yaml directly
uxie ds-rom /path/to/config.yaml

# Output as JSON
uxie ds-rom /path/to/ds-rom-project/ --json
```

### parse-enum

Parse C enum definitions from header files:

```shell
uxie parse-enum /path/to/header.h

# Output as JSON
uxie parse-enum /path/to/header.h --json
```

### parse-defines

Parse `#define` constants from header files with expression evaluation:

```shell
# Parse and resolve complex expressions
uxie parse-defines constants.h

# Filter by prefix (e.g., pokeplatinum constants)
uxie parse-defines constants.h -p MAP_
```

Example output:
```text
#define TRAINER          (1 << 0) (= 1)
#define DOUBLES          (1 << 1) (= 2)
#define TRAINER_DOUBLES  (DOUBLES | TRAINER) (= 3)
```

JSON output includes a `resolved` field for easy integration:
```shell
uxie parse-defines constants.h --json
```

```json
[
  {
    "name": "TRAINER_DOUBLES",
    "value": "(DOUBLES | TRAINER)",
    "resolved": 3
  }
]
```

## Integration

`uxie` is designed to bridge the gap between different toolchains in the Gen 4
romhacking ecosystem:

- **Decompilation Projects**: Fully compatible with the JSON formats used by
  `pokeplatinum` and `pokeheartgold`. Provides a seamless bridge between raw
  binary and modern source-controlled data.
- **DSPRE / Binary Tools**: Ensures 1:1 binary round-tripping for map headers
  and event files, maintaining compatibility with standard ROM editing tools.
- **ds-rom**: Native support for `ds-rom` project structures, providing a
  modern Rust interface for legacy configurations.

## Library Usage

### Reading ROM Headers

```rust
use uxie::RomHeader;

// Auto-detect format from path
let header = RomHeader::open("path/to/header.bin")?;
let header = RomHeader::open("path/to/header.yaml")?;
let header = RomHeader::open("path/to/ds-rom-project/")?;

// Or use specific methods
let header = RomHeader::from_binary("path/to/header.bin")?;
let header = RomHeader::from_ds_rom_yaml("path/to/header.yaml")?;
let header = RomHeader::from_ds_rom_project("path/to/project/")?;

// Access header data
println!("Game: {:?}", header.detect_game());       // Some(Platinum)
println!("Family: {:?}", header.detect_game_family()); // Some(Platinum)
println!("Region: {:?}", header.region());          // Some("USA")
println!("Source: {:?}", header.source);            // NdsTool, DsRom, or Decomp
```

### Reading Map Headers

```rust
use uxie::{Arm9Provider, DataProvider, GameFamily};

let provider = Arm9Provider::new(
    "path/to/arm9.bin",
    0xE601C,  // Header table offset for Platinum US
    559,      // Number of map headers
    GameFamily::Platinum,
);

let header = provider.get_map_header(0)?;
println!("Script file: {}", header.script_file_id());
println!("Text archive: {}", header.text_archive_id());
```

### Working with ds-rom Projects

```rust
use uxie::DsRomProject;

let project = DsRomProject::open("path/to/config.yaml")?;

// Access unified ROM header
println!("Game: {:?}", project.game());
println!("Title: {}", project.header.game_title);

// Access ARM9 config
println!("Base address: 0x{:08X}", project.arm9_config.base_address);
println!("SDK version: {}", project.arm9_config.sdk_version_string());

// Get file paths
let arm9_path = project.arm9_bin_path();
let files_dir = project.files_dir();
```

### Parsing C Headers

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
    } else {
        println!("#define {} {}", d.name, d.value);
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
