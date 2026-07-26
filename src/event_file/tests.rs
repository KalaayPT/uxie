#[cfg(test)]
mod event_file_tests {
    use crate::c_parser::SymbolTable;
    use crate::event_file::binary::{BgEvent, CoordEvent, EventFile, ObjectEvent, WarpEvent};
    use crate::event_file::hgss_json::HgssEventJson;
    use crate::event_file::platinum_json::PlatinumEventJson;
    use crate::game::GameFamily;
    use proptest::prelude::*;
    use std::io::Cursor;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_binary_record_sizes() {
        // Validate serialized record sizes match what the decomps expect.
        fn written_size(file: &EventFile) -> usize {
            let mut buf = Cursor::new(Vec::new());
            file.to_binary(&mut buf).unwrap();
            // subtract 4 count fields = 16 bytes header
            buf.into_inner().len() - 16
        }

        let one_bg = EventFile {
            bg_events: vec![BgEvent {
                script: 0,
                event_type: 0,
                x: 0,
                z: 0,
                y: 0,
                player_facing_dir: 0,
            }],
            object_events: vec![],
            warp_events: vec![],
            coord_events: vec![],
        };
        assert_eq!(written_size(&one_bg), 0x14, "BgEvent must be 0x14 bytes");

        let one_obj = EventFile {
            bg_events: vec![],
            object_events: vec![ObjectEvent {
                local_id: 0,
                graphics_id: 0,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: 0,
                script: 0,
                dir: 0,
                data: [0; 3],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 0,
                z: 0,
                y: 0,
            }],
            warp_events: vec![],
            coord_events: vec![],
        };
        assert_eq!(
            written_size(&one_obj),
            0x20,
            "ObjectEvent must be 0x20 bytes"
        );

        let one_warp = EventFile {
            bg_events: vec![],
            object_events: vec![],
            warp_events: vec![WarpEvent {
                x: 0,
                z: 0,
                dest_header_id: 0,
                dest_warp_id: 0,
                height: 0,
            }],
            coord_events: vec![],
        };
        assert_eq!(
            written_size(&one_warp),
            0x0c,
            "WarpEvent must be 0x0c bytes"
        );

        let one_coord = EventFile {
            bg_events: vec![],
            object_events: vec![],
            warp_events: vec![],
            coord_events: vec![CoordEvent {
                script: 0,
                x: 0,
                z: 0,
                width: 0,
                length: 0,
                y: 0,
                value: 0,
                var: 0,
            }],
        };
        assert_eq!(
            written_size(&one_coord),
            0x10,
            "CoordEvent must be 0x10 bytes"
        );
    }

