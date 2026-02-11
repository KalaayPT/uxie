use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uxie::{
    BinaryEncounterFile, DspreProject, EggMoveData, EvolutionData, EvolutionMethod, GameFamily,
    GameStrings, ItemData, JsonEncounterFile, LearnsetData, MapHeader, MapHeaderJson, MoveData,
    Narc, PersonalData, RomHeader, SymbolTable, TrainerData, Workspace,
};

#[derive(Parser)]
#[command(name = "uxie")]
#[command(author = "Kalaay")]
#[command(version)]
#[command(about = "Data fetching tool for Pokemon Gen 4 Romhacking", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args)]
struct ProjectArgs {
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    #[arg(short, long)]
    decomp: Option<PathBuf>,

    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    Header {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long)]
        json: bool,
    },

    Map {
        id: u16,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Event {
        id: u32,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Encounter {
        id: u32,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Symbols {
        path: PathBuf,

        #[arg(long)]
        only_defines: bool,

        #[arg(long)]
        only_enums: bool,

        #[arg(long)]
        json: bool,
    },

    ResolveScript {
        path: PathBuf,

        #[arg(short, long, default_value = ".")]
        decomp: PathBuf,
    },

    Personal {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Move {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Item {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Trainer {
        id: u16,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Evolution {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    Learnset {
        id: String,

        #[command(flatten)]
        args: ProjectArgs,
    },

    EggMoves {
        species: Option<String>,

        #[command(flatten)]
        args: ProjectArgs,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Header { path, json } => cmd_header(&path, json),
        Commands::Map { id, args } => cmd_map(id, &args.project, args.decomp, args.json),
        Commands::Event { id, args } => cmd_event(id, &args.project, args.decomp, args.json),
        Commands::Encounter { id, args } => {
            cmd_encounter(id, &args.project, args.decomp, args.json)
        }
        Commands::Symbols {
            path,
            only_defines,
            only_enums,
            json,
        } => cmd_parse_header(&path, only_defines, only_enums, json),
        Commands::ResolveScript { path, decomp } => cmd_resolve_script(&path, &decomp),
        Commands::Personal { id, args } => cmd_personal(&id, &args.project, args.decomp, args.json),
        Commands::Move { id, args } => cmd_move(&id, &args.project, args.decomp, args.json),
        Commands::Item { id, args } => cmd_item(&id, &args.project, args.decomp, args.json),
        Commands::Trainer { id, args } => cmd_trainer(id, &args.project, args.decomp, args.json),
        Commands::Evolution { id, args } => {
            cmd_evolution(&id, &args.project, args.decomp, args.json)
        }
        Commands::Learnset { id, args } => cmd_learnset(&id, &args.project, args.decomp, args.json),
        Commands::EggMoves { species, args } => {
            cmd_egg_moves(species, &args.project, args.decomp, args.json)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_header(path: &PathBuf, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let header = RomHeader::open(path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&header)?);
    } else {
        println!("ROM Header Information");
        println!("======================");
        println!("Source:      {:?}", header.source);
        println!("Title:       {}", header.game_title);
        println!("Game Code:   {}", header.game_code);
        println!("Maker Code:  {}", header.maker_code);
        println!("ROM Version: {}", header.rom_version);

        if let Some(game) = header.detect_game() {
            println!("Game:        {:?}", game);
            println!("Family:      {:?}", game.family());
        } else {
            println!("Game:        Unknown");
        }

        if let Some(region) = header.region() {
            println!("Region:      {}", region);
        }

        if let Some(offset) = header.arm9_rom_offset {
            println!("\nARM9 Offset: 0x{:08X}", offset);
        }
        if let Some(size) = header.arm9_size {
            println!("ARM9 Size:   0x{:X} ({} bytes)", size, size);
        }
        if let Some(offset) = header.arm7_rom_offset {
            println!("ARM7 Offset: 0x{:08X}", offset);
        }
        if let Some(size) = header.arm7_size {
            println!("ARM7 Size:   0x{:X} ({} bytes)", size, size);
        }

        if let (Some(fnt_off), Some(fnt_sz)) = (header.fnt_offset, header.fnt_size) {
            println!("\nFNT Offset:  0x{:08X} (size: 0x{:X})", fnt_off, fnt_sz);
        }
        if let (Some(fat_off), Some(fat_sz)) = (header.fat_offset, header.fat_size) {
            println!("FAT Offset:  0x{:08X} (size: 0x{:X})", fat_off, fat_sz);
        }

        if let Some(crc) = header.header_crc {
            println!("\nHeader CRC:  0x{:04X}", crc);
        }
    }

    Ok(())
}

fn cmd_map(
    id: u16,
    path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(path, decomp.as_ref())?;

    let header = ws.provider.get_map_header(id)?;

    if json {
        let json_header = MapHeaderJson::from_binary(&header, &ws.symbols);
        println!("{}", serde_json::to_string_pretty(&json_header)?);
    } else {
        print_map_header(&header, id, &ws.symbols, &ws);
    }
    Ok(())
}

fn cmd_event(
    id: u32,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let dspre = DspreProject::open(project_path)?;
    let bin_event = dspre.load_event_file(id)?;
    let json_event = uxie::JsonEventFile::from_binary(&bin_event, &ws.symbols);

    println!("{}", serde_json::to_string_pretty(&json_event)?);
    Ok(())
}

fn cmd_encounter(
    id: u32,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let narc_path = match ws.family {
        GameFamily::DP => project_path.join("data/fielddata/encountdata/d_enc_data.narc"),
        GameFamily::Platinum => project_path.join("data/fielddata/encountdata/pl_enc_data.narc"),
        GameFamily::HGSS => project_path.join("data/a/0/3/7"),
    };

    let bin_data = if narc_path.exists() {
        let mut file = std::fs::File::open(narc_path)?;
        let narc = uxie::narc::Narc::from_binary(&mut file)?;
        narc.members
            .get(id as usize)
            .cloned()
            .ok_or("Encounter ID out of range in NARC")?
    } else {
        let unpacked_path = project_path
            .join("unpacked/encounters")
            .join(format!("{:04}", id));
        if unpacked_path.exists() {
            std::fs::read(unpacked_path)?
        } else {
            return Err("Encounter data not found (tried NARC and unpacked/encounters)".into());
        }
    };

    let mut reader = std::io::Cursor::new(bin_data);
    let bin = BinaryEncounterFile::from_binary(&mut reader, ws.family)?;
    let json = JsonEncounterFile::from_binary(&bin, &ws.symbols, ws.family);
    println!("{}", serde_json::to_string_pretty(&json)?);

    Ok(())
}

fn cmd_parse_header(
    path: &PathBuf,
    only_defines: bool,
    only_enums: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut symbols = SymbolTable::new();

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext {
        "txt" => symbols.load_list_file_str(&content)?,
        "json" => {
            symbols.load_text_bank_json(path)?;
        }
        _ => symbols.load_header_str(&content)?,
    }

    if json {
        #[derive(serde::Serialize)]
        struct HeaderOutput {
            defines: Option<Vec<uxie::c_parser::defines::CDefine>>,
            enums: Option<HashMap<String, Vec<(String, Option<i64>)>>>,
        }

        let defines = if !only_enums {
            Some(
                symbols
                    .get_all_defines()
                    .iter()
                    .map(|(n, v)| uxie::c_parser::defines::CDefine {
                        name: n.clone(),
                        value: v.to_string(),
                        resolved: Some(*v),
                    })
                    .collect(),
            )
        } else {
            None
        };

        let output = HeaderOutput {
            defines,
            enums: if !only_defines {
                // Return all symbols as a single "default" enum if requested
                let mut map = HashMap::new();
                let mut all_syms: Vec<_> = symbols
                    .get_all_defines()
                    .into_iter()
                    .map(|(n, v)| (n, Some(v)))
                    .collect();
                all_syms.sort_by_key(|(n, _)| n.clone());
                map.insert("Symbols".to_string(), all_syms);
                Some(map)
            } else {
                None
            },
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if !only_enums {
            let mut defs: Vec<_> = symbols
                .get_all_defines()
                .into_iter()
                .map(|(n, v)| uxie::c_parser::defines::CDefine {
                    name: n,
                    value: v.to_string(),
                    resolved: Some(v),
                })
                .collect();
            defs.sort_by_key(|d| d.name.clone());
            if !defs.is_empty() {
                println!("Symbols:");
                for d in defs {
                    if let Some(resolved) = d.resolved {
                        println!("  {} = {}", d.name, resolved);
                    } else {
                        println!("  {}", d.name);
                    }
                }
            }
        }
        let enums = symbols.get_enums_std();
        if !only_defines && !enums.is_empty() {
            println!("\nEnums:");
            for (name, variants) in &enums {
                println!("  enum {} {{", name);
                for (v_name, v_val) in variants {
                    println!("    {} = {:?},", v_name, v_val);
                }
                println!("  }}");
            }
        }
    }

    Ok(())
}

fn cmd_resolve_script(
    path: &PathBuf,
    decomp_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let ws = Workspace::open(decomp_path)?;
    println!("{}", ws.resolve_script_symbols(&content));
    Ok(())
}

fn open_workspace_with_decomp(
    project_path: &std::path::Path,
    decomp: Option<&PathBuf>,
) -> Result<Workspace, Box<dyn std::error::Error>> {
    let mut ws = Workspace::open(project_path)?;
    if let Some(d) = decomp {
        let mut symbols = (*ws.symbols).clone();
        load_symbols_from_decomp(&mut symbols, d.as_path())?;
        ws.symbols = Arc::new(symbols);
    }
    Ok(ws)
}

fn load_symbols_from_decomp(
    symbols: &mut SymbolTable,
    d: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let d_str = d.to_string_lossy();
    if d_str.starts_with("http") || d_str.contains("github.com") {
        let mut base = if d_str.starts_with("git@github.com:") {
            let repo = d_str.replace("git@github.com:", "").replace(".git", "");
            format!("https://raw.githubusercontent.com/{}/master/", repo)
        } else if d_str.contains("github.com") && !d_str.contains("raw.githubusercontent.com") {
            let repo_path = if let Some(pos) = d_str.find("github.com/") {
                &d_str[pos + 11..]
            } else {
                &d_str
            };
            let repo_path = repo_path.trim_end_matches('/');
            format!("https://raw.githubusercontent.com/{}/master/", repo_path)
        } else {
            d_str.to_string()
        };

        if !base.ends_with('/') {
            base.push('/');
        }

        let files = [
            "generated/sdat.txt",
            "generated/vars_flags.txt",
            "generated/maps.txt",
            "generated/species.txt",
            "generated/moves.txt",
            "generated/items.txt",
            "generated/object_events.txt",
            "generated/movement_types.txt",
            "generated/trainer_types.txt",
            "generated/bg_event_dirs.txt",
            "generated/bg_event_types.txt",
            "generated/map_headers.txt",
            "generated/battle_backgrounds.txt",
            "generated/overworld_weather.txt",
            "include/constants/species.h",
            "include/constants/moves.h",
            "include/constants/items.h",
            "include/constants/flags.h",
            "include/constants/vars.h",
            "include/constants/map_object.h",
            "include/constants/map_sections.h",
            "include/constants/overworld_weather.h",
            "include/constants/battle.h",
        ];
        for f in files {
            let url = format!("{}{}", base, f);
            symbols.load_from_url(&url).map_err(|e| {
                std::io::Error::other(format!("Failed loading symbols from {}: {}", url, e))
            })?;
        }
    } else {
        if !d.exists() {
            return Err(format!("Decomp path does not exist: {}", d.display()).into());
        }

        let include_count = symbols.load_headers_from_dir(d.join("include/constants"))?;
        let generated_count = symbols.load_headers_from_dir(d.join("generated"))?;
        let build_generated_count = symbols.load_headers_from_dir(d.join("build/generated"))?;

        if include_count + generated_count + build_generated_count == 0 {
            return Err(
                std::io::Error::other(format!(
                    "No symbol source files found under {} (checked include/constants, generated, build/generated)",
                    d.display()
                ))
                .into(),
            );
        }
    }
    Ok(())
}

fn print_map_header(header: &MapHeader, id: u16, symbols: &SymbolTable, ws: &Workspace) {
    let (game_name, divider_len) = match header {
        MapHeader::DP(_) => ("Diamond/Pearl", 29),
        MapHeader::Pt(_) => ("Platinum", 25),
        MapHeader::HGSS(_) => ("HeartGold/SoulSilver", 36),
    };

    let resolve = |val: i64, prefix: &str| -> String {
        symbols
            .resolve_name(val, prefix)
            .unwrap_or_else(|| val.to_string())
    };

    let internal_name = ws
        .get_map_internal_name(id)
        .unwrap_or_else(|| "Unknown".to_string());
    let location_id = match header {
        MapHeader::DP(h) => h.location_name as u16,
        MapHeader::Pt(h) => h.location_name as u16,
        MapHeader::HGSS(h) => h.location_name as u16,
    };
    let pretty_name = ws
        .get_map_location_name(location_id as u8)
        .unwrap_or_else(|| "Unknown".to_string());

    println!("Map Header {} ({})", id, game_name);
    println!("Internal Name:   {}", internal_name);
    println!("Pretty Name:     {}", pretty_name);
    println!("{}", "=".repeat(divider_len));

    println!("Area Data ID:    {}", header.area_data_id());
    println!("Matrix ID:       {}", header.matrix_id());
    println!("Script File ID:  {}", header.script_file_id());
    println!("Level Script ID: {}", header.level_script_id());
    println!("Text Archive ID: {}", header.text_archive_id());

    match header {
        MapHeader::DP(h) => {
            println!(
                "Music Day:       {}",
                resolve(h.music_day_id as i64, "SEQ_")
            );
            println!(
                "Music Night:     {}",
                resolve(h.music_night_id as i64, "SEQ_")
            );
            println!("Wild Pokemon:    {}", h.wild_pokemon);
            println!("Event File ID:   {}", h.event_file_id);
            println!(
                "Location Name:   {}",
                resolve(h.location_name as i64, "MAPSEC_")
            );
            println!(
                "Weather:         {}",
                resolve(h.weather_id as i64, "OVERWORLD_WEATHER_")
            );
            println!(
                "Camera:          {}",
                resolve(h.camera_angle_id as i64, "CAMERA_TYPE_")
            );
            println!(
                "Battle BG:       {}",
                resolve(h.battle_background as i64, "BATTLE_BG_")
            );
            println!("Flags:           0x{:02X}", h.flags);
        }
        MapHeader::Pt(h) => {
            println!(
                "Music Day:       {}",
                resolve(h.music_day_id as i64, "SEQ_")
            );
            println!(
                "Music Night:     {}",
                resolve(h.music_night_id as i64, "SEQ_")
            );
            println!("Wild Pokemon:    {}", h.wild_pokemon);
            println!("Event File ID:   {}", h.event_file_id);
            println!(
                "Location Name:   {}",
                resolve(h.location_name as i64, "MAPSEC_")
            );
            println!("Area Icon:       {}", h.area_icon);
            println!(
                "Weather:         {}",
                resolve(h.weather_id as i64, "OVERWORLD_WEATHER_")
            );
            println!(
                "Camera:          {}",
                resolve(h.camera_angle_id as i64, "CAMERA_TYPE_")
            );
            println!(
                "Battle BG:       {}",
                resolve(h.battle_background as i64, "BATTLE_BG_")
            );
            println!("Flags:           0x{:02X}", h.flags);
        }
        MapHeader::HGSS(h) => {
            println!(
                "Music Day:       {}",
                resolve(h.music_day_id as i64, "SEQ_")
            );
            println!(
                "Music Night:     {}",
                resolve(h.music_night_id as i64, "SEQ_")
            );
            println!("Wild Pokemon:    {}", h.wild_pokemon);
            println!("Event File ID:   {}", h.event_file_id);
            println!(
                "Location Name:   {}",
                resolve(h.location_name as i64, "MAPSEC_")
            );
            println!("Area Icon:       {}", h.area_icon);
            println!(
                "Weather:         {}",
                resolve(h.weather_id as i64, "OVERWORLD_WEATHER_")
            );
            println!(
                "Camera:          {}",
                resolve(h.camera_angle_id as i64, "CAMERA_TYPE_")
            );
            println!("Worldmap:        ({}, {})", h.worldmap_x, h.worldmap_y);
            println!("Kanto:           {}", h.kanto_flag);
            println!(
                "Battle BG:       {}",
                resolve(h.battle_background as i64, "BATTLE_BG_")
            );
            println!("Flags:           0x{:02X}", h.flags);
        }
    }
}

fn cmd_personal(
    id: &str,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc_path = project_path.join("data/poketool/personal/pl_personal.narc");
    let narc = load_narc(&narc_path)?;

    let data = narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Personal data ID {} out of range", id))?;

    let mut cursor = std::io::Cursor::new(data);
    let personal = PersonalData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&personal)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Personal Data {} (Platinum)", id);
        println!("========================");
        println!("Species:         {}", resolve(id, "SPECIES_"));
        println!("HP:              {}", personal.hp);
        println!("Attack:          {}", personal.attack);
        println!("Defense:         {}", personal.defense);
        println!("Speed:           {}", personal.speed);
        println!("Sp. Attack:      {}", personal.sp_attack);
        println!("Sp. Defense:     {}", personal.sp_defense);
        println!(
            "Type 1:          {}",
            resolve(personal.type1 as u16, "TYPE_")
        );
        println!(
            "Type 2:          {}",
            resolve(personal.type2 as u16, "TYPE_")
        );
        println!("Catch Rate:      {}", personal.catch_rate);
        println!("Base Exp:        {}", personal.base_exp);
        println!(
            "Ability 1:       {}",
            resolve(personal.ability1 as u16, "ABILITY_")
        );
        println!(
            "Ability 2:       {}",
            resolve(personal.ability2 as u16, "ABILITY_")
        );
        println!("Gender Ratio:    {}", personal.gender_ratio);
        println!("Egg Cycles:      {}", personal.egg_cycles);
        println!("Base Friendship: {}", personal.base_friendship);
        println!("Growth Rate:     {}", personal.growth_rate);
        println!(
            "Egg Group 1:     {}",
            resolve(personal.egg_group1 as u16, "EGG_GROUP_")
        );
        println!(
            "Egg Group 2:     {}",
            resolve(personal.egg_group2 as u16, "EGG_GROUP_")
        );
    }

    Ok(())
}

fn cmd_move(
    id: &str,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let id = resolve_id(id, "MOVE_", &ws.symbols, &ws.game_strings)?;

    let narc_path = project_path.join("data/poketool/waza/pl_waza_tbl.narc");
    let narc = load_narc(&narc_path)?;

    let data = narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Move data ID {} out of range", id))?;

    let mut cursor = std::io::Cursor::new(data);
    let move_data = MoveData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&move_data)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Move Data {} (Platinum)", id);
        println!("===================");
        println!("Move:            {}", resolve(id, "MOVE_"));
        println!("Effect:          {}", move_data.battle_effect);
        println!("Split:           {:?}", move_data.split);
        println!("Power:           {}", move_data.power);
        println!(
            "Type:            {}",
            resolve(move_data.move_type as u16, "TYPE_")
        );
        println!("Accuracy:        {}", move_data.accuracy);
        println!("PP:              {}", move_data.pp);
        println!("Effect Chance:   {}", move_data.side_effect_chance);
        println!("Target:          {}", move_data.target);
        println!("Priority:        {}", move_data.priority);
        println!("Flags:           {:?}", move_data.flags);
        println!("Contest Effect:  {}", move_data.contest_appeal);
        println!("Contest Type:    {}", move_data.contest_condition);
    }

    Ok(())
}

fn cmd_item(
    id: &str,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let id = resolve_id(id, "ITEM_", &ws.symbols, &ws.game_strings)?;

    let narc_path = project_path.join("data/itemtool/itemdata/pl_item_data.narc");
    let narc = load_narc(&narc_path)?;

    let data = narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Item data ID {} out of range", id))?;

    let mut cursor = std::io::Cursor::new(data);
    let item = ItemData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&item)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Item Data {} (Platinum)", id);
        println!("===================");
        println!("Item:            {}", resolve(id, "ITEM_"));
        println!("Price:           {}", item.price);
        println!("Hold Effect:     {}", item.hold_effect);
        println!("Hold Param:      {}", item.hold_effect_param);
        println!("Natural Gift Pow: {}", item.natural_gift_power);
        println!("Fling Effect:    {}", item.fling_effect);
        println!("Fling Power:     {}", item.fling_power);
        println!(
            "Natural Gift Ty: {}",
            resolve(item.natural_gift_type as u16, "TYPE_")
        );
        println!("Prevent Toss:    {}", item.prevent_toss);
        println!("Is Selectable:   {}", item.is_selectable);
        println!("Field Pocket:    {:?}", item.field_pocket);
        println!("Battle Pocket:   {:?}", item.battle_pocket);
        println!("Field Function:  {}", item.field_use_func);
        println!("Battle Function: {}", item.battle_use_func);
        println!("Party Use:       {}", item.party_use);
    }

    Ok(())
}

fn cmd_trainer(
    id: u16,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let trdata_path = project_path.join("data/poketool/trainer/trdata.narc");
    let trpoke_path = project_path.join("data/poketool/trainer/trpoke.narc");

    let trdata_narc = load_narc(&trdata_path)?;
    let trpoke_narc = load_narc(&trpoke_path)?;

    let props_data = trdata_narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Trainer data ID {} out of range", id))?;
    let party_data = trpoke_narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Trainer party ID {} out of range", id))?;

    let mut props_cursor = std::io::Cursor::new(props_data);
    let mut party_cursor = std::io::Cursor::new(party_data);

    let trainer = TrainerData::from_binary_parts(&mut props_cursor, &mut party_cursor, ws.family)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&trainer)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Trainer Data {} (Platinum)", id);
        println!("======================");
        println!("Flags:           {:?}", trainer.properties.flags);
        println!(
            "Trainer Class:   {}",
            resolve(trainer.properties.trainer_class as u16, "TRAINERTYPE_")
        );
        println!("Double Battle:   {}", trainer.properties.double_battle);
        println!("Party Size:      {}", trainer.properties.party_count);

        for (i, item) in trainer.properties.items.iter().enumerate() {
            if *item != 0 {
                println!("Item {}:          {}", i + 1, resolve(*item, "ITEM_"));
            }
        }

        println!("AI Mask:         {:?}", trainer.properties.ai_flags);

        println!("\nParty:");
        for (i, mon) in trainer.party.iter().enumerate() {
            println!(
                "  {}. Lv{} {} (Diff={})",
                i + 1,
                mon.level,
                resolve(mon.species, "SPECIES_"),
                mon.difficulty
            );
            if let Some(item) = mon.held_item {
                println!("     Held: {}", resolve(item, "ITEM_"));
            }
            if let Some(ref moves) = mon.moves {
                let move_strs: Vec<String> = moves
                    .iter()
                    .filter(|&&m| m != 0)
                    .map(|&m| resolve(m, "MOVE_"))
                    .collect();
                if !move_strs.is_empty() {
                    println!("     Moves: {}", move_strs.join(", "));
                }
            }
        }
    }

    Ok(())
}

fn cmd_evolution(
    id: &str,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc_path = project_path.join("data/poketool/personal/evo.narc");
    let narc = load_narc(&narc_path)?;

    let data = narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Evolution data ID {} out of range", id))?;

    let mut cursor = std::io::Cursor::new(data);
    let evo = EvolutionData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&evo)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Evolution Data for {} (Platinum)", resolve(id, "SPECIES_"));
        println!("================================");

        let active: Vec<_> = evo.active_evolutions().collect();
        if active.is_empty() {
            println!("No evolutions.");
        } else {
            for entry in active {
                let method = EvolutionMethod::from(entry.method);
                let param_str = match method {
                    EvolutionMethod::UseItem
                    | EvolutionMethod::UseItemMale
                    | EvolutionMethod::UseItemFemale
                    | EvolutionMethod::TradeWithItem
                    | EvolutionMethod::LevelUpWithItem => resolve(entry.param, "ITEM_"),
                    EvolutionMethod::LevelUp
                    | EvolutionMethod::LevelUpMale
                    | EvolutionMethod::LevelUpFemale => {
                        format!("Lv{}", entry.param)
                    }
                    EvolutionMethod::LevelUpWithPartyMember => resolve(entry.param, "SPECIES_"),
                    EvolutionMethod::LevelUpWithMoveType => resolve(entry.param, "TYPE_"),
                    _ => entry.param.to_string(),
                };
                println!(
                    "  {} ({}) -> {}",
                    method,
                    param_str,
                    resolve(entry.target_species, "SPECIES_")
                );
            }
        }
    }

    Ok(())
}

