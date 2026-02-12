use rustc_hash::FxHashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TextBankTable {
    pub(crate) names: Vec<String>,
    pub(crate) name_to_id: FxHashMap<String, usize>,
}

impl TextBankTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_list_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_list_str(&content)
    }

    pub fn load_list_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let name = if let Some(pos) = line.find('=') {
                line[..pos].trim().to_string()
            } else {
                line.to_string()
            };

            if !self.name_to_id.contains_key(&name) {
                self.name_to_id.insert(name.clone(), self.names.len());
                self.names.push(name);
            }
        }
        Ok(())
    }

    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.names.get(id).map(|s| s.as_str())
    }

    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.name_to_id.get(name).copied()
    }

    pub fn get_all_names(&self) -> &[String] {
        &self.names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn unique_names_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(any::<u16>(), 0..64).prop_map(|ids| {
            ids.into_iter()
                .enumerate()
                .map(|(idx, value)| format!("TEXT_BANK_{}_{}", idx, value))
                .collect()
        })
    }

    fn names_with_duplicates_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(any::<u8>(), 0..64).prop_map(|ids| {
            ids.into_iter()
                .map(|value| format!("TEXT_BANK_DUP_{}", value % 16))
                .collect()
        })
    }

    #[test]
    fn test_load_list_skips_comments_and_blank_lines() {
        let mut table = TextBankTable::new();
        table
            .load_list_str(
                r"
                # hash comment
                // slash comment

                TEXT_BANK_COMMON = 0x0D5
                TEXT_BANK_CITY
                ",
            )
            .unwrap();

        assert_eq!(
            table.get_all_names(),
            vec!["TEXT_BANK_COMMON".to_string(), "TEXT_BANK_CITY".to_string()].as_slice()
        );
        assert_eq!(table.get_id("TEXT_BANK_COMMON"), Some(0));
        assert_eq!(table.get_id("TEXT_BANK_CITY"), Some(1));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_unique_names_roundtrip(names in unique_names_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push('\n');
            }

            let mut table = TextBankTable::new();
            table.load_list_str(&content).unwrap();

            prop_assert_eq!(table.get_all_names().len(), names.len());
            for (idx, name) in names.iter().enumerate() {
                prop_assert_eq!(table.get_name(idx), Some(name.as_str()));
                prop_assert_eq!(table.get_id(name), Some(idx));
            }
        }

        #[test]
        fn prop_assignment_syntax_extracts_lhs(
            names in unique_names_strategy(),
            values in prop::collection::vec(any::<u16>(), 0..64)
        ) {
            let mut content = String::new();
            for (idx, name) in names.iter().enumerate() {
                let value = values.get(idx).copied().unwrap_or(0);
                content.push_str("  ");
                content.push_str(name);
                content.push_str("   =   ");
                content.push_str(&format!("0x{:X}", value));
                content.push('\n');
            }

            let mut table = TextBankTable::new();
            table.load_list_str(&content).unwrap();

            prop_assert_eq!(table.get_all_names(), names.as_slice());
            for (idx, name) in names.iter().enumerate() {
                prop_assert_eq!(table.get_id(name), Some(idx));
            }
        }

        #[test]
        fn prop_duplicate_names_first_wins(names in names_with_duplicates_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push_str(" = 1\n");
            }

            let mut table = TextBankTable::new();
            table.load_list_str(&content).unwrap();

            let mut unique_names = Vec::<String>::new();
            for name in &names {
                if !unique_names.contains(name) {
                    unique_names.push(name.clone());
                }
            }

            prop_assert_eq!(table.get_all_names(), unique_names.as_slice());
            for (unique_idx, name) in unique_names.iter().enumerate() {
                prop_assert_eq!(table.get_id(name), Some(unique_idx));
            }
        }

        #[test]
        fn prop_unknown_or_oob_queries_return_none(names in unique_names_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push('\n');
            }

            let mut table = TextBankTable::new();
            table.load_list_str(&content).unwrap();

            let unknown = "TEXT_BANK_DOES_NOT_EXIST";
            prop_assert_eq!(table.get_id(unknown), None);
            prop_assert_eq!(table.get_name(names.len()), None);
            prop_assert_eq!(table.get_name(names.len().saturating_add(100)), None);
        }
    }
}
