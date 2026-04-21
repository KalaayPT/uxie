#[cfg(test)]
mod c_parser_tests {
    use crate::c_parser::{ConstantFamily, SourceManager, SymbolTable, canonicalize_constant_name};
    use crate::{
        GameFamily, TrainerData, TrainerFlags, TrainerProperties, load_all_dspre_trainers,
    };
    use proptest::prelude::*;
    use regex::Regex;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::thread;
    use tempfile::tempdir;

    fn create_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "{}", content).unwrap();
        path
    }

    fn load_symbol_table_from_real_dirs(
        root: &Path,
        dirs: &[&str],
        context: &str,
    ) -> Option<SymbolTable> {
        let mut table = SymbolTable::new();
        for rel_dir in dirs {
            let dir = root.join(rel_dir);
            if !dir.exists() {
                eprintln!(
                    "Skipping {}: fixture directory does not exist: {}",
                    context,
                    dir.display()
                );
                return None;
            }

            let loaded = table.load_headers_from_dir(&dir).unwrap();
            assert!(
                loaded > 0,
                "{} should contain at least one file",
                dir.display()
            );
        }

        Some(table)
    }

    fn assert_resolved_constants(table: &SymbolTable, expected: &[(&str, i64)], context: &str) {
        for &(name, value) in expected {
            assert_eq!(
                table.resolve_constant(name),
                Some(value),
                "{}: expected {} = {}",
                context,
                name,
                value
            );
        }
    }

    fn sample_trainer(
        family: GameFamily,
        trainer_class: u8,
        species: u16,
        level: u16,
    ) -> TrainerData {
        TrainerData {
            properties: TrainerProperties {
                flags: TrainerFlags::empty(),
                trainer_class,
                unknown: 0,
                party_count: 1,
                items: [0; 4],
                ai_flags: crate::AiFlags::BASIC,
                double_battle: 0,
            },
            party: vec![crate::PartyPokemon {
                difficulty: 7,
                gender_ability_override: 0,
                level,
                species,
                form: 0,
                held_item: None,
                moves: None,
                ball_seal: if family == GameFamily::DP {
                    None
                } else {
                    Some(0)
                },
            }],
        }
    }

    /// Load one symbol name per line (used for Platinum `generated/trainers.txt`).
    fn load_trainer_constants_txt(path: &Path) -> Vec<String> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Parse `#define NAME value` lines (used for HGSS `include/constants/trainers.h`).
    fn load_trainer_constants_h(path: &Path) -> Vec<String> {
        let define_regex =
            Regex::new(r"^\s*#define\s+(?P<name>TRAINER_[A-Z0-9_]+)\s+(?P<value>\d+)\s*$").unwrap();
        let mut constants = Vec::new();
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let Some(captures) = define_regex.captures(line) else {
                continue;
            };
            let value: usize = captures["value"].parse().unwrap();
            if constants.len() <= value {
                constants.resize(value + 1, String::new());
            }
            constants[value] = captures["name"].to_string();
        }
        constants
    }

    fn count_matching_symbols_by_index(table: &SymbolTable, expected: &[String]) -> usize {
        let actual_by_value: HashMap<i64, String> = table
            .get_all_defines()
            .into_iter()
            .map(|(name, value)| (value, name))
            .collect();
        expected
            .iter()
            .enumerate()
            .filter(|(index, expected_name)| {
                actual_by_value
                    .get(&(*index as i64))
                    .is_some_and(|actual| actual == *expected_name)
            })
            .count()
    }

    fn curl_available() -> bool {
        std::process::Command::new("curl")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn spawn_single_response_server(
        status_line: &str,
        body: &str,
    ) -> std::io::Result<(String, thread::JoinHandle<()>)> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr().unwrap();
        let status_line = status_line.to_string();
        let body = body.to_string();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request_buf = [0u8; 1024];
            let _ = stream.read(&mut request_buf);

            let response = format!(
                "{status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        });

        Ok((format!("http://{}/symbols.h", addr), handle))
    }

    #[test]
    fn test_diamond_dependency_caching() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        create_file(dir.path(), "d.h", "#define VAL_D 400");
        create_file(dir.path(), "b.h", "#include \"d.h\"\n#define VAL_B 200");
        create_file(dir.path(), "c.h", "#include \"d.h\"\n#define VAL_C 300");
        let a_path = create_file(
            dir.path(),
            "a.h",
            "#include \"b.h\"\n#include \"c.h\"\n#define VAL_A 100",
        );

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&a_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("VAL_A"), Some(100));
        assert_eq!(table.resolve_constant("VAL_B"), Some(200));
        assert_eq!(table.resolve_constant("VAL_C"), Some(300));
        assert_eq!(table.resolve_constant("VAL_D"), Some(400));

        assert_eq!(table.get_source_manager().len(), 4);
    }

    #[test]
    fn test_circular_dependency() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let a_path = create_file(dir.path(), "a.h", "#include \"b.h\"\n#define VAL_A 1");
        create_file(dir.path(), "b.h", "#include \"a.h\"\n#define VAL_B 2");

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&a_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("VAL_A"), Some(1));
        assert_eq!(table.resolve_constant("VAL_B"), Some(2));
    }

    #[test]
    fn test_include_path_priority() {
        let dir = tempdir().unwrap();
        let global_dir = dir.path().join("global");
        let local_dir = dir.path().join("local");
        let sm = SourceManager::new();

        create_file(&global_dir, "config.h", "#define CONF 1");
        create_file(&local_dir, "config.h", "#define CONF 2");

        let main_path = create_file(&local_dir, "main.h", "#include \"config.h\"");

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&main_path, &[global_dir]).unwrap();

        assert_eq!(table.resolve_constant("CONF"), Some(2));
    }

    #[test]
    fn test_cross_file_expression_resolution() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        create_file(dir.path(), "consts.h", "#define BASE 10");
        let main_path = create_file(
            dir.path(),
            "main.h",
            "#include \"consts.h\"\n#define DERIVED (BASE + 5)",
        );

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&main_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("DERIVED"), Some(15));
    }

    #[test]
    fn test_function_macro_expression_resolution() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let main_path = create_file(
            dir.path(),
            "main.h",
            "#include \"map_sections.h\"\n#define METLOC_PRIMO 2001\n#define PRIMO_LOC MAPLOC(METLOC_PRIMO)",
        );
        create_file(
            dir.path(),
            "map_sections.h",
            "#define MAPLOC(sec) ((sec) % 1000)",
        );

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&main_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("PRIMO_LOC"), Some(1));
        assert_eq!(table.evaluate_expression("MAPLOC(METLOC_PRIMO)"), Some(1));
    }

    #[test]
    fn test_function_macro_expression_resolution_with_nested_args() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let main_path = create_file(
            dir.path(),
            "main.h",
            "#include \"map_sections.h\"\n#define METLOC_PRIMO 2001",
        );
        create_file(
            dir.path(),
            "map_sections.h",
            "#define MAPLOC(sec) ((sec) % 1000)",
        );

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&main_path, &[]).unwrap();

        assert_eq!(
            table.evaluate_expression("MAPLOC(METLOC_PRIMO + 1000)"),
            Some(1)
        );
    }

    #[test]
    fn test_load_header_missing_file_returns_error() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("missing.h");
        let mut table = SymbolTable::new();

        let err = table.load_header(&missing).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_load_headers_from_dir_propagates_read_errors() {
        let dir = tempdir().unwrap();
        let invalid = dir.path().join("broken.h");
        std::fs::write(&invalid, [0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let mut table = SymbolTable::new();
        let err = table.load_headers_from_dir(dir.path()).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_load_list_file_str_propagates_assignment_eval_errors() {
        let mut table = SymbolTable::new();
        let err = table
            .load_list_file_str("CONST_A = 1\nCONST_B = UNKNOWN_SYMBOL\n")
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("CONST_B"));
        assert!(err.to_string().contains("UNKNOWN_SYMBOL"));
    }

    #[test]
    fn test_load_list_file_str_assignment_with_inline_comment() {
        let mut table = SymbolTable::new();
        table
            .load_list_file_str("CONST_A = 1 # comment\nCONST_B\n")
            .unwrap();

        assert_eq!(table.resolve_constant("CONST_A"), Some(1));
        assert_eq!(table.resolve_constant("CONST_B"), Some(2));
    }

    #[test]
    fn test_load_list_file_str_supports_comparison_and_logical_expression() {
        let mut table = SymbolTable::new();
        table
            .load_list_file_str("CONST_A = 1 < 2\nCONST_B = CONST_A && 0\n")
            .unwrap();
        assert_eq!(table.resolve_constant("CONST_A"), Some(1));
        assert_eq!(table.resolve_constant("CONST_B"), Some(0));
    }

    #[test]
    fn test_load_list_file_str_supports_logical_short_circuit() {
        let mut table = SymbolTable::new();
        table
            .load_list_file_str("CONST_A = 0 && UNKNOWN_SYMBOL\nCONST_B = 1 || UNKNOWN_SYMBOL\n")
            .unwrap();
        assert_eq!(table.resolve_constant("CONST_A"), Some(0));
        assert_eq!(table.resolve_constant("CONST_B"), Some(1));
    }

    #[test]
    fn test_load_list_file_str_rejects_unsupported_ternary_expression() {
        let mut table = SymbolTable::new();
        let err = table
            .load_list_file_str("CONST_A = 1 ? 2 : 3\n")
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("CONST_A"));
        assert!(err.to_string().contains("1 ? 2 : 3"));
    }

    #[test]
    fn test_load_headers_from_dir_uses_generated_meson_mask_metadata() {
        let dir = tempdir().unwrap();
        let generated_dir = dir.path().join("generated");
        std::fs::create_dir_all(&generated_dir).unwrap();

        std::fs::write(
            generated_dir.join("meson.build"),
            r"
metang_generators = {
    'player_transitions': { 'type': 'mask', 'tag': 'PlayerTransition' },
    'moves': { 'type': 'enum', 'tag': 'Move' },
}
",
        )
        .unwrap();

        std::fs::write(
            generated_dir.join("player_transitions.txt"),
            "PLAYER_TRANSITION_WALKING\nPLAYER_TRANSITION_CYCLING\nPLAYER_TRANSITION_SURFING\n",
        )
        .unwrap();

        std::fs::write(
            generated_dir.join("moves.txt"),
            "MOVE_POUND\nMOVE_KARATE_CHOP\nMOVE_DOUBLE_SLAP\n",
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table.load_headers_from_dir(&generated_dir).unwrap();

        assert_eq!(loaded, 3);
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_WALKING"), Some(1));
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_CYCLING"), Some(2));
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_SURFING"), Some(4));

        assert_eq!(table.resolve_constant("MOVE_POUND"), Some(0));
        assert_eq!(table.resolve_constant("MOVE_KARATE_CHOP"), Some(1));
        assert_eq!(table.resolve_constant("MOVE_DOUBLE_SLAP"), Some(2));
    }

    #[test]
    fn test_load_headers_from_dir_defaults_txt_to_enum_without_meson_mask_entry() {
        let dir = tempdir().unwrap();
        let generated_dir = dir.path().join("generated");
        std::fs::create_dir_all(&generated_dir).unwrap();

        std::fs::write(
            generated_dir.join("meson.build"),
            r"
metang_generators = {
    'moves': { 'type': 'enum', 'tag': 'Move' },
}
",
        )
        .unwrap();

        std::fs::write(
            generated_dir.join("player_transitions.txt"),
            "PLAYER_TRANSITION_WALKING\nPLAYER_TRANSITION_CYCLING\nPLAYER_TRANSITION_SURFING\n",
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table.load_headers_from_dir(&generated_dir).unwrap();

        assert_eq!(loaded, 2);
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_WALKING"), Some(0));
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_CYCLING"), Some(1));
        assert_eq!(table.resolve_constant("PLAYER_TRANSITION_SURFING"), Some(2));
    }

    proptest! {
                #[test]
                fn prop_load_headers_from_dir_generated_mask_values_follow_bit_positions(entry_count in 1usize..=20) {
                    let dir = tempdir().unwrap();
                    let generated_dir = dir.path().join("generated");
                    std::fs::create_dir_all(&generated_dir).unwrap();

                    std::fs::write(
                        generated_dir.join("meson.build"),
                        r"
metang_generators = {
    'player_transitions': { 'type': 'mask', 'tag': 'PlayerTransition' },
}
",
                    )
                    .unwrap();

                    let mut content = String::new();
                    for i in 0..entry_count {
                        use std::fmt::Write as _;
                        writeln!(&mut content, "PLAYER_TRANSITION_{i}").unwrap();
                    }
                    std::fs::write(generated_dir.join("player_transitions.txt"), content).unwrap();

                    let mut table = SymbolTable::new();
                    table.load_headers_from_dir(&generated_dir).unwrap();

                    for i in 0..entry_count {
                        let expected = 1_i64 << i;
                        prop_assert_eq!(
                            table.resolve_constant(&format!("PLAYER_TRANSITION_{i}")),
                            Some(expected)
                        );
                    }
                }
            }

    #[test]
    fn test_load_python_enum_str_propagates_assignment_eval_errors() {
        let mut table = SymbolTable::new();
        let err = table
            .load_python_enum_str_with_tag(
                "SPECIES_OK = 1\nSPECIES_BAD = UNKNOWN_SYMBOL\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("SPECIES_BAD"));
        assert!(err.to_string().contains("UNKNOWN_SYMBOL"));
    }

    #[test]
    fn test_constant_family_metadata_and_family_specific_resolution() {
        let mut table = SymbolTable::new();
        table
            .load_header_str(
                "\
#define SPECIES_BULBASAUR 1
#define ITEM_MASTER_BALL 1
#define TRAINER_RIVAL_BARRY_001 2
#define TRAINER_CLASS_LEADER_ROARK 2
#define LOCATION_JUBILIFE_CITY 3
#define MAPSEC_JUBILIFE_CITY 4
#define MOVE_TACKLE 5
#define SEQ_SE_CONFIRM 6
",
            )
            .unwrap();

        assert_eq!(
            table.constant_family("SPECIES_BULBASAUR"),
            Some(ConstantFamily::Species)
        );
        assert_eq!(
            table.constant_family("ITEM_MASTER_BALL"),
            Some(ConstantFamily::Item)
        );
        assert_eq!(
            table.constant_family("TRAINER_RIVAL_BARRY_001"),
            Some(ConstantFamily::Trainer)
        );
        assert_eq!(
            table.constant_family("TRAINER_CLASS_LEADER_ROARK"),
            Some(ConstantFamily::TrainerClass)
        );
        assert_eq!(
            table.constant_family("LOCATION_JUBILIFE_CITY"),
            Some(ConstantFamily::Location)
        );
        assert_eq!(
            table.constant_family("MAPSEC_JUBILIFE_CITY"),
            Some(ConstantFamily::Location)
        );
        assert_eq!(
            table.constant_family("MOVE_TACKLE"),
            Some(ConstantFamily::Move)
        );
        assert_eq!(
            table.constant_family("SEQ_SE_CONFIRM"),
            Some(ConstantFamily::Sound)
        );
        assert_eq!(table.constant_family("TRUE"), None);

        assert_eq!(
            table.resolve_name_in_family(1, ConstantFamily::Species),
            Some("SPECIES_BULBASAUR".to_string())
        );
        assert_eq!(
            table.resolve_name_in_family(1, ConstantFamily::Item),
            Some("ITEM_MASTER_BALL".to_string())
        );
        assert_eq!(
            table.resolve_name_in_family(2, ConstantFamily::Trainer),
            Some("TRAINER_RIVAL_BARRY_001".to_string())
        );
        assert_eq!(
            table.resolve_name_in_family(2, ConstantFamily::TrainerClass),
            Some("TRAINER_CLASS_LEADER_ROARK".to_string())
        );
    }

    #[test]
    fn test_load_python_enum_str_allows_non_constant_assignments() {
        let mut table = SymbolTable::new();
        table
            .load_python_enum_str_with_tag(
                "__all__ = [\"SPECIES_OK\"]\nhelper = 5\nSPECIES_OK = 1\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap();

        assert_eq!(table.resolve_constant("SPECIES_OK"), Some(1));
        assert_eq!(table.resolve_constant("__all__"), None);
        assert_eq!(table.resolve_constant("helper"), None);
    }

    #[test]
    fn test_load_python_enum_str_assignment_with_inline_comment() {
        let mut table = SymbolTable::new();
        table
            .load_python_enum_str_with_tag(
                "SPECIES_OK = 1 # comment\nSPECIES_NEXT = SPECIES_OK + 1\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap();

        assert_eq!(table.resolve_constant("SPECIES_OK"), Some(1));
        assert_eq!(table.resolve_constant("SPECIES_NEXT"), Some(2));
    }

    #[test]
    fn test_load_python_enum_str_supports_comparison_and_logical_expression() {
        let mut table = SymbolTable::new();
        table
            .load_python_enum_str_with_tag(
                "SPECIES_OK = 1 < 2\nSPECIES_BOOL = SPECIES_OK && 0\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap();

        assert_eq!(table.resolve_constant("SPECIES_OK"), Some(1));
        assert_eq!(table.resolve_constant("SPECIES_BOOL"), Some(0));
    }

    #[test]
    fn test_load_python_enum_str_supports_logical_short_circuit() {
        let mut table = SymbolTable::new();
        table
            .load_python_enum_str_with_tag(
                "SPECIES_A = 0 && UNKNOWN_SYMBOL\nSPECIES_B = 1 || UNKNOWN_SYMBOL\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap();

        assert_eq!(table.resolve_constant("SPECIES_A"), Some(0));
        assert_eq!(table.resolve_constant("SPECIES_B"), Some(1));
    }

    #[test]
    fn test_load_python_enum_str_rejects_unsupported_ternary_expression() {
        let mut table = SymbolTable::new();
        let err = table
            .load_python_enum_str_with_tag(
                "SPECIES_BAD = 1 ? 2 : 3\n",
                Path::new("inline.py"),
                crate::c_parser::SymbolTag::Global,
            )
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("SPECIES_BAD"));
        assert!(err.to_string().contains("1 ? 2 : 3"));
    }

    #[test]
    fn test_load_headers_from_dir_propagates_text_bank_json_schema_errors() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("bank.json");
        std::fs::write(&json_path, "{}").unwrap();

        let mut table = SymbolTable::new();
        let err = table.load_headers_from_dir(dir.path()).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("messages"));
        assert!(err.to_string().contains("object_events"));
    }

    #[test]
    fn test_load_headers_from_dir_propagates_text_bank_json_entry_id_errors() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("bank.json");
        std::fs::write(
            &json_path,
            r#"{ "messages": [ { "id": "MSG_HELLO" }, {} ] }"#,
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let err = table.load_headers_from_dir(dir.path()).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("messages[1].id"));
    }

    #[test]
    fn test_canonicalize_constant_name_examples() {
        assert_eq!(canonicalize_constant_name("Poké Ball"), "POKE_BALL");
        assert_eq!(canonicalize_constant_name("Farfetch'd"), "FARFETCHD");
        assert_eq!(canonicalize_constant_name("Nidoran♀"), "NIDORAN_F");
        assert_eq!(canonicalize_constant_name("Mr. Mime"), "MR_MIME");
        assert_eq!(canonicalize_constant_name("Ho-Oh"), "HO_OH");
        assert_eq!(canonicalize_constant_name("Amy & Mimi"), "AMY_AND_MIMI");
    }

    #[test]
    fn test_load_text_bank_json_constants_emits_canonical_prefixed_symbols() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("species.json");
        std::fs::write(
            &json_path,
            r#"
            {
              "messages": [
                { "id": "msg_0000", "en_US": "-----" },
                { "id": "msg_0001", "en_US": "Farfetch'd" },
                { "id": "msg_0002", "en_US": "Nidoran♀" },
                { "id": "msg_0003", "en_US": "Mr. Mime" }
              ]
            }
            "#,
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "SPECIES_", None)
            .unwrap();

        assert_eq!(loaded, 4);
        assert_eq!(table.resolve_constant("SPECIES_NONE"), Some(0));
        assert_eq!(table.resolve_constant("SPECIES_FARFETCHD"), Some(1));
        assert_eq!(table.resolve_constant("SPECIES_NIDORAN_F"), Some(2));
        assert_eq!(table.resolve_constant("SPECIES_MR_MIME"), Some(3));
    }

    #[test]
    fn test_load_text_bank_json_constants_uses_item_bank_specific_rules() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("items.json");
        std::fs::write(
            &json_path,
            r#"
            {
              "messages": [
                { "id": "msg_0000", "en_US": "None" },
                { "id": "msg_0001", "en_US": "EnergyPowder" },
                { "id": "msg_0002", "en_US": "Up-Grade" },
                { "id": "msg_0003", "en_US": "X Defend" },
                { "id": "msg_0004", "en_US": "???" },
                { "id": "msg_0005", "en_US": "DeepSeaTooth" }
              ]
            }
            "#,
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "ITEM_", None)
            .unwrap();

        assert_eq!(loaded, 6);
        assert_eq!(table.resolve_constant("ITEM_NONE"), Some(0));
        assert_eq!(table.resolve_constant("ITEM_ENERGYPOWDER"), Some(1));
        assert_eq!(table.resolve_constant("ITEM_UPGRADE"), Some(2));
        assert_eq!(table.resolve_constant("ITEM_X_DEFENSE"), Some(3));
        assert_eq!(table.resolve_constant("ITEM_UNUSED_4"), Some(4));
        assert_eq!(table.resolve_constant("ITEM_DEEPSEATOOTH"), Some(5));
    }

    #[test]
    fn test_load_text_bank_json_constants_uses_move_bank_specific_rules() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("moves.json");
        std::fs::write(
            &json_path,
            r#"
            {
              "messages": [
                { "id": "msg_0000", "en_US": "-" },
                { "id": "msg_0001", "en_US": "DoubleSlap" },
                { "id": "msg_0002", "en_US": "ViceGrip" },
                { "id": "msg_0003", "en_US": "Will-O-Wisp" },
                { "id": "msg_0004", "en_US": "U-turn" },
                { "id": "msg_0005", "en_US": "X-Scissor" }
              ]
            }
            "#,
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "MOVE_", None)
            .unwrap();

        assert_eq!(loaded, 6);
        assert_eq!(table.resolve_constant("MOVE_NONE"), Some(0));
        assert_eq!(table.resolve_constant("MOVE_DOUBLE_SLAP"), Some(1));
        assert_eq!(table.resolve_constant("MOVE_VICE_GRIP"), Some(2));
        assert_eq!(table.resolve_constant("MOVE_WILL_O_WISP"), Some(3));
        assert_eq!(table.resolve_constant("MOVE_U_TURN"), Some(4));
        assert_eq!(table.resolve_constant("MOVE_X_SCISSOR"), Some(5));
    }

    #[test]
    fn test_load_text_bank_json_constants_supports_index_suffixes_for_trainers() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("trainers.json");
        std::fs::write(
            &json_path,
            r#"
            {
              "messages": [
                { "id": "msg_0000", "en_US": "{TRAINER_NAME: -}" },
                { "id": "msg_0001", "en_US": "{TRAINER_NAME:Silver}" },
                { "id": "msg_0002", "en_US": "{TRAINER_NAME:Amy & Mimi}" }
              ]
            }
            "#,
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "TRAINER_", Some(3))
            .unwrap();

        assert_eq!(loaded, 3);
        assert_eq!(table.resolve_constant("TRAINER_NONE"), Some(0));
        assert_eq!(table.resolve_constant("TRAINER_SILVER_001"), Some(1));
        assert_eq!(table.resolve_constant("TRAINER_AMY_AND_MIMI_002"), Some(2));
    }

    #[test]
    fn test_load_dspre_trainer_archive_constants_handles_platinum_dummy_detection() {
        let dir = tempdir().unwrap();
        let trainer_names_path = dir.path().join("0618.json");
        let trainer_classes_path = dir.path().join("0619.json");
        std::fs::write(
            &trainer_names_path,
            r#"
            {
              "messages": [
                { "id": "msg_0618_00000", "en_US": "{TRAINER_NAME: -}" },
                { "id": "msg_0618_00001", "en_US": "{TRAINER_NAME:Tristan}" },
                { "id": "msg_0618_00002", "en_US": "{TRAINER_NAME:Mickey}" }
              ]
            }
            "#,
        )
        .unwrap();
        std::fs::write(
            &trainer_classes_path,
            r#"
            {
              "messages": [
                { "id": "msg_0619_00000", "en_US": "[PK][MN] Trainer" },
                { "id": "msg_0619_00001", "en_US": "Youngster" },
                { "id": "msg_0619_00002", "en_US": "Camper" }
              ]
            }
            "#,
        )
        .unwrap();

        let trainers = vec![
            sample_trainer(GameFamily::Platinum, 0, 0, 0),
            sample_trainer(GameFamily::Platinum, 1, 396, 5),
            sample_trainer(GameFamily::Platinum, 2, 19, 5),
        ];

        let mut table = SymbolTable::new();
        let loaded = table
            .load_dspre_trainer_archive_constants(
                &trainer_names_path,
                &trainer_classes_path,
                &trainers,
                GameFamily::Platinum,
            )
            .unwrap();

        assert_eq!(loaded, 3);
        assert_eq!(table.resolve_constant("TRAINER_NONE"), Some(0));
        assert_eq!(table.resolve_constant("TRAINER_YOUNGSTER_TRISTAN"), Some(1));
        assert_eq!(table.resolve_constant("TRAINER_DUMMY_002"), Some(2));
    }

    #[test]
    fn test_load_dspre_trainer_archive_constants_handles_hgss_suffixes() {
        let dir = tempdir().unwrap();
        let trainer_names_path = dir.path().join("0729.json");
        let trainer_classes_path = dir.path().join("0730.json");
        std::fs::write(
            &trainer_names_path,
            r#"
            {
              "messages": [
                { "id": "msg_0729_00000", "en_US": "{TRAINER_NAME: -}" },
                { "id": "msg_0729_00001", "en_US": "{TRAINER_NAME:Silver}" },
                { "id": "msg_0729_00002", "en_US": "{TRAINER_NAME:Silver}" },
                { "id": "msg_0729_00003", "en_US": "{TRAINER_NAME:Falkner}" }
              ]
            }
            "#,
        )
        .unwrap();
        std::fs::write(
            &trainer_classes_path,
            r#"
            {
              "messages": [
                { "id": "msg_0730_00000", "en_US": "[PK][MN] Trainer" },
                { "id": "msg_0730_00001", "en_US": "Rival" },
                { "id": "msg_0730_00002", "en_US": "Leader" }
              ]
            }
            "#,
        )
        .unwrap();

        let trainers = vec![
            sample_trainer(GameFamily::HGSS, 0, 0, 0),
            sample_trainer(GameFamily::HGSS, 1, 155, 5),
            sample_trainer(GameFamily::HGSS, 1, 158, 5),
            sample_trainer(GameFamily::HGSS, 2, 16, 9),
        ];

        let mut table = SymbolTable::new();
        let loaded = table
            .load_dspre_trainer_archive_constants(
                &trainer_names_path,
                &trainer_classes_path,
                &trainers,
                GameFamily::HGSS,
            )
            .unwrap();

        assert_eq!(loaded, 4);
        assert_eq!(table.resolve_constant("TRAINER_NONE"), Some(0));
        assert_eq!(table.resolve_constant("TRAINER_RIVAL_SILVER"), Some(1));
        assert_eq!(table.resolve_constant("TRAINER_RIVAL_SILVER_2"), Some(2));
        assert_eq!(
            table.resolve_constant("TRAINER_LEADER_FALKNER_FALKNER"),
            Some(3)
        );
    }

    #[test]
    fn test_load_dspre_sound_archive_constants_uses_platinum_sound_id_ranges() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("pt_sounds.json");
        let mut names = vec!["UNUSED".to_string(); 1013];
        names[0] = "PV001".to_string();
        names[1] = "PV".to_string();
        names[2] = "PV-END".to_string();
        names[229] = "LAST-BGM".to_string();
        names[230] = "PL-W012".to_string();
        names[264] = "DUMMY01".to_string();
        names[287] = "DUMMY02".to_string();
        names[380] = "DP-SELECT".to_string();
        let messages: Vec<_> = names
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                serde_json::json!({
                    "id": format!("msg_0545_{index:05}"),
                    "en_US": text,
                })
            })
            .collect();
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&serde_json::json!({ "messages": messages })).unwrap(),
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_dspre_sound_archive_constants(&json_path)
            .unwrap();

        assert_eq!(loaded, 1013);
        assert_eq!(table.resolve_constant("SEQ_PV001"), Some(1));
        assert_eq!(table.resolve_constant("SEQ_LAST_BGM"), Some(1226));
        assert_eq!(table.resolve_constant("SEQ_SE_PL_W012"), Some(1350));
        assert_eq!(table.resolve_constant("SEQ_DUMMY01"), Some(1384));
        assert_eq!(table.resolve_constant("SEQ_DUMMY02"), Some(1407));
        assert_eq!(table.resolve_constant("SEQ_SE_DP_SELECT"), Some(1500));
        assert_eq!(table.resolve_constant("SEQ_SE_CONFIRM"), Some(1500));
        assert_eq!(
            table.constant_family("SEQ_SE_DP_SELECT"),
            Some(ConstantFamily::Sound)
        );
        assert_eq!(
            table.constant_family("SEQ_SE_CONFIRM"),
            Some(ConstantFamily::Sound)
        );
    }

    #[test]
    fn test_load_dspre_sound_archive_constants_uses_hgss_sound_id_ranges() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("hg_sounds.json");
        let mut names = vec!["UNUSED".to_string(); 1372];
        names[0] = "PV001".to_string();
        names[1] = "PV".to_string();
        names[2] = "PV-END".to_string();
        names[364] = "LAST-BGM".to_string();
        names[365] = "PL-W012".to_string();
        names[401] = "DUMMY01".to_string();
        names[493] = "DP-SELECT".to_string();
        let messages: Vec<_> = names
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                serde_json::json!({
                    "id": format!("msg_0437_{index:05}"),
                    "en_US": text,
                })
            })
            .collect();
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&serde_json::json!({ "messages": messages })).unwrap(),
        )
        .unwrap();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_dspre_sound_archive_constants(&json_path)
            .unwrap();

        assert_eq!(loaded, 1372);
        assert_eq!(table.resolve_constant("SEQ_PV001"), Some(1));
        assert_eq!(table.resolve_constant("SEQ_LAST_BGM"), Some(1361));
        assert_eq!(table.resolve_constant("SEQ_SE_PL_W012"), Some(1372));
        assert_eq!(table.resolve_constant("SEQ_DUMMY01"), Some(1408));
        assert_eq!(table.resolve_constant("SEQ_SE_DP_SELECT"), Some(1500));
        assert_eq!(table.resolve_constant("SEQ_SE_CONFIRM"), Some(1500));
        assert_eq!(
            table.constant_family("SEQ_SE_DP_SELECT"),
            Some(ConstantFamily::Sound)
        );
        assert_eq!(
            table.constant_family("SEQ_SE_CONFIRM"),
            Some(ConstantFamily::Sound)
        );
    }

    #[test]
    fn test_load_headers_from_dir_allows_non_symbol_message_arrays() {
        let dir = tempdir().unwrap();
        let json_path = dir.path().join("location_names.json");
        std::fs::write(&json_path, r#"{ "messages": [ { "en_US": "Test" } ] }"#).unwrap();

        let mut table = SymbolTable::new();
        let loaded = table.load_headers_from_dir(dir.path()).unwrap();

        assert_eq!(loaded, 1);
        assert_eq!(table.resolve_constant("en_US"), None);
    }

    #[test]
    fn test_load_recursive_missing_include_without_handler_leaves_symbols_unresolved() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let main_path = create_file(dir.path(), "main.h", "#include \"res/field/events/test.h\"");

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive(&main_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("LOCALID_HIKER"), None);
    }

    #[test]
    fn test_load_recursive_strict_errors_on_missing_include() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let main_path = create_file(dir.path(), "main.h", "#include \"missing.h\"");

        let mut table = SymbolTable::with_source_manager(sm);
        let err = table.load_recursive_strict(&main_path, &[]).unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert!(err.to_string().contains("Unresolved include 'missing.h'"));
    }

    #[test]
    fn test_load_recursive_strict_ignores_commented_out_include_lines() {
        let dir = tempdir().unwrap();
        let sm = SourceManager::new();

        let main_path = create_file(
            dir.path(),
            "main.h",
            "// #include \"missing.h\"\n#define TEST_VALUE 42",
        );

        let mut table = SymbolTable::with_source_manager(sm);
        table.load_recursive_strict(&main_path, &[]).unwrap();

        assert_eq!(table.resolve_constant("TEST_VALUE"), Some(42));
    }

    #[test]
    fn test_load_from_url_http_404_returns_error() {
        if !curl_available() {
            eprintln!("Skipping: curl is not available");
            return;
        }

        let (url, handle) = match spawn_single_response_server("HTTP/1.1 404 Not Found", "missing")
        {
            Ok(values) => values,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "Skipping: local loopback bind is not permitted in this environment ({err})"
                );
                return;
            }
            Err(err) => panic!("failed to start local HTTP fixture server: {err}"),
        };
        let mut table = SymbolTable::new();

        let err = table.load_from_url(&url).unwrap_err();
        handle.join().unwrap();

        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert!(err.to_string().contains("Failed to fetch URL"));
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn test_load_from_url_http_200_loads_symbols() {
        if !curl_available() {
            eprintln!("Skipping: curl is not available");
            return;
        }

        let (url, handle) = match spawn_single_response_server(
            "HTTP/1.1 200 OK",
            "#define TEST_REMOTE_CONST 123",
        ) {
            Ok(values) => values,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "Skipping: local loopback bind is not permitted in this environment ({err})"
                );
                return;
            }
            Err(err) => panic!("failed to start local HTTP fixture server: {err}"),
        };
        let mut table = SymbolTable::new();

        table.load_from_url(&url).unwrap();
        handle.join().unwrap();

        assert_eq!(table.resolve_constant("TEST_REMOTE_CONST"), Some(123));
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_headers_from_dir_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "c_parser integration test (Platinum decomp)",
        ) else {
            return;
        };

        // Platinum keeps many symbol lists in generated/*.txt while shared defines
        // stay under include/constants.
        let Some(table) = load_symbol_table_from_real_dirs(
            &root,
            &["generated", "include/constants"],
            "c_parser integration test (Platinum decomp)",
        ) else {
            return;
        };

        assert_resolved_constants(
            &table,
            &[
                ("SPECIES_BULBASAUR", 1),
                ("MOVE_TACKLE", 33),
                ("ITEM_MASTER_BALL", 1),
                ("POCKET_BALLS", 2),
            ],
            "c_parser integration test (Platinum decomp)",
        );
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_headers_from_dir_hgss_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "c_parser integration test (HGSS decomp)",
        ) else {
            return;
        };

        let Some(table) = load_symbol_table_from_real_dirs(
            &root,
            &["include/constants"],
            "c_parser integration test (HGSS decomp)",
        ) else {
            return;
        };

        assert_resolved_constants(
            &table,
            &[
                ("SPECIES_BULBASAUR", 1),
                ("MOVE_TACKLE", 33),
                ("ITEM_MASTER_BALL", 1),
                ("POCKET_BALLS", 2),
            ],
            "c_parser integration test (HGSS decomp)",
        );
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_canonicalize_platinum_item_and_move_banks_match_generated_constants() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "c_parser canonicalization integration test (Platinum decomp)",
        ) else {
            return;
        };

        // Items and moves follow stable per-bank naming rules in Platinum, so
        // the text archive canonicalization can be checked exhaustively against
        // the generated decomp banks. Trainer classes intentionally collapse
        // multiple generated constants into one display string and do not appear
        // in script sources, so they are excluded here.
        for (json_rel, generated_rel, prefix) in [
            ("res/text/item_names.json", "generated/items.txt", "ITEM_"),
            ("res/text/move_names.json", "generated/moves.txt", "MOVE_"),
        ] {
            let json_path = root.join(json_rel);
            assert!(
                json_path.is_file(),
                "missing json fixture for canonicalization test: {}",
                json_path.display()
            );
            let generated_path = root.join(generated_rel);
            assert!(
                generated_path.is_file(),
                "missing generated bank fixture for canonicalization test: {}",
                generated_path.display()
            );
            let expected_symbols: Vec<_> = std::fs::read_to_string(&generated_path)
                .unwrap()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with("MAX_"))
                .map(ToOwned::to_owned)
                .collect();

            let mut table = SymbolTable::new();
            let loaded = table
                .load_text_bank_json_constants(&json_path, prefix, None)
                .unwrap();
            assert_eq!(
                loaded,
                expected_symbols.len(),
                "{json_rel} should load the same number of rows as {generated_rel}"
            );

            for (index, symbol) in expected_symbols.iter().enumerate() {
                assert_eq!(
                    table.resolve_constant(symbol),
                    Some(index as i64),
                    "{} row {} did not canonicalize to {}",
                    json_rel,
                    index,
                    symbol
                );
            }
        }
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_canonicalize_platinum_species_bank_matches_generated_constants() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "c_parser species canonicalization integration test (Platinum decomp)",
        ) else {
            return;
        };

        let json_path = root.join("build/res/text/species_name.json");
        assert!(
            json_path.is_file(),
            "missing species archive fixture for canonicalization test: {}",
            json_path.display()
        );
        let generated_path = root.join("generated/species.txt");
        assert!(
            generated_path.is_file(),
            "missing generated species bank fixture for canonicalization test: {}",
            generated_path.display()
        );
        let expected_symbols: Vec<_> = std::fs::read_to_string(&generated_path)
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("MAX_"))
            .map(ToOwned::to_owned)
            .collect();

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "SPECIES_", None)
            .unwrap();
        assert_eq!(
            loaded,
            expected_symbols.len(),
            "species_name.json should load the same number of rows as generated/species.txt"
        );

        for (index, symbol) in expected_symbols.iter().enumerate() {
            assert_eq!(
                table.resolve_constant(symbol),
                Some(index as i64),
                "build/res/text/species_name.json row {} did not canonicalize to {}",
                index,
                symbol
            );
        }
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_canonicalize_hgss_species_bank_matches_species_constants() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "c_parser species canonicalization integration test (HGSS decomp)",
        ) else {
            return;
        };

        let gmm_path = root.join("files/msgdata/msg/msg_0237.gmm");
        assert!(
            gmm_path.is_file(),
            "missing species archive fixture for canonicalization test: {}",
            gmm_path.display()
        );
        let species_constants_path = root.join("include/constants/species.h");
        assert!(
            species_constants_path.is_file(),
            "missing species constants fixture for canonicalization test: {}",
            species_constants_path.display()
        );

        let gmm = std::fs::read_to_string(&gmm_path).unwrap();
        let row_regex = Regex::new(
            r#"(?s)<row id="[^"]+" index="(?P<index>\d+)">.*?<language name="English">(?P<text>.*?)</language>"#,
        )
        .unwrap();
        let mut rows = vec![String::new(); 496];
        for captures in row_regex.captures_iter(&gmm) {
            let index: usize = captures["index"].parse().unwrap();
            let text = captures["text"]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&");
            if index < rows.len() {
                rows[index] = text;
            }
        }
        assert!(
            rows.iter().all(|row| !row.is_empty()),
            "failed to extract all HGSS species rows from {}",
            gmm_path.display()
        );

        let dir = tempdir().unwrap();
        let json_path = dir.path().join("hgss_species.json");
        let messages: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(index, text)| {
                serde_json::json!({
                    "id": format!("msg_0237_{index:05}"),
                    "en_US": text,
                })
            })
            .collect();
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&serde_json::json!({ "messages": messages })).unwrap(),
        )
        .unwrap();

        let define_regex =
            Regex::new(r#"^\s*#define\s+(?P<name>SPECIES_[A-Z0-9_]+)\s+(?P<value>\d+)\s*$"#)
                .unwrap();
        let mut expected_symbols = vec![String::new(); rows.len()];
        for line in std::fs::read_to_string(&species_constants_path)
            .unwrap()
            .lines()
        {
            let Some(captures) = define_regex.captures(line) else {
                continue;
            };
            let value: usize = captures["value"].parse().unwrap();
            if value < expected_symbols.len() {
                expected_symbols[value] = captures["name"].to_string();
            }
        }
        assert!(
            expected_symbols.iter().all(|symbol| !symbol.is_empty()),
            "failed to extract all HGSS species constants from {}",
            species_constants_path.display()
        );

        let mut table = SymbolTable::new();
        let loaded = table
            .load_text_bank_json_constants(&json_path, "SPECIES_", None)
            .unwrap();
        assert_eq!(
            loaded,
            expected_symbols.len(),
            "HGSS species archive should load the same number of rows as include/constants/species.h for the base species bank"
        );

        for (index, symbol) in expected_symbols.iter().enumerate() {
            assert_eq!(
                table.resolve_constant(symbol),
                Some(index as i64),
                "files/msgdata/msg/msg_0237.gmm row {} did not canonicalize to {}",
                index,
                symbol
            );
        }
    }

    #[test]
    #[ignore = "requires real DSPRE and decomp projects via UXIE_TEST_PLATINUM_DSPRE_PATH and UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_best_effort_platinum_trainer_canonicalization_reaches_expected_floor() {
        let Some(dspre_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "c_parser trainer canonicalization integration test (Platinum DSPRE)",
        ) else {
            return;
        };
        let Some(decomp_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "c_parser trainer canonicalization integration test (Platinum decomp)",
        ) else {
            return;
        };

        let trainer_names_path = dspre_root.join("expanded/textArchives/0618.json");
        let trainer_classes_path = dspre_root.join("expanded/textArchives/0619.json");
        let expected_path = decomp_root.join("generated/trainers.txt");
        assert!(
            trainer_names_path.is_file(),
            "missing trainer names archive fixture: {}",
            trainer_names_path.display()
        );
        assert!(
            trainer_classes_path.is_file(),
            "missing trainer classes archive fixture: {}",
            trainer_classes_path.display()
        );
        assert!(
            expected_path.is_file(),
            "missing trainer constants fixture: {}",
            expected_path.display()
        );

        let trainers = load_all_dspre_trainers(&dspre_root, GameFamily::Platinum).unwrap();
        let mut table = SymbolTable::new();
        table
            .load_dspre_trainer_archive_constants(
                &trainer_names_path,
                &trainer_classes_path,
                &trainers,
                GameFamily::Platinum,
            )
            .unwrap();

        let expected = load_trainer_constants_txt(&expected_path);
        let matches = count_matching_symbols_by_index(&table, &expected);
        assert!(
            matches >= 490,
            "best-effort Platinum trainer canonicalization regressed: matched {matches}/{} exact constants",
            expected.len()
        );
    }

    #[test]
    #[ignore = "requires real DSPRE and decomp projects via UXIE_TEST_HGSS_DSPRE_PATH and UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_best_effort_hgss_trainer_canonicalization_reaches_expected_floor() {
        let Some(dspre_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            "c_parser trainer canonicalization integration test (HGSS DSPRE)",
        ) else {
            return;
        };
        let Some(decomp_root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "c_parser trainer canonicalization integration test (HGSS decomp)",
        ) else {
            return;
        };

        let trainer_names_path = dspre_root.join("expanded/textArchives/0729.json");
        let trainer_classes_path = dspre_root.join("expanded/textArchives/0730.json");
        let expected_path = decomp_root.join("include/constants/trainers.h");
        assert!(
            trainer_names_path.is_file(),
            "missing trainer names archive fixture: {}",
            trainer_names_path.display()
        );
        assert!(
            trainer_classes_path.is_file(),
            "missing trainer classes archive fixture: {}",
            trainer_classes_path.display()
        );
        assert!(
            expected_path.is_file(),
            "missing trainer constants fixture: {}",
            expected_path.display()
        );

        let trainers = load_all_dspre_trainers(&dspre_root, GameFamily::HGSS).unwrap();
        let mut table = SymbolTable::new();
        table
            .load_dspre_trainer_archive_constants(
                &trainer_names_path,
                &trainer_classes_path,
                &trainers,
                GameFamily::HGSS,
            )
            .unwrap();

        let expected = load_trainer_constants_h(&expected_path);
        let matches = count_matching_symbols_by_index(&table, &expected);
        assert!(
            matches >= 540,
            "best-effort HGSS trainer canonicalization regressed: matched {matches}/{} exact constants",
            expected.len()
        );
    }
}
