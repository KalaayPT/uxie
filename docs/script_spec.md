# Script Resolution System Specification

This document describes the script ID resolution system in Gen 4 Pokémon games (Diamond/Pearl, Platinum, HeartGold/SoulSilver) and how uxie handles the various relationships between script files, map headers, and text archives.

## Overview

The Gen 4 script system has two main categories of scripts:

1. **Local Scripts** (ID 0-1999): Scripts that belong to a specific map's script file
2. **Common Scripts** (ID 2000+): Global scripts accessible from anywhere, defined in a global script table

## Core Data Structures

### Map Header

Every map in the game has a header containing references to associated files:

| Field | Description |
|-------|-------------|
| `script_file_id` | Index into the script NARC (`scr_seq.narc`) - contains NPC/event scripts |
| `level_script_id` | Index into the script NARC - contains level/init scripts (run on map load) |
| `text_archive_id` | Index into the message NARC (`msg.narc`) - contains text strings for this map |
| `event_file_id` | Index into the event NARC - contains NPC/trigger definitions |

### Script Files

Script files are stored in `fielddata/script/scr_seq.narc`. Each file contains:
- Multiple script entry points (indexed 0, 1, 2, ...)
- Bytecode for script commands
- References to text strings and other scripts

### Level Script Files

Level scripts (also called "init scripts") are stored in the same NARC as regular scripts but serve a different purpose:
- Run automatically on map transitions
- Check game state variables
- Trigger cutscenes or spawn NPCs based on story progress

**Important**: Level script files are separate from regular script files. A map has:
- One `script_file_id` for regular scripts
- One `level_script_id` for level/init scripts

These are typically different file IDs (e.g., `scripts_jubilife_city` and `scripts_init_jubilife_city`).

### Common Scripts (Global Script Table)

Scripts with IDs >= 2000 are "common scripts" that can be called from any map. These are organized in ranges, each range corresponding to a specific script file:

| ID Range | Script File | Purpose |
|----------|-------------|---------|
| 2000-2499 | File 211 | "common" scripts |
| 2800-2999 | File 212 | Berry tree scripts |
| ... | ... | ... |

The global script table maps script IDs to:
- `script_file_id`: Which file contains this common script
- `text_bank_id`: Which text archive to use for this script
- `local_script_id`: The script index within that file (ID - range_start)

## Resolution Scenarios

### Scenario 1: Resolving a Local Script from a Regular Script File

**Context**: You're in script file X (a map's `script_file_id`) and encounter `CallScript(5)`.

**Resolution**:
1. Script ID 5 < 2000, so it's a local script
2. Find the map where `map_header.script_file_id == X`
3. Return that map's script resolution info:
   - `script_file_id`: X (same file)
   - `text_archive_id`: from the map header
   - `map_id`: the found map
   - `event_file_id`: from the map header

**API**: `resolve_script_id_by_file(script_id=5, script_file_id=X, ...)`

### Scenario 2: Resolving a Local Script from a Level Script File

**Context**: You're in level script file Y (a map's `level_script_id`) and encounter `CallScript(3)`.

**Resolution**:
1. Script ID 3 < 2000, so it's a local script
2. Find the map where `map_header.level_script_id == Y`
3. The local script lives in the map's *regular* script file, not the level script file
4. Return:
   - `script_file_id`: from the map header (NOT Y!)
   - `text_archive_id`: from the map header
   - `map_id`: the found map
   - `event_file_id`: from the map header

**API**: `resolve_script_id_by_level_script_file(script_id=3, level_script_file_id=Y, ...)`

### Scenario 3: Resolving a Common Script

**Context**: From any script file, encounter `CallCommonScript(2018)`.

**Resolution**:
1. Script ID 2018 >= 2000, so it's a common script
2. Look up in the global script table
3. Return:
   - `script_file_id`: from the global table entry
   - `text_archive_id`: from the global table entry
   - No map_id (common scripts are global)

**API**: Any of the resolution functions with a script ID >= 2000

## File Relationships Diagram

```
                    Map Header (map_id=N)
                           │
           ┌───────────────┼───────────────┐
           │               │               │
           ▼               ▼               ▼
    script_file_id   level_script_id  text_archive_id
           │               │               │
           ▼               ▼               ▼
    ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
    │ Script File  │ │ Level Script │ │ Text Archive │
    │ (NPC/events) │ │ (map init)   │ │ (strings)    │
    └──────────────┘ └──────────────┘ └──────────────┘
           │               │
           │               │
           ▼               ▼
    Local scripts     Level scripts
    (ID 0-1999)       reference local
    live here         scripts in the
                      Script File
```

## Project-Specific Considerations

### DSPRE Projects

DSPRE extracts ROM data into an `unpacked/` directory structure. When you open an editor in DSPRE (e.g., Script Editor, Text Editor), it extracts the corresponding NARC files into individual binary files.

#### Project Structure

```
{romname}_DSPRE_contents/
├── arm9.bin                    # ARM9 binary (contains map header table)
├── header.bin                  # ROM header
├── data/
│   └── fielddata/
│       ├── script/scr_seq.narc # Script NARC (if not unpacked)
│       └── maptable/           # Map table data
└── unpacked/                   # Extracted NARC contents
    ├── scripts/                # From scr_seq.narc
    │   ├── 0000                # Script file ID 0
    │   ├── 0001                # Script file ID 1
    │   └── ...
    ├── textArchives/           # From msg.narc
    │   ├── 0000                # Text archive ID 0
    │   ├── 0001                # Text archive ID 1
    │   └── ...
    ├── eventFiles/             # From zone_event.narc
    │   ├── 0000                # Event file ID 0
    │   └── ...
    ├── dynamicHeaders/         # Map headers (alternative to arm9.bin)
    │   ├── 0000                # Map header ID 0
    │   └── ...
    ├── maps/                   # Map model data
    ├── matrices/               # Map matrix data
    ├── learnsets/              # Pokémon learnset data
    ├── personalPokeData/       # Pokémon base stats
    ├── trainerProperties/      # Trainer data
    ├── trainerParty/           # Trainer party data
    └── ...
```

