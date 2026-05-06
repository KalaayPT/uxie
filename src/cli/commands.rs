use super::paths::{
    egg_move_narc_path, egg_move_overlay_path, encounter_narc_path, evolution_narc_path,
    family_name, item_narc_path, learnset_narc_path, move_narc_path, personal_narc_path,
};
use super::render::{print_encounter_file, print_event_file, print_map_header};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uxie::{
    BinaryEncounterFile, DspreProject, EggMoveData, EvolutionData, EvolutionMethod, GameFamily,
    GameStrings, ItemData, JsonEncounterFile, LearnsetData, MapHeaderJson, MoveData, Narc,
    PersonalData, RomHeader, SymbolTable, Workspace, decode_text_archives, encode_text_archives,
    load_dspre_trainer,
};

pub fn cmd_header(path: &Path, json: bool) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn cmd_map(
    id: u16,
    path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(path, decomp.as_deref())?;

    let header = ws.provider.get_map_header(id)?;

    if json {
        let json_header = MapHeaderJson::from_binary(&header, &ws.symbols);
        println!("{}", serde_json::to_string_pretty(&json_header)?);
    } else {
        print_map_header(&header, id, &ws.symbols, &ws);
    }
    Ok(())
}

pub fn cmd_event(
    id: u32,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let dspre = DspreProject::open(project_path)?;
    let bin_event = dspre.load_event_file(id)?;
    let event = uxie::JsonEventFile::from_binary(&bin_event, &ws.symbols);

    if json {
        println!("{}", serde_json::to_string_pretty(&event)?);
    } else {
        print_event_file(&event, id);
    }
    Ok(())
}

pub fn cmd_encounter(
    id: u32,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let narc_path = encounter_narc_path(project_path, ws.family);

    let bin = if narc_path.exists() {
        let mut file = std::fs::File::open(narc_path)?;
        let narc = uxie::narc::Narc::from_binary(&mut file)?;
        let data = narc.member(id as usize)?;
        let mut reader = std::io::Cursor::new(data);
        BinaryEncounterFile::from_binary(&mut reader, ws.family)?
    } else {
        let unpacked_path = project_path
            .join("unpacked/encounters")
            .join(format!("{:04}", id));
        let bin_data = if unpacked_path.exists() {
            std::fs::read(unpacked_path)?
        } else {
            return Err("Encounter data not found (tried NARC and unpacked/encounters)".into());
        };
        let mut reader = std::io::Cursor::new(bin_data.as_slice());
        BinaryEncounterFile::from_binary(&mut reader, ws.family)?
    };

    let encounter = JsonEncounterFile::from_binary(&bin, &ws.symbols, ws.family);

    if json {
        println!("{}", serde_json::to_string_pretty(&encounter)?);
    } else {
        print_encounter_file(&encounter, id, ws.family);
    }

    Ok(())
}

