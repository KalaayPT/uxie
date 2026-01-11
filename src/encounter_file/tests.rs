#[cfg(test)]
mod tests {
    use crate::c_parser::SymbolTable;
    use crate::encounter_file::binary::{BinaryEncounterFile, EncounterEntry, WaterEncounterEntry};
    use crate::encounter_file::json::JsonEncounterFile;
    use crate::game::GameFamily;
    use std::io::Cursor;

    #[test]
    fn test_encounter_binary_roundtrip_dppt() {
        let mut grass = Vec::new();
        for i in 0..12 {
            grass.push(EncounterEntry {
                level: i as u8,
                species: i as u32 + 1,
            });
        }

        let mut water = Vec::new();
        for i in 0..5 {
            water.push(WaterEncounterEntry {
                min_level: i as u8,
                max_level: i as u8 + 5,
                species: i as u32 + 100,
            });
        }

        let file = BinaryEncounterFile {
            walking_rate: 30,
            grass_encounters: grass,
            swarm_encounters: vec![1, 2],
            day_encounters: vec![3, 4],
            night_encounters: vec![5, 6],
            radar_encounters: vec![7, 8, 9, 10],
            form_encounter_rates: vec![0, 0, 0, 0, 0],
            unown_table_id: 0,
            dual_slot_ruby: vec![11, 12],
            dual_slot_sapphire: vec![13, 14],
            dual_slot_emerald: vec![15, 16],
            dual_slot_firered: vec![17, 18],
            dual_slot_leafgreen: vec![19, 20],
            surf_rate: 10,
            surf_encounters: water.clone(),
            old_rod_rate: 5,
            old_rod_encounters: water.clone(),
            good_rod_rate: 15,
            good_rod_encounters: water.clone(),
            super_rod_rate: 20,
            super_rod_encounters: water.clone(),
            rock_smash_rate: 0,
            rock_smash_encounters: Vec::new(),
            morning_encounters: Vec::new(),
        };

        let mut buffer = Vec::new();
        file.to_binary(&mut buffer, GameFamily::Platinum).unwrap();

        let mut reader = Cursor::new(&buffer);
        let decoded = BinaryEncounterFile::from_binary(&mut reader, GameFamily::Platinum).unwrap();

        assert_eq!(file, decoded);
    }

    #[test]
    fn test_encounter_json_resolution() {
        let mut symbols = SymbolTable::new();
        symbols.defines.insert("SPECIES_BULBASAUR".into(), 1);
        symbols.defines.insert("SPECIES_IVYSAUR".into(), 2);

        let json = r#"{
            "land_rate": 30,
            "land_encounters": [{"level": 5, "species": "SPECIES_BULBASAUR"}],
            "swarms": ["SPECIES_IVYSAUR", "2"],
            "day": [], "night": [], "radar": [],
            "rate_form0": 0, "rate_form1": 0, "rate_form2": 0, "rate_form3": 0, "rate_form4": 0,
            "unown_table": 0,
            "ruby": [], "sapphire": [], "emerald": [], "firered": [], "leafgreen": [],
            "surf_rate": 0, "surf_encounters": [],
            "old_rod_rate": 0, "old_rod_encounters": [],
            "good_rod_rate": 0, "good_rod_encounters": [],
            "super_rod_rate": 0, "super_rod_encounters": []
        }"#;

        let encounter: JsonEncounterFile = serde_json::from_str(json).unwrap();
        let bin = encounter.to_binary(&symbols, GameFamily::Platinum);

        assert_eq!(bin.grass_encounters[0].species, 1);
        assert_eq!(bin.swarm_encounters[0], 2);
        assert_eq!(bin.swarm_encounters[1], 2);
    }
}
