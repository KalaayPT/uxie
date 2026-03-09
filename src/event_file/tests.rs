#[cfg(test)]
mod event_file_tests {
    use crate::c_parser::SymbolTable;
    use crate::event_file::binary::{
        BgEventBinary, BinaryEventFile, CoordEventBinary, ObjectEventBinary, WarpEventBinary,
    };
    use crate::event_file::json::JsonEventFile;
    use proptest::prelude::*;
    use std::io::Cursor;

    #[test]
    fn test_event_binary_roundtrip() {
        let file = BinaryEventFile {
            bg_events: vec![BgEventBinary {
                script: 1,
                event_type: 0,
                x: 10,
                z: 20,
                y: 0,
                player_facing_dir: 1,
            }],
            object_events: vec![ObjectEventBinary {
                local_id: 1,
                graphics_id: 100,
                movement_type: 2,
                trainer_type: 0,
                hidden_flag: 0,
                script: 5,
                dir: 3,
                data: [0, 0, 0],
                movement_range_x: 1,
                movement_range_z: 1,
                x: 15,
                z: 25,
                y: 0,
            }],
            warp_events: vec![WarpEventBinary {
                x: 30,
                z: 40,
                dest_header_id: 123,
                dest_warp_id: 1,
            }],
            coord_events: vec![CoordEventBinary {
                script: 10,
                x: 5,
                z: 5,
                width: 2,
                length: 2,
                y: 0,
                value: 1,
                var: 0x4000,
            }],
        };

        let mut buffer = Cursor::new(Vec::new());
        file.to_binary(&mut buffer).unwrap();

        buffer.set_position(0);
        let decoded = BinaryEventFile::from_binary(&mut buffer).unwrap();

        assert_eq!(file.bg_events, decoded.bg_events);
        assert_eq!(file.object_events, decoded.object_events);
        assert_eq!(file.warp_events, decoded.warp_events);
        assert_eq!(file.coord_events, decoded.coord_events);
    }

    #[test]
    fn test_json_resolution() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("MAP_HEADER_JUBILIFE_CITY".into(), 10);
        symbols.insert_define("VAR_STORY_PROGRESS".into(), 0x4000);
        symbols.insert_define("OBJ_EVENT_GFX_PLAYER".into(), 1);
        symbols.insert_define("BG_EVENT_DIR_NORTH".into(), 1);

