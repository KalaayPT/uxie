#[cfg(test)]
mod encounter_tests {
    use crate::c_parser::SymbolTable;
    use crate::encounter_file::binary::{BinaryEncounterFile, EncounterEntry, WaterEncounterEntry};
    use crate::encounter_file::json::JsonEncounterFile;
    use crate::game::GameFamily;
    use crate::narc::Narc;
    use crate::workspace::Workspace;
    use proptest::prelude::*;
    use std::io::{self, Cursor};
    use std::path::Path;

    fn load_encounter_from_project_root(
        project_root: &Path,
        family: GameFamily,
        id: u32,
    ) -> io::Result<BinaryEncounterFile> {
        let narc_path = match family {
            GameFamily::DP => project_root.join("data/fielddata/encountdata/d_enc_data.narc"),
            GameFamily::Platinum => {
                project_root.join("data/fielddata/encountdata/pl_enc_data.narc")
            }
            GameFamily::HGSS => project_root.join("data/a/0/3/7"),
        };

        if narc_path.exists() {
            let mut file = std::fs::File::open(&narc_path)?;
            let narc = Narc::from_binary(&mut file)?;
            let data = narc.members.get(id as usize).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Encounter ID {} out of range in {}",
                        id,
                        narc_path.display()
                    ),
                )
            })?;
            let mut reader = Cursor::new(data.as_slice());
            BinaryEncounterFile::from_binary(&mut reader, family)
        } else {
            let unpacked_path = project_root
                .join("unpacked/encounters")
                .join(format!("{:04}", id));
            let bin_data = std::fs::read(unpacked_path)?;
            let mut reader = Cursor::new(bin_data.as_slice());
            BinaryEncounterFile::from_binary(&mut reader, family)
        }
    }

    fn build_dppt_fixture() -> BinaryEncounterFile {
        let grass = core::array::from_fn(|i| EncounterEntry {
            level: (i + 1) as u8,
            species: i as u32 + 1,
        });

        let water: [WaterEncounterEntry; 5] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 1) as u8,
            max_level: (i + 6) as u8,
            species: i as u32 + 100,
        });

        BinaryEncounterFile {
            walking_rate: 30,
            grass_encounters: grass,
            swarm_encounters: [1, 2, 0, 0],
            day_encounters: [3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            night_encounters: [5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            radar_encounters: [7, 8, 9, 10],
            music_encounters: [0, 0, 0, 0],
            form_encounter_rates: [0, 0, 0, 0, 0],
            unown_table_id: 0,
            dual_slot_ruby: [11, 12],
            dual_slot_sapphire: [13, 14],
            dual_slot_emerald: [15, 16],
            dual_slot_firered: [17, 18],
            dual_slot_leafgreen: [19, 20],
            surf_rate: 10,
            surf_encounters: water,
            old_rod_rate: 5,
            old_rod_encounters: water,
            good_rod_rate: 15,
            good_rod_encounters: water,
            super_rod_rate: 20,
            super_rod_encounters: water,
            rock_smash_rate: 0,
            rock_smash_encounters: [WaterEncounterEntry::default(); 2],
            morning_encounters: [EncounterEntry::default(); 12],
        }
    }

    fn build_hgss_fixture() -> BinaryEncounterFile {
        let morning: [EncounterEntry; 12] = core::array::from_fn(|i| EncounterEntry {
            level: (i + 2) as u8,
            species: (i + 1) as u32,
        });

        let day: [u32; 12] = core::array::from_fn(|i| (i + 20) as u32);
        let night: [u32; 12] = core::array::from_fn(|i| (i + 40) as u32);
        let swarm = [100, 101, 102, 103];
        let radar = [200, 201, 202, 203];

        let surf: [WaterEncounterEntry; 5] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 1) as u8,
            max_level: (i + 6) as u8,
            species: (300 + i) as u32,
        });
        let rock: [WaterEncounterEntry; 2] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 1) as u8,
            max_level: (i + 4) as u8,
            species: (400 + i) as u32,
        });
        let old_rod: [WaterEncounterEntry; 5] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 2) as u8,
            max_level: (i + 5) as u8,
            species: (500 + i) as u32,
        });
        let good_rod: [WaterEncounterEntry; 5] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 3) as u8,
            max_level: (i + 6) as u8,
            species: (600 + i) as u32,
        });
        let super_rod: [WaterEncounterEntry; 5] = core::array::from_fn(|i| WaterEncounterEntry {
            min_level: (i + 4) as u8,
            max_level: (i + 7) as u8,
            species: (700 + i) as u32,
        });

        BinaryEncounterFile {
            walking_rate: 20,
            grass_encounters: morning,
            swarm_encounters: swarm,
            day_encounters: day,
            night_encounters: night,
            radar_encounters: [0, 0, 0, 0],
            music_encounters: radar,
            form_encounter_rates: [0; 5],
            unown_table_id: 0,
            dual_slot_ruby: [0; 2],
            dual_slot_sapphire: [0; 2],
            dual_slot_emerald: [0; 2],
            dual_slot_firered: [0; 2],
            dual_slot_leafgreen: [0; 2],
            surf_rate: 10,
            surf_encounters: surf,
            old_rod_rate: 5,
            old_rod_encounters: old_rod,
            good_rod_rate: 15,
            good_rod_encounters: good_rod,
            super_rod_rate: 20,
            super_rod_encounters: super_rod,
            rock_smash_rate: 12,
            rock_smash_encounters: rock,
            morning_encounters: morning,
        }
    }

    #[test]
    fn test_encounter_binary_roundtrip_dppt() {
        let file = build_dppt_fixture();

        let mut buffer = Vec::new();
        file.to_binary(&mut buffer, GameFamily::Platinum).unwrap();

        let mut reader = Cursor::new(&buffer);
        let decoded = BinaryEncounterFile::from_binary(&mut reader, GameFamily::Platinum).unwrap();

        assert_eq!(file, decoded);
    }

    #[test]
    fn test_encounter_binary_roundtrip_hgss() {
        let file = build_hgss_fixture();

        let mut buffer = Vec::new();
        file.to_binary(&mut buffer, GameFamily::HGSS).unwrap();

        let mut reader = Cursor::new(&buffer);
        let decoded = BinaryEncounterFile::from_binary(&mut reader, GameFamily::HGSS).unwrap();

        assert_eq!(file, decoded);
    }

    #[test]
    fn test_encounter_binary_rejects_hgss_rate_overflow() {
        let mut file = build_hgss_fixture();
        file.walking_rate = 300;

        let mut buffer = Vec::new();
        let err = file.to_binary(&mut buffer, GameFamily::HGSS).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("walking_rate"));
    }

    #[test]
    fn test_encounter_json_resolution() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("SPECIES_BULBASAUR".into(), 1);
        symbols.insert_define("SPECIES_IVYSAUR".into(), 2);

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
        let bin = encounter.to_binary(&symbols, GameFamily::Platinum).unwrap();

        assert_eq!(bin.grass_encounters[0].species, 1);
        assert_eq!(bin.swarm_encounters[0], 2);
        assert_eq!(bin.swarm_encounters[1], 2);
    }

    #[test]
    fn test_encounter_json_unresolved_species_returns_error_platinum() {
        let symbols = SymbolTable::new();
        let json = r#"{
            "land_rate": 30,
            "land_encounters": [{"level": 5, "species": "SPECIES_DOES_NOT_EXIST"}],
            "swarms": [],
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

        let err = encounter
            .to_binary(&symbols, GameFamily::Platinum)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("SPECIES_DOES_NOT_EXIST"));
    }

    #[test]
    fn test_encounter_json_unresolved_species_returns_error_hgss_music() {
        let symbols = SymbolTable::new();
        let json = r#"{
            "land_rate": 30,
            "land_encounters": [],
            "swarms": [],
            "day": [], "night": [], "radar": [],
            "rate_form0": 0, "rate_form1": 0, "rate_form2": 0, "rate_form3": 0, "rate_form4": 0,
            "unown_table": 0,
            "ruby": [], "sapphire": [], "emerald": [], "firered": [], "leafgreen": [],
            "surf_rate": 0, "surf_encounters": [],
            "old_rod_rate": 0, "old_rod_encounters": [],
            "good_rod_rate": 0, "good_rod_encounters": [],
            "super_rod_rate": 0, "super_rod_encounters": [],
            "music": ["SPECIES_DOES_NOT_EXIST"],
            "rock_smash_rate": 0,
            "rock_smash_encounters": [],
            "morning": []
        }"#;
        let encounter: JsonEncounterFile = serde_json::from_str(json).unwrap();

        let err = encounter.to_binary(&symbols, GameFamily::HGSS).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("SPECIES_DOES_NOT_EXIST"));
    }

    #[test]
    fn test_encounter_json_missing_hgss_rock_smash_rate_returns_error() {
        let symbols = SymbolTable::new();
        let json = r#"{
            "land_rate": 30,
            "land_encounters": [],
            "swarms": [],
            "day": [], "night": [], "radar": [],
            "rate_form0": 0, "rate_form1": 0, "rate_form2": 0, "rate_form3": 0, "rate_form4": 0,
            "unown_table": 0,
            "ruby": [], "sapphire": [], "emerald": [], "firered": [], "leafgreen": [],
            "surf_rate": 0, "surf_encounters": [],
            "old_rod_rate": 0, "old_rod_encounters": [],
            "good_rod_rate": 0, "good_rod_encounters": [],
            "super_rod_rate": 0, "super_rod_encounters": [],
            "music": [],
            "rock_smash_encounters": [],
            "morning": []
        }"#;
        let encounter: JsonEncounterFile = serde_json::from_str(json).unwrap();

        let err = encounter.to_binary(&symbols, GameFamily::HGSS).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("rock_smash_rate"));
    }

    #[test]
    #[ignore = "requires local Platinum DSPRE fixture via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_load_encounter_platinum_real_fixture() {
        let Some(project_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "encounter integration test (platinum)",
        ) else {
            return;
        };

        let ws = Workspace::open(&project_root).unwrap();
        assert_eq!(ws.family, GameFamily::Platinum);

        let bin = load_encounter_from_project_root(&project_root, GameFamily::Platinum, 0).unwrap();
        let json = JsonEncounterFile::from_binary(&bin, &ws.symbols, GameFamily::Platinum);
        let rebuilt = json.to_binary(&ws.symbols, GameFamily::Platinum).unwrap();

        assert_eq!(bin, rebuilt);
    }

    #[test]
    #[ignore = "requires local HGSS DSPRE fixture via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_load_encounter_hgss_real_fixture() {
        let Some(project_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            "encounter integration test (hgss)",
        ) else {
            return;
        };

        let ws = Workspace::open(&project_root).unwrap();
        assert_eq!(ws.family, GameFamily::HGSS);

        let bin = load_encounter_from_project_root(&project_root, GameFamily::HGSS, 0).unwrap();
        let json = JsonEncounterFile::from_binary(&bin, &ws.symbols, GameFamily::HGSS);
        let rebuilt = json.to_binary(&ws.symbols, GameFamily::HGSS).unwrap();

        assert_eq!(bin, rebuilt);
    }

    fn encounter_entry_strategy(
        species: impl Strategy<Value = u32>,
    ) -> impl Strategy<Value = EncounterEntry> {
        (1u8..=100u8, species).prop_map(|(level, species)| EncounterEntry { level, species })
    }

    fn water_entry_strategy(
        species: impl Strategy<Value = u32>,
    ) -> impl Strategy<Value = WaterEncounterEntry> {
        (1u8..=100u8, 1u8..=100u8, species).prop_map(|(a, b, species)| {
            let (min_level, max_level) = if a <= b { (a, b) } else { (b, a) };
            WaterEncounterEntry {
                min_level,
                max_level,
                species,
            }
        })
    }

    fn encounter_array_strategy<const N: usize>(
        species: impl Strategy<Value = u32> + Clone,
    ) -> impl Strategy<Value = [EncounterEntry; N]> {
        proptest::array::uniform(encounter_entry_strategy(species))
    }

    fn water_array_strategy<const N: usize>(
        species: impl Strategy<Value = u32> + Clone,
    ) -> impl Strategy<Value = [WaterEncounterEntry; N]> {
        proptest::array::uniform(water_entry_strategy(species))
    }

    fn species_array_strategy<const N: usize>(
        species: impl Strategy<Value = u32> + Clone,
    ) -> impl Strategy<Value = [u32; N]> {
        proptest::array::uniform(species)
    }

    fn dppt_encounter_strategy() -> impl Strategy<Value = BinaryEncounterFile> {
        let rate = 0u32..=255u32;
        let species_u16 = 0u32..=65535u32;

        let part1 = (
            rate.clone(),
            encounter_array_strategy::<12>(species_u16.clone()),
            species_array_strategy::<2>(species_u16.clone()),
            species_array_strategy::<2>(species_u16.clone()),
            species_array_strategy::<2>(species_u16.clone()),
            species_array_strategy::<4>(species_u16.clone()),
            species_array_strategy::<5>(any::<u32>()),
            any::<u32>(),
            species_array_strategy::<2>(any::<u32>()),
            species_array_strategy::<2>(any::<u32>()),
            species_array_strategy::<2>(any::<u32>()),
        );
        let part2 = (
            species_array_strategy::<2>(any::<u32>()),
            species_array_strategy::<2>(any::<u32>()),
            rate.clone(),
            water_array_strategy::<5>(species_u16.clone()),
            rate.clone(),
            water_array_strategy::<5>(species_u16.clone()),
            rate.clone(),
            water_array_strategy::<5>(species_u16.clone()),
            rate,
            water_array_strategy::<5>(species_u16),
        );

        (part1, part2).prop_map(
            |(
                (
                    walking_rate,
                    grass_encounters,
                    swarm_pair,
                    day_pair,
                    night_pair,
                    radar_encounters,
                    form_encounter_rates,
                    unown_table_id,
                    dual_slot_ruby,
                    dual_slot_sapphire,
                    dual_slot_emerald,
                ),
                (
                    dual_slot_firered,
                    dual_slot_leafgreen,
                    surf_rate,
                    surf_encounters,
                    old_rod_rate,
                    old_rod_encounters,
                    good_rod_rate,
                    good_rod_encounters,
                    super_rod_rate,
                    super_rod_encounters,
                ),
            )| {
                BinaryEncounterFile {
                    walking_rate,
                    grass_encounters,
                    swarm_encounters: [swarm_pair[0], swarm_pair[1], 0, 0],
                    day_encounters: [day_pair[0], day_pair[1], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    night_encounters: [night_pair[0], night_pair[1], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    radar_encounters,
                    music_encounters: [0, 0, 0, 0],
                    form_encounter_rates,
                    unown_table_id,
                    dual_slot_ruby,
                    dual_slot_sapphire,
                    dual_slot_emerald,
                    dual_slot_firered,
                    dual_slot_leafgreen,
                    surf_rate,
                    surf_encounters,
                    old_rod_rate,
                    old_rod_encounters,
                    good_rod_rate,
                    good_rod_encounters,
                    super_rod_rate,
                    super_rod_encounters,
                    rock_smash_rate: 0,
                    rock_smash_encounters: [WaterEncounterEntry::default(); 2],
                    morning_encounters: [EncounterEntry::default(); 12],
                }
            },
        )
    }

    fn hgss_encounter_strategy() -> impl Strategy<Value = BinaryEncounterFile> {
        let part1 = (
            0u32..=255,
            0u32..=255,
            0u32..=255,
            0u32..=255,
            0u32..=255,
            0u32..=255,
            encounter_array_strategy::<12>(0u32..=65535),
            species_array_strategy::<12>(0u32..=65535),
        );
        let part2 = (
            species_array_strategy::<12>(0u32..=65535),
            species_array_strategy::<4>(0u32..=65535),
            species_array_strategy::<4>(0u32..=65535),
            water_array_strategy::<5>(0u32..=65535),
            water_array_strategy::<2>(0u32..=65535),
            water_array_strategy::<5>(0u32..=65535),
            water_array_strategy::<5>(0u32..=65535),
            water_array_strategy::<5>(0u32..=65535),
        );

        (part1, part2).prop_map(
            |(
                (
                    walking_rate,
                    surf_rate,
                    rock_smash_rate,
                    old_rod_rate,
                    good_rod_rate,
                    super_rod_rate,
                    morning_encounters,
                    day_encounters,
                ),
                (
                    night_encounters,
                    swarm_encounters,
                    radar_encounters,
                    surf_encounters,
                    rock_smash_encounters,
                    old_rod_encounters,
                    good_rod_encounters,
                    super_rod_encounters,
                ),
            )| {
                BinaryEncounterFile {
                    walking_rate,
                    grass_encounters: morning_encounters,
                    swarm_encounters,
                    day_encounters,
                    night_encounters,
                    radar_encounters: [0, 0, 0, 0],
                    music_encounters: radar_encounters,
                    form_encounter_rates: [0; 5],
                    unown_table_id: 0,
                    dual_slot_ruby: [0; 2],
                    dual_slot_sapphire: [0; 2],
                    dual_slot_emerald: [0; 2],
                    dual_slot_firered: [0; 2],
                    dual_slot_leafgreen: [0; 2],
                    surf_rate,
                    surf_encounters,
                    old_rod_rate,
                    old_rod_encounters,
                    good_rod_rate,
                    good_rod_encounters,
                    super_rod_rate,
                    super_rod_encounters,
                    rock_smash_rate,
                    rock_smash_encounters,
                    morning_encounters,
                }
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 48,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_encounter_dppt_roundtrip(file in dppt_encounter_strategy()) {
            let mut buf = Vec::new();
            file.to_binary(&mut buf, GameFamily::Platinum).unwrap();
            let mut reader = Cursor::new(buf);
            let decoded = BinaryEncounterFile::from_binary(&mut reader, GameFamily::Platinum).unwrap();
            prop_assert_eq!(file, decoded);
        }

        #[test]
        fn prop_encounter_hgss_roundtrip(file in hgss_encounter_strategy()) {
            let mut buf = Vec::new();
            file.to_binary(&mut buf, GameFamily::HGSS).unwrap();
            let mut reader = Cursor::new(buf);
            let decoded = BinaryEncounterFile::from_binary(&mut reader, GameFamily::HGSS).unwrap();
            prop_assert_eq!(file, decoded);
        }
    }
}
