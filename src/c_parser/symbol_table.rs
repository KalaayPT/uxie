use crate::c_parser::constant_cache::SymbolSnapshot;
use crate::game::{GameFamily, GameLanguage};
use crate::trainer_data::TrainerData;
use bitcode::{Decode, Encode};
use dashmap::DashMap;
use deunicode::deunicode;
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Callback used by recursive include loading when a non-system `#include "..."`
/// cannot be resolved as a normal on-disk file.
///
/// The callback receives:
/// - the symbol table being populated
/// - the directory of the current file/source being processed
/// - the configured include directories
/// - the unresolved include path exactly as written
///
/// Return `Ok(true)` if the include was handled by some caller-specific fallback,
/// `Ok(false)` if it was not handled, or an error if handling failed.
type UnresolvedIncludeHandler<'a> =
    dyn FnMut(&mut SymbolTable, &Path, &[PathBuf], &str) -> std::io::Result<bool> + 'a;

pub use crate::c_parser::defines::parse_defines;
pub use crate::c_parser::defines::parse_value;
pub use crate::c_parser::enums::{parse_enum, parse_enums};
pub use crate::c_parser::source_manager::SourceManager;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum SymbolTag {
    Global,
    Map(u16),
    CommonScript(u16),
    TextBank(u16),
}

/// Semantic constant families tracked by the symbol table.
///
/// This metadata is currently inferred from symbol prefixes at insert time
/// (for example `ITEM_`, `MOVE_`, `SPECIES_`, `SEQ_`). Loader paths do not yet
/// attach families explicitly, so prefix inference remains the source of truth
/// until loader-level classification is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum ConstantFamily {
    Item,
    Species,
    Move,
    Location,
    Trainer,
    TrainerClass,
    Sound,
}

impl ConstantFamily {
    pub fn from_symbol_name(name: &str) -> Option<Self> {
        if name.starts_with("TRAINER_CLASS_") {
            Some(Self::TrainerClass)
        } else if name.starts_with("TRAINER_") {
            Some(Self::Trainer)
        } else if name.starts_with("SPECIES_") {
            Some(Self::Species)
        } else if name.starts_with("ITEM_") {
            Some(Self::Item)
        } else if name.starts_with("MOVE_") {
            Some(Self::Move)
        } else if name.starts_with("LOCATION_") || name.starts_with("MAPSEC_") {
            Some(Self::Location)
        } else if name.starts_with("SEQ_") {
            Some(Self::Sound)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub(crate) parent: Option<Arc<SymbolTable>>,
    pub(crate) symbols: FxHashMap<String, i64>,
    pub(crate) pending: FxHashMap<String, String>,
    pub(crate) function_macros: FxHashMap<String, crate::c_parser::defines::CFunctionMacro>,
    pub(crate) value_to_names: FxHashMap<i64, Vec<String>>,
    pub(crate) symbol_to_file: FxHashMap<String, PathBuf>,
    pub(crate) symbol_to_tags: FxHashMap<String, FxHashSet<SymbolTag>>,
    pub(crate) symbol_to_family: FxHashMap<String, ConstantFamily>,
    pub(crate) loaded_files: FxHashSet<PathBuf>,
    pub(crate) eval_cache: Arc<DashMap<String, i64>>,
    pub(crate) shortest_name_cache: Arc<DashMap<i64, String>>,
    pub(crate) source_manager: Option<SourceManager>,
}

static RE_PYTHON_ENUM: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+)$").unwrap()
});

static RE_METANG_MASK_TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"'(?P<name>[A-Za-z0-9_]+)'\s*:\s*\{\s*'type'\s*:\s*'(?P<kind>enum|mask)'").unwrap()
});

/// Canonicalize a display name into the constant spelling used by script banks.
///
/// This is intentionally insert-side only: callers use it when populating a
/// symbol table from human-readable text archives, not when resolving lookup
/// names later.
pub fn canonicalize_constant_name(name: &str) -> String {
    let normalized = name
        .replace(['\'', '’', '‘'], "")
        .replace('♀', "_F")
        .replace('♂', "_M")
        .replace('&', " AND ");
    let ascii = deunicode(&normalized);

    let mut canonical = String::with_capacity(ascii.len());
    let mut pending_separator = false;

    for ch in ascii.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !canonical.is_empty() {
                canonical.push('_');
            }
            canonical.push(ch.to_ascii_uppercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }

    canonical
}

fn split_compound_words(name: &str) -> String {
    let mut split = String::with_capacity(name.len() + 4);
    let mut prev: Option<char> = None;

    for ch in name.chars() {
        if prev.is_some_and(|prev| prev.is_ascii_lowercase() && ch.is_ascii_uppercase()) {
            split.push(' ');
        }
        split.push(ch);
        prev = Some(ch);
    }

    split
}

fn canonicalize_text_bank_constant(
    display_name: &str,
    prefix: &str,
    index: usize,
) -> Option<String> {
    if prefix == "ITEM_" && display_name == "???" {
        return Some(format!("ITEM_UNUSED_{index}"));
    }

    let normalized = match prefix {
        "ITEM_" => display_name.replace('-', ""),
        "MOVE_" => split_compound_words(display_name),
        _ => display_name.to_string(),
    };
    let canonical = canonicalize_constant_name(&normalized);
    if canonical.is_empty() {
        return None;
    }

    let canonical = match (prefix, canonical.as_str()) {
        ("ITEM_", "X_DEFEND") => "X_DEFENSE".to_string(),
        _ => canonical,
    };

    Some(format!("{prefix}{canonical}"))
}

/// offsets for sound names in sounds text archive,
/// needs to be hard coded for us to be able to parse them directly.
fn dspre_sound_constant_value_and_prefix(
    index: usize,
    row_count: usize,
) -> Option<(i64, &'static str)> {
    match row_count {
        1013 => {
            if index <= 2 {
                Some((index as i64 + 1, "SEQ_"))
            } else if index <= 229 {
                Some((index as i64 + 997, "SEQ_"))
            } else {
                Some((index as i64 + 1120, "SEQ_SE_"))
            }
        }
        1372 => {
            if index <= 2 {
                Some((index as i64 + 1, "SEQ_"))
            } else if index <= 364 {
                Some((index as i64 + 997, "SEQ_"))
            } else {
                Some((index as i64 + 1007, "SEQ_SE_"))
            }
        }
        _ => None,
    }
}

