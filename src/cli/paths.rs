use std::path::{Path, PathBuf};
use uxie::GameFamily;
use uxie::decomp_data::DecompPaths;

pub fn family_name(family: GameFamily) -> &'static str {
    match family {
        GameFamily::DP => "Diamond/Pearl",
        GameFamily::Platinum => "Platinum",
        GameFamily::HGSS => "HeartGold/SoulSilver",
    }
}

pub fn encounter_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP => paths
            .root()
            .join("data/fielddata/encountdata/d_enc_data.narc"),
        GameFamily::Platinum => paths.dspre_encounters_narc(),
        GameFamily::HGSS => paths.root().join("data/a/0/3/7"),
    }
}

pub fn personal_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_personal_narc(),
        GameFamily::HGSS => paths.root().join("data/pbr/personal.narc"),
    }
}

pub fn move_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_moves_narc(),
        GameFamily::HGSS => paths.root().join("data/data/kowaza.narc"),
    }
}

pub fn item_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_items_narc(),
        GameFamily::HGSS => paths.root().join("data/pbr/item_data.narc"),
    }
}

pub fn trainer_data_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_trdata_narc(),
        GameFamily::HGSS => paths.root().join("data/a/0/5/5"),
    }
}

pub fn trainer_party_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_trpoke_narc(),
        GameFamily::HGSS => paths.root().join("data/a/0/5/6"),
    }
}

pub fn evolution_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_evo_narc(),
        GameFamily::HGSS => paths.root().join("data/a/0/3/4"),
    }
}

pub fn learnset_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.dspre_wotbl_narc(),
        GameFamily::HGSS => paths.root().join("data/pbr/pms.narc"),
    }
}

pub fn egg_move_narc_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::HGSS => paths.root().join("data/data/kowaza.narc"),
        GameFamily::DP | GameFamily::Platinum => PathBuf::new(),
    }
}

pub fn egg_move_overlay_path(project_path: &Path, family: GameFamily) -> PathBuf {
    let paths = project_paths(project_path);
    match family {
        GameFamily::DP | GameFamily::Platinum => paths.root().join("overlay/overlay_0005.bin"),
        GameFamily::HGSS => PathBuf::new(),
    }
}

fn project_paths(project_path: &Path) -> DecompPaths {
    DecompPaths::new(project_path)
}
