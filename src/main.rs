use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uxie::{
    SymbolTable,
    MapHeaderJson,
    Workspace,
    MapHeader,
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
    Header {
        #[arg(default_value = ".")]
        path: PathBuf,

        #[arg(long)]
        json: bool,
    },

    Map {
        id: u16,

        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        #[arg(short, long)]
        decomp: Option<PathBuf>,

        #[arg(long)]
        json: bool,
    },

    Event {
        id: u32,

        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        #[arg(short, long)]
        decomp: Option<PathBuf>,

        #[arg(long)]
        json: bool,
    },

    Encounter {
        id: u32,

        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        #[arg(short, long)]
        decomp: Option<PathBuf>,

        #[arg(long)]
        json: bool,
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
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Header { path, json } => cmd_header(&path, json),
        Commands::Map { id, project, decomp, json } => cmd_map(id, &project, decomp, json),
        Commands::Event { id, project, decomp, json } => cmd_event(id, &project, decomp, json),
        Commands::Encounter { id, project, decomp, json } => cmd_encounter(id, &project, decomp, json),
        Commands::Symbols { path, only_defines, only_enums, json } => {
            cmd_parse_header(&path, only_defines, only_enums, json)
        }
        Commands::ResolveScript { path, decomp } => cmd_resolve_script(&path, &decomp),
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
    let mut ws = Workspace::open(path)?;
    if let Some(d) = decomp {
        ws.symbols.load_headers_from_dir(d.join("include/constants"))?;
        ws.symbols.load_headers_from_dir(d.join("generated"))?;
    }

    let header = ws.provider.get_map_header(id)?;

    if json {
        let json_header = MapHeaderJson::from_binary(&header, &ws.symbols);
        println!("{}", serde_json::to_string_pretty(&json_header)?);
    } else {
        print_map_header(&header, id);
    }
    Ok(())
}

fn cmd_event(
    id: u32,
    project_path: &PathBuf,
    decomp: Option<PathBuf>,
    _json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ws = Workspace::open(project_path)?;
    if let Some(d) = decomp {
        ws.symbols.load_headers_from_dir(d.join("include/constants"))?;
        ws.symbols.load_headers_from_dir(d.join("generated"))?;
    }

    let dspre = uxie::DspreProject::open(project_path)?;
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
    let mut ws = Workspace::open(project_path)?;
    if let Some(d) = decomp {
        ws.symbols.load_headers_from_dir(d.join("include/constants"))?;
        ws.symbols.load_headers_from_dir(d.join("generated"))?;
    }

    let narc_path = match ws.family {
        uxie::GameFamily::DP => project_path.join("data/fielddata/encountdata/d_enc_data.narc"),
        uxie::GameFamily::Platinum => project_path.join("data/fielddata/encountdata/pl_enc_data.narc"),
        uxie::GameFamily::HGSS => project_path.join("data/a/0/3/7"),
    };

    let bin_data = if narc_path.exists() {
        let mut file = std::fs::File::open(narc_path)?;
        let narc = uxie::narc::Narc::from_binary(&mut file)?;
        narc.members.get(id as usize).cloned().ok_or("Encounter ID out of range in NARC")?
    } else {
        let unpacked_path = project_path.join("unpacked/encounters").join(format!("{:04}", id));
        if unpacked_path.exists() {
            std::fs::read(unpacked_path)?
        } else {
            return Err("Encounter data not found (tried NARC and unpacked/encounters)".into());
        }
    };

    let mut reader = std::io::Cursor::new(bin_data);
    let bin = uxie::encounter_file::BinaryEncounterFile::from_binary(&mut reader, ws.family)?;
    let json = uxie::JsonEncounterFile::from_binary(&bin, &ws.symbols, ws.family);
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

    let is_txt = path.extension().and_then(|s| s.to_str()) == Some("txt");

    if is_txt {
        symbols.load_list_file_str(&content)?;
    } else {
        symbols.load_header_str(&content)?;
    }

    if json {
        #[derive(serde::Serialize)]
        struct HeaderOutput {
            defines: Option<Vec<uxie::c_parser::defines::CDefine>>,
            enums: Option<std::collections::HashMap<String, Vec<(String, Option<i64>)>>>,
        }
        
        let defines = if !only_enums {
            if is_txt {
                Some(symbols.defines.iter().map(|(n, v)| {
                    uxie::c_parser::defines::CDefine {
                        name: n.clone(),
                        value: v.to_string(),
                        resolved: Some(*v),
                    }
                }).collect())
            } else {
                Some(uxie::c_parser::defines::parse_and_resolve_defines(&content))
            }
        } else {
            None
        };

        let output = HeaderOutput {
            defines,
            enums: if !only_defines { Some(symbols.enums.clone()) } else { None },
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if !only_enums {
            if is_txt {
                let mut defs: Vec<_> = symbols.defines.iter().collect();
                defs.sort_by_key(|(n, _)| *n);
                if !defs.is_empty() {
                    println!("Symbols (from .txt):");
                    for (name, value) in defs {
                        println!("  {} = {}", name, value);
                    }
                }
            } else {
                let defs = uxie::c_parser::defines::parse_and_resolve_defines(&content);
                if !defs.is_empty() {
                    println!("Defines:");
                    for d in defs {
                        if let Some(resolved) = d.resolved {
                            println!("  #define {} {} (= {})", d.name, d.value, resolved);
                        } else {
                            println!("  #define {} {}", d.name, d.value);
                        }
                    }
                }
            }
        }
        if !only_defines && !symbols.enums.is_empty() {
            println!("\nEnums:");
            for (name, variants) in &symbols.enums {
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

fn cmd_resolve_script(path: &PathBuf, decomp_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let ws = Workspace::open_decomp(decomp_path)?;
    println!("{}", ws.resolve_script_symbols(&content));
    Ok(())
}

fn print_map_header(header: &MapHeader, id: u16) {
    let (game_name, divider_len) = match header {
        MapHeader::DP(_) => ("Diamond/Pearl", 29),
        MapHeader::Pt(_) => ("Platinum", 25),
        MapHeader::HGSS(_) => ("HeartGold/SoulSilver", 36),
    };
    
    println!("Map Header {} ({})", id, game_name);
    println!("{}", "=".repeat(divider_len));
    
    println!("Area Data ID:    {}", header.area_data_id());
    println!("Matrix ID:       {}", header.matrix_id());
    println!("Script File ID:  {}", header.script_file_id());
    println!("Level Script ID: {}", header.level_script_id());
    println!("Text Archive ID: {}", header.text_archive_id());
    
    match header {
        MapHeader::DP(h) => {
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
        MapHeader::Pt(h) => {
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
        MapHeader::HGSS(h) => {
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
    }
}
