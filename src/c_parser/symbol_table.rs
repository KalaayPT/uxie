use dashmap::DashMap;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use crate::c_parser::defines::parse_defines;
pub use crate::c_parser::defines::parse_value;
pub use crate::c_parser::enums::{parse_enum, parse_enums};
pub use crate::c_parser::source_manager::SourceManager;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolTag {
    Global,
    Map(u16),
    CommonScript(u16),
    TextBank(u16),
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub(crate) parent: Option<Arc<SymbolTable>>,
    pub(crate) symbols: FxHashMap<String, i64>,
    pub(crate) pending: FxHashMap<String, String>,
    pub(crate) value_to_names: FxHashMap<i64, Vec<String>>,
    pub(crate) symbol_to_file: FxHashMap<String, PathBuf>,
    pub(crate) symbol_to_tags: FxHashMap<String, FxHashSet<SymbolTag>>,
    pub(crate) loaded_files: FxHashSet<PathBuf>,
    pub(crate) eval_cache: Arc<DashMap<String, i64>>,
    pub(crate) shortest_name_cache: Arc<DashMap<i64, String>>,
    pub(crate) source_manager: Option<SourceManager>,
}

static RE_PYTHON_ENUM: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$").unwrap()
});

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self::default();
        table.symbols.insert("TRUE".to_string(), 1);
        table.symbols.insert("FALSE".to_string(), 0);
        table
            .value_to_names
            .entry(1)
            .or_default()
            .push("TRUE".to_string());
        table
            .value_to_names
            .entry(0)
            .or_default()
            .push("FALSE".to_string());
        table
    }

    pub fn with_source_manager(sm: SourceManager) -> Self {
        let mut table = Self::new();
        table.source_manager = Some(sm);
        table
    }

    pub fn with_parent(parent: Arc<SymbolTable>) -> Self {
        let sm = parent.source_manager.clone();
        let eval_cache = parent.eval_cache.clone();
        let shortest_name_cache = parent.shortest_name_cache.clone();
        Self {
            parent: Some(parent),
            source_manager: sm,
            eval_cache,
            shortest_name_cache,
            ..Default::default()
        }
    }

    pub(crate) fn with_source_manager_and_caches(
        sm: SourceManager,
        eval_cache: Arc<DashMap<String, i64>>,
        shortest_name_cache: Arc<DashMap<i64, String>>,
    ) -> Self {
        let mut table = Self::new();
        table.source_manager = Some(sm);
        table.eval_cache = eval_cache;
        table.shortest_name_cache = shortest_name_cache;
        table
    }

    pub fn load_header(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.load_file_with_tag(path, SymbolTag::Global)
    }

    pub fn load_file_with_tag(
        &mut self,
        path: impl AsRef<Path>,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self.loaded_files.contains(&canonical) {
            return Ok(());
        }

        if let Some(sm) = &self.source_manager {
            let entry = sm.get_or_parse(path)?;
            self.load_file_entry(&entry, &canonical, tag);
        } else {
            let content = std::fs::read_to_string(path)?;
            self.load_header_str_with_tag(&content, &canonical, tag)?;
        }

        self.loaded_files.insert(canonical);
        Ok(())
    }

    pub fn load_recursive(
        &mut self,
        path: impl AsRef<Path>,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        let sm = self
            .source_manager
            .get_or_insert_with(SourceManager::new)
            .clone();
        let mut visited = FxHashSet::default();
        self.load_recursive_internal(path, include_dirs, &sm, &mut visited, SymbolTag::Global)
    }

    pub fn load_recursive_str(
        &mut self,
        content: &str,
        root_dir: impl AsRef<Path>,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        let root_dir = root_dir.as_ref();
        let sm = self
            .source_manager
            .get_or_insert_with(SourceManager::new)
            .clone();

        let dummy_path = root_dir.join("inline_source.h");

        for def in parse_defines(content) {
            self.process_define(def.name, def.value, &dummy_path, SymbolTag::Global);
        }
        for e in parse_enums(content) {
            self.process_enum(e, &dummy_path, SymbolTag::Global);
        }

        let includes = crate::c_parser::includes::parse_includes(content);
        for inc in includes {
            if inc.is_system {
                continue;
            }
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
                self.load_recursive_internal(
                    &p,
                    include_dirs,
                    &sm,
                    &mut visited,
                    SymbolTag::Global,
                )?;
            }
        }
        Ok(())
    }

    fn load_recursive_internal(
        &mut self,
        path: &Path,
        include_dirs: &[PathBuf],
        sm: &SourceManager,
        visited: &mut FxHashSet<PathBuf>,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        let canonical = sm.canonicalize(path);
        if !visited.insert(canonical.clone()) {
            return Ok(());
        }

        let entry = sm.get_or_parse(path)?;
        self.load_file_entry(&entry, &canonical, tag.clone());
        self.loaded_files.insert(canonical.clone());

        let parent_dir = path.parent().unwrap_or(Path::new("."));

        for inc in &entry.includes {
            if inc.is_system {
                continue;
            }

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
                self.load_recursive_internal(&p, include_dirs, sm, visited, tag.clone())?;
            } else if inc.path.contains("res/field/events/") && inc.path.ends_with(".h") {
                let json_path_str = inc.path.replace(".h", ".json");
                let json_rel = parent_dir.join(&json_path_str);
                if json_rel.exists() {
                    let _ = self.load_events_json(&json_rel);
                } else {
                    for dir in include_dirs {
                        let json_p = dir.join(&json_path_str);
                        if json_p.exists() {
                            let _ = self.load_events_json(&json_p);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn load_file_entry(
        &mut self,
        entry: &crate::c_parser::source_manager::FileEntry,
        path: &Path,
        tag: SymbolTag,
    ) {
        for def in &entry.defines {
            self.process_define(def.name.clone(), def.value.clone(), path, tag.clone());
        }
        for e in &entry.enums {
            self.process_enum(e.clone(), path, tag.clone());
        }
    }

    fn process_define(&mut self, name: String, value: String, path: &Path, tag: SymbolTag) {
        self.symbol_to_file.insert(name.clone(), path.to_path_buf());
        self.symbol_to_tags
            .entry(name.clone())
            .or_default()
            .insert(tag);

        let val_trimmed = value.trim();
        if val_trimmed.starts_with("0x") || val_trimmed.starts_with("0X") {
            if let Ok(val) = i64::from_str_radix(&val_trimmed[2..], 16) {
                self.symbols.insert(name.clone(), val);
                self.value_to_names
                    .entry(val)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name, value);
                return;
            }
        }
        if let Ok(val) = val_trimmed.parse::<i64>() {
            self.symbols.insert(name.clone(), val);
            self.value_to_names
                .entry(val)
                .or_default()
                .push(name.clone());
            self.pending.insert(name, value);
            return;
        }
        if let Some(&val) = self.symbols.get(val_trimmed) {
            self.symbols.insert(name.clone(), val);
            self.value_to_names
                .entry(val)
                .or_default()
                .push(name.clone());
            self.pending.insert(name, value);
            return;
        }
        if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
            val_trimmed,
            &self.pending,
            &self.symbols,
            &self.eval_cache,
        ) {
            self.symbols.insert(name.clone(), val);
            self.value_to_names
                .entry(val)
                .or_default()
                .push(name.clone());
            self.pending.insert(name, value);
            return;
        }
        self.pending.insert(name, value);
    }

    fn process_enum(&mut self, e: crate::c_parser::enums::CEnum, path: &Path, tag: SymbolTag) {
        let mut current = 0i64;
        for v in &e.variants {
            if let Some(val) = v.value {
                current = val;
            } else if let Some(ref raw) = v.raw_value {
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    raw,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                ) {
                    current = val;
                }
            }
            self.symbols.insert(v.name.clone(), current);
            self.value_to_names
                .entry(current)
                .or_default()
                .push(v.name.clone());
            self.symbol_to_file
                .insert(v.name.clone(), path.to_path_buf());
            self.symbol_to_tags
                .entry(v.name.clone())
                .or_default()
                .insert(tag.clone());
            current += 1;
        }
    }

    pub fn load_header_str(&mut self, content: &str) -> std::io::Result<()> {
        self.load_header_str_with_tag(content, Path::new("inline.h"), SymbolTag::Global)
    }

    pub fn load_header_str_with_tag(
        &mut self,
        content: &str,
        path: &Path,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        for e in parse_enums(content) {
            self.process_enum(e, path, tag.clone());
        }
        for def in parse_defines(content) {
            self.process_define(def.name, def.value, path, tag.clone());
        }
        Ok(())
    }

    pub fn resolve_constant(&self, name: &str) -> Option<i64> {
        if let Some(val) = self.symbols.get(name) {
            return Some(*val);
        }
        if let Some(val) = self.eval_cache.get(name) {
            return Some(*val);
        }

        if let Some(parent) = &self.parent {
            if let Some(val) = parent.resolve_constant(name) {
                return Some(val);
            }
        }

        let expr = self.pending.get(name)?;
        let val = if let Some(parent) = &self.parent {
            crate::c_parser::defines::eval_expr_with_parent(
                expr,
                &self.pending,
                &self.symbols,
                &self.eval_cache,
                &|n| parent.resolve_constant(n),
            )?
        } else {
            crate::c_parser::defines::eval_expr_with_context(
                expr,
                &self.pending,
                &self.symbols,
                &self.eval_cache,
            )?
        };

        self.eval_cache.insert(name.to_string(), val);
        Some(val)
    }

    pub fn evaluate_expression(&self, expr: &str) -> Option<i64> {
        if let Some(val) = self.resolve_constant(expr) {
            return Some(val);
        }

        if let Some(parent) = &self.parent {
            crate::c_parser::defines::eval_expr_with_parent(
                expr,
                &self.pending,
                &self.symbols,
                &self.eval_cache,
                &|n| parent.resolve_constant(n),
            )
        } else {
            crate::c_parser::defines::eval_expr_with_context(
                expr,
                &self.pending,
                &self.symbols,
                &self.eval_cache,
            )
        }
    }

    pub fn resolve_all(&mut self) {
        let keys: Vec<String> = self.pending.keys().cloned().collect();
        for name in keys {
            if let Some(val) = self.resolve_constant(&name) {
                self.symbols.insert(name.clone(), val);
                self.value_to_names.entry(val).or_default().push(name);
            }
        }
        self.pending.clear();
    }

    pub fn collect_for_file(
        path: impl AsRef<Path>,
        include_dirs: &[PathBuf],
        sm: SourceManager,
    ) -> std::io::Result<Self> {
        let mut table = Self::with_source_manager(sm);
        table.load_recursive(path, include_dirs)?;
        Ok(table)
    }

    pub fn get_source_manager(&self) -> SourceManager {
        self.source_manager.clone().unwrap_or_default()
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        self.resolve_name_with_tag(value, prefix, &SymbolTag::Global)
    }

    pub fn resolve_name_with_tag(
        &self,
        value: i64,
        prefix: &str,
        tag: &SymbolTag,
    ) -> Option<String> {
        if prefix.is_empty() {
            if let Some(cached) = self.shortest_name_cache.get(&value) {
                return Some(cached.clone());
            }
        }

        if let Some(names) = self.value_to_names.get(&value) {
            // Priority 1: Matches tag and prefix
            if let Some(name) = names
                .iter()
                .filter(|n| n.starts_with(prefix))
                .filter(|n| {
                    self.symbol_to_tags
                        .get(*n)
                        .is_some_and(|tags| tags.contains(tag))
                })
                .min_by_key(|n| n.len())
            {
                if prefix.is_empty() {
                    self.shortest_name_cache.insert(value, name.clone());
                }
                return Some(name.clone());
            }

            // Priority 2: Matches prefix (Global/Fallback)
            if let Some(name) = names
                .iter()
                .filter(|n| n.starts_with(prefix))
                .min_by_key(|n| n.len())
            {
                if prefix.is_empty() {
                    self.shortest_name_cache.insert(value, name.clone());
                }
                return Some(name.clone());
            }
        }
        None
    }

    pub fn resolve_names(&self, value: i64, prefixes: &[&str]) -> Vec<String> {
        let mut matches = FxHashSet::default();

        if let Some(names) = self.value_to_names.get(&value) {
            for name in names {
                if prefixes.is_empty() || prefixes.iter().any(|p| name.starts_with(p)) {
                    matches.insert(name.clone());
                }
            }
        }

        let mut res: Vec<_> = matches.into_iter().collect();
        res.sort_by_key(|n| n.len());
        res
    }

    pub fn load_list_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        self.load_list_file_with_tag(path, SymbolTag::Global)
    }

    pub fn load_list_file_with_tag(
        &mut self,
        path: impl AsRef<Path>,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        self.load_list_file_str_with_tag(&content, path, tag)
    }

    pub fn load_list_file_str(&mut self, content: &str) -> std::io::Result<()> {
        self.load_list_file_str_with_tag(content, Path::new("inline.txt"), SymbolTag::Global)
    }

    pub fn load_list_file_str_with_tag(
        &mut self,
        content: &str,
        path: &Path,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        let mut current_index = 0i64;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let name = line[..pos].trim().to_string();
                let expr = line[pos + 1..].trim().to_string();
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    &expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                ) {
                    current_index = val;
                }
                self.symbols.insert(name.clone(), current_index);
                self.value_to_names
                    .entry(current_index)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name.clone(), current_index.to_string());
                self.symbol_to_file.insert(name.clone(), path.to_path_buf());
                self.symbol_to_tags
                    .entry(name)
                    .or_default()
                    .insert(tag.clone());
                current_index += 1;
            } else {
                // Strip inline comments (e.g., "CONSTANT  # comment")
                let name = if let Some(comment_pos) = line.find('#') {
                    line[..comment_pos].trim()
                } else {
                    line
                }
                .to_string();
                if name.is_empty() {
                    continue;
                }
                self.symbols.insert(name.clone(), current_index);
                self.value_to_names
                    .entry(current_index)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name.clone(), current_index.to_string());
                self.symbol_to_file.insert(name.clone(), path.to_path_buf());
                self.symbol_to_tags
                    .entry(name)
                    .or_default()
                    .insert(tag.clone());
                current_index += 1;
            }
        }
        Ok(())
    }

    pub fn load_text_bank_json(&mut self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut count = 0;

        if let Some(messages) = json.get("messages").and_then(|v| v.as_array()) {
            for (index, msg) in messages.iter().enumerate() {
                if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                    let val = index as i64;
                    self.symbols.insert(id.to_string(), val);
                    self.value_to_names
                        .entry(val)
                        .or_default()
                        .push(id.to_string());
                    self.symbol_to_file
                        .insert(id.to_string(), path.to_path_buf());
                    count += 1;
                }
            }
        }

        if let Some(events) = json.get("object_events").and_then(|v| v.as_array()) {
            for (index, event) in events.iter().enumerate() {
                if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                    let val = index as i64;
                    self.symbols.insert(id.to_string(), val);
                    self.value_to_names
                        .entry(val)
                        .or_default()
                        .push(id.to_string());
                    self.symbol_to_file
                        .insert(id.to_string(), path.to_path_buf());
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    pub fn load_events_json(&mut self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut count = 0;

        if let Some(events) = json.get("object_events").and_then(|v| v.as_array()) {
            for (index, event) in events.iter().enumerate() {
                if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                    let val = index as i64;
                    self.symbols.insert(id.to_string(), val);
                    self.value_to_names
                        .entry(val)
                        .or_default()
                        .push(id.to_string());
                    self.symbol_to_file
                        .insert(id.to_string(), path.to_path_buf());
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

        let sm = self
            .source_manager
            .get_or_insert_with(SourceManager::new)
            .clone();
        let eval_cache = self.eval_cache.clone();
        let shortest_cache = self.shortest_name_cache.clone();

        let results: Vec<SymbolTable> = files
            .into_par_iter()
            .map(|path| {
                let mut table = SymbolTable::with_source_manager_and_caches(
                    sm.clone(),
                    eval_cache.clone(),
                    shortest_cache.clone(),
                );
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                match ext.to_lowercase().as_str() {
                    "h" | "hpp" => table.load_header(&path)?,
                    "txt" => table.load_list_file(&path)?,
                    "py" => table.load_python_enum(&path)?,
                    "json" => {
                        table.load_text_bank_json(&path)?;
                    }
                    _ => {}
                };
                Ok(table)
            })
            .collect::<Vec<std::io::Result<SymbolTable>>>()
            .into_iter()
            .collect::<std::io::Result<Vec<_>>>()?;

        for table in results {
            self.extend(table);
        }

        Ok(count)
    }

    fn collect_header_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
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
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        self.load_python_enum_str_with_tag(&content, path, SymbolTag::Global)
    }

    pub fn load_python_enum_str_with_tag(
        &mut self,
        content: &str,
        path: &Path,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        for line in content.lines() {
            if let Some(caps) = RE_PYTHON_ENUM.captures(line.trim()) {
                let name = caps[1].to_string();
                let expr = caps[2].trim().to_string();
                if let Some(val) = crate::c_parser::defines::eval_expr_with_context(
                    &expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                ) {
                    self.symbols.insert(name.clone(), val);
                    self.value_to_names
                        .entry(val)
                        .or_default()
                        .push(name.clone());
                    self.pending.insert(name.clone(), val.to_string());
                    self.symbol_to_file.insert(name.clone(), path.to_path_buf());
                    self.symbol_to_tags
                        .entry(name)
                        .or_default()
                        .insert(tag.clone());
                }
            }
        }
        Ok(())
    }

    pub fn load_from_url(&mut self, url: &str) -> std::io::Result<()> {
        let output = std::process::Command::new("curl")
            .arg("-L")
            .arg("-s")
            .arg(url)
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "Failed to fetch URL: {}",
                url
            )));
        }
        let content = String::from_utf8_lossy(&output.stdout);
        let dummy_path = Path::new("url_source.h");
        if url.ends_with(".txt") {
            self.load_list_file_str_with_tag(&content, dummy_path, SymbolTag::Global)
        } else if url.ends_with(".py") {
            self.load_python_enum_str_with_tag(&content, dummy_path, SymbolTag::Global)
        } else {
            self.load_header_str_with_tag(&content, dummy_path, SymbolTag::Global)
        }
    }

    pub fn insert_define(&mut self, name: String, value: i64) {
        self.symbols.insert(name.clone(), value);
        self.value_to_names.entry(value).or_default().push(name);
    }

    pub fn insert_enum(&mut self, _name: String, variants: Vec<(String, Option<i64>)>) {
        for (v_name, v_val) in &variants {
            if let Some(val) = v_val {
                self.symbols.insert(v_name.clone(), *val);
                self.value_to_names
                    .entry(*val)
                    .or_default()
                    .push(v_name.clone());
            }
        }
    }

    pub fn get_all_defines(&self) -> HashMap<String, i64> {
        let mut res = if let Some(parent) = &self.parent {
            parent.get_all_defines()
        } else {
            HashMap::with_capacity(self.symbols.len() + self.eval_cache.len())
        };

        for (k, v) in &self.symbols {
            res.insert(k.clone(), *v);
        }

        for entry in self.eval_cache.iter() {
            res.insert(entry.key().clone(), *entry.value());
        }
        res
    }

    pub fn get_enums_std(&self) -> HashMap<String, Vec<(String, Option<i64>)>> {
        // Enums are now merged into symbols, but we can return an empty map for legacy support
        HashMap::new()
    }

    pub fn extend(&mut self, other: SymbolTable) {
        self.symbols.extend(other.symbols);
        self.pending.extend(other.pending);
        for (val, names) in other.value_to_names {
            self.value_to_names.entry(val).or_default().extend(names);
        }
        self.symbol_to_file.extend(other.symbol_to_file);
        self.symbol_to_tags.extend(other.symbol_to_tags);
        self.loaded_files.extend(other.loaded_files);
        if !Arc::ptr_eq(&self.eval_cache, &other.eval_cache) {
            for entry in other.eval_cache.iter() {
                self.eval_cache.insert(entry.key().clone(), *entry.value());
            }
        }
        if !Arc::ptr_eq(&self.shortest_name_cache, &other.shortest_name_cache) {
            for entry in other.shortest_name_cache.iter() {
                self.shortest_name_cache
                    .insert(*entry.key(), entry.value().clone());
            }
        }
    }

    pub fn get_symbols_by_tag(&self, tag: &SymbolTag) -> Vec<String> {
        self.symbol_to_tags
            .iter()
            .filter(|(_, tags)| tags.contains(tag))
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_symbols_by_file(&self, path: &Path) -> Vec<String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.symbol_to_file
            .iter()
            .filter(|(_, p)| **p == canonical)
            .map(|(name, _)| name.clone())
            .collect()
    }
}