    #[test]
    fn test_event_binary_roundtrip() {
        let file = EventFile {
            bg_events: vec![BgEvent {
                script: 1,
                event_type: 0,
                x: 10,
                z: 20,
                y: 0,
                player_facing_dir: 1,
            }],
            object_events: vec![ObjectEvent {
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
            warp_events: vec![WarpEvent {
                x: 30,
                z: 40,
                dest_header_id: 123,
                dest_warp_id: 1,
                height: 0,
            }],
            coord_events: vec![CoordEvent {
                script: 10,
                x: -3,
                z: 4,
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
        let decoded = EventFile::from_binary(&mut buffer).unwrap();
        assert_eq!(file, decoded);
    }

    #[test]
    fn test_platinum_warp_height_rejected() {
        let file = EventFile {
            bg_events: vec![],
            object_events: vec![],
            warp_events: vec![WarpEvent {
                x: 1,
                z: 2,
                dest_header_id: 3,
                dest_warp_id: 4,
                height: 1,
            }],
            coord_events: vec![],
        };
        let mut buf = Cursor::new(Vec::new());
        assert!(file.to_binary_for(&mut buf, GameFamily::Platinum).is_err());
        let mut buf = Cursor::new(Vec::new());
        assert!(file.to_binary_for(&mut buf, GameFamily::HGSS).is_ok());
    }

    #[test]
    fn test_platinum_json_resolution() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("MAP_HEADER_JUBILIFE_CITY".into(), 10);
        symbols.insert_define("VAR_STORY_PROGRESS".into(), 0x4000);
        symbols.insert_define("OBJ_EVENT_GFX_PLAYER".into(), 1);
        symbols.insert_define("BG_EVENT_DIR_NORTH".into(), 1);

        let json = r#"{
            "bg_events": [{"script": 1, "type": "0", "x": 10, "z": 10, "y": 0,
                           "player_facing_dir": "BG_EVENT_DIR_NORTH"}],
            "object_events": [{"id": "OBJ_1", "graphics_id": "OBJ_EVENT_GFX_PLAYER",
                               "movement_type": "0", "trainer_type": "0", "hidden_flag": "0",
                               "script": 1, "initial_dir": 0, "x": 5, "z": 5, "y": 0}],
            "warp_events": [{"x": 1, "z": 1, "dest_header_id": "MAP_HEADER_JUBILIFE_CITY",
                             "dest_warp_id": 0}],
            "coord_events": [{"script": 1, "x": 0, "z": 0, "y": 0, "width": 1, "length": 1,
                              "var": "VAR_STORY_PROGRESS", "value": 5}]
        }"#;

        let parsed: PlatinumEventJson = serde_json::from_str(json).unwrap();
        let file = EventFile::from_platinum_json(&parsed, &symbols).unwrap();
        assert_eq!(file.bg_events[0].player_facing_dir, 1);
        assert_eq!(file.object_events[0].graphics_id, 1);
        assert_eq!(file.warp_events[0].dest_header_id, 10);
        assert_eq!(file.coord_events[0].var, 0x4000);
    }

    #[test]
    fn test_binary_to_json_preserves_large_coordinates() {
        // Regression: coordinates must not be truncated with % 32.
        // Values from Platinum fixture member 486 (great marsh 6).
        let file = EventFile {
            bg_events: vec![BgEvent {
                script: 1,
                event_type: 0,
                x: 69,
                z: 106,
                y: 0,
                player_facing_dir: 0,
            }],
            object_events: vec![],
            warp_events: vec![WarpEvent {
                x: 68,
                z: 118,
                dest_header_id: 0,
                dest_warp_id: 0,
                height: 0,
            }],
            coord_events: vec![CoordEvent {
                script: 1,
                x: 33,
                z: -5,
                width: 1,
                length: 1,
                y: 0,
                value: 0,
                var: 0,
            }],
        };

        let symbols = SymbolTable::new();
        let json = file.to_platinum_json(&symbols).unwrap();
        assert_eq!(json.bg_events[0].x, 69);
        assert_eq!(json.bg_events[0].z, 106);
        assert_eq!(json.warp_events[0].x, 68);
        assert_eq!(json.warp_events[0].z, 118);
        assert_eq!(json.coord_events[0].x, 33);
        assert_eq!(json.coord_events[0].z, -5);
    }

    #[test]
    fn test_platinum_object_y_shift_and_clone_id() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("OBJ_EVENT_GFX_PLAYER".into(), 1);
        symbols.insert_define("FLAG_HIDE_FOO".into(), 42);

        let json = r#"{
            "bg_events": [],
            "object_events": [{"id": "LOCALID_FOO", "clone_id": 2,
                               "graphics_id": "OBJ_EVENT_GFX_PLAYER",
                               "movement_type": "0", "trainer_type": "0",
                               "hidden_flag": "FLAG_HIDE_FOO", "script": 5,
                               "initial_dir": 0, "data": [1, 2], "x": 10, "z": 20, "y": 3}],
            "warp_events": [],
            "coord_events": []
        }"#;

        let parsed: PlatinumEventJson = serde_json::from_str(json).unwrap();
        let file = EventFile::from_platinum_json(&parsed, &symbols).unwrap();
        assert_eq!(file.object_events[0].local_id, 2);
        assert_eq!(file.object_events[0].y, 3 * 0x10000);
        assert_eq!(file.object_events[0].hidden_flag, 42);
        assert_eq!(file.object_events[0].data, [1, 2, 0]);
    }

    #[test]
    fn test_hgss_expression_and_direct_y() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("obj_UNION_pcwoman3".into(), 4);
        symbols.insert_define("_EV_scr_seq_UNION_007".into(), 10);
        symbols.insert_define("SPRITE_PCWOMAN3".into(), 99);
        symbols.insert_define("FLAG_NOTHING".into(), 0);

        let json = r#"{
            "objects": [{"id": "obj_UNION_pcwoman3", "spriteId": "SPRITE_PCWOMAN3",
                         "movement": 0, "type": 0, "eventFlag": "FLAG_NOTHING",
                         "scriptId": "_EV_scr_seq_UNION_007 + 1", "facingDirection": 1,
                         "param0": 0, "param1": 0, "param2": 0,
                         "xRange": 0, "yRange": 0, "x": 3, "z": 3, "y": 5}],
            "warps": [{"x": 1, "z": 2, "header": 3, "anchor": 4, "y": 7}]
        }"#;

