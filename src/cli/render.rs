use uxie::encounter_file::json::WaterEncounterEntryJson;
use uxie::{GameFamily, JsonEncounterFile, JsonEventFile, MapHeader, SymbolTable, Workspace};

pub fn print_event_file(event: &JsonEventFile, id: u32) {
    println!("Event File {}", id);
    println!("============");
    println!("BG Events:      {}", event.bg_events.len());
    println!("Object Events:  {}", event.object_events.len());
    println!("Warp Events:    {}", event.warp_events.len());
    println!("Coord Events:   {}", event.coord_events.len());

    if !event.bg_events.is_empty() {
        println!("\nBackground Events");
        for (i, bg) in event.bg_events.iter().enumerate() {
            println!(
                "  #{i}: type={} script={} pos=({}, {}, {}) facing={}",
                bg.event_type,
                bg.script,
                bg.x,
                bg.y,
                bg.z,
                bg.player_facing_dir.as_deref().unwrap_or("None")
            );
        }
    }

    if !event.object_events.is_empty() {
        println!("\nObject Events");
        for (i, obj) in event.object_events.iter().enumerate() {
            println!(
                "  #{i}: {} gfx={} script={} pos=({}, {}, {})",
                obj.id, obj.graphics_id, obj.script, obj.x, obj.y, obj.z
            );
        }
    }

    if !event.warp_events.is_empty() {
        println!("\nWarp Events");
        for (i, warp) in event.warp_events.iter().enumerate() {
            println!(
                "  #{i}: pos=({}, {}) -> {}:{}",
                warp.x, warp.z, warp.dest_header_id, warp.dest_warp_id
            );
        }
    }

    if !event.coord_events.is_empty() {
        println!("\nCoord Events");
        for (i, coord) in event.coord_events.iter().enumerate() {
            println!(
                "  #{i}: script={} area=({}, {}) {}x{} y={} var={} value={}",
                coord.script,
                coord.x,
                coord.z,
                coord.width,
                coord.length,
                coord.y,
                coord.var.as_deref().unwrap_or("None"),
                coord.value
            );
        }
    }
}

pub fn print_encounter_file(encounter: &JsonEncounterFile, id: u32, family: GameFamily) {
    let join_or_none = |values: &[String]| {
        if values.is_empty() {
            "None".to_string()
        } else {
            values.join(", ")
        }
    };

    println!("Encounter File {} ({family:?})", id);
    println!("=======================");
    println!("Land Rate:      {}", encounter.land_rate);
    println!("Surf Rate:      {}", encounter.surf_rate);
    println!("Old Rod Rate:   {}", encounter.old_rod_rate);
    println!("Good Rod Rate:  {}", encounter.good_rod_rate);
    println!("Super Rod Rate: {}", encounter.super_rod_rate);
    println!(
        "Form Rates:     [{}, {}, {}, {}, {}]",
        encounter.rate_form0,
        encounter.rate_form1,
        encounter.rate_form2,
        encounter.rate_form3,
        encounter.rate_form4
    );
    println!("Unown Table:    {}", encounter.unown_table);

    if !encounter.land_encounters.is_empty() {
        println!("\nLand Encounters");
        for (i, e) in encounter.land_encounters.iter().enumerate() {
            println!("  #{i}: {} @ lv{}", e.species, e.level);
        }
    }

    println!("\nSpecial Slots");
    println!("  Swarms:    {}", join_or_none(&encounter.swarms));
    println!("  Day:       {}", join_or_none(&encounter.day));
    println!("  Night:     {}", join_or_none(&encounter.night));
    println!("  Radar:     {}", join_or_none(&encounter.radar));
    println!("  Ruby:      {}", join_or_none(&encounter.ruby));
    println!("  Sapphire:  {}", join_or_none(&encounter.sapphire));
    println!("  Emerald:   {}", join_or_none(&encounter.emerald));
    println!("  FireRed:   {}", join_or_none(&encounter.firered));
    println!("  LeafGreen: {}", join_or_none(&encounter.leafgreen));

    let print_water = |label: &str, entries: &[WaterEncounterEntryJson]| {
        if entries.is_empty() {
            println!("\n{label}: None");
            return;
        }
        println!("\n{label}");
        for (i, e) in entries.iter().enumerate() {
            println!("  #{i}: {} @ lv{}-{}", e.species, e.level_min, e.level_max);
        }
    };

    print_water("Surf Encounters", &encounter.surf_encounters);
    print_water("Old Rod Encounters", &encounter.old_rod_encounters);
    print_water("Good Rod Encounters", &encounter.good_rod_encounters);
    print_water("Super Rod Encounters", &encounter.super_rod_encounters);

    if family == GameFamily::HGSS {
        println!(
            "\nRock Smash Rate: {}",
            encounter.rock_smash_rate.unwrap_or_default()
        );
        if let Some(rock_smash) = &encounter.rock_smash_encounters {
            print_water("Rock Smash Encounters", rock_smash);
        }

        if let Some(morning) = &encounter.morning {
            if morning.is_empty() {
                println!("\nMorning Encounters: None");
            } else {
                println!("\nMorning Encounters");
                for (i, e) in morning.iter().enumerate() {
                    println!("  #{i}: {} @ lv{}", e.species, e.level);
                }
            }
        }
    }
}

pub fn print_map_header(header: &MapHeader, id: u16, symbols: &SymbolTable, ws: &Workspace) {
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
