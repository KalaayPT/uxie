use uxie::Workspace;
use uxie::script_file::{
    resolve_level_script, resolve_level_script_by_file, resolve_script_id,
    resolve_script_id_by_file, resolve_script_id_by_level_script_file,
};
use uxie::test_env::existing_path_from_env;

const ENV_PLATINUM_DECOMP_PATH: &str = "UXIE_TEST_PLATINUM_DECOMP_PATH";
const ENV_PLATINUM_DSPRE_PATH: &str = "UXIE_TEST_PLATINUM_DSPRE_PATH";
const ENV_HGSS_DECOMP_PATH: &str = "UXIE_TEST_HGSS_DECOMP_PATH";
const ENV_HGSS_DSPRE_PATH: &str = "UXIE_TEST_HGSS_DSPRE_PATH";

macro_rules! require_ws {
    ($ws:expr) => {
        match $ws {
            Some(ws) => ws,
            None => return,
        }
    };
}

fn test_decomp_workspace() -> Option<Workspace> {
    existing_path_from_env(ENV_PLATINUM_DECOMP_PATH, "decomp test")
        .map(|path| Workspace::open(&path).expect("Failed to open workspace"))
}

fn test_dspre_workspace() -> Option<Workspace> {
    existing_path_from_env(ENV_PLATINUM_DSPRE_PATH, "DSPRE test")
        .map(|path| Workspace::open(&path).expect("Failed to open workspace"))
}

fn test_hgss_decomp_workspace() -> Option<Workspace> {
    existing_path_from_env(ENV_HGSS_DECOMP_PATH, "HGSS decomp test")
        .map(|path| Workspace::open(&path).expect("Failed to open workspace"))
}

fn test_hgss_dspre_workspace() -> Option<Workspace> {
    existing_path_from_env(ENV_HGSS_DSPRE_PATH, "HGSS DSPRE test")
        .map(|path| Workspace::open(&path).expect("Failed to open workspace"))
}

#[test]
fn test_resolve_common_script_by_file_decomp() {
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_dspre_workspace());

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
    let ws = require_ws!(test_dspre_workspace());

    let script_file_id = 100;
    let local_script_id = 1;

    let result = resolve_script_id_by_file(
        local_script_id,
        script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve map script");

    if let Some(resolution) = result {
        assert!(
            resolution.is_map_script(),
            "Should be resolved as map script"
        );
        assert_eq!(resolution.script_file_id(), script_file_id);
    }
}

#[test]
fn test_resolve_level_script_by_file_dspre() {
    let ws = require_ws!(test_dspre_workspace());

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

    if let Some(resolution) = result {
        assert_eq!(
            resolution.script_file_id(),
            header.script_file_id(),
            "Resolved script file should match map's script file"
        );
    }
}

#[test]
fn test_resolve_script_id_by_level_script_file_dspre() {
    let ws = require_ws!(test_dspre_workspace());

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

    if let Some(resolution) = result {
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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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
    let ws = require_ws!(test_dspre_workspace());

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
    let ws = require_ws!(test_decomp_workspace());

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

#[test]
fn test_resolve_map_script_by_file_hgss_decomp() {
    let ws = require_ws!(test_hgss_decomp_workspace());

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
    .expect("Failed to resolve map script in HGSS decomp");

    assert!(result.is_some(), "Map script should resolve");
    let resolution = result.unwrap();
    assert!(resolution.is_map_script());
    assert_eq!(resolution.script_file_id(), script_file_id);
    assert_eq!(resolution.text_archive_id(), header.text_archive_id());
}

#[test]
fn test_resolve_level_script_by_file_hgss_decomp_matches_direct_map_resolution() {
    let ws = require_ws!(test_hgss_decomp_workspace());

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();
    let map_id = ws
        .provider
        .find_map_by_level_script_file_id(level_script_file_id)
        .expect("Failed to find map by level script file id")
        .expect("Expected map for level script file id");

    let by_file = resolve_level_script_by_file(
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve level script by file");

    let direct = resolve_level_script(map_id, &ws.global_script_table, ws.provider.as_ref())
        .expect("Failed to resolve level script by map id");

    assert_eq!(
        by_file, direct,
        "HGSS level-script-by-file should match direct map resolution"
    );
}

#[test]
fn test_resolve_script_id_by_level_script_file_hgss_dspre_matches_direct() {
    let ws = require_ws!(test_hgss_dspre_workspace());

    let header = ws
        .provider
        .get_map_header(0)
        .expect("Failed to get map header 0");
    let level_script_file_id = header.level_script_id();
    let map_id = ws
        .provider
        .find_map_by_level_script_file_id(level_script_file_id)
        .expect("Failed to find map by level script file id")
        .expect("Expected map for level script file id");

    let by_level_file = resolve_script_id_by_level_script_file(
        1,
        level_script_file_id,
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve script by level script file");

    let direct = resolve_script_id(
        1,
        Some(map_id),
        &ws.global_script_table,
        ws.provider.as_ref(),
    )
    .expect("Failed to resolve direct script id");

    assert_eq!(
        by_level_file, direct,
        "HGSS DSPRE level-script-file resolution should match direct map resolution"
    );
}