        let parsed: HgssEventJson = serde_json::from_str(json).unwrap();
        let file = EventFile::from_hgss_json(&parsed, Arc::new(symbols), std::path::Path::new("."))
            .unwrap();
        assert_eq!(file.object_events[0].local_id, 4);
        assert_eq!(file.object_events[0].script, 11);
        assert_eq!(file.object_events[0].y, 5); // no * 0x10000 shift in HGSS
        assert_eq!(file.warp_events[0].height, 7);
    }

    #[test]
    fn test_hgss_json_missing_header_returns_contextual_error() {
        let symbols = Arc::new(SymbolTable::new());
        let dir = tempdir().unwrap();
        let json: HgssEventJson =
            serde_json::from_str(r#"{ "header": "fielddata/script/scr_seq/missing.h" }"#).unwrap();

        let err = EventFile::from_hgss_json(&json, symbols, dir.path()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert!(err.to_string().contains("missing.h"));
    }

    #[test]
    fn test_platinum_json_semantic_roundtrip() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("OBJ_EVENT_GFX_PLAYER".into(), 1);
        symbols.insert_define("BG_EVENT_DIR_NORTH".into(), 1);

        let json = r#"{
            "bg_events": [{"script": 1, "type": "0", "x": 69, "z": 106, "y": 0,
                           "player_facing_dir": "BG_EVENT_DIR_NORTH"}],
            "object_events": [{"id": "OBJ_0", "graphics_id": "OBJ_EVENT_GFX_PLAYER",
                               "movement_type": "0", "trainer_type": "0", "hidden_flag": "0",
                               "script": 2, "initial_dir": 0, "x": 10, "z": 20, "y": 0}],
            "warp_events": [],
            "coord_events": []
        }"#;

        let parsed: PlatinumEventJson = serde_json::from_str(json).unwrap();
        let native = EventFile::from_platinum_json(&parsed, &symbols).unwrap();
        let emitted = native.to_platinum_json(&symbols).unwrap();
        let roundtrip = EventFile::from_platinum_json(&emitted, &symbols).unwrap();
        assert_eq!(native, roundtrip);
    }

    #[test]
    fn test_platinum_signed_coord_json_roundtrip() {
        let symbols = SymbolTable::new();
        let file = EventFile {
            bg_events: vec![],
            object_events: vec![],
            warp_events: vec![],
            coord_events: vec![CoordEvent {
                script: 1,
                x: -3,
                z: -5,
                width: 1,
                length: 1,
                y: 0,
                value: 0,
                var: 0,
            }],
        };

        let emitted = file.to_platinum_json(&symbols).unwrap();
        assert_eq!(emitted.coord_events[0].x, -3);
        assert_eq!(emitted.coord_events[0].z, -5);
        let reparsed = EventFile::from_platinum_json(&emitted, &symbols).unwrap();
        assert_eq!(file, reparsed);
    }

    #[test]
    fn test_platinum_object_data_trims_only_trailing_zeroes() {
        let symbols = SymbolTable::new();
        let file = EventFile {
            bg_events: vec![],
            object_events: vec![ObjectEvent {
                local_id: 0,
                graphics_id: 0,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: 0,
                script: 0,
                dir: 0,
                data: [0, 1, 0],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 0,
                z: 0,
                y: 0,
            }],
            warp_events: vec![],
            coord_events: vec![],
        };

        let emitted = file.to_platinum_json(&symbols).unwrap();
        assert_eq!(emitted.object_events[0].data, vec![0, 1]);
        let reparsed = EventFile::from_platinum_json(&emitted, &symbols).unwrap();
        assert_eq!(file, reparsed);
    }

    #[test]
    fn test_platinum_trainer_scripts_emit_symbolic_json_when_possible() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("TRAINER_RIVAL_1".into(), 25);

        let file = EventFile {
            bg_events: vec![],
            object_events: vec![
                ObjectEvent {
                    local_id: 0,
                    graphics_id: 0,
                    movement_type: 0,
                    trainer_type: 0,
                    hidden_flag: 0,
                    script: 3024,
                    dir: 0,
                    data: [0; 3],
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: 0,
                    z: 0,
                    y: 0,
                },
                ObjectEvent {
                    local_id: 1,
                    graphics_id: 0,
                    movement_type: 0,
                    trainer_type: 0,
                    hidden_flag: 0,
                    script: 5024,
                    dir: 0,
                    data: [0; 3],
                    movement_range_x: 0,
                    movement_range_z: 0,
                    x: 0,
                    z: 0,
                    y: 0,
                },
            ],
            warp_events: vec![],
            coord_events: vec![],
        };

        let emitted = file.to_platinum_json(&symbols).unwrap();
        assert_eq!(
            emitted.object_events[0].script,
            serde_json::json!("TRAINER_RIVAL_1")
        );
        assert_eq!(emitted.object_events[0].double_battle_id, Some(1));
        assert_eq!(
            emitted.object_events[1].script,
            serde_json::json!("TRAINER_RIVAL_1")
        );
        assert_eq!(emitted.object_events[1].double_battle_id, Some(2));

        let reparsed = EventFile::from_platinum_json(&emitted, &symbols).unwrap();
        assert_eq!(file, reparsed);
    }

    #[test]
    fn test_platinum_clone_id_preserves_u16_values() {
        let symbols = SymbolTable::new();
        let file = EventFile {
            bg_events: vec![],
            object_events: vec![ObjectEvent {
                local_id: 300,
                graphics_id: 0,
                movement_type: 0,
                trainer_type: 0,
                hidden_flag: 0,
                script: 0,
                dir: 0,
                data: [0; 3],
                movement_range_x: 0,
                movement_range_z: 0,
                x: 0,
                z: 0,
                y: 0,
            }],
            warp_events: vec![],
            coord_events: vec![],
        };

        let emitted = file.to_platinum_json(&symbols).unwrap();
        assert_eq!(emitted.object_events[0].clone_id, Some(300));
        let reparsed = EventFile::from_platinum_json(&emitted, &symbols).unwrap();
        assert_eq!(file, reparsed);
    }

    #[test]
    fn test_platinum_json_rejects_nonzero_warp_height() {
        let symbols = SymbolTable::new();
        let file = EventFile {
            bg_events: vec![],
            object_events: vec![],
            warp_events: vec![WarpEvent {
                x: 0,
                z: 0,
                dest_header_id: 0,
                dest_warp_id: 0,
                height: 1,
            }],
            coord_events: vec![],
        };

        assert!(file.to_platinum_json(&symbols).is_err());
    }

    fn bg_event_strategy() -> impl Strategy<Value = BgEvent> {
        (
            any::<u16>(),
            any::<u16>(),
            any::<i32>(),
            any::<i32>(),
            any::<i32>(),
            any::<u16>(),
        )
            .prop_map(|(script, event_type, x, z, y, player_facing_dir)| BgEvent {
                script,
                event_type,
                x,
                z,
                y,
                player_facing_dir,
            })
    }

    fn object_event_strategy() -> impl Strategy<Value = ObjectEvent> {
        (
            (
                any::<u16>(),
                any::<u16>(),
                any::<u16>(),
                any::<u16>(),
                any::<u16>(),
                any::<u16>(),
            ),
            (
                any::<i16>(),
                any::<[u16; 3]>(),
                any::<i16>(),
                any::<i16>(),
                any::<u16>(),
                any::<u16>(),
            ),
            // y is always a multiple of 0x10000 in valid Platinum binary
            (0i32..=256i32),
        )
            .prop_map(
                |(
                    (local_id, graphics_id, movement_type, trainer_type, hidden_flag, script),
                    (dir, data, movement_range_x, movement_range_z, x, z),
                    y_units,
                )| {
                    ObjectEvent {
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
                        y: y_units * 0x10000,
                    }
                },
            )
    }

    fn warp_event_strategy() -> impl Strategy<Value = WarpEvent> {
        (
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u32>(),
        )
            .prop_map(|(x, z, dest_header_id, dest_warp_id, height)| WarpEvent {
                x,
                z,
                dest_header_id,
                dest_warp_id,
                height,
            })
    }

    fn coord_event_strategy() -> impl Strategy<Value = CoordEvent> {
        (
            any::<u16>(),
            any::<i16>(),
            any::<i16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
            any::<u16>(),
        )
            .prop_map(|(script, x, z, width, length, y, value, var)| CoordEvent {
                script,
                x,
                z,
                width,
                length,
                y,
                value,
                var,
            })
    }

    fn event_file_strategy() -> impl Strategy<Value = EventFile> {
        (
            prop::collection::vec(bg_event_strategy(), 0..24),
            prop::collection::vec(object_event_strategy(), 0..24),
            prop::collection::vec(warp_event_strategy(), 0..24),
            prop::collection::vec(coord_event_strategy(), 0..24),
        )
            .prop_map(
                |(bg_events, object_events, warp_events, coord_events)| EventFile {
                    bg_events,
                    object_events,
                    warp_events,
                    coord_events,
                },
            )
    }

    fn platinum_event_file_strategy() -> impl Strategy<Value = EventFile> {
        event_file_strategy().prop_map(|mut file| {
            for warp in &mut file.warp_events {
                warp.height = 0;
            }
            file
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

        #[test]
        fn prop_event_binary_roundtrip(file in event_file_strategy()) {
            let mut buffer = Cursor::new(Vec::new());
            file.to_binary(&mut buffer).unwrap();
            buffer.set_position(0);
            let decoded = EventFile::from_binary(&mut buffer).unwrap();
            prop_assert_eq!(file, decoded);
        }

        #[test]
        fn prop_binary_to_json_preserves_coordinates(file in platinum_event_file_strategy()) {
            let symbols = SymbolTable::new();
            let json = file.to_platinum_json(&symbols).unwrap();

            for (bg_json, bg) in json.bg_events.iter().zip(file.bg_events.iter()) {
                prop_assert_eq!(bg_json.x, bg.x);
                prop_assert_eq!(bg_json.z, bg.z);
            }
            for (obj_json, obj) in json.object_events.iter().zip(file.object_events.iter()) {
                prop_assert_eq!(obj_json.x, obj.x);
                prop_assert_eq!(obj_json.z, obj.z);
            }
            for (warp_json, warp) in json.warp_events.iter().zip(file.warp_events.iter()) {
                prop_assert_eq!(warp_json.x, warp.x);
                prop_assert_eq!(warp_json.z, warp.z);
            }
            for (coord_json, coord) in json.coord_events.iter().zip(file.coord_events.iter()) {
                prop_assert_eq!(coord_json.x, coord.x);
                prop_assert_eq!(coord_json.z, coord.z);
            }
        }
    }
}
