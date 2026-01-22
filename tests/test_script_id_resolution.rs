use std::path::PathBuf;

use uxie::Workspace;
use uxie::script_file::{
    resolve_level_script_by_file, resolve_script_id, resolve_script_id_by_file,
    resolve_script_id_by_level_script_file,
};

const PLATINUM_DECOMP_PATH: &str = "C:/dev/pokeplatinum";
const DSPRE_PLATINUM_PATH: &str = "C:/dev/romhacking/renHERgade platinum/pt_DSPRE_contents";

fn test_decomp_workspace() -> Workspace {
    let path = PathBuf::from(PLATINUM_DECOMP_PATH);
    if !path.exists() {
        panic!(
            "pokeplatinum decompilation project not found at {}.
            Please clone the repository from https://github.com/pret/pokeplatinum",
            PLATINUM_DECOMP_PATH
        );
    }
    Workspace::open(&path).expect("Failed to open pokeplatinum workspace")
}

fn test_dspre_workspace() -> Workspace {
    let path = PathBuf::from(DSPRE_PLATINUM_PATH);
    if !path.exists() {
        panic!(
            "DSPRE Platinum project not found at {}.
            Please create or download a DSPRE project",
            DSPRE_PLATINUM_PATH
        );
    }
    Workspace::open(&path).expect("Failed to open DSPRE workspace")
}

#[test]
fn test_resolve_common_script_by_file_decomp() {
    let ws = test_decomp_workspace();

    let script_file_id = 211;
    let common_script_id = 2018;

    let result = resolve_script_id_by_file(
        common_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve common script");

    assert!(result.is_some(), "Common script should resolve");
    let resolution = result.unwrap();
    assert!(
        resolution.is_common_script(),
        "Should be resolved as common script"
    );
}

#[test]
fn test_resolve_map_script_by_file_decomp() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let script_file_id = header.script_file_id();
    let local_script_id = 1;

    let result = resolve_script_id_by_file(
        local_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve map script");

    assert!(result.is_some(), "Map script should resolve");
    let resolution = result.unwrap();
    assert!(
        resolution.is_map_script(),
        "Should be resolved as map script"
    );
    assert_eq!(resolution.script_file_id(), script_file_id);
}

#[test]
fn test_resolve_level_script_by_file_decomp() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();

    let result = resolve_level_script_by_file(
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve level script by file");

    assert!(result.is_some(), "Level script should resolve");
    let resolution = result.unwrap();

    assert_eq!(
        resolution.script_file_id(),
        header.script_file_id(),
        "Resolved script file should match map's script file"
    );
}

#[test]
fn test_resolve_script_id_by_level_script_file_decomp_local() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();
    let local_script_id = 1;

    let result = resolve_script_id_by_level_script_file(
        local_script_id,
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve script by level script file");

    assert!(result.is_some(), "Local script should resolve");
    let resolution = result.unwrap();
    assert!(
        resolution.is_map_script(),
        "Should be resolved as map script"
    );
    assert_eq!(
        resolution.script_file_id(),
        header.script_file_id(),
        "Should resolve to the map's script file, not the level script file"
    );
    assert_eq!(resolution.text_archive_id(), header.text_archive_id());
}

#[test]
fn test_resolve_script_id_by_level_script_file_decomp_common() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();
    let common_script_id = 2018;

    let result = resolve_script_id_by_level_script_file(
        common_script_id,
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve common script by level script file");

    assert!(result.is_some(), "Common script should resolve");
    let resolution = result.unwrap();
    assert!(
        resolution.is_common_script(),
        "Should be resolved as common script"
    );
}

#[test]
fn test_resolve_script_by_file_not_found_decomp() {
    let ws = test_decomp_workspace();

    let script_file_id = 9999;
    let local_script_id = 1;

    let result = resolve_script_id_by_file(
        local_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Should not error, just return None");

    assert!(
        result.is_none(),
        "Non-existent script file should return None"
    );
}

#[test]
fn test_resolve_common_script_by_file_dspre() {
    let ws = test_dspre_workspace();

    let script_file_id = 211;
    let common_script_id = 2018;

    let result = resolve_script_id_by_file(
        common_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve common script");

    assert!(result.is_some(), "Common script should resolve");
    let resolution = result.unwrap();
    assert!(
        resolution.is_common_script(),
        "Should be resolved as common script"
    );
}

#[test]
fn test_resolve_map_script_by_file_dspre() {
    let ws = test_dspre_workspace();

    let script_file_id = 100;
    let local_script_id = 1;

    let result = resolve_script_id_by_file(
        local_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve map script");

    if result.is_some() {
        let resolution = result.unwrap();
        assert!(
            resolution.is_map_script(),
            "Should be resolved as map script"
        );
        assert_eq!(resolution.script_file_id(), script_file_id);
    }
}

#[test]
fn test_resolve_level_script_by_file_dspre() {
    let ws = test_dspre_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();

    let result = resolve_level_script_by_file(
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve level script by file");

    if result.is_some() {
        let resolution = result.unwrap();
        assert_eq!(
            resolution.script_file_id(),
            header.script_file_id(),
            "Resolved script file should match map's script file"
        );
    }
}

#[test]
fn test_resolve_script_id_by_level_script_file_dspre() {
    let ws = test_dspre_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();
    let local_script_id = 1;

    let result = resolve_script_id_by_level_script_file(
        local_script_id,
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve script by level script file");

    if result.is_some() {
        let resolution = result.unwrap();
        assert!(
            resolution.is_map_script(),
            "Should be resolved as map script"
        );
        assert_eq!(
            resolution.script_file_id(),
            header.script_file_id(),
            "Should resolve to the map's script file"
        );
    }
}

#[test]
fn test_old_vs_new_api_comparison() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let script_file_id = header.script_file_id();

    let map_id = ws
        .provider
        .find_map_by_script_file_id(script_file_id)
        .expect("Failed to find map")
        .expect("Map should exist");

    let old_result = resolve_script_id(
        1,
        Some(map_id),
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve with old API");

    let new_result = resolve_script_id_by_file(
        1,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve with new API");

    assert_eq!(
        old_result, new_result,
        "Old and new APIs should produce the same result"
    );
}

#[test]
fn test_find_map_by_level_script_file_id_decomp() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();

    let found_map_id = ws
        .provider
        .find_map_by_level_script_file_id(level_script_file_id)
        .expect("Failed to find map by level script file id");

    assert!(
        found_map_id.is_some(),
        "Should find a map for level script file id {}",
        level_script_file_id
    );
}

#[test]
fn test_find_map_by_level_script_file_id_dspre() {
    let ws = test_dspre_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();

    let found_map_id = ws
        .provider
        .find_map_by_level_script_file_id(level_script_file_id)
        .expect("Failed to find map by level script file id");

    assert!(
        found_map_id.is_some(),
        "Should find a map for level script file id {}",
        level_script_file_id
    );
}

#[test]
fn test_multiple_maps_same_script_file() {
    let ws = test_decomp_workspace();

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let script_file_id = header.script_file_id();

    let result = resolve_script_id_by_file(
        1,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve");

    assert!(result.is_some());
    let resolution = result.unwrap();

    assert_eq!(resolution.script_file_id(), script_file_id);
    assert_eq!(resolution.text_archive_id(), header.text_archive_id());
}
