use std::path::{Path, PathBuf};

/// Path resolver for decomp project data files
pub struct DecompPaths {
    root: PathBuf,
}

impl DecompPaths {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    // --- Build directory NARC paths (preferred if build exists) ---

    pub fn build_personal_narc(&self) -> PathBuf {
        self.root.join("build/res/pokemon/pl_personal.narc")
    }

    pub fn build_evo_narc(&self) -> PathBuf {
        self.root.join("build/res/pokemon/evo.narc")
    }

    pub fn build_wotbl_narc(&self) -> PathBuf {
        self.root.join("build/res/pokemon/wotbl.narc")
    }

    pub fn build_moves_narc(&self) -> PathBuf {
        self.root.join("build/res/battle/moves/pl_waza_tbl.narc")
    }

    pub fn build_items_narc(&self) -> PathBuf {
        self.root.join("build/res/items/pl_item_data.narc")
    }

    pub fn build_trdata_narc(&self) -> PathBuf {
        self.root.join("build/res/trainers/trdata.narc")
    }

    pub fn build_trpoke_narc(&self) -> PathBuf {
        self.root.join("build/res/trainers/trpoke.narc")
    }

    pub fn build_zone_event_narc(&self) -> PathBuf {
        self.root.join("build/res/field/events/zone_event.narc")
    }

    pub fn build_encounter_narc(&self) -> PathBuf {
        self.root
            .join("build/res/field/encounters/pl_enc_data.narc")
    }

    // --- Source file paths (fallback) ---

    pub fn source_pokemon_dir(&self) -> PathBuf {
        self.root.join("res/pokemon")
    }

    pub fn source_pokemon_data(&self, species_name: &str) -> PathBuf {
        self.root.join(format!(
            "res/pokemon/{}/data.json",
            species_name.to_lowercase()
        ))
    }

    pub fn source_moves_dir(&self) -> PathBuf {
        self.root.join("res/battle/moves")
    }

    pub fn source_move_data(&self, move_name: &str) -> PathBuf {
        self.root.join(format!(
            "res/battle/moves/{}/data.json",
            move_name.to_lowercase()
        ))
    }

    pub fn source_items_csv(&self) -> PathBuf {
        self.root.join("res/items/pl_item_data.csv")
    }

    pub fn source_trainers_dir(&self) -> PathBuf {
        self.root.join("res/trainers/data")
    }

    pub fn source_trainer_data(&self, trainer_name: &str) -> PathBuf {
        self.root.join(format!(
            "res/trainers/data/{}.json",
            trainer_name.to_lowercase()
        ))
    }

    pub fn source_events_dir(&self) -> PathBuf {
        self.root.join("res/field/events")
    }

    pub fn source_event_file(&self, map_name: &str) -> PathBuf {
        self.root.join(format!(
            "res/field/events/events_{}.json",
            map_name.to_lowercase()
        ))
    }

    pub fn source_encounters_dir(&self) -> PathBuf {
        self.root.join("res/field/encounters")
    }

    pub fn source_encounter_file(&self, map_name: &str) -> PathBuf {
        self.root.join(format!(
            "res/field/encounters/encounters_{}.json",
            map_name.to_lowercase()
        ))
    }

    // --- ds-rom paths ---

    pub fn dspre_personal_narc(&self) -> PathBuf {
        self.root.join("data/poketool/personal/pl_personal.narc")
    }

    pub fn dspre_evo_narc(&self) -> PathBuf {
        self.root.join("data/poketool/personal/evo.narc")
    }

    pub fn dspre_wotbl_narc(&self) -> PathBuf {
        self.root.join("data/poketool/personal/wotbl.narc")
    }

    pub fn dspre_moves_narc(&self) -> PathBuf {
        self.root.join("data/poketool/waza/pl_waza_tbl.narc")
    }

    pub fn dspre_items_narc(&self) -> PathBuf {
        self.root.join("data/itemtool/itemdata/pl_item_data.narc")
    }

    pub fn dspre_trdata_narc(&self) -> PathBuf {
        self.root.join("data/poketool/trainer/trdata.narc")
    }

    pub fn dspre_trpoke_narc(&self) -> PathBuf {
        self.root.join("data/poketool/trainer/trpoke.narc")
    }

    pub fn dspre_encounters_narc(&self) -> PathBuf {
        self.root
            .join("data/fielddata/encountdata/pl_enc_data.narc")
    }

    // --- Helper: check if build directory exists ---

    pub fn has_build_dir(&self) -> bool {
        self.root.join("build").exists()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_events_order(&self) -> PathBuf {
        self.root.join("res/field/events/zone_event.order")
    }

    pub fn source_encounters_order(&self) -> PathBuf {
        self.root.join("res/field/encounters/encounters.order")
    }
}
