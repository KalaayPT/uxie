#[cfg(test)]
mod tests {
    use crate::c_parser::SymbolTable;
    use crate::event_file::binary::{
        BgEventBinary, BinaryEventFile, CoordEventBinary, ObjectEventBinary, WarpEventBinary,
    };
    use crate::event_file::json::JsonEventFile;
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

        let mut buffer = Vec::new();
        file.to_binary(&mut buffer).unwrap();

        let mut reader = Cursor::new(&buffer);
        let decoded = BinaryEventFile::from_binary(&mut reader).unwrap();

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
    #[ignore]
    fn integration_dspre_event_eterna_dp_gym() {
        use crate::ds_rom::DspreProject;
        use std::path::Path;

        let dspre_path = Path::new("/home/kalaay/Desktop/pt_DSPRE_contents");
        let headers_path = Path::new("/home/kalaay/dev/pokeplatinum/build/generated");
        let expected_path = Path::new(
            "/home/kalaay/dev/pokeplatinum/res/field/events/events_eterna_city_dp_gym.json",
        );

        if !dspre_path.exists() || !headers_path.exists() || !expected_path.exists() {
            eprintln!("Skipping: test data not available");
            return;
        }

        let project = DspreProject::open(dspre_path).unwrap();
        let mut symbols = SymbolTable::new();
        symbols.load_headers_from_dir(headers_path).unwrap();

        let bin_event = project.load_event_file(67).unwrap();
        let json_event = JsonEventFile::from_binary(&bin_event, &symbols);

        let expected: JsonEventFile =
            serde_json::from_str(&std::fs::read_to_string(expected_path).unwrap()).unwrap();

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
}
