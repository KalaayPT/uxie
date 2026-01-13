use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

pub use crate::c_parser::defines::parse_defines;
pub use crate::c_parser::defines::parse_value;
pub use crate::c_parser::enums::{parse_enum, parse_enums};
pub use crate::c_parser::source_manager::SourceManager;

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub(crate) defines: FxHashMap<String, i64>,
    pub(crate) pending_defines: FxHashMap<String, String>,
    pub(crate) enums: FxHashMap<String, Vec<(String, Option<i64>)>>,
    pub(crate) variant_to_value: FxHashMap<String, i64>,
    pub(crate) defines_by_value: FxHashMap<i64, Vec<String>>,
    pub(crate) enum_by_value: FxHashMap<i64, Vec<String>>,
    pub(crate) loaded_includes: FxHashSet<String>,
    pub(crate) cache: Arc<DashMap<String, i64>>,
    pub(crate) shortest_name_cache: Arc<DashMap<i64, String>>,
    pub(crate) source_manager: Option<SourceManager>,
    pub(crate) parent: Option<Arc<SymbolTable>>,
}

static RE_PYTHON_ENUM: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$").unwrap()
});

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source_manager(sm: SourceManager) -> Self {
        Self {
            source_manager: Some(sm),
            ..Default::default()
        }
    }

    pub fn with_parent(parent: Arc<SymbolTable>) -> Self {
        Self {
            cache: Arc::clone(&parent.cache),
            shortest_name_cache: Arc::clone(&parent.shortest_name_cache),
            source_manager: parent.source_manager.clone(),
            parent: Some(parent),
            ..Default::default()
        }
    }

    pub fn load_header(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().into_owned();
        if self.loaded_includes.contains(&path_str) {
            return Ok(());
        }

        if let Some(sm) = &self.source_manager {
            let entry = sm.get_or_parse(path)?;
            self.load_file_entry(&entry);
        } else {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return Ok(()),
            };
            self.load_header_str(&content)?;
        }
        
        self.loaded_includes.insert(path_str);
        Ok(())
    }

    pub fn load_recursive(&mut self, path: impl AsRef<Path>, include_dirs: &[PathBuf]) -> std::io::Result<()> {
        let path = path.as_ref();
        let sm = self.source_manager.get_or_insert_with(SourceManager::new).clone();
        let mut visited = FxHashSet::default();
        self.load_recursive_internal(path, include_dirs, &sm, &mut visited)
    }

    pub fn load_recursive_str(&mut self, content: &str, root_dir: impl AsRef<Path>, include_dirs: &[PathBuf]) -> std::io::Result<()> {
        let root_dir = root_dir.as_ref();
        let sm = self.source_manager.get_or_insert_with(SourceManager::new).clone();
        
        for def in parse_defines(content) {
            self.process_define(def.name, def.value);
        }
        for e in parse_enums(content) {
            self.process_enum(e);
        }
        
        let includes = crate::c_parser::includes::parse_includes(content);
        for inc in includes {
            if inc.is_system { continue; }
            let mut found_path = None;
            let rel = root_dir.join(&inc.path);
            if rel.exists() {
                found_path = Some(rel);
            } else {
                for dir in include_dirs {
                    let p = dir.join(&inc.path);
                    if p.exists() {
                        found_path = Some(p);
                        break;
                    }
                }
            }

            if let Some(p) = found_path {
                let mut visited = FxHashSet::default();
                self.load_recursive_internal(&p, include_dirs, &sm, &mut visited)?;
            }
        }
        Ok(())
    }

    fn load_recursive_internal(&mut self, path: &Path, include_dirs: &[PathBuf], sm: &SourceManager, visited: &mut FxHashSet<PathBuf>) -> std::io::Result<()> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !visited.insert(canonical.clone()) {
            return Ok(());
        }

        let entry = sm.get_or_parse(path)?;
        self.load_file_entry(&entry);
        self.loaded_includes.insert(path.to_string_lossy().into_owned());

        let parent_dir = path.parent().unwrap_or(Path::new("."));

        for inc in &entry.includes {
            if inc.is_system { continue; }
            
            let mut found_path = None;
            let rel = parent_dir.join(&inc.path);
            if rel.exists() {
                found_path = Some(rel);
            } else {
                for dir in include_dirs {
                    let p = dir.join(&inc.path);
                    if p.exists() {
                        found_path = Some(p);
                        break;
                    }
                }
            }

            if let Some(p) = found_path {
                self.load_recursive_internal(&p, include_dirs, sm, visited)?;
            }
        }

        Ok(())
    }

    fn load_file_entry(&mut self, entry: &crate::c_parser::source_manager::FileEntry) {
        for def in &entry.defines {
            self.process_define(def.name.clone(), def.value.clone());
        }
        for e in &entry.enums {
            self.process_enum(e.clone());
        }
    }

    fn process_define(&mut self, name: String, value: String) {
        let val_trimmed = value.trim();
        if val_trimmed.starts_with("0x") || val_trimmed.starts_with("0X") {
            if let Ok(val) = i64::from_str_radix(&val_trimmed[2..], 16) {
                self.defines.insert(name.clone(), val);
                self.defines_by_value.entry(val).or_default().push(name.clone());
                self.pending_defines.insert(name, value);
                return;
            }
        }
        if let Ok(val) = val_trimmed.parse::<i64>() {
            self.defines.insert(name.clone(), val);
            self.defines_by_value.entry(val).or_default().push(name.clone());
            self.pending_defines.insert(name, value);
            return;
        }
        self.pending_defines.insert(name, value);
    }

    fn process_enum(&mut self, e: crate::c_parser::enums::CEnum) {
        let mut current = 0i64;
        let mut variants = Vec::new();
        for v in &e.variants {
            if let Some(val) = v.value { current = val; }
            self.defines.insert(v.name.clone(), current);
            self.variant_to_value.insert(v.name.clone(), current);
            self.defines_by_value.entry(current).or_default().push(v.name.clone());
            variants.push((v.name.clone(), Some(current)));
            current += 1;
        }
        if let Some(name) = &e.name {
            self.enums.insert(name.clone(), variants.clone());
            for (v_name, v_val) in variants {
                if let Some(val) = v_val {
                    self.enum_by_value.entry(val).or_default().push(v_name);
                }
            }
        }
    }

    pub fn load_header_str(&mut self, content: &str) -> std::io::Result<()> {
        for def in parse_defines(content) {
            self.process_define(def.name, def.value);
        }
        for e in parse_enums(content) {
            self.process_enum(e);
        }
        Ok(())
    }

    fn lookup_defines(&self, name: &str) -> Option<i64> {
        self.defines.get(name).copied()
            .or_else(|| self.parent.as_ref()?.lookup_defines(name))
    }

    fn lookup_variant(&self, name: &str) -> Option<i64> {
        self.variant_to_value.get(name).copied()
            .or_else(|| self.parent.as_ref()?.lookup_variant(name))
    }

    fn lookup_pending(&self, name: &str) -> Option<&String> {
        self.pending_defines.get(name)
            .or_else(|| self.parent.as_ref()?.lookup_pending(name))
    }

    pub fn resolve_constant(&self, name: &str) -> Option<i64> {
        if let Some(val) = self.lookup_defines(name) { return Some(val); }
        if let Some(val) = self.lookup_variant(name) { return Some(val); }
        if let Some(val) = self.cache.get(name) { return Some(*val); }
        
        let expr = self.lookup_pending(name)?;
        let val = crate::c_parser::defines::eval_expr_with_context(
            expr, 
            &self.pending_defines, 
            &self.defines,
            &self.cache
        )?;
        Some(val)
    }

    pub fn evaluate_expression(&self, expr: &str) -> Option<i64> {
        crate::c_parser::defines::eval_expr_with_context(
            expr,
            &self.pending_defines,
            &self.defines,
            &self.cache
        )
    }

    pub fn collect_for_file(path: impl AsRef<Path>, include_dirs: &[PathBuf], sm: SourceManager) -> std::io::Result<Self> {
        let mut table = Self::with_source_manager(sm);
        table.load_recursive(path, include_dirs)?;
        Ok(table)
    }

    pub fn get_source_manager(&self) -> SourceManager {
        self.source_manager.clone().unwrap_or_else(SourceManager::new)
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        if prefix.is_empty() {
            if let Some(cached) = self.shortest_name_cache.get(&value) {
                return Some(cached.clone());
            }
        }

        let mut best: Option<String> = None;

        let mut check_table = |table: &SymbolTable| {
            if let Some(names) = table.defines_by_value.get(&value) {
                if let Some(name) = names.iter().filter(|n| n.starts_with(prefix)).min_by_key(|n| n.len()) {
                    if best.is_none() || name.len() < best.as_ref().unwrap().len() {
                        best = Some(name.clone());
                    }
                }
            }
            if let Some(names) = table.enum_by_value.get(&value) {
                if let Some(name) = names.iter().filter(|n| n.starts_with(prefix)).min_by_key(|n| n.len()) {
                    if best.is_none() || name.len() < best.as_ref().unwrap().len() {
                        best = Some(name.clone());
                    }
                }
            }
        };

        check_table(self);
        let mut current = self.parent.as_ref();
        while let Some(p) = current {
            check_table(p);
            current = p.parent.as_ref();
        }

        if let Some(name) = best {
            if prefix.is_empty() {
                self.shortest_name_cache.insert(value, name.clone());
            }
            return Some(name);
        }

        None
    }

    pub fn resolve_names(&self, value: i64, prefixes: &[&str]) -> Vec<String> {
        let mut matches = FxHashSet::default();
        
        let mut collect_from = |table: &SymbolTable| {
            if let Some(names) = table.defines_by_value.get(&value) {
                for name in names {
                    if prefixes.is_empty() || prefixes.iter().any(|p| name.starts_with(p)) {
                        matches.insert(name.clone());
                    }
                }
            }
            if let Some(names) = table.enum_by_value.get(&value) {
                for name in names {
                    if prefixes.is_empty() || prefixes.iter().any(|p| name.starts_with(p)) {
                        matches.insert(name.clone());
                    }
                }
            }
        };

        collect_from(self);
        let mut current = self.parent.as_ref();
        while let Some(p) = current {
            collect_from(p);
            current = p.parent.as_ref();
        }

        let mut res: Vec<_> = matches.into_iter().collect();
        res.sort_by_key(|n| n.len());
        res
    }

    pub fn load_list_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
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
                    &self.cache
                ) {
                    current_index = val;
                }
                self.defines.insert(name.clone(), current_index);
                self.defines_by_value.entry(current_index).or_default().push(name.clone());
                self.pending_defines.insert(name, current_index.to_string());
                current_index += 1;
            } else {
                let name = line.to_string();
                self.defines.insert(name.clone(), current_index);
                self.defines_by_value.entry(current_index).or_default().push(name.clone());
                self.pending_defines.insert(name, current_index.to_string());
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
                    let val = index as i64;
                    self.defines.insert(id.to_string(), val);
                    self.defines_by_value.entry(val).or_default().push(id.to_string());
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    pub fn load_headers_from_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<usize> {
        let mut files = Vec::new();
        self.collect_header_files(dir.as_ref(), &mut files)?;
        let count = files.len();
        
        let sm = self.source_manager.get_or_insert_with(SourceManager::new).clone();

        let results: Vec<SymbolTable> = files.into_par_iter().map(|path| {
            let mut table = SymbolTable::with_source_manager(sm.clone());
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            match ext.to_lowercase().as_str() {
                "h" | "hpp" => { let _ = table.load_header(&path); }
                "txt" => { let _ = table.load_list_file(&path); }
                "py" => { let _ = table.load_python_enum(&path); }
                "json" => { let _ = table.load_text_bank_json(&path); }
                _ => {}
            }
            table
        }).collect();

        for table in results {
            self.extend(table);
        }

        Ok(count)
    }

    fn collect_header_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if !dir.is_dir() { return Ok(()); }
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) != Some(".git") {
                    self.collect_header_files(&path, files)?;
                }
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    pub fn load_python_enum(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.load_python_enum_str(&content)
    }

    pub fn load_python_enum_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            if let Some(caps) = RE_PYTHON_ENUM.captures(line.trim()) {
                let name = caps[1].to_string();
                let expr = caps[2].trim().to_string();
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    &expr, 
                    &self.pending_defines, 
                    &self.defines,
                    &self.cache
                ) {
                    self.defines.insert(name.clone(), val);
                    self.defines_by_value.entry(val).or_default().push(name.clone());
                    self.pending_defines.insert(name, val.to_string());
                }
            }
        }
        Ok(())
    }

    pub fn load_from_url(&mut self, url: &str) -> std::io::Result<()> {
        let output = std::process::Command::new("curl").arg("-L").arg("-s").arg(url).output()?;
        if !output.status.success() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to fetch URL: {}", url)));
        }
        let content = String::from_utf8_lossy(&output.stdout);
        if url.ends_with(".txt") {
            self.load_list_file_str(&content)
        } else if url.ends_with(".py") {
            self.load_python_enum_str(&content)
        } else {
            self.load_header_str(&content)
        }
    }

    pub fn insert_define(&mut self, name: String, value: i64) {
        self.defines.insert(name.clone(), value);
        self.defines_by_value.entry(value).or_default().push(name);
    }

    pub fn insert_enum(&mut self, name: String, variants: Vec<(String, Option<i64>)>) {
        for (v_name, v_val) in &variants {
            if let Some(val) = v_val {
                self.variant_to_value.insert(v_name.clone(), *val);
                self.enum_by_value.entry(*val).or_default().push(v_name.clone());
            }
        }
        self.enums.insert(name, variants);
    }

    pub fn get_all_defines(&self) -> HashMap<String, i64> {
        let mut res = HashMap::with_capacity(self.defines.len() + self.cache.len());
        
        let mut collect_from = |table: &SymbolTable| {
            for (k, v) in &table.defines {
                res.insert(k.clone(), *v);
            }
            for (k, v) in &table.variant_to_value {
                res.insert(k.clone(), *v);
            }
        };

        collect_from(self);
        let mut current = self.parent.as_ref();
        while let Some(p) = current {
            collect_from(p);
            current = p.parent.as_ref();
        }

        for entry in self.cache.iter() {
            res.insert(entry.key().clone(), *entry.value());
        }
        res
    }

    pub fn get_enums_std(&self) -> HashMap<String, Vec<(String, Option<i64>)>> {
        let mut res = HashMap::with_capacity(self.enums.len());
        
        let mut collect_from = |table: &SymbolTable| {
            for (k, v) in &table.enums {
                res.insert(k.clone(), v.clone());
            }
        };

        collect_from(self);
        let mut current = self.parent.as_ref();
        while let Some(p) = current {
            collect_from(p);
            current = p.parent.as_ref();
        }

        res
    }

    pub fn extend(&mut self, other: SymbolTable) {
        self.defines.extend(other.defines);
        self.pending_defines.extend(other.pending_defines);
        self.enums.extend(other.enums);
        self.variant_to_value.extend(other.variant_to_value);
        self.defines_by_value.extend(other.defines_by_value);
        self.enum_by_value.extend(other.enum_by_value);
        self.loaded_includes.extend(other.loaded_includes);
        for entry in other.cache.iter() {
            self.cache.insert(entry.key().clone(), *entry.value());
        }
        for entry in other.shortest_name_cache.iter() {
            self.shortest_name_cache.insert(*entry.key(), entry.value().clone());
        }
    }
}