fn cmd_learnset(
    id: &str,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc_path = project_path.join("data/poketool/personal/wotbl.narc");
    let narc = load_narc(&narc_path)?;

    let data = narc
        .members
        .get(id as usize)
        .ok_or_else(|| format!("Learnset data ID {} out of range", id))?;

    let mut cursor = std::io::Cursor::new(data);
    let learnset = LearnsetData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&learnset)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Learnset for {} (Platinum)", resolve(id, "SPECIES_"));
        println!("==========================");

        if learnset.entries.is_empty() {
            println!("No level-up moves.");
        } else {
            for entry in &learnset.entries {
                println!(
                    "  Lv {:3}: {}",
                    entry.level,
                    resolve(entry.move_id, "MOVE_")
                );
            }
        }
    }

    Ok(())
}

fn cmd_egg_moves(
    species: Option<String>,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_ref())?;

    let egg_data = load_egg_move_data(project_path, ws.family)?;

    let gs = &ws.game_strings;
    let symbols = &ws.symbols;
    let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

    let game_name = match ws.family {
        GameFamily::Platinum => "Platinum",
        GameFamily::DP => "Diamond/Pearl",
        GameFamily::HGSS => "HeartGold/SoulSilver",
    };

    let species_id: Option<u16> = species.as_ref().and_then(|s| {
        if let Ok(id) = s.parse::<u16>() {
            Some(id)
        } else {
            resolve_id(s, "SPECIES_", &ws.symbols, &ws.game_strings).ok()
        }
    });

    if let Some(species_arg) = &species {
        let species_id = species_id.ok_or_else(|| format!("Unknown species: {}", species_arg))?;
        if let Some(entry) = egg_data.get_by_species(species_id) {
            if json {
                println!("{}", serde_json::to_string_pretty(&entry)?);
            } else {
                println!(
                    "Egg Moves for {} ({})",
                    resolve(species_id, "SPECIES_"),
                    game_name
                );
                println!("===========================");
                if entry.move_ids.is_empty() {
                    println!("No egg moves.");
                } else {
                    for move_id in &entry.move_ids {
                        println!("  {}", resolve(*move_id, "MOVE_"));
                    }
                }
            }
        } else {
            println!("No egg move data found for species {}", species_id);
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&egg_data)?);
    } else {
        println!("Egg Move Data ({})", game_name);
        println!("========================");
        println!("Total species with egg moves: {}", egg_data.entries.len());
        for entry in &egg_data.entries {
            println!(
                "\n{} ({} moves):",
                resolve(entry.species_id, "SPECIES_"),
                entry.move_ids.len()
            );
            for move_id in &entry.move_ids {
                println!("  {}", resolve(*move_id, "MOVE_"));
            }
        }
    }

    Ok(())
}