        let json = r#"{
            "bg_events": [{
                "script": 1,
                "type": "0",
                "x": 10,

                "z": 10,
                "y": 0,
                "player_facing_dir": "BG_EVENT_DIR_NORTH"
            }],
            "object_events": [{
                "id": "OBJ_1",
                "graphics_id": "OBJ_EVENT_GFX_PLAYER",
                "movement_type": "0",
                "trainer_type": "0",
                "hidden_flag": "0",
                "script": 1,
                "initial_dir": 0,
                "x": 5, "z": 5, "y": 0
            }],
            "warp_events": [{
                "x": 1, "z": 1,
                "dest_header_id": "MAP_HEADER_JUBILIFE_CITY",
                "dest_warp_id": 0
            }],
            "coord_events": [{
                "script": 1, "x": 0, "z": 0, "y": 0, "width": 1, "length": 1,
                "var": "VAR_STORY_PROGRESS",
                "value": 5
            }]
        }"#;

        let mut file: JsonEventFile = serde_json::from_str(json).unwrap();
        file.resolve_constants(&symbols);

        assert_eq!(file.bg_events[0].player_facing_dir, Some("1".into()));
        assert_eq!(file.object_events[0].graphics_id, "1");
        assert_eq!(file.warp_events[0].dest_header_id, "10");
        assert_eq!(file.coord_events[0].var, Some("16384".into()));
    }

    #[test]
    fn test_json_from_binary_coordinate_wrapping_examples() {
        let file = BinaryEventFile {
            bg_events: vec![BgEventBinary {
                script: 1,
                event_type: 0,
                x: 65,
                z: -33,
                y: 0,
                player_facing_dir: 0,
            }],
            object_events: vec![ObjectEventBinary {
                local_id: 1,
                graphics_id: 1,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: 0,
                script: 1,
                dir: 0,
                data: [0, 0, 0],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 95,
                z: 64,
                y: 0,
            }],
            warp_events: vec![WarpEventBinary {
                x: 63,
                z: 32,
                dest_header_id: 0,
                dest_warp_id: 0,
            }],
            coord_events: vec![CoordEventBinary {
                script: 1,
                x: 33,
                z: 96,
                width: 1,
                length: 1,
                y: 0,
                value: 0,
                var: 0,
            }],
        };

        let symbols = SymbolTable::new();
        let json = JsonEventFile::from_binary(&file, &symbols);

        assert_eq!(json.bg_events[0].x, 1);
        assert_eq!(json.bg_events[0].z, -1);
        assert_eq!(json.object_events[0].x, 31);
        assert_eq!(json.object_events[0].z, 0);
        assert_eq!(json.warp_events[0].x, 31);
        assert_eq!(json.warp_events[0].z, 0);
        assert_eq!(json.coord_events[0].x, 1);
        assert_eq!(json.coord_events[0].z, 0);
    }

    #[test]
    #[ignore = "requires local DSPRE+decomp fixtures via UXIE_TEST_PLATINUM_DSPRE_PATH and UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_dspre_event_eterna_dp_gym() {
        use crate::ds_rom::DspreProject;
        let Some(dspre_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "event integration test",
        ) else {
            return;
        };
        let Some(decomp_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "event integration test",
        ) else {
            return;
        };
        let headers_path = decomp_path.join("build/generated");
        let expected_path = decomp_path.join("res/field/events/events_eterna_city_dp_gym.json");

        if !dspre_path.exists() || !headers_path.exists() || !expected_path.exists() {
            eprintln!("Skipping: test data not available at configured paths");
            return;
        }

        let project = DspreProject::open(&dspre_path).unwrap();
        let mut symbols = SymbolTable::new();
        symbols.load_headers_from_dir(&headers_path).unwrap();

        let bin_event = project.load_event_file(67).unwrap();
        let json_event = JsonEventFile::from_binary(&bin_event, &symbols);

        let expected: JsonEventFile =
            serde_json::from_str(&std::fs::read_to_string(&expected_path).unwrap()).unwrap();

        assert_eq!(json_event.bg_events.len(), expected.bg_events.len());
        assert_eq!(json_event.object_events.len(), expected.object_events.len());
        assert_eq!(json_event.warp_events.len(), expected.warp_events.len());
        assert_eq!(json_event.coord_events.len(), expected.coord_events.len());

        for (got, exp) in json_event
            .warp_events
            .iter()
            .zip(expected.warp_events.iter())
        {
            assert_eq!(got.x, exp.x);
            assert_eq!(got.z, exp.z);
            assert_eq!(got.dest_header_id, exp.dest_header_id);
            assert_eq!(got.dest_warp_id, exp.dest_warp_id);
        }
    }

    #[test]
    #[ignore = "requires local HGSS DSPRE+decomp fixtures via UXIE_TEST_HGSS_DSPRE_PATH and UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_dspre_event_hgss_zone_event_parity() {
        let Some(dspre_narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["data/a/0/3/2", "data/fielddata/eventdata/zone_event.narc"],
            "event HGSS parity integration test",
        ) else {
            return;
        };
        let Some(decomp_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "event HGSS parity integration test",
        ) else {
            return;
        };
        let decomp_narc_path = decomp_path.join("files/fielddata/eventdata/zone_event.narc");
        if !decomp_narc_path.exists() {
            eprintln!(
                "Skipping event HGSS parity integration test: decomp NARC not found: {}",
                decomp_narc_path.display()
            );
            return;
        }

        let mut dspre_reader =
            std::io::BufReader::new(std::fs::File::open(&dspre_narc_path).unwrap());
        let dspre_narc = crate::Narc::from_binary(&mut dspre_reader).unwrap();

        let mut decomp_reader =
            std::io::BufReader::new(std::fs::File::open(&decomp_narc_path).unwrap());
        let decomp_narc = crate::Narc::from_binary(&mut decomp_reader).unwrap();

        assert_eq!(
            dspre_narc.len(),
            decomp_narc.len(),
            "zone_event member count mismatch between {} and {}",
            dspre_narc_path.display(),
            decomp_narc_path.display()
        );
        assert!(
            !dspre_narc.is_empty(),
            "expected non-empty zone_event NARC at {}",
            dspre_narc_path.display()
        );

        for event_id in 0..usize::min(20, dspre_narc.len()) {
            let dspre_member = dspre_narc.member(event_id).unwrap();
            let decomp_member = decomp_narc.member(event_id).unwrap();

            let mut dspre_cursor = Cursor::new(dspre_member);
            let dspre_event = BinaryEventFile::from_binary(&mut dspre_cursor).unwrap();

            let mut decomp_cursor = Cursor::new(decomp_member);
            let decomp_event = BinaryEventFile::from_binary(&mut decomp_cursor).unwrap();

            assert_eq!(
                dspre_event.bg_events, decomp_event.bg_events,
                "event {} bg_events mismatch between DSPRE and decomp zone_event sources",
                event_id
            );
            assert_eq!(
                dspre_event.object_events, decomp_event.object_events,
                "event {} object_events mismatch between DSPRE and decomp zone_event sources",
                event_id
            );
            assert_eq!(
                dspre_event.warp_events, decomp_event.warp_events,
                "event {} warp_events mismatch between DSPRE and decomp zone_event sources",
                event_id
            );
            assert_eq!(
                dspre_event.coord_events, decomp_event.coord_events,
                "event {} coord_events mismatch between DSPRE and decomp zone_event sources",
                event_id
            );
        }
    }

    fn bg_event_strategy() -> impl Strategy<Value = BgEventBinary> {
        (
            any::<u16>(),
            any::<u16>(),
            any::<i32>(),
            any::<i32>(),
            any::<i32>(),
            any::<u16>(),
        )
            .prop_map(
                |(script, event_type, x, z, y, player_facing_dir)| BgEventBinary {
                    script,
                    event_type,
                    x,
                    z,
                    y,
                    player_facing_dir,
                },
            )
    }

    fn object_event_strategy() -> impl Strategy<Value = ObjectEventBinary> {
        let part1 = (
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        );
        let part2 = (
            any::<i16>(),
            any::<[u16; 3]>(),
            any::<i16>(),
            any::<i16>(),
            any::<u16>(),
            any::<u16>(),
            any::<i32>(),
        );

        (part1, part2).prop_map(
            |(
                (local_id, graphics_id, movement_type, trainer_type, hidden_flag, script),
                (dir, data, movement_range_x, movement_range_z, x, z, y),
            )| ObjectEventBinary {
                local_id,
                graphics_id,
                movement_type,
                trainer_type,
                hidden_flag,
                script,
                dir,
                data,
                movement_range_x,
                movement_range_z,
                x,
                z,
                y,
            },
        )
    }

    fn warp_event_strategy() -> impl Strategy<Value = WarpEventBinary> {
        (any::<u16>(), any::<u16>(), any::<u16>(), any::<u16>()).prop_map(
            |(x, z, dest_header_id, dest_warp_id)| WarpEventBinary {
                x,
                z,
                dest_header_id,
                dest_warp_id,
            },
        )
    }

    fn coord_event_strategy() -> impl Strategy<Value = CoordEventBinary> {
        (
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        )
            .prop_map(
                |(script, x, z, width, length, y, value, var)| CoordEventBinary {
                    script,
                    x,
                    z,
                    width,
                    length,
                    y,
                    value,
                    var,
                },
            )
    }

    fn binary_event_file_strategy() -> impl Strategy<Value = BinaryEventFile> {
        (
            prop::collection::vec(bg_event_strategy(), 0..24),
            prop::collection::vec(object_event_strategy(), 0..24),
            prop::collection::vec(warp_event_strategy(), 0..24),
            prop::collection::vec(coord_event_strategy(), 0..24),
        )
            .prop_map(|(bg_events, object_events, warp_events, coord_events)| {
                BinaryEventFile {
                    bg_events,
                    object_events,
                    warp_events,
                    coord_events,
                }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_event_binary_roundtrip(file in binary_event_file_strategy()) {
            let mut buffer = Cursor::new(Vec::new());
            file.to_binary(&mut buffer).unwrap();
            buffer.set_position(0);
            let decoded = BinaryEventFile::from_binary(&mut buffer).unwrap();
            prop_assert_eq!(file.bg_events, decoded.bg_events);
            prop_assert_eq!(file.object_events, decoded.object_events);
            prop_assert_eq!(file.warp_events, decoded.warp_events);
            prop_assert_eq!(file.coord_events, decoded.coord_events);
        }

        #[test]
        fn prop_json_from_binary_coordinate_normalization(file in binary_event_file_strategy()) {
            let symbols = SymbolTable::new();
            let json = JsonEventFile::from_binary(&file, &symbols);

            prop_assert_eq!(json.bg_events.len(), file.bg_events.len());
            prop_assert_eq!(json.object_events.len(), file.object_events.len());
            prop_assert_eq!(json.warp_events.len(), file.warp_events.len());
            prop_assert_eq!(json.coord_events.len(), file.coord_events.len());

            for (bg_json, bg_bin) in json.bg_events.iter().zip(file.bg_events.iter()) {
                prop_assert_eq!(bg_json.x, bg_bin.x % 32);
                prop_assert_eq!(bg_json.z, bg_bin.z % 32);
            }

            for (obj_json, obj_bin) in json.object_events.iter().zip(file.object_events.iter()) {
                prop_assert_eq!(obj_json.x, obj_bin.x % 32);
                prop_assert_eq!(obj_json.z, obj_bin.z % 32);
            }

            for (warp_json, warp_bin) in json.warp_events.iter().zip(file.warp_events.iter()) {
                prop_assert_eq!(warp_json.x, warp_bin.x % 32);
                prop_assert_eq!(warp_json.z, warp_bin.z % 32);
            }

            for (coord_json, coord_bin) in json.coord_events.iter().zip(file.coord_events.iter()) {
                prop_assert_eq!(coord_json.x, coord_bin.x % 32);
                prop_assert_eq!(coord_json.z, coord_bin.z % 32);
            }
        }
    }
}
