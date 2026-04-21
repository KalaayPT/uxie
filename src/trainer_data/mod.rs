mod binary;
mod types;

pub use binary::{TRAINER_PROPERTIES_SIZE, load_all_dspre_trainers, load_dspre_trainer};
pub use types::{AiFlags, PartyPokemon, TrainerData, TrainerFlags, TrainerProperties};
