use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uxie::{
    c_parser::{parse_defines, parse_enum},
    ds_rom::DsRomProject,
    game::GameFamily,
    map_header::{MapHeader, MapHeaderDP, MapHeaderPt, MapHeaderHGSS},
    provider::{Arm9Provider, DataProvider},
    RomHeader,
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

#[derive(Subcommand)]
enum Commands {
    /// Read and display ROM header information
    RomHeader {
        /// Path to header.bin, header.yaml, or ds-rom project directory
        #[arg(short, long)]
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Read map header from ARM9 binary
    MapHeader {
        /// Map header ID (0-based index)
        id: u16,

        /// Path to arm9.bin file
        #[arg(short, long)]
        arm9: PathBuf,

        /// Game family: dp, pt, or hgss
        #[arg(short, long)]
        game: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Parse C enum definitions from a header file
    ParseEnum {
        /// Path to the C header file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Parse #define constants from a header file
    ParseDefines {
        /// Path to the C header file
        path: PathBuf,

        /// Filter by prefix (e.g., "MAP_" to only show MAP_* defines)
        #[arg(short, long)]
        prefix: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Find text archive ID for a script file
    ScriptText {
        /// Script file ID
        script_id: u16,

        /// Path to arm9.bin file
        #[arg(short, long)]
        arm9: PathBuf,

        /// Game family: dp, pt, or hgss
        #[arg(short, long)]
        game: String,
    },

    /// Read ds-rom extracted project info
    DsRom {
        /// Path to config.yaml or project directory
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn parse_game_family(s: &str) -> Result<GameFamily, String> {
    match s.to_lowercase().as_str() {
        "dp" | "diamond" | "pearl" => Ok(GameFamily::DP),
        "pt" | "platinum" => Ok(GameFamily::Platinum),
        "hgss" | "heartgold" | "soulsilver" => Ok(GameFamily::HGSS),
        _ => Err(format!(
            "Unknown game family '{}'. Use: dp, pt, or hgss",
            s
        )),
    }
}

fn get_header_table_config(family: GameFamily) -> (u64, usize) {
    match family {
        GameFamily::DP => (0xE4B24, 559),
        GameFamily::Platinum => (0xE601C, 559),
        GameFamily::HGSS => (0xF6BE0, 540),
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::RomHeader { path, json } => cmd_rom_header(&path, json),
        Commands::MapHeader { id, arm9, game, json } => cmd_map_header(id, &arm9, &game, json),
        Commands::ParseEnum { path, json } => cmd_parse_enum(&path, json),
        Commands::ParseDefines { path, prefix, json } => {
            cmd_parse_defines(&path, prefix.as_deref(), json)
        }
        Commands::ScriptText { script_id, arm9, game } => cmd_script_text(script_id, &arm9, &game),
        Commands::DsRom { path, json } => cmd_ds_rom(&path, json),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_rom_header(path: &PathBuf, json: bool) -> Result<(), Box<dyn std::error::Error>> {
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

        if header.arm9_rom_offset.is_some() || header.arm9_size.is_some() {
            println!();
            if let Some(offset) = header.arm9_rom_offset {
                println!("ARM9 Offset: 0x{:08X}", offset);
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
        }

        if header.fnt_offset.is_some() {
            println!();
            if let (Some(fnt_off), Some(fnt_sz)) = (header.fnt_offset, header.fnt_size) {
                println!("FNT Offset:  0x{:08X} (size: 0x{:X})", fnt_off, fnt_sz);
            }
            if let (Some(fat_off), Some(fat_sz)) = (header.fat_offset, header.fat_size) {
                println!("FAT Offset:  0x{:08X} (size: 0x{:X})", fat_off, fat_sz);
            }
        }

        if let Some(crc) = header.header_crc {
            println!();
            println!("Header CRC:  0x{:04X}", crc);
        }
    }

    Ok(())
}

fn cmd_map_header(
    id: u16,
    arm9_path: &PathBuf,
    game_str: &str,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let family = parse_game_family(game_str)?;
    let (offset, count) = get_header_table_config(family);

    if id as usize >= count {
        return Err(format!("Map ID {} out of range (max: {})", id, count - 1).into());
    }

    let provider = Arm9Provider::new(arm9_path, offset, count, family);
    let header = provider.get_map_header(id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&header)?);
    } else {
        match header {
            MapHeader::DP(h) => print_map_header_dp(&h, id),
            MapHeader::Pt(h) => print_map_header_pt(&h, id),
            MapHeader::HGSS(h) => print_map_header_hgss(&h, id),
        }
    }

    Ok(())
}

fn print_map_header_dp(h: &MapHeaderDP, id: u16) {
    println!("Map Header {} (Diamond/Pearl)", id);
    println!("=============================");
    println!("Area Data ID:    {}", h.area_data_id);
    println!("Matrix ID:       {}", h.matrix_id);
    println!("Script File ID:  {}", h.script_file_id);
    println!("Level Script ID: {}", h.level_script_id);
    println!("Text Archive ID: {}", h.text_archive_id);
    println!("Music Day:       {}", h.music_day_id);
    println!("Music Night:     {}", h.music_night_id);
    println!("Wild Pokemon:    {}", h.wild_pokemon);
    println!("Event File ID:   {}", h.event_file_id);
    println!("Location Name:   {}", h.location_name);
    println!("Weather:         {}", h.weather_id);
    println!("Camera:          {}", h.camera_angle_id);
    println!("Battle BG:       {}", h.battle_background);
    println!("Flags:           0x{:02X}", h.flags);
}

fn print_map_header_pt(h: &MapHeaderPt, id: u16) {
    println!("Map Header {} (Platinum)", id);
    println!("=========================");
    println!("Area Data ID:    {}", h.area_data_id);
    println!("Matrix ID:       {}", h.matrix_id);
    println!("Script File ID:  {}", h.script_file_id);
    println!("Level Script ID: {}", h.level_script_id);
    println!("Text Archive ID: {}", h.text_archive_id);
    println!("Music Day:       {}", h.music_day_id);
    println!("Music Night:     {}", h.music_night_id);
    println!("Wild Pokemon:    {}", h.wild_pokemon);
    println!("Event File ID:   {}", h.event_file_id);
    println!("Location Name:   {}", h.location_name);
    println!("Area Icon:       {}", h.area_icon);
    println!("Weather:         {}", h.weather_id);
    println!("Camera:          {}", h.camera_angle_id);
    println!("Battle BG:       {}", h.battle_background);
    println!("Flags:           0x{:02X}", h.flags);
}

fn print_map_header_hgss(h: &MapHeaderHGSS, id: u16) {
    println!("Map Header {} (HeartGold/SoulSilver)", id);
    println!("====================================");
    println!("Area Data ID:    {}", h.area_data_id);
    println!("Matrix ID:       {}", h.matrix_id);
    println!("Script File ID:  {}", h.script_file_id);
    println!("Level Script ID: {}", h.level_script_id);
    println!("Text Archive ID: {}", h.text_archive_id);
    println!("Music Day:       {}", h.music_day_id);
    println!("Music Night:     {}", h.music_night_id);
    println!("Wild Pokemon:    {}", h.wild_pokemon);
    println!("Event File ID:   {}", h.event_file_id);
    println!("Location Name:   {}", h.location_name);
    println!("Area Icon:       {}", h.area_icon);
    println!("Weather:         {}", h.weather_id);
    println!("Camera:          {}", h.camera_angle_id);
    println!("Worldmap:        ({}, {})", h.worldmap_x, h.worldmap_y);
    println!("Kanto:           {}", h.kanto_flag);
    println!("Battle BG:       {}", h.battle_background);
    println!("Flags:           0x{:02X}", h.flags);
}

fn cmd_parse_enum(path: &PathBuf, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    
    match parse_enum(&content) {
        Some(e) => {
            if json {
                #[derive(serde::Serialize)]
                struct EnumVariant {
                    name: String,
                    value: i64,
                }
                #[derive(serde::Serialize)]
                struct EnumOutput {
                    name: Option<String>,
                    variants: Vec<EnumVariant>,
                }
                let mut current = 0i64;
                let variants = e.variants.iter().map(|v| {
                    if let Some(val) = v.value {
                        current = val;
                    }
                    let variant = EnumVariant { name: v.name.clone(), value: current };
                    current += 1;
                    variant
                }).collect();
                let output = EnumOutput {
                    name: e.name.clone(),
                    variants,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                if let Some(name) = &e.name {
                    println!("enum {} {{", name);
                } else {
                    println!("enum {{");
                }
                let mut current = 0i64;
                for v in &e.variants {
                    if let Some(val) = v.value {
                        current = val;
                    }
                    println!("    {} = {},", v.name, current);
                    current += 1;
                }
                println!("}}");
            }
        }
        None => {
            println!("No enum found in {}", path.display());
        }
    }

    Ok(())
}

fn cmd_parse_defines(
    path: &PathBuf,
    prefix: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut defs = parse_defines(&content);

    if let Some(prefix) = prefix {
        defs.retain(|d| d.name.starts_with(prefix));
    }

    if defs.is_empty() {
        if let Some(prefix) = prefix {
            println!("No defines found with prefix '{}' in {}", prefix, path.display());
        } else {
            println!("No defines found in {}", path.display());
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&defs)?);
    } else {
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        for d in &defs {
            println!("#define {} {}", d.name, d.value);
        }
    }

    Ok(())
}

fn cmd_script_text(
    script_id: u16,
    arm9_path: &PathBuf,
    game_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let family = parse_game_family(game_str)?;
    let (offset, count) = get_header_table_config(family);

    let provider = Arm9Provider::new(arm9_path, offset, count, family);

    match provider.get_text_archive_for_script(script_id)? {
        Some(text_id) => println!("Script {} uses text archive {}", script_id, text_id),
        None => println!("No map found using script {}", script_id),
    }

    Ok(())
}

fn cmd_ds_rom(path: &PathBuf, json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = if path.is_dir() {
        path.join("config.yaml")
    } else {
        path.clone()
    };

    let project = DsRomProject::open(&config_path)?;

    if json {
        #[derive(serde::Serialize)]
        struct ProjectInfo {
            title: String,
            gamecode: String,
            makercode: String,
            rom_version: u8,
            game: Option<String>,
            family: Option<String>,
            region: Option<String>,
            arm9_base: String,
            arm9_entry: String,
            sdk_version: String,
            arm9_bin: String,
            files_dir: String,
        }
        let info = ProjectInfo {
            title: project.header.game_title.clone(),
            gamecode: project.header.game_code.clone(),
            makercode: project.header.maker_code.clone(),
            rom_version: project.header.rom_version,
            game: project.game().map(|g| format!("{:?}", g)),
            family: project.game_family().map(|f| format!("{:?}", f)),
            region: project.header.region().map(|s| s.to_string()),
            arm9_base: format!("0x{:08X}", project.arm9_config.base_address),
            arm9_entry: format!("0x{:08X}", project.arm9_config.entry_function),
            sdk_version: project.arm9_config.sdk_version_string(),
            arm9_bin: project.arm9_bin_path().display().to_string(),
            files_dir: project.files_dir().display().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("ds-rom Project Information");
        println!("==========================");
        println!("Title:       {}", project.header.game_title);
        println!("Game Code:   {}", project.header.game_code);
        println!("Maker Code:  {}", project.header.maker_code);
        println!("ROM Version: {}", project.header.rom_version);

        if let Some(game) = project.game() {
            println!("Game:        {:?}", game);
            println!("Family:      {:?}", game.family());
        }

        if let Some(region) = project.header.region() {
            println!("Region:      {}", region);
        }

        println!();
        println!("ARM9 Base:   0x{:08X}", project.arm9_config.base_address);
        println!("ARM9 Entry:  0x{:08X}", project.arm9_config.entry_function);
        println!("SDK Version: {}", project.arm9_config.sdk_version_string());
        
        if project.arm9_config.compressed {
            println!("ARM9:        compressed");
        }
        if project.arm9_config.encrypted {
            println!("ARM9:        encrypted");
        }

        println!();
        println!("ARM9 Binary: {}", project.arm9_bin_path().display());
        println!("Files Dir:   {}", project.files_dir().display());
    }

    Ok(())
}
