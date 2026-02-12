use super::types::{EVOLUTION_FILE_SIZE, EvolutionData};
#[cfg(test)]
use super::types::{EvolutionEntry, EvolutionMethod};
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

impl EvolutionData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(EVOLUTION_FILE_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Cursor;

    #[test]
    fn test_roundtrip() {
        let mut evo = EvolutionData::default();
        evo.entries[0] = EvolutionEntry {
            method: 4,
            param: 16,
            target_species: 2,
        };
        evo.entries[1] = EvolutionEntry {
            method: 1,
            param: 32,
            target_species: 3,
        };

        let bytes = evo.to_bytes();
        assert_eq!(bytes.len(), EVOLUTION_FILE_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = EvolutionData::from_binary(&mut cursor).unwrap();

        assert_eq!(evo, parsed);
    }

    #[test]
    fn test_active_evolutions() {
        let mut evo = EvolutionData::default();
        evo.entries[0] = EvolutionEntry {
            method: 4,
            param: 16,
            target_species: 2,
        };
        evo.entries[2] = EvolutionEntry {
            method: 1,
            param: 32,
            target_species: 3,
        };

        assert_eq!(evo.active_evolutions().count(), 2);
    }

    fn evolution_entry_strategy() -> impl Strategy<Value = EvolutionEntry> {
        (any::<u16>(), any::<u16>(), any::<u16>()).prop_map(|(method, param, target_species)| {
            EvolutionEntry {
                method,
                param,
                target_species,
            }
        })
    }

    fn evolution_data_strategy() -> impl Strategy<Value = EvolutionData> {
        prop::collection::vec(evolution_entry_strategy(), 7).prop_map(|entries| EvolutionData {
            entries: entries.try_into().unwrap(),
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_evolution_data_roundtrip(evo in evolution_data_strategy()) {
            let bytes = evo.to_bytes();
            prop_assert_eq!(bytes.len(), EVOLUTION_FILE_SIZE);
            let mut cursor = Cursor::new(bytes);
            let parsed = EvolutionData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(evo, parsed);
        }

        #[test]
        fn prop_active_evolutions_count_matches_non_empty(evo in evolution_data_strategy()) {
            let expected = evo.entries.iter().filter(|e| !e.is_empty()).count();
            let actual = evo.active_evolutions().count();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_evolution_method_from_u16_maps(v in any::<u16>()) {
            let mapped = EvolutionMethod::from(v);
            let expected = match v {
                0 => EvolutionMethod::None,
                1 => EvolutionMethod::Happiness,
                2 => EvolutionMethod::HappinessDay,
                3 => EvolutionMethod::HappinessNight,
                4 => EvolutionMethod::LevelUp,
                5 => EvolutionMethod::Trade,
                6 => EvolutionMethod::TradeWithItem,
                7 => EvolutionMethod::UseItem,
                8 => EvolutionMethod::LevelUpAtkGtDef,
                9 => EvolutionMethod::LevelUpAtkEqDef,
                10 => EvolutionMethod::LevelUpDefGtAtk,
                11 => EvolutionMethod::LevelUpPersonalityLow,
                12 => EvolutionMethod::LevelUpPersonalityHigh,
                13 => EvolutionMethod::LevelUpSpawnPokemon,
                14 => EvolutionMethod::LevelUpSpawnEmptySlot,
                15 => EvolutionMethod::Beauty,
                16 => EvolutionMethod::UseItemMale,
                17 => EvolutionMethod::UseItemFemale,
                18 => EvolutionMethod::LevelUpWithItem,
                19 => EvolutionMethod::LevelUpWithMoveType,
                20 => EvolutionMethod::LevelUpWithPartyMember,
                21 => EvolutionMethod::LevelUpMale,
                22 => EvolutionMethod::LevelUpFemale,
                23 => EvolutionMethod::LevelUpAtLocation,
                24 => EvolutionMethod::LevelUpAtMtCoronet,
                25 => EvolutionMethod::LevelUpNearMossRock,
                26 => EvolutionMethod::LevelUpNearIceRock,
                n => EvolutionMethod::Unknown(n),
            };
            prop_assert_eq!(mapped, expected);
        }
    }
}