pub fn cmd_parse_header(path: &Path, json: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut constants: Vec<_> = symbols.get_all_defines().into_iter().collect();
    constants.sort_by(|(a, _), (b, _)| a.cmp(b));

    if json {
        #[derive(serde::Serialize)]
        struct ConstantOutput {
            name: String,
            value: i64,
        }

        #[derive(serde::Serialize)]
        struct HeaderOutput {
            constants: Vec<ConstantOutput>,
        }

        let output = HeaderOutput {
            constants: constants
                .iter()
                .map(|(name, value)| ConstantOutput {
                    name: name.clone(),
                    value: *value,
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if !constants.is_empty() {
        println!("Constants:");
        for (name, value) in &constants {
            println!("  {} = {}", name, value);
        }
    }

    Ok(())
}

pub fn cmd_resolve_script(
    path: &Path,
    decomp_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let ws = Workspace::open(decomp_path)?;
    let symbols = ws.collect_constants_for_file(path)?;
    println!("{}", ws.resolve_script_symbols_with(&content, &symbols));
    Ok(())
}

pub fn cmd_personal(
    id: &str,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc = load_narc(&personal_narc_path(project_path, ws.family))?;

    let data = narc.member(id as usize)?;

    let mut cursor = std::io::Cursor::new(data);
    let personal = PersonalData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&personal)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Personal Data {} ({})", id, family_name(ws.family));
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

pub fn cmd_move(
    id: &str,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let id = resolve_id(id, "MOVE_", &ws.symbols, &ws.game_strings)?;

    let narc = load_narc(&move_narc_path(project_path, ws.family))?;

    let data = narc.member(id as usize)?;

    let mut cursor = std::io::Cursor::new(data);
    let move_data = MoveData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&move_data)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Move Data {} ({})", id, family_name(ws.family));
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

pub fn cmd_item(
    id: &str,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let id = resolve_id(id, "ITEM_", &ws.symbols, &ws.game_strings)?;

    let narc = load_narc(&item_narc_path(project_path, ws.family))?;

    let data = narc.member(id as usize)?;

    let mut cursor = std::io::Cursor::new(data);
    let item = ItemData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&item)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Item Data {} ({})", id, family_name(ws.family));
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

pub fn cmd_trainer(
    id: u16,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;
    let trainer = load_dspre_trainer(project_path, ws.family, id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&trainer)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!("Trainer Data {} ({})", id, family_name(ws.family));
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

pub fn cmd_evolution(
    id: &str,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc = load_narc(&evolution_narc_path(project_path, ws.family))?;

    let data = narc.member(id as usize)?;

    let mut cursor = std::io::Cursor::new(data);
    let evo = EvolutionData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&evo)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!(
            "Evolution Data for {} ({})",
            resolve(id, "SPECIES_"),
            family_name(ws.family)
        );
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

pub fn cmd_learnset(
    id: &str,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

    let id = resolve_id(id, "SPECIES_", &ws.symbols, &ws.game_strings)?;

    let narc = load_narc(&learnset_narc_path(project_path, ws.family))?;

    let data = narc.member(id as usize)?;

    let mut cursor = std::io::Cursor::new(data);
    let learnset = LearnsetData::from_binary(&mut cursor)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&learnset)?);
    } else {
        let gs = &ws.game_strings;
        let symbols = &ws.symbols;
        let resolve = |val: u16, prefix: &str| -> String { resolve_name(val, prefix, symbols, gs) };

        println!(
            "Learnset for {} ({})",
            resolve(id, "SPECIES_"),
            family_name(ws.family)
        );
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

pub fn cmd_egg_moves(
    species: Option<String>,
    project_path: &Path,
    decomp: Option<PathBuf>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws = open_workspace_with_decomp(project_path, decomp.as_deref())?;

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

pub fn open_workspace_with_decomp(
    project_path: &Path,
    decomp: Option<&Path>,
) -> Result<Workspace, Box<dyn std::error::Error>> {
    let mut ws = Workspace::open(project_path)?;
    if let Some(d) = decomp {
        let mut symbols = (*ws.symbols).clone();
        load_symbols_from_decomp(&mut symbols, d)?;
        ws.symbols = Arc::new(symbols);
    }
    Ok(ws)
}

pub fn load_symbols_from_decomp(
    symbols: &mut SymbolTable,
    d: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let d_str = d.to_string_lossy();
    if d_str.starts_with("http") || d_str.contains("github.com") {
        let mut base = if d_str.starts_with("git@github.com:") {
            let repo = d_str.replace("git@github.com:", "").replace(".git", "");
            format!("https://raw.githubusercontent.com/{}/master/", repo)
        } else if d_str.contains("github.com") && !d_str.contains("raw.githubusercontent.com") {
            let repo_path = d_str
                .find("github.com/")
                .map_or(d_str.as_ref(), |pos| &d_str[pos + 11..]);
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
            return Err(std::io::Error::other(format!(
                "No symbol source files found under {} (checked include/constants, generated, build/generated)",
                d.display()
            ))
            .into());
        }
    }
    Ok(())
}

fn load_egg_move_data(
    project_path: &Path,
    family: GameFamily,
) -> Result<EggMoveData, Box<dyn std::error::Error>> {
    let overlay_path = egg_move_overlay_path(project_path, family);
    if overlay_path.exists() {
        let overlay_data = std::fs::read(&overlay_path)?;
        let offset = match family {
            GameFamily::Platinum => 0x29222,
            GameFamily::DP => 0x20668,
            GameFamily::HGSS => unreachable!(),
        };
        let mut cursor = std::io::Cursor::new(&overlay_data[offset..]);
        return Ok(EggMoveData::from_binary(&mut cursor)?);
    }

    let narc_path = egg_move_narc_path(project_path, family);
    if !narc_path.exists() {
        return Err("Egg move data not found for this game family".into());
    }

    let narc = load_narc(&narc_path)?;
    let data = narc.first_member()?;
    let mut cursor = std::io::Cursor::new(data);
    Ok(EggMoveData::from_binary(&mut cursor)?)
}

fn load_narc(path: &Path) -> Result<Narc, Box<dyn std::error::Error>> {
    Ok(Narc::open(path)?)
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

pub fn cmd_text_decode(
    binary_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    decode_text_archives(binary_dir, output_dir)?;
    println!(
        "Decoded text archives from {} to {}",
        binary_dir.display(),
        output_dir.display()
    );
    Ok(())
}

pub fn cmd_text_encode(
    source_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    encode_text_archives(source_dir, output_dir)?;
    println!(
        "Encoded text archives from {} to {}",
        source_dir.display(),
        output_dir.display()
    );
    Ok(())
}
