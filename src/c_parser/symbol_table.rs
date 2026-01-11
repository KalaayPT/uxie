use std::collections::HashMap;
use std::path::Path;

pub use crate::c_parser::defines::parse_defines;
pub use crate::c_parser::defines::parse_value;
pub use crate::c_parser::enums::parse_enum;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub defines: HashMap<String, i64>,
    pub enums: HashMap<String, Vec<(String, Option<i64>)>>,
    pub loaded_includes: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            defines: HashMap::new(),
            enums: HashMap::new(),
            loaded_includes: Vec::new(),
        }
    }

    pub fn load_header(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };
        self.load_header_str(&content)?;

        let path_str = path.to_string_lossy().into_owned();
        if !self.loaded_includes.contains(&path_str) {
            self.loaded_includes.push(path_str);
        }

        Ok(())
    }

    pub fn load_header_str(&mut self, content: &str) -> std::io::Result<()> {
        let defines = parse_defines(content);
        for def in &defines {
            let value = parse_value(&def.value, &defines);
            if let Some(v) = value {
                self.defines.insert(def.name.clone(), v);
            }
        }

        if let Some(e) = parse_enum(content) {
            let mut current = 0i64;
            let mut variants = Vec::new();
            for v in &e.variants {
                if let Some(val) = v.value {
                    current = val;
                }
                self.defines.insert(v.name.clone(), current);
                variants.push((v.name.clone(), Some(current)));
                current += 1;
            }
            if let Some(name) = &e.name {
                self.enums.insert(name.clone(), variants);
            }
        }
        Ok(())
    }

    pub fn resolve_constant(&self, name: &str) -> Option<i64> {
        if let Some(val) = self.defines.get(name) {
            return Some(*val);
        }

        for (_enum_name, variants) in &self.enums {
            for (variant_name, value) in variants {
                if variant_name == name {
                    return *value;
                }
            }
        }

        None
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        let mut best_match = None;
        for (name, &val) in &self.defines {
            if val == value && name.starts_with(prefix) {
                if best_match
                    .as_ref()
                    .map_or(true, |m: &String| name.len() < m.len())
                {
                    best_match = Some(name.clone());
                }
            }
        }
        best_match
    }

    pub fn resolve_names(&self, value: i64, prefixes: &[&str]) -> Vec<String> {
        let mut matches = Vec::new();
        for (name, &val) in &self.defines {
            if val == value {
                if prefixes.is_empty() || prefixes.iter().any(|p| name.starts_with(p)) {
                    matches.push(name.clone());
                }
            }
        }

        for (_enum_name, variants) in &self.enums {
            for (variant_name, val) in variants {
                if val == &Some(value) {
                    if prefixes.is_empty() || prefixes.iter().any(|p| variant_name.starts_with(p)) {
                        matches.push(variant_name.clone());
                    }
                }
            }
        }

        matches.sort_by_key(|n| n.len());
        matches
    }

    pub fn load_list_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(()),
        };
        self.load_list_file_str(&content)
    }

    pub fn load_list_file_str(&mut self, content: &str) -> std::io::Result<()> {
        let mut current_index = 0i64;
        let mut pending_defines: Vec<crate::c_parser::defines::CDefine> = self
            .defines
            .iter()
            .map(|(n, v)| crate::c_parser::defines::CDefine {
                name: n.clone(),
                value: v.to_string(),
                resolved: Some(*v),
            })
            .collect();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") {
                continue;
            }

            if let Some(pos) = line.find('=') {
                let name = line[..pos].trim().to_string();
                let expr = line[pos + 1..].trim().to_string();

                if let Some(val) = crate::c_parser::parse_value(&expr, &pending_defines) {
                    current_index = val;
                }

                self.defines.insert(name.clone(), current_index);
                pending_defines.push(crate::c_parser::defines::CDefine {
                    name,
                    value: expr,
                    resolved: Some(current_index),
                });
                current_index += 1;
            } else {
                self.defines.insert(line.to_string(), current_index);
                pending_defines.push(crate::c_parser::defines::CDefine {
                    name: line.to_string(),
                    value: current_index.to_string(),
                    resolved: Some(current_index),
                });
                current_index += 1;
            }
        }
        Ok(())
    }

    pub fn load_headers_from_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<usize> {
        let mut count = 0;
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str());
            let ext_str = ext.unwrap_or("");

            if ext_str.eq_ignore_ascii_case("h") || ext_str.eq_ignore_ascii_case("hpp") {
                self.load_header(&path)?;
                count += 1;
            } else if ext_str.eq_ignore_ascii_case("txt") {
                self.load_list_file(&path)?;
                count += 1;
            } else if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) == Some(".git") {
                    continue;
                }
                count += self.load_headers_from_dir(&path)?;
            }
        }
        Ok(count)
    }

    pub fn load_from_url(&mut self, url: &str) -> std::io::Result<()> {
        let output = std::process::Command::new("curl")
            .arg("-s")
            .arg(url)
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to fetch URL: {}", url),
            ));
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let is_txt = url.ends_with(".txt");

        if is_txt {
            self.load_list_file_str(&content)
        } else {
            self.load_header_str(&content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();

        table.defines.insert("FOO".into(), 42);
        table.defines.insert("BAR".into(), 24);
        table.enums.insert(
            "Direction".into(),
            vec![
                ("NORTH".into(), Some(0)),
                ("SOUTH".into(), Some(1)),
                ("EAST".into(), Some(2)),
                ("WEST".into(), Some(3)),
            ],
        );

        assert_eq!(table.resolve_constant("FOO"), Some(42));
        assert_eq!(table.resolve_constant("BAR"), Some(24));
        assert_eq!(table.resolve_constant("Direction"), None);
        assert_eq!(table.resolve_constant("NORTH"), Some(0));
        assert_eq!(table.resolve_constant("SOUTH"), Some(1));
        assert_eq!(table.resolve_constant("UNKNOWN"), None);
    }

    #[test]
    fn test_load_header() {
        let source = r#"
#define FOO 42
#define BAR 24
enum Direction {
    NORTH = 0,
    SOUTH = 1,
    EAST = 2,
    WEST = 3,
}
        "#;

        let mut table = SymbolTable::new();
        table.load_header_str(source).unwrap();

        assert_eq!(table.resolve_constant("FOO"), Some(42));
        assert_eq!(table.resolve_constant("BAR"), Some(24));
        assert_eq!(table.resolve_constant("NORTH"), Some(0));
        assert_eq!(table.resolve_constant("SOUTH"), Some(1));
        assert_eq!(table.resolve_constant("EAST"), Some(2));
        assert_eq!(table.resolve_constant("WEST"), Some(3));
    }

    #[test]
    fn test_load_list_file() {
        let source = r#"
VARS_START = 16384
MAP_LOCAL_VARS_START = VARS_START
VAR_MAP_LOCAL_0 = MAP_LOCAL_VARS_START
VAR_MAP_LOCAL_1
SOME_ENUM_VAL
        "#;

        let mut table = SymbolTable::new();
        table.load_list_file_str(source).unwrap();

        assert_eq!(table.resolve_constant("VARS_START"), Some(16384));
        assert_eq!(table.resolve_constant("MAP_LOCAL_VARS_START"), Some(16384));
        assert_eq!(table.resolve_constant("VAR_MAP_LOCAL_0"), Some(16384));
        assert_eq!(table.resolve_constant("VAR_MAP_LOCAL_1"), Some(16385));
        assert_eq!(table.resolve_constant("SOME_ENUM_VAL"), Some(16386));
    }
}