fn load_egg_move_data(
    project_path: &PathBuf,
    family: GameFamily,
) -> Result<EggMoveData, Box<dyn std::error::Error>> {
    match family {
        GameFamily::HGSS => {
            let narc_path = project_path.join("data/data/kowaza.narc");
            if !narc_path.exists() {
                return Err("HGSS egg moves NARC not found (data/data/kowaza.narc)".into());
            }
            let narc = load_narc(&narc_path)?;
            let data = narc.members.first().ok_or("Empty kowaza.narc")?;
            let mut cursor = std::io::Cursor::new(data);
            Ok(EggMoveData::from_binary(&mut cursor)?)
        }
        GameFamily::Platinum | GameFamily::DP => {
            let overlay_path = project_path.join("overlay/overlay_0005.bin");
            if !overlay_path.exists() {
                return Err("Overlay 5 not found (DPPt egg moves are in ARM9 overlay 5)".into());
            }
            let overlay_data = std::fs::read(&overlay_path)?;
            let offset = match family {
                GameFamily::Platinum => 0x29222,
                GameFamily::DP => 0x20668,
                GameFamily::HGSS => unreachable!(),
            };
            let mut cursor = std::io::Cursor::new(&overlay_data[offset..]);
            Ok(EggMoveData::from_binary(&mut cursor)?)
        }
    }
}

