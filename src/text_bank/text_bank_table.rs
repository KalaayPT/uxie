use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TextBankTable {
    pub names: Vec<String>,
    pub name_to_id: HashMap<String, usize>,
}

impl TextBankTable {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn load_list_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_list_str(&content)
    }

    pub fn load_list_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") || line.starts_with("//") {
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
}
