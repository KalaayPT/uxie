use rustc_hash::FxHashMap;
use std::collections::HashMap;
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

    pub fn get_name(&self, id: usize) -> Option<&String> {
        self.names.get(id)
    }

    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.name_to_id.get(name).copied()
    }

    pub fn get_all_names(&self) -> &Vec<String> {
        &self.names
    }

    pub fn get_name_to_id_std(&self) -> HashMap<String, usize> {
        let mut res = HashMap::with_capacity(self.name_to_id.len());
        for (k, v) in &self.name_to_id {
            res.insert(k.clone(), *v);
        }
        res
    }
}