fn load_narc(path: &PathBuf) -> Result<Narc, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    Ok(Narc::from_binary(&mut file)?)
}

fn resolve_id(
    input: &str,
    prefix: &str,
    symbols: &SymbolTable,
    game_strings: &GameStrings,
) -> Result<u16, String> {
    if let Ok(id) = input.parse::<u16>() {
        return Ok(id);
    }

    let name = if input.starts_with(prefix) {
        input.to_uppercase()
    } else {
        format!("{}{}", prefix, input.to_uppercase())
    };

    if let Some(v) = symbols.resolve_constant(&name) {
        return Ok(v as u16);
    }

    let lowercase_input = input.to_lowercase();
    match prefix {
        "SPECIES_" => game_strings
            .get_species_id(&lowercase_input)
            .ok_or_else(|| format!("Unknown species: {}", input)),
        "ITEM_" => game_strings
            .get_item_id(&lowercase_input)
            .ok_or_else(|| format!("Unknown item: {}", input)),
        "MOVE_" => game_strings
            .get_move_id(&lowercase_input)
            .ok_or_else(|| format!("Unknown move: {}", input)),
        _ => Err(format!(
            "Unknown {}: {}",
            prefix.trim_end_matches('_').to_lowercase(),
            input
        )),
    }
}

fn resolve_name(
    id: u16,
    prefix: &str,
    symbols: &SymbolTable,
    game_strings: &GameStrings,
) -> String {
    if let Some(name) = symbols.resolve_name(id as i64, prefix) {
        return name;
    }

    match prefix {
        "SPECIES_" => game_strings
            .get_species_name(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.to_string()),
        "ITEM_" => game_strings
            .get_item_name(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.to_string()),
        "MOVE_" => game_strings
            .get_move_name(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.to_string()),
        "ABILITY_" => game_strings
            .get_ability_name(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.to_string()),
        "TYPE_" => game_strings
            .get_type_name(id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.to_string()),
        _ => symbols
            .resolve_name(id as i64, prefix)
            .unwrap_or_else(|| id.to_string()),
    }
}
