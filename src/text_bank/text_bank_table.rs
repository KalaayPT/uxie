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

    pub fn get_all_names(&self) -> &Vec<String> {
        &self.names
    }
}