#### File Path Mapping

| Data Type | DSPRE Unpacked Path | File Format |
|-----------|---------------------|-------------|
| Script files | `unpacked/scripts/{id:04}` | Binary script bytecode |
| Level script files | `unpacked/scripts/{id:04}` | Same NARC, different ID |
| Text archives | `unpacked/textArchives/{id:04}` | Binary text format |
| Event files | `unpacked/eventFiles/{id:04}` | Binary event format |
| Map headers | `unpacked/dynamicHeaders/{id:04}` | Binary header (24 bytes) |

#### Availability

The `unpacked/` subdirectories only exist if you've opened the corresponding editor in DSPRE:
- `scripts/` - Created when opening Script Editor
- `textArchives/` - Created when opening Text Editor  
- `eventFiles/` - Created when opening Event Editor
- `dynamicHeaders/` - Created when opening Header Editor

Use `DspreProject::has_unpacked_scripts()`, `has_unpacked_text_archives()`, etc. to check availability.

#### API Usage

```rust
let dspre = DspreProject::open("path/to/rom_DSPRE_contents")?;

// Get paths
let script_path = dspre.script_file_path(100);      // unpacked/scripts/0100
let text_path = dspre.text_archive_path(50);        // unpacked/textArchives/0050
let event_path = dspre.event_file_path(25);         // unpacked/eventFiles/0025

// Load files directly
let script_bytes = dspre.load_script_file(100)?;
let text_bytes = dspre.load_text_archive(50)?;
let event = dspre.load_event_file(25)?;
```

### Decompilation Projects (pokeplatinum/pokeheartgold)

#### Project Structure

```
pokeplatinum/
├── include/
│   ├── constants/              # Game constants
│   │   ├── species.h
│   │   ├── items.h
│   │   └── ...
│   └── data/
│       └── map_headers.h       # Map header definitions
├── res/
│   └── field/
│       ├── scripts/
│       │   ├── scripts.order   # Script name → file ID mapping
│       │   ├── scripts_jubilife_city.s
│       │   ├── scripts_init_jubilife_city.s
│       │   └── ...
│       └── events/
│           ├── zone_event.order
│           └── events_jubilife_city.json
└── build/
    └── generated/              # Generated header files
```

#### scripts.order File

Maps symbolic script names to file IDs. Line number (0-indexed) = file ID:

```
scripts_unk_0000              # File ID 0
scripts_unk_0001              # File ID 1
scripts_jubilife_city         # File ID 2
scripts_jubilife_city_mart    # File ID 3
...
scripts_init_jubilife_city    # File ID 503 (level script)
scripts_init_jubilife_city_mart  # File ID 504
```

#### Map Header Format (C)

```c
[MAP_HEADER_JUBILIFE_CITY] = {
    .scriptsArchiveID = scripts_jubilife_city,           // script_file_id
    .initScriptsArchiveID = scripts_init_jubilife_city,  // level_script_id
    .msgArchiveID = TEXT_BANK_JUBILIFE_CITY,             // text_archive_id
    .eventsArchiveID = events_jubilife_city,             // event_file_id
    // ... other fields
},
```

#### File Path Mapping

| Data Type | Decomp Path | File Format |
|-----------|-------------|-------------|
| Script files | `res/field/scripts/{name}.s` | Assembly source |
| Script order | `res/field/scripts/scripts.order` | Text file (name per line) |
| Event files | `res/field/events/{name}.json` | JSON |
| Map headers | `include/data/map_headers.h` | C header |
| Text banks | Generated/built from source | N/A (use constants) |

## API Reference

### Primary Resolution Functions

| Function | Input | Use Case |
|----------|-------|----------|
| `resolve_script_id` | script_id, map_id | When you know the current map |
| `resolve_script_id_by_file` | script_id, script_file_id | When you're in a regular script file |
| `resolve_script_id_by_level_script_file` | script_id, level_script_file_id | When you're in a level script file |

### Level Script Resolution

| Function | Input | Use Case |
|----------|-------|----------|
| `resolve_level_script` | map_id | Get info about a map's level script |
| `resolve_level_script_by_file` | level_script_file_id | Get info given only the level script file |

### Lookup Functions

| Function | Description |
|----------|-------------|
| `find_map_by_script_file_id` | Find map using a given script file |
| `find_map_by_level_script_file_id` | Find map using a given level script file |
| `find_maps_for_script_file` | Find ALL maps using a given script file |
| `find_maps_for_level_script_file` | Find ALL maps using a given level script file |

## Resolution Result

All resolution functions return `ScriptResolution`:

```rust
enum ScriptResolution {
    CommonScript {
        script_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
    },
    MapScript {
        script_id: u16,
        map_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
        event_file_id: u16,
    },
}
```

## Common Pitfalls

1. **Confusing script_file_id with level_script_id**: These are different file IDs in the map header. Level scripts can call scripts in the regular script file.

2. **Checking the wrong ID for common scripts**: The *script_id* determines if something is common (>= 2000), not the file ID.

3. **Assuming 1:1 map-to-script-file mapping**: Multiple maps can share the same script file. The resolution functions return the *first* matching map.

4. **Level scripts calling local scripts**: When a level script calls a local script (ID < 2000), that script lives in the map's regular `script_file_id`, not in the level script file itself.