fn dspre_sound_constant_symbol(
    index: usize,
    row_count: usize,
    display_name: &str,
) -> Option<String> {
    let (_, prefix) = dspre_sound_constant_value_and_prefix(index, row_count)?;
    let canonical = canonicalize_constant_name(display_name);
    if canonical.is_empty() {
        return None;
    }

    Some(match (row_count, index) {
        (1013, 264) | (1372, 401) => "SEQ_DUMMY01".to_string(),
        (1013, 287) => "SEQ_DUMMY02".to_string(),
        _ => format!("{prefix}{canonical}"),
    })
}

fn dspre_sound_constant_aliases(index: usize, row_count: usize) -> &'static [&'static str] {
    match (row_count, index) {
        (1013, 380) | (1372, 493) => &["SEQ_SE_CONFIRM"],
        _ => &[],
    }
}

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

    fn record_loaded_file(&mut self, path: &Path) {
        let tracked = self.source_manager.as_ref().map_or_else(
            || path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            |sm| sm.canonicalize(path),
        );
        self.loaded_files.insert(tracked);
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
        let canonical = if let Some(sm) = &self.source_manager {
            sm.canonicalize_strict(path)?
        } else {
            path.canonicalize()?
        };
        if self.loaded_files.contains(&canonical) {
            return Ok(());
        }

        if let Some(sm) = &self.source_manager {
            let entry = sm.get_or_parse(path)?;
            self.load_file_entry(&entry, &canonical, tag);
        } else {
            let bytes = std::fs::read(path).map_err(|err| {
                std::io::Error::new(
                    err.kind(),
                    format!("Failed to read header {}: {err}", path.display()),
                )
            })?;
            let content = String::from_utf8_lossy(&bytes);
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
        self.load_recursive_with_handler(path, include_dirs, None)
    }

    /// Recursively load a file and its local `#include` dependencies.
    /// Returns `NotFound` if a non-system include cannot be resolved from the
    /// parent directory or `include_dirs`.
    pub fn load_recursive_strict(
        &mut self,
        path: impl AsRef<Path>,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        let mut unresolved_include_handler = |_table: &mut SymbolTable,
                                              parent_dir: &Path,
                                              _include_dirs: &[PathBuf],
                                              include_path: &str|
         -> std::io::Result<bool> {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Unresolved include '{}' (searched from {})",
                    include_path,
                    parent_dir.display()
                ),
            ))
        };

        self.load_recursive_with_handler(path, include_dirs, Some(&mut unresolved_include_handler))
    }

    /// Recursively load a file and its local `#include` dependencies, with an
    /// optional callback for unresolved non-system includes.
    ///
    /// The handler is only called after normal include-path resolution fails.
    /// This keeps core recursive loading generic while allowing caller-specific
    /// fallback behavior when needed.
    pub fn load_recursive_with_handler(
        &mut self,
        path: impl AsRef<Path>,
        include_dirs: &[PathBuf],
        mut unresolved_include_handler: Option<&mut UnresolvedIncludeHandler<'_>>,
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        let sm = self
            .source_manager
            .get_or_insert_with(SourceManager::new)
            .clone();
        let mut visited = FxHashSet::default();
        self.load_recursive_internal(
            path,
            include_dirs,
            &sm,
            &mut visited,
            SymbolTag::Global,
            &mut unresolved_include_handler,
        )
    }

    pub fn load_recursive_str(
        &mut self,
        content: &str,
        root_dir: impl AsRef<Path>,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        self.load_recursive_str_with_handler(content, root_dir, include_dirs, None)
    }

    /// Recursively load includes referenced from inline source content, with an
    /// optional callback for unresolved non-system includes.
    ///
    /// The handler has the same semantics as `load_recursive_with_handler` and
    /// is only invoked when ordinary include resolution fails.
    pub fn load_recursive_str_with_handler(
        &mut self,
        content: &str,
        root_dir: impl AsRef<Path>,
        include_dirs: &[PathBuf],
        mut unresolved_include_handler: Option<&mut UnresolvedIncludeHandler<'_>>,
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
        for function_macro in crate::c_parser::defines::parse_function_macros(content) {
            self.process_function_macro(function_macro, &dummy_path, SymbolTag::Global);
        }
        for e in parse_enums(content) {
            self.process_enum(e, &dummy_path, SymbolTag::Global);
        }

        let includes = crate::c_parser::includes::parse_includes(content);
        for inc in includes {
            if inc.is_system {
                continue;
            }

            if let Some(p) = Self::resolve_include_path(root_dir, include_dirs, &inc.path) {
                let mut visited = FxHashSet::default();
                self.load_recursive_internal(
                    &p,
                    include_dirs,
                    &sm,
                    &mut visited,
                    SymbolTag::Global,
                    &mut unresolved_include_handler,
                )?;
            } else if let Some(handler) = unresolved_include_handler.as_mut() {
                handler(self, root_dir, include_dirs, &inc.path)?;
            }
        }
        Ok(())
    }

    fn resolve_include_path(
        parent_dir: &Path,
        include_dirs: &[PathBuf],
        include_path: &str,
    ) -> Option<PathBuf> {
        let rel = parent_dir.join(include_path);
        if rel.exists() {
            return Some(rel);
        }

        for dir in include_dirs {
            let p = dir.join(include_path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    fn load_recursive_internal(
        &mut self,
        path: &Path,
        include_dirs: &[PathBuf],
        sm: &SourceManager,
        visited: &mut FxHashSet<PathBuf>,
        tag: SymbolTag,
        unresolved_include_handler: &mut Option<&mut UnresolvedIncludeHandler<'_>>,
    ) -> std::io::Result<()> {
        let canonical = sm.canonicalize_strict(path)?;
        if !visited.insert(canonical.clone()) {
            return Ok(());
        }

        // If the parent symbol table already covers this file (and its entire
        // transitive include tree), skip re-processing its defines into this
        // child.  All symbols remain reachable via the parent chain at lookup
        // time, and the shared eval_cache makes repeat lookups cheap.
        if let Some(parent) = &self.parent {
            if parent.loaded_files.contains(&canonical) {
                return Ok(());
            }
        }

        let entry = sm.get_or_parse(path)?;
        self.load_file_entry(&entry, &canonical, tag.clone());
        self.loaded_files.insert(canonical.clone());

        let parent_dir = path.parent().unwrap_or(Path::new("."));

        for inc in &entry.includes {
            if inc.is_system {
                continue;
            }

            if let Some(p) = Self::resolve_include_path(parent_dir, include_dirs, &inc.path) {
                self.load_recursive_internal(
                    &p,
                    include_dirs,
                    sm,
                    visited,
                    tag.clone(),
                    unresolved_include_handler,
                )?;
            } else if let Some(handler) = unresolved_include_handler.as_mut() {
                handler(self, parent_dir, include_dirs, &inc.path)?;
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
        for function_macro in &entry.function_macros {
            self.process_function_macro(function_macro.clone(), path, tag.clone());
        }
        for e in &entry.enums {
            self.process_enum(e.clone(), path, tag.clone());
        }
    }

    fn process_define(&mut self, name: String, value: String, path: &Path, tag: SymbolTag) {
        self.record_symbol_origin(&name, path, Some(&tag));
        self.assign_constant_family(&name);

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

    fn process_function_macro(
        &mut self,
        function_macro: crate::c_parser::defines::CFunctionMacro,
        path: &Path,
        tag: SymbolTag,
    ) {
        self.record_symbol_origin(&function_macro.name, path, Some(&tag));
        self.function_macros
            .insert(function_macro.name.clone(), function_macro);
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
            self.record_symbol_origin(&v.name, path, Some(&tag));
            self.assign_constant_family(&v.name);
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
        for function_macro in crate::c_parser::defines::parse_function_macros(content) {
            self.process_function_macro(function_macro, path, tag.clone());
        }
        Ok(())
    }

    fn get_function_macro(&self, name: &str) -> Option<&crate::c_parser::defines::CFunctionMacro> {
        self.function_macros.get(name).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_function_macro(name))
        })
    }

    #[allow(clippy::option_option)]
    fn expand_function_macros(&self, expr: &str, depth: usize) -> Option<Option<String>> {
        const MAX_DEPTH: usize = 32;
        if depth > MAX_DEPTH {
            return None;
        }

        let mut out = String::new();
        let mut chars = expr.char_indices().peekable();
        let mut changed = false;

        while let Some((start, ch)) = chars.next() {
            if ch.is_ascii_alphabetic() || ch == '_' {
                let mut end = start + ch.len_utf8();
                while let Some(&(idx, next_ch)) = chars.peek() {
                    if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                        end = idx + next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                let ident = &expr[start..end];
                let mut lookahead = chars.clone();
                while let Some(&(_, next_ch)) = lookahead.peek() {
                    if next_ch.is_whitespace() {
                        lookahead.next();
                    } else {
                        break;
                    }
                }

                if let Some(&(paren_idx, '(')) = lookahead.peek() {
                    if let Some(function_macro) = self.get_function_macro(ident) {
                        let close_idx = Self::find_matching_paren(expr, paren_idx)?;
                        let args_str = &expr[paren_idx + 1..close_idx];
                        let raw_args = Self::split_call_args(args_str);
                        if raw_args.len() != function_macro.params.len() {
                            return None;
                        }

                        let mut expanded_args = Vec::with_capacity(raw_args.len());
                        for arg in raw_args {
                            let expanded = self.expand_function_macros(arg, depth + 1)?;
                            expanded_args.push(expanded.unwrap_or_else(|| arg.trim().to_string()));
                        }

                        let expanded_body = self.expand_function_macro_body(
                            function_macro,
                            &expanded_args,
                            depth + 1,
                        )?;
                        out.push_str(&expanded_body);
                        changed = true;

                        while let Some(&(idx, _)) = chars.peek() {
                            if idx <= close_idx {
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                }

                out.push_str(ident);
                continue;
            }

            out.push(ch);
        }
        Some(changed.then_some(out))
    }

    fn expand_function_macro_body(
        &self,
        function_macro: &crate::c_parser::defines::CFunctionMacro,
        args: &[String],
        depth: usize,
    ) -> Option<String> {
        let mut body = function_macro.value.clone();
        for (param, arg) in function_macro.params.iter().zip(args.iter()) {
            body = Self::replace_identifier_tokens(&body, param, arg);
        }

        let expanded = self.expand_function_macros(&body, depth)?;
        Some(expanded.unwrap_or(body))
    }

    fn replace_identifier_tokens(input: &str, name: &str, replacement: &str) -> String {
        let mut out = String::new();
        let mut chars = input.char_indices().peekable();

        while let Some((start, ch)) = chars.next() {
            if ch.is_ascii_alphabetic() || ch == '_' {
                let mut end = start + ch.len_utf8();
                while let Some(&(idx, next_ch)) = chars.peek() {
                    if next_ch.is_ascii_alphanumeric() || next_ch == '_' {
                        end = idx + next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                let ident = &input[start..end];
                if ident == name {
                    out.push('(');
                    out.push_str(replacement);
                    out.push(')');
                } else {
                    out.push_str(ident);
                }
                continue;
            }

            out.push(ch);
        }

        out
    }

    fn split_call_args(args: &str) -> Vec<&str> {
        let mut result = Vec::new();
        let mut depth = 0usize;
        let mut start = 0usize;

        for (idx, ch) in args.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    result.push(args[start..idx].trim());
                    start = idx + 1;
                }
                _ => {}
            }
        }

        let tail = args[start..].trim();
        if !tail.is_empty() {
            result.push(tail);
        } else if !args.trim().is_empty() {
            result.push("");
        }

        result
    }

    fn find_matching_paren(input: &str, open_idx: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (idx, ch) in input[open_idx..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(open_idx + idx);
                    }
                }
                _ => {}
            }
        }
        None
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

        // DSPRE text-bank constants may contain non-ASCII characters (e.g. German
        // umlauts) that are de-unicoded on the insert side. Try a de-unicoded
        // fallback so lookups like ITEM_STERNENSTÜCK resolve to ITEM_STERNENSTUCK.
        if !name.is_ascii() {
            let ascii = deunicode::deunicode(name);
            if ascii != name {
                if let Some(val) = self.symbols.get(&ascii) {
                    return Some(*val);
                }
                if let Some(val) = self.eval_cache.get(&ascii) {
                    return Some(*val);
                }
                if let Some(parent) = &self.parent {
                    if let Some(val) = parent.resolve_constant(&ascii) {
                        return Some(val);
                    }
                }
            }
        }

        let expr = self.pending.get(name)?;
        let expanded = self.expand_function_macros(expr, 0)?;
        let eval_target = expanded.as_deref().unwrap_or(expr);
        let val = if let Some(parent) = &self.parent {
            crate::c_parser::defines::eval_expr_with_parent(
                eval_target,
                &self.pending,
                &self.symbols,
                &self.eval_cache,
                &|n| parent.resolve_constant(n),
            )?
        } else {
            crate::c_parser::defines::eval_expr_with_context(
                eval_target,
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

        let expanded = self.expand_function_macros(expr, 0)?;
        let expr = expanded.as_deref().unwrap_or(expr);

        self.parent.as_ref().map_or_else(
            || {
                crate::c_parser::defines::eval_expr_with_context(
                    expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                )
            },
            |parent| {
                crate::c_parser::defines::eval_expr_with_parent(
                    expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                    &|n| parent.resolve_constant(n),
                )
            },
        )
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

    pub fn constant_family(&self, name: &str) -> Option<ConstantFamily> {
        self.symbol_to_family.get(name).copied().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.constant_family(name))
        })
    }

    pub fn resolve_name_in_family(&self, value: i64, family: ConstantFamily) -> Option<String> {
        if let Some(names) = self.value_to_names.get(&value) {
            if let Some(name) = names
                .iter()
                .filter(|name| self.constant_family(name.as_str()) == Some(family))
                .min_by_key(|name| name.len())
            {
                return Some(name.clone());
            }
        }

        self.parent
            .as_ref()
            .and_then(|parent| parent.resolve_name_in_family(value, family))
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
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read list file {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        let is_mask = Self::list_file_is_metang_mask(path)?;
        self.record_loaded_file(path);
        self.load_list_file_str_with_tag(&content, path, tag, is_mask)
    }

    pub fn load_list_file_str(&mut self, content: &str) -> std::io::Result<()> {
        self.load_list_file_str_with_tag(content, Path::new("inline.txt"), SymbolTag::Global, false)
    }

    pub fn load_list_file_str_with_tag(
        &mut self,
        content: &str,
        path: &Path,
        tag: SymbolTag,
        is_mask: bool,
    ) -> std::io::Result<()> {
        let mut current_index = 0i64;
        for (line_idx, line) in content.lines().enumerate() {
            let line_number = line_idx + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let name = line[..pos].trim().to_string();
                let expr_raw = line[pos + 1..].trim();
                let expr = expr_raw
                    .split('#')
                    .next()
                    .map(str::trim)
                    .unwrap_or(expr_raw);
                let val = crate::c_parser::defines::eval_expr_with_context(
                    expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                )
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Failed to evaluate assignment expression '{}' for '{}' at {}:{}",
                            expr,
                            name,
                            path.display(),
                            line_number
                        ),
                    )
                })?;
                current_index = val;
                self.symbols.insert(name.clone(), current_index);
                self.value_to_names
                    .entry(current_index)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name.clone(), current_index.to_string());
                self.record_symbol_origin(&name, path, Some(&tag));
                self.assign_constant_family(&name);
            } else {
                // Strip inline comments (e.g., "CONSTANT  # comment")
                let name = line
                    .find('#')
                    .map_or(line, |comment_pos| line[..comment_pos].trim())
                    .to_string();
                if name.is_empty() {
                    continue;
                }

                let value = if is_mask {
                    1_i64.checked_shl(current_index as u32).ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Mask constant '{}' at {}:{} exceeds i64 bit width",
                                name,
                                path.display(),
                                line_number
                            ),
                        )
                    })?
                } else {
                    current_index
                };

                self.symbols.insert(name.clone(), value);
                self.value_to_names
                    .entry(value)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name.clone(), value.to_string());
                self.record_symbol_origin(&name, path, Some(&tag));
                self.assign_constant_family(&name);
            }
            current_index += 1;
        }
        Ok(())
    }

    fn list_file_is_metang_mask(path: &Path) -> std::io::Result<bool> {
        let file_stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("List file has no valid UTF-8 stem: {}", path.display()),
                )
            })?;

        let generated_dir = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("List file has no parent directory: {}", path.display()),
            )
        })?;

        let meson_path = generated_dir.join("meson.build");
        if !meson_path.is_file() {
            return Ok(false);
        }

        let meson = std::fs::read_to_string(&meson_path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read metang metadata {} as UTF-8: {err}",
                    meson_path.display()
                ),
            )
        })?;
        for caps in RE_METANG_MASK_TYPE.captures_iter(&meson) {
            if caps.name("name").map(|m| m.as_str()) == Some(file_stem) {
                return Ok(caps.name("kind").map(|m| m.as_str()) == Some("mask"));
            }
        }

        Ok(false)
    }

    fn insert_symbol_at_value(&mut self, id: &str, value: i64, path: &Path) {
        self.symbols.insert(id.to_string(), value);
        self.value_to_names
            .entry(value)
            .or_default()
            .push(id.to_string());
        self.record_symbol_origin(id, path, None);
        self.assign_constant_family(id);
    }

    fn insert_indexed_symbol(&mut self, id: &str, index: usize, path: &Path) {
        let val = index as i64;
        self.insert_symbol_at_value(id, val, path);
    }

    fn text_bank_display_name(
        path: &Path,
        index: usize,
        message: &serde_json::Map<String, serde_json::Value>,
        language: GameLanguage,
    ) -> std::io::Result<Option<String>> {
        let preferred = language.locale_key();
        let display_name = if let Some(value) = message.get(preferred) {
            Some(value)
        } else if let Some(value) = message.get("en_US") {
            Some(value)
        } else if let Some(value) = message.get("ja_JP") {
            Some(value)
        } else {
            message.iter().find_map(|(key, value)| {
                if key == "id" {
                    None
                } else if value.is_string() || value.is_array() {
                    Some(value)
                } else {
                    None
                }
            })
        };

        let Some(display_name) = display_name else {
            return Ok(None);
        };

        let display_name = match display_name {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Array(parts) => {
                let mut text = String::new();
                for (part_index, part) in parts.iter().enumerate() {
                    let segment = part.as_str().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Text bank JSON {} has non-string 'messages[{}]' content part {}",
                                path.display(),
                                index,
                                part_index
                            ),
                        )
                    })?;
                    text.push_str(segment);
                }
                text
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Text bank JSON {} has unsupported message text value at messages[{}]",
                        path.display(),
                        index
                    ),
                ));
            }
        };

        let display_name = display_name.trim();
        let display_name = if display_name.starts_with('{') && display_name.ends_with('}') {
            display_name[1..display_name.len() - 1]
                .split_once(':')
                .map_or(display_name, |(_, inner)| inner.trim())
        } else {
            display_name
        };

        Ok(Some(display_name.to_string()))
    }

    /// Parse a JSON text-archive file and return the display name for each row.
    ///
    /// This is the format used by pokeplatinum and exported by DSPRE — a top-level
    /// `messages` array where each entry has language keys such as `en_US`.
    fn parse_json_text_archive(
        path: &Path,
        language: GameLanguage,
    ) -> std::io::Result<Vec<Option<String>>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{}: {e}", path.display()),
            )
        })?;
        let messages = json
            .get("messages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{}: missing 'messages' array", path.display()),
                )
            })?;

        messages
            .iter()
            .enumerate()
            .map(|(i, msg)| {
                let obj = msg.as_object().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("{}: messages[{i}] is not an object", path.display()),
                    )
                })?;
                Self::text_bank_display_name(path, i, obj, language)
            })
            .collect()
    }

    pub fn load_dspre_sound_archive_constants(
        &mut self,
        path: impl AsRef<Path>,
        language: GameLanguage,
    ) -> std::io::Result<usize> {
        let path = path.as_ref();
        self.record_loaded_file(path);
        let names = Self::parse_json_text_archive(path, language)?;
        let row_count = names.len();

        if dspre_sound_constant_value_and_prefix(0, row_count).is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported sound archive with {row_count} rows"),
            ));
        }

        let mut count = 0;
        for (index, name) in names.iter().enumerate() {
            let Some(name) = name else { continue };
            let Some((value, _)) = dspre_sound_constant_value_and_prefix(index, row_count) else {
                continue;
            };
            let Some(symbol) = dspre_sound_constant_symbol(index, row_count, name) else {
                continue;
            };

            self.insert_symbol_at_value(&symbol, value, path);
            for alias in dspre_sound_constant_aliases(index, row_count) {
                self.insert_symbol_at_value(alias, value, path);
            }
            count += 1;
        }

        Ok(count)
    }

    /// Load best-effort DSPRE trainer constants from the trainer-name archive,
    /// trainer-class archive, and parsed trainer metadata.
    ///
    /// This is intentionally heuristic: DSPRE trainer metadata does not encode
    /// every identity distinction that decomp trainer constants carry, such as
    /// Platinum's location-based rival/grunt names or HGSS's specialized class
    /// stems like `BIRD_KEEPER_GS`. Use this for canonicalization testing and
    /// investigation, not as a claim of full decomp-equivalent identity.
    pub fn load_dspre_trainer_archive_constants(
        &mut self,
        trainer_names_path: impl AsRef<Path>,
        trainer_classes_path: impl AsRef<Path>,
        trainers: &[TrainerData],
        family: GameFamily,
        language: GameLanguage,
    ) -> std::io::Result<usize> {
        let trainer_names_path = trainer_names_path.as_ref();
        let trainer_classes_path = trainer_classes_path.as_ref();
        self.record_loaded_file(trainer_names_path);
        self.record_loaded_file(trainer_classes_path);

        let names = Self::parse_json_text_archive(trainer_names_path, language)?;
        let classes = Self::parse_json_text_archive(trainer_classes_path, language)?;

        if names.len() != trainers.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "trainer-name archive has {} rows but {} trainers were loaded",
                    names.len(),
                    trainers.len()
                ),
            ));
        }

        let trainer_class_names: Vec<String> =
            classes.into_iter().map(|n| n.unwrap_or_default()).collect();

        let mut hgss_occurrences = HashMap::new();
        for (index, (name, trainer)) in names.iter().zip(trainers).enumerate() {
            let Some(name_display) = name else { continue };
            let canonical_name = canonicalize_constant_name(name_display);

            let symbol = if index == 0 || canonical_name.is_empty() || name_display == "-" {
                "TRAINER_NONE".to_string()
            } else if family == GameFamily::Platinum
                && matches!(name_display.as_str(), "Mickey" | "Angelica" | "Tara & Tim")
                && !trainer.party.is_empty()
                && trainer
                    .party
                    .iter()
                    .all(|p| p.species_id() == 19 && p.level == 5)
            {
                format!("TRAINER_DUMMY_{index:03}")
            } else {
                let class_index = trainer.properties.trainer_class as usize;
                let class_display = trainer_class_names.get(class_index).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("trainer {index} references out-of-range class {class_index}"),
                    )
                })?;

                let class_display = class_display
                    .strip_prefix("[PK][MN] ")
                    .unwrap_or(class_display)
                    .trim();
                let class_stem = match family {
                    GameFamily::DP | GameFamily::Platinum => canonicalize_constant_name(
                        class_display.trim_end_matches(['♂', '♀']),
                    ),
                    GameFamily::HGSS if class_display == "Trainer" => "PKMN_TRAINER".to_string(),
                    GameFamily::HGSS => canonicalize_constant_name(class_display),
                };

                let mut symbol = format!("TRAINER_{class_stem}_{canonical_name}");
                if family == GameFamily::HGSS
                    && matches!(class_display, "Leader" | "Elite Four" | "Trainer")
                {
                    symbol.push('_');
                    symbol.push_str(&canonical_name);
                }
                if family == GameFamily::HGSS {
                    let occurrence = hgss_occurrences.entry(symbol.clone()).or_insert(0usize);
                    *occurrence += 1;
                    if *occurrence > 1 {
                        symbol.push('_');
                        symbol.push_str(&occurrence.to_string());
                    }
                }
                symbol
            };

            self.insert_indexed_symbol(&symbol, index, trainer_names_path);
        }

        Ok(names.len())
    }

    fn load_text_bank_messages(
        &mut self,
        path: &Path,
        messages_value: &serde_json::Value,
    ) -> std::io::Result<usize> {
        let messages = messages_value.as_array().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Text bank JSON {} has non-array 'messages' field",
                    path.display()
                ),
            )
        })?;
        let expects_message_ids = messages.iter().any(|msg| msg.get("id").is_some());
        let mut count = 0;

        for (index, msg) in messages.iter().enumerate() {
            let Some(id_value) = msg.get("id") else {
                if expects_message_ids {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Text bank JSON {} has non-string or missing 'messages[{}].id'",
                            path.display(),
                            index
                        ),
                    ));
                }
                continue;
            };

            let id = id_value.as_str().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Text bank JSON {} has non-string or missing 'messages[{}].id'",
                        path.display(),
                        index
                    ),
                )
            })?;
            self.insert_indexed_symbol(id, index, path);
            count += 1;
        }

        Ok(count)
    }

    fn load_text_bank_events(
        &mut self,
        path: &Path,
        events_value: &serde_json::Value,
    ) -> std::io::Result<usize> {
        let events = events_value.as_array().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Text bank JSON {} has non-array 'object_events' field",
                    path.display()
                ),
            )
        })?;
        let mut count = 0;

        for (index, event) in events.iter().enumerate() {
            let id = event.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Text bank JSON {} has non-string or missing 'object_events[{}].id'",
                        path.display(),
                        index
                    ),
                )
            })?;
            self.insert_indexed_symbol(id, index, path);
            count += 1;
        }

        Ok(count)
    }

    pub fn load_text_bank_json(&mut self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read text bank JSON {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        self.record_loaded_file(path);
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse text bank JSON {}: {e}", path.display()),
            )
        })?;

        let messages_field = json.get("messages");
        let events_field = json.get("object_events");
        if messages_field.is_none() && events_field.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Text bank JSON {} is missing both 'messages' and 'object_events' arrays",
                    path.display()
                ),
            ));
        }

        let mut count = 0;
        if let Some(messages_value) = messages_field {
            count += self.load_text_bank_messages(path, messages_value)?;
        }
        if let Some(events_value) = events_field {
            count += self.load_text_bank_events(path, events_value)?;
        }

        Ok(count)
    }

    /// Load a Chatot/DSPRE text bank JSON as prefixed script constants derived
    /// from the human-readable display names instead of the message `id` fields.
    ///
    /// `index_suffix_width` is used for archives whose constants carry their
    /// archive index in the symbol name, such as `TRAINER_SILVER_001`.
    pub fn load_text_bank_json_constants(
        &mut self,
        path: impl AsRef<Path>,
        prefix: &str,
        index_suffix_width: Option<usize>,
        language: GameLanguage,
    ) -> std::io::Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read text bank JSON {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        self.record_loaded_file(path);
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse text bank JSON {}: {e}", path.display()),
            )
        })?;

        let messages = json
            .get("messages")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Text bank JSON {} is missing a 'messages' array",
                        path.display()
                    ),
                )
            })?;

        let mut count = 0;
        for (index, msg) in messages.iter().enumerate() {
            let message = msg.as_object().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Text bank JSON {} has non-object 'messages[{}]' entry",
                        path.display(),
                        index
                    ),
                )
            })?;

            let Some(display_name) = Self::text_bank_display_name(path, index, message, language)? else {
                continue;
            };
            let Some(symbol) = canonicalize_text_bank_constant(&display_name, prefix, index) else {
                if index == 0 {
                    let symbol = format!("{prefix}NONE");
                    self.insert_indexed_symbol(&symbol, index, path);
                    count += 1;
                }
                continue;
            };

            let symbol = if let Some(width) = index_suffix_width {
                format!("{symbol}_{index:0width$}")
            } else {
                symbol
            };
            self.insert_indexed_symbol(&symbol, index, path);
            count += 1;
        }

        Ok(count)
    }

    pub fn load_events_json(&mut self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read events JSON {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        self.record_loaded_file(path);
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse events JSON {}: {e}", path.display()),
            )
        })?;
        let mut count = 0;

        let events = json
            .get("object_events")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Events JSON {} is missing an 'object_events' array",
                        path.display()
                    ),
                )
            })?;

        for (index, event) in events.iter().enumerate() {
            let id = event.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Events JSON {} has non-string or missing 'object_events[{}].id'",
                        path.display(),
                        index
                    ),
                )
            })?;
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

        Ok(count)
    }

    pub fn load_headers_from_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<usize> {
        let mut files = Vec::new();
        Self::collect_header_files(dir.as_ref(), &mut files)?;
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
                }
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

    /// Resolve an unresolved `#include "generated/foo.h"` by loading the
    /// sibling `.txt` list file if present.
    ///
    /// pokeplatinum keeps authoritative
    /// constant lists as `generated/<name>.txt` and only emits the matching
    /// `<name>.h` during build. Sources still `#include` the `.h`, so when
    /// the build hasn't run we can't resolve them. This fallback redirects
    /// any such include to its `.txt` equivalent, loading the same symbols
    /// the built header would have produced.
    ///
    /// Returns `Ok(true)` if a `.txt` equivalent was found and loaded.
    pub fn try_load_generated_header_fallback(
        &mut self,
        parent_dir: &Path,
        include_dirs: &[PathBuf],
        include_path: &str,
    ) -> std::io::Result<bool> {
        if !include_path.ends_with(".h") {
            return Ok(false);
        }

        let txt_path_str = format!("{}.txt", &include_path[..include_path.len() - 2]);
        let direct = parent_dir.join(&txt_path_str);
        if direct.is_file() {
            self.load_list_file(&direct)?;
            return Ok(true);
        }

        for dir in include_dirs {
            let candidate = dir.join(&txt_path_str);
            if candidate.is_file() {
                self.load_list_file(&candidate)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn try_load_text_bank_include_json(
        &mut self,
        parent_dir: &Path,
        include_dirs: &[PathBuf],
        include_path: &str,
    ) -> std::io::Result<bool> {
        if !include_path.ends_with(".h") || !include_path.contains("/bank/") {
            return Ok(false);
        }

        let without_ext = &include_path[..include_path.len() - 2];
        let json_path_str = without_ext.replacen("/bank/", "/", 1) + ".json";

        let direct = parent_dir.join(&json_path_str);
        if direct.is_file() {
            self.load_text_bank_json(&direct)?;
            return Ok(true);
        }

        for dir in include_dirs {
            let candidate = dir.join(&json_path_str);
            if candidate.is_file() {
                self.load_text_bank_json(&candidate)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Attempt to load constants from a `.gmm` (Game Message XML) file when the
    /// expected `.h` header is missing. Returns `true` if a `.gmm` was found and
    /// parsed, `false` otherwise.
    pub fn try_load_gmm_fallback(
        &mut self,
        parent_dir: &Path,
        include_dirs: &[PathBuf],
        include_path: &str,
    ) -> std::io::Result<bool> {
        if !include_path.ends_with(".h") {
            return Ok(false);
        }

        let gmm_path_str = format!("{}.gmm", &include_path[..include_path.len() - 2]);

        let direct = parent_dir.join(&gmm_path_str);
        if direct.is_file() {
            self.load_gmm_file(&direct)?;
            return Ok(true);
        }

        for dir in include_dirs {
            let candidate = dir.join(&gmm_path_str);
            if candidate.is_file() {
                self.load_gmm_file(&candidate)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Parse a `.gmm` XML file and insert row IDs as indexed constants.
    fn load_gmm_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        #[derive(serde::Deserialize)]
        struct GmmArchive {
            #[serde(rename = "row")]
            rows: Vec<GmmRow>,
        }

        #[derive(serde::Deserialize)]
        struct GmmRow {
            #[serde(rename = "@id")]
            id: String,
            #[serde(rename = "@index")]
            index: i64,
        }

        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!("Failed to read GMM file '{}': {err}", path.display()),
            )
        })?;
        self.record_loaded_file(path);

        let archive: GmmArchive = quick_xml::de::from_str(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse GMM '{}': {e}", path.display()),
            )
        })?;

        for row in archive.rows {
            self.symbols.insert(row.id.clone(), row.index);
            self.value_to_names
                .entry(row.index)
                .or_default()
                .push(row.id.clone());
            self.record_symbol_origin(&row.id, path, Some(&SymbolTag::Global));
            self.assign_constant_family(&row.id);
        }

        Ok(())
    }

    fn collect_header_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) != Some(".git") {
                    Self::collect_header_files(&path, files)?;
                }
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    pub fn load_python_enum(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read Python enum {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        self.record_loaded_file(path);
        self.load_python_enum_str_with_tag(&content, path, SymbolTag::Global)
    }

    pub fn load_python_enum_str_with_tag(
        &mut self,
        content: &str,
        path: &Path,
        tag: SymbolTag,
    ) -> std::io::Result<()> {
        for (line_idx, line) in content.lines().enumerate() {
            let line_number = line_idx + 1;
            if let Some(caps) = RE_PYTHON_ENUM.captures(line.trim()) {
                let name = caps[1].to_string();
                let expr_raw = caps[2].trim();
                let expr = expr_raw
                    .split('#')
                    .next()
                    .map(str::trim)
                    .unwrap_or(expr_raw);

                let is_constant_like = name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                if !is_constant_like {
                    continue;
                }

                let val = crate::c_parser::defines::eval_expr_with_context(
                    expr,
                    &self.pending,
                    &self.symbols,
                    &self.eval_cache,
                )
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Failed to evaluate python enum expression '{}' for '{}' at {}:{}",
                            expr,
                            name,
                            path.display(),
                            line_number
                        ),
                    )
                })?;

                self.symbols.insert(name.clone(), val);
                self.value_to_names
                    .entry(val)
                    .or_default()
                    .push(name.clone());
                self.pending.insert(name.clone(), val.to_string());
                self.record_symbol_origin(&name, path, Some(&tag));
                self.assign_constant_family(&name);
            }
        }
        Ok(())
    }

    pub fn load_from_url(&mut self, url: &str) -> std::io::Result<()> {
        let output = std::process::Command::new("curl")
            .arg("-f")
            .arg("-L")
            .arg("-sS")
            .arg(url)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(std::io::Error::other(format!(
                "Failed to fetch URL {} (status: {}): {}",
                url,
                output.status,
                stderr.trim()
            )));
        }
        let content = String::from_utf8_lossy(&output.stdout);
        let dummy_path_buf = PathBuf::from(
            url.rsplit('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .unwrap_or("url_source.h"),
        );
        let dummy_path = dummy_path_buf.as_path();
        if url.ends_with(".txt") {
            self.load_list_file_str_with_tag(&content, dummy_path, SymbolTag::Global, false)
        } else if url.ends_with(".py") {
            self.load_python_enum_str_with_tag(&content, dummy_path, SymbolTag::Global)
        } else {
            self.load_header_str_with_tag(&content, dummy_path, SymbolTag::Global)
        }
    }

    pub fn insert_define(&mut self, name: String, value: i64) {
        self.symbols.insert(name.clone(), value);
        self.value_to_names
            .entry(value)
            .or_default()
            .push(name.clone());
        self.assign_constant_family(&name);
    }

    /// Add DSPRE-specific aliases for canonicalization gaps.
    ///
    /// These bridge differences between DSPRE's raw text-bank output and the
    /// canonical constant names used by rotom/uxie (e.g. periods becoming
    /// separators, null-move conventions).
    pub fn add_dspre_aliases(&mut self) {
        for (alias, canonical) in [
            ("SPECIES_NIDORANF", "SPECIES_NIDORAN_F"),
            ("SPECIES_NIDORANM", "SPECIES_NIDORAN_M"),
            ("ITEM_SS_TICKET", "ITEM_S_S_TICKET"),
            ("MOVE__", "MOVE_NONE"),
            // German Platinum: DSPRE canonicalizes hyphens as underscores for
            // items, whereas rotom strips them. Also handles dot-to-underscore
            // differences (e.g. Mitgl.Karte).
            ("ITEM_TOP_TRANK", "ITEM_TOPTRANK"),
            ("ITEM_ADAMANT_ORB", "ITEM_ADAMANTORB"),
            ("ITEM_MITGLKARTE", "ITEM_MITGL_KARTE"),
            ("ITEM_L_SCHLUSSEL", "ITEM_LSCHLUSSEL"),
            ("ITEM_K_SCHLUSSEL", "ITEM_KSCHLUSSEL"),
            ("ITEM_G_SCHLUSSEL", "ITEM_GSCHLUSSEL"),
            ("ITEM_B_SCHLUSSEL", "ITEM_BSCHLUSSEL"),
            ("ITEM_WEISS_ORB", "ITEM_WEISSORB"),
            ("ITEM_WEIß_ORB", "ITEM_WEISSORB"),
            ("ITEM_X_ANGRIFF", "ITEM_XANGRIFF"),
            ("ITEM_?_OFFNER", "ITEM_OFFNER"),
        ] {
            if self.resolve_constant(alias).is_some() {
                continue;
            }
            if let Some(value) = self.resolve_constant(canonical) {
                self.insert_define(alias.to_string(), value);
            }
        }
    }

    pub fn insert_enum(&mut self, _name: String, variants: Vec<(String, Option<i64>)>) {
        for (v_name, v_val) in &variants {
            if let Some(val) = v_val {
                self.symbols.insert(v_name.clone(), *val);
                self.value_to_names
                    .entry(*val)
                    .or_default()
                    .push(v_name.clone());
                self.assign_constant_family(v_name);
            }
        }
    }

    pub fn get_all_defines(&self) -> HashMap<String, i64> {
        let mut res = self.parent.as_ref().map_or_else(
            || HashMap::with_capacity(self.symbols.len() + self.eval_cache.len()),
            |parent| parent.get_all_defines(),
        );

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
        self.symbol_to_family.extend(other.symbol_to_family);
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

    pub fn to_snapshot(&self) -> SymbolSnapshot {
        let symbols = self
            .symbols
            .iter()
            .map(|(name, value)| (name.clone(), *value))
            .collect();
        let value_to_names = self
            .value_to_names
            .iter()
            .map(|(value, names)| (*value, names.clone()))
            .collect();
        let pending = self
            .pending
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        let function_macros = self
            .function_macros
            .iter()
            .map(|(name, function_macro)| (name.clone(), function_macro.clone()))
            .collect();
        let symbol_to_tags = self
            .symbol_to_tags
            .iter()
            .map(|(name, tags)| {
                (
                    name.clone(),
                    tags.iter()
                        .cloned()
                        .collect::<std::collections::HashSet<_>>(),
                )
            })
            .collect();
        let symbol_to_family = self
            .symbol_to_family
            .iter()
            .map(|(name, family)| (name.clone(), *family))
            .collect();

        SymbolSnapshot {
            symbols,
            value_to_names,
            pending,
            function_macros,
            symbol_to_tags,
            symbol_to_family,
        }
    }

    pub fn from_snapshot(snapshot: &SymbolSnapshot) -> Self {
        Self {
            parent: None,
            symbols: snapshot
                .symbols
                .iter()
                .map(|(name, value)| (name.clone(), *value))
                .collect(),
            pending: snapshot
                .pending
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            function_macros: snapshot
                .function_macros
                .iter()
                .map(|(name, function_macro)| (name.clone(), function_macro.clone()))
                .collect(),
            value_to_names: snapshot
                .value_to_names
                .iter()
                .map(|(value, names)| (*value, names.clone()))
                .collect(),
            symbol_to_file: FxHashMap::default(),
            symbol_to_tags: snapshot
                .symbol_to_tags
                .iter()
                .map(|(name, tags)| (name.clone(), tags.iter().cloned().collect::<FxHashSet<_>>()))
                .collect(),
            symbol_to_family: snapshot
                .symbol_to_family
                .iter()
                .map(|(name, family)| (name.clone(), *family))
                .collect(),
            loaded_files: FxHashSet::default(),
            eval_cache: Arc::default(),
            shortest_name_cache: Arc::default(),
            source_manager: None,
        }
    }

    /// Returns the canonical on-disk files that contributed symbols to this table.
    /// Child tables only report files they loaded themselves, not parent snapshots.
    pub fn loaded_file_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.loaded_files.iter().cloned().collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn record_symbol_origin(&mut self, name: &str, path: &Path, tag: Option<&SymbolTag>) {
        self.symbol_to_file
            .insert(name.to_string(), path.to_path_buf());
        if let Some(tag) = tag {
            self.symbol_to_tags
                .entry(name.to_string())
                .or_default()
                .insert(tag.clone());
        }
    }

    fn assign_constant_family(&mut self, name: &str) {
        if let Some(family) = ConstantFamily::from_symbol_name(name) {
            self.symbol_to_family.insert(name.to_string(), family);
        }
    }
}
