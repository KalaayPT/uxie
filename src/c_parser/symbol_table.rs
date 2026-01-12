use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub use crate::c_parser::defines::parse_defines;
pub use crate::c_parser::defines::parse_value;
pub use crate::c_parser::enums::parse_enum;

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub defines: HashMap<String, i64>,
    pub pending_defines: HashMap<String, String>,
    pub enums: HashMap<String, Vec<(String, Option<i64>)>>,
    pub loaded_includes: Vec<String>,
    pub cache: Arc<RwLock<HashMap<String, i64>>>,
}

static RE_PYTHON_ENUM: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$").unwrap()
});

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
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
        for def in defines {
            self.pending_defines.insert(def.name, def.value);
        }
        if let Some(e) = parse_enum(content) {
            let mut current = 0i64;
            let mut variants = Vec::new();
            for v in &e.variants {
                if let Some(val) = v.value { current = val; }
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
        if let Some(&val) = self.defines.get(name) { return Some(val); }
        if let Some(&val) = self.cache.read().unwrap().get(name) { return Some(val); }
        
        for variants in self.enums.values() {
            for (variant_name, value) in variants {
                if variant_name == name { return *value; }
            }
        }

        let expr = self.pending_defines.get(name)?;
        let val = crate::c_parser::defines::eval_expr_with_context(
            expr, 
            &self.pending_defines, 
            &self.defines,
            &mut self.cache.write().unwrap()
        )?;
        Some(val)
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        let mut best_match = None;
        for (name, &val) in &self.defines {
            if val == value && name.starts_with(prefix) {
                if best_match.as_ref().map_or(true, |m: &String| name.len() < m.len()) {
                    best_match = Some(name.clone());
                }
            }
        }
        for variants in self.enums.values() {
            for (variant_name, val) in variants {
                if val == &Some(value) && variant_name.starts_with(prefix) {
                    if best_match.as_ref().map_or(true, |m: &String| variant_name.len() < m.len()) {
                        best_match = Some(variant_name.clone());
                    }
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
        for variants in self.enums.values() {
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
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") { continue; }
            if let Some(pos) = line.find('=') {
                let name = line[..pos].trim().to_string();
                let expr = line[pos + 1..].trim().to_string();
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    &expr, 
                    &self.pending_defines, 
                    &self.defines,
                    &mut self.cache.write().unwrap()
                ) {
                    current_index = val;
                }
                self.defines.insert(name.clone(), current_index);
                self.pending_defines.insert(name, current_index.to_string());
                current_index += 1;
            } else {
                self.defines.insert(line.to_string(), current_index);
                self.pending_defines.insert(line.to_string(), current_index.to_string());
                current_index += 1;
            }
        }
        Ok(())
    }

    pub fn load_text_bank_json(&mut self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let messages = json.get("messages").and_then(|v| v.as_array());
        let mut count = 0;
        if let Some(messages) = messages {
            for (index, msg) in messages.iter().enumerate() {
                if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                    self.defines.insert(id.to_string(), index as i64);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    pub fn load_headers_from_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<usize> {
        let mut count = 0;
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("h") || ext.eq_ignore_ascii_case("hpp") {
                self.load_header(&path)?;
                count += 1;
            } else if ext.eq_ignore_ascii_case("txt") {
                self.load_list_file(&path)?;
                count += 1;
            } else if ext.eq_ignore_ascii_case("py") {
                self.load_python_enum(&path)?;
                count += 1;
            } else if ext.eq_ignore_ascii_case("json") {
                if let Ok(c) = self.load_text_bank_json(&path) { count += c; }
            } else if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) == Some(".git") { continue; }
                count += self.load_headers_from_dir(&path)?;
            }
        }
        Ok(count)
    }

    pub fn load_from_url(&mut self, url: &str) -> std::io::Result<()> {
        let output = std::process::Command::new("curl").arg("-L").arg("-s").arg(url).output()?;
        if !output.status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to fetch URL: {}", url)));
        }
        let content = String::from_utf8_lossy(&output.stdout);
        if url.ends_with(".txt") { self.load_list_file_str(&content) }
        else if url.ends_with(".py") { self.load_python_enum_str(&content) }
        else { self.load_header_str(&content) }
    }

    pub fn load_python_enum(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_python_enum_str(&content)
    }

    pub fn load_python_enum_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if let Some(caps) = RE_PYTHON_ENUM.captures(line) {
                let name = caps[1].to_string();
                let expr = caps[2].trim().to_string();
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    &expr, 
                    &self.pending_defines, 
                    &self.defines,
                    &mut self.cache.write().unwrap()
                ) {
                    self.defines.insert(name.clone(), val);
                    self.pending_defines.insert(name, val.to_string());
                }
            }
        }
        Ok(())
    }

    pub fn extend(&mut self, other: SymbolTable) {
        self.defines.extend(other.defines);
        self.pending_defines.extend(other.pending_defines);
        self.enums.extend(other.enums);
        self.loaded_includes.extend(other.loaded_includes);
        let mut cache = self.cache.write().unwrap();
        let other_cache = other.cache.read().unwrap();
        for (k, v) in other_cache.iter() {
            cache.insert(k.clone(), *v);
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
        assert_eq!(table.resolve_constant("NORTH"), Some(0));
        assert_eq!(table.resolve_constant("SOUTH"), Some(1));

        assert_eq!(table.resolve_name(42, "FOO"), Some("FOO".into()));
        assert_eq!(table.resolve_name(0, "NORTH"), Some("NORTH".into()));
    }

    #[test]
    fn test_recursive_defines() {
        let mut table = SymbolTable::new();
        table.load_header_str(r#"
            #define A B
            #define B C
            #define C 100
        "#).unwrap();
        assert_eq!(table.resolve_constant("A"), Some(100));
        assert_eq!(table.resolve_constant("B"), Some(100));
    }

    #[test]
    fn test_rgb_macro() {
        let mut table = SymbolTable::new();
        table.load_header_str(r#"
            #define R 31
            #define G 0
            #define B 0
            #define MY_COLOR RGB(R, G, B)
        "#).unwrap();
        assert_eq!(table.resolve_constant("MY_COLOR"), Some(31));
    }

    #[test]
    fn test_complex_rgb() {
        let mut table = SymbolTable::new();
        table.load_header_str(r#"
            #define COLOR_RGB_R_MASK  0x001F
            #define COLOR_RGB_G_SHIFT 5
            #define COLOR_RGB_B_SHIFT 10
            #define RGB(r, g, b)   (((b) << COLOR_RGB_B_SHIFT) | ((g) << COLOR_RGB_G_SHIFT) | (r))
            #define COLOR_WHITE       RGB(31, 31, 31)
        "#).unwrap();
        assert_eq!(table.resolve_constant("COLOR_WHITE"), Some(32767));
    }

    #[test]
    fn test_load_python_enum() {
        let mut table = SymbolTable::new();
        table.load_python_enum_str(r#"
            VAR_A = 10
            VAR_B = VAR_A << 2
        "#).unwrap();
        assert_eq!(table.resolve_constant("VAR_A"), Some(10));
        assert_eq!(table.resolve_constant("VAR_B"), Some(40));
    }

    #[test]
    fn test_load_text_bank_json() {
        let mut table = SymbolTable::new();
        let json = r#"{
            "messages": [
                { "id": "MSG_HELLO", "text": "Hello" },
                { "id": "MSG_BYE", "text": "Bye" }
            ]
        }"#;
        
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        temp.write_all(json.as_bytes()).unwrap();
        
        table.load_text_bank_json(temp.path()).unwrap();
        assert_eq!(table.resolve_constant("MSG_HELLO"), Some(0));
        assert_eq!(table.resolve_constant("MSG_BYE"), Some(1));
    }

    #[test]
    fn test_performance_caching() {
        let mut table = SymbolTable::new();
        let mut source = String::new();
        source.push_str("#define L0 1\n");
        for i in 1..200 {
            source.push_str(&format!("#define L{} (L{} << 1) | L{}\n", i, i-1, i-1));
        }
        
        table.load_header_str(&source).unwrap();
        
        let start = std::time::Instant::now();
        let val = table.resolve_constant("L199");
        let duration = start.elapsed();
        
        assert!(val.is_some());
        
        let start2 = std::time::Instant::now();
        let val2 = table.resolve_constant("L199");
        let duration2 = start2.elapsed();
        
        assert_eq!(val, val2);
        assert!(duration2 < duration);
    }
}
