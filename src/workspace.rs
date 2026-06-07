//! High-level workspace API for managing ROM hacking projects
//!
//! The [`Workspace`] struct provides a unified interface for working with both
//! DSPRE projects and decompilation sources (pokeplatinum/pokeheartgold).

use crate::c_parser::ConstantCache;
use crate::c_parser::{SourceManager, SymbolTable};
use crate::game::{Game, GameFamily, GameLanguage};
use crate::provider::{Arm9Provider, DataProvider, DecompProvider};
use crate::rom_header::RomHeader;
use crate::script_file::{
    GlobalScriptTable, MapScriptInfo, ScriptResolution, ScriptTable, is_common_script_id,
};
use crate::text_bank::{GameStrings, TextBankTable};
use dashmap::DashMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    /// ds-rom project (config.yaml or header.bin, arm9.bin).
    Dspre,
    /// pokeplatinum / pokeheartgold decompilation project.
    Decomp,
    /// hg-engine code-injection framework project (armips/, narcs.mk, rom.nds).
    HgEngine,
}

pub struct Workspace {
    pub project_path: PathBuf,
    pub project_type: ProjectType,
    pub game: Game,
    pub family: GameFamily,
    pub language: GameLanguage,
    pub provider: Box<dyn DataProvider>,
    pub symbols: Arc<SymbolTable>,
    pub scripts: ScriptTable,
    pub text_banks: TextBankTable,
    pub game_strings: GameStrings,
    pub global_script_table: GlobalScriptTable,
    pub source_manager: SourceManager,
    location_names: Option<Vec<String>>,
    internal_names: Option<Vec<String>>,
    /// Lazily-populated cache mapping archive IDs to their on-disk paths.
    ///
    /// Populated on first access via [`Workspace::read_message`]; subsequent
    /// calls for the same ID are O(1).
    text_archive_paths: DashMap<u16, PathBuf>,
    /// Full message text for each loaded archive, keyed by archive ID.
    /// Lazily populated on first access. Replaces `pending_messages`
    /// as the general session cache; still used for compilation dedup
    /// and flush.
    message_cache: DashMap<u16, Vec<String>>,
    /// Map from message id string → (archive_id, index).
    /// Lazily populated by `load_archive_into_cache`.
    /// `Arc` so it can be shared with `ConstantDb` — inserts are visible
    /// across all clones of ConstantDb.
    pub(crate) message_ids: Arc<DashMap<String, (u16, u16)>>,
}

impl Workspace {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let mut ws = match Self::detect_project_type(&path) {
            ProjectType::Decomp => Self::open_decomp(path)?,
            ProjectType::Dspre => Self::open_dspre(path)?,
            ProjectType::HgEngine => Self::open_hg_engine(path)?,
        };

        ws.load_names()?;
        Ok(ws)
    }

    /// Detect the project type by checking for marker files.
    /// HgEngine markers are checked first since they share `include/constants/`
    /// with decomp projects but have a fundamentally different structure.
    fn detect_project_type(path: &Path) -> ProjectType {
        let hge_markers = [path.join("armips"), path.join("narcs.mk")];
        if hge_markers.iter().any(|marker| marker.exists()) {
            return ProjectType::HgEngine;
        }

        let decomp_markers = [
            path.join("include/constants"),
            path.join("res/field/scripts/scripts.order"),
            path.join("include/data/map_headers.h"),
            path.join("src/data/map_headers.h"),
            path.join("src/script_manager.c"),
            path.join("src/fieldmap.c"),
            path.join("files/fielddata/script/scr_seq"),
            path.join("files/msgdata/msg"),
        ];
        let dspre_markers = [
            path.join("header.bin"),
            path.join("config.yaml"),
            path.join("arm9/arm9.bin"),
            path.join("arm9.bin"),
        ];

        if decomp_markers.iter().any(|marker| marker.exists()) {
            ProjectType::Decomp
        } else {
            let _has_dspre_marker = dspre_markers.iter().any(|marker| marker.exists());
            ProjectType::Dspre
        }
    }

    fn load_names(&mut self) -> std::io::Result<()> {
        self.internal_names = self.load_internal_names()?;
        self.location_names = self.load_location_names()?;
        Ok(())
    }

    fn load_internal_names(&self) -> std::io::Result<Option<Vec<String>>> {
        match self.project_type {
            ProjectType::Dspre => self.load_dspre_internal_names(),
            ProjectType::Decomp => self.load_decomp_internal_names(),
            ProjectType::HgEngine => Ok(None),
        }
    }

    fn load_dspre_internal_names(&self) -> std::io::Result<Option<Vec<String>>> {
        let mapname_bin = self
            .project_path
            .join("data/fielddata/maptable/mapname.bin");
        if !mapname_bin.exists() {
            return Ok(None);
        }

        let data = std::fs::read(mapname_bin)?;
        Ok(Some(
            data.chunks_exact(16)
                .map(|chunk| {
                    String::from_utf8_lossy(chunk)
                        .trim_end_matches('\0')
                        .to_string()
                })
                .collect(),
        ))
    }

    fn load_decomp_internal_names(&self) -> std::io::Result<Option<Vec<String>>> {
        let maps_txt = self.project_path.join("generated/maps.txt");
        if !maps_txt.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(maps_txt)?;
        Ok(Some(
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(|line| {
                    line.find('=')
                        .map_or_else(|| line.to_string(), |pos| line[..pos].trim().to_string())
                })
                .collect(),
        ))
    }

    #[allow(dead_code)]
    fn location_text_archive_id(&self) -> u16 {
        match self.family {
            GameFamily::DP => 382,
            GameFamily::Platinum => 433,
            GameFamily::HGSS => 279,
        }
    }

    fn load_location_names(&self) -> std::io::Result<Option<Vec<String>>> {
        match self.project_type {
            ProjectType::Dspre => self.load_dspre_location_names(),
            ProjectType::Decomp => self.load_decomp_location_names(),
            ProjectType::HgEngine => Ok(None),
        }
    }

    fn load_dspre_location_names(&self) -> std::io::Result<Option<Vec<String>>> {
        Ok(None)
    }

    fn load_decomp_location_names(&self) -> std::io::Result<Option<Vec<String>>> {
        let location_json = self.project_path.join("res/text/location_names.json");
        if !location_json.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&location_json)?;
        let json = serde_json::from_str::<serde_json::Value>(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Failed to parse location names JSON {}: {e}",
                    location_json.display()
                ),
            )
        })?;

        let messages = json
            .get("messages")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Location names JSON {} is missing a 'messages' array",
                        location_json.display()
                    ),
                )
            })?;

        let mut names = Vec::with_capacity(messages.len());
        for (idx, message) in messages.iter().enumerate() {
            let value = message
                .get("en_US")
                .or_else(|| message.get("ja_JP"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "Location names JSON {} has message[{}] missing string 'en_US'/'ja_JP'",
                            location_json.display(),
                            idx
                        ),
                    )
                })?;
            names.push(value.to_string());
        }

        Ok(Some(names))
    }

    pub fn get_map_internal_name(&self, id: u16) -> Option<String> {
        self.internal_names.as_ref()?.get(id as usize).cloned()
    }

    pub fn get_map_location_name(&self, location_id: u8) -> Option<String> {
        self.location_names
            .as_ref()?
            .get(location_id as usize)
            .cloned()
    }

    pub fn get_script_file_for_map(&self, map_id: u16) -> Option<String> {
        let header = self.provider.get_map_header(map_id).ok()?;
        self.scripts
            .get_name(header.script_file_id() as usize)
            .map(str::to_string)
    }

    /// Return the `text_archive_id` associated with a script file.
    ///
    /// First scans map headers for the script file.  Falls back to the
    /// global script table for common scripts that are not tied to any
    /// single map header.
    ///
    /// For DSPRE projects the script table is empty; the file name is the
    /// numeric NARC index (`"0213"` → 213), so that is tried directly when
    /// the name-based lookup returns `None`.
    pub fn text_archive_for_script_file(&self, script_file_name: &str) -> Option<u16> {
        let file_id = self
            .scripts
            .get_id(script_file_name)
            .or_else(|| script_file_name.parse::<u16>().ok().map(|n| n as usize))?;
        if let Ok(Some(id)) = self
            .provider
            .get_text_archive_for_script_file(file_id as u16)
        {
            return Some(id);
        }
        self.global_script_table
            .find_by_script_file_id(file_id as u16)
            .map(|e| e.text_archive_id)
    }

    /// Resolve message text by archive ID and zero-based message index.
    ///
    /// Checks `message_cache` first (fast path), then lazily loads the archive
    /// from disk on miss. Returns `None` for out-of-range indices and
    /// whitespace-only (garbage) slots.
    pub fn read_message(&self, archive_id: u16, msg_index: u16) -> Option<String> {
        if let Some(msgs) = self.message_cache.get(&archive_id) {
            let text = msgs.get(msg_index as usize)?;
            return if text.chars().all(char::is_whitespace) {
                None
            } else {
                Some(text.clone())
            };
        }
        self.load_archive_into_cache(archive_id).ok()?;
        let msgs = self.message_cache.get(&archive_id)?;
        let text = msgs.get(msg_index as usize)?;
        if text.chars().all(char::is_whitespace) {
            None
        } else {
            Some(text.clone())
        }
    }

    /// Return the cached path for `archive_id`, resolving and caching it on
    /// first access.
    pub fn cached_text_archive_path(&self, archive_id: u16) -> Option<PathBuf> {
        if let Some(entry) = self.text_archive_paths.get(&archive_id) {
            return Some(entry.clone());
        }
        let path = self.find_text_archive_path(archive_id)?;
        self.text_archive_paths.insert(archive_id, path.clone());
        Some(path)
    }

    /// Load archive `archive_id` into `message_cache` and populate `message_ids`.
    ///
    /// No-op if the archive is already loaded. Texts are read via
    /// `read_all_messages` (the single, tested locale-fallback implementation).
    /// IDs are extracted in a separate lightweight pass over the `"id"` fields
    /// (JSON archives only; GMM/binary archives have no id fields).
    fn load_archive_into_cache(&self, archive_id: u16) -> std::io::Result<()> {
        if self.message_cache.contains_key(&archive_id) {
            return Ok(());
        }
        let path = self.cached_text_archive_path(archive_id).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no text archive found for ID {archive_id}"),
            )
        })?;

        let messages = read_all_messages(&path, self.language)?;
        self.message_cache.insert(archive_id, messages);

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = json["messages"].as_array() {
                        for (idx, entry) in arr.iter().enumerate() {
                            if let Some(id) = entry["id"].as_str() {
                                self.message_ids
                                    .insert(id.to_string(), (archive_id, idx as u16));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Ensure archive `archive_id` is loaded into the message cache.
    ///
    /// Callers that need the warm guarantee (e.g. the warm step in
    /// `compile_file_internal`) use this instead of accessing `message_cache`
    /// directly.
    pub fn ensure_archive_loaded(&self, archive_id: u16) -> std::io::Result<()> {
        self.load_archive_into_cache(archive_id)
    }

    /// Enumerate all message IDs for a specific archive, for use in completions.
    pub fn message_ids_for_archive(&self, archive_id: u16) -> Vec<(String, u16)> {
        self.message_ids
            .iter()
            .filter(|e| e.value().0 == archive_id)
            .map(|e| (e.key().clone(), e.value().1))
            .collect()
    }

    /// Look up a message id string, returning `(archive_id, msg_index)` if found.
    pub fn resolve_message_id(&self, name: &str) -> Option<(u16, u16)> {
        self.message_ids.get(name).map(|e| *e.value())
    }

    /// Return a clone of the shared message-ID Arc for wiring into `ConstantDb`.
    pub fn shared_message_ids(&self) -> Arc<DashMap<String, (u16, u16)>> {
        self.message_ids.clone()
    }

    /// Locate the on-disk file for `archive_id` without consulting the cache.
    fn find_text_archive_path(&self, archive_id: u16) -> Option<PathBuf> {
        // Named archives (Platinum/HGSS decomp): TEXT_BANK_FOO → foo.json / foo.gmm
        if let Some(name) = self.text_banks.get_name(archive_id as usize)
            && let Some(stem) = name.strip_prefix("TEXT_BANK_")
        {
            let lower = stem.to_lowercase();
            let json = self
                .project_path
                .join("res/text")
                .join(format!("{lower}.json"));
            if json.exists() {
                return Some(json);
            }
            for dir in ["files/msgdata/msg", "msgdata/msg"] {
                let gmm = self.project_path.join(dir).join(format!("{lower}.gmm"));
                if gmm.exists() {
                    return Some(gmm);
                }
            }
        }

        // HGSS direct: msg_{id:04}.gmm, with optional locale suffix
        let stem = format!("msg_{archive_id:04}");
        let gmm_dir = self.project_path.join("files/msgdata/msg");
        let exact = gmm_dir.join(format!("{stem}.gmm"));
        if exact.exists() {
            return Some(exact);
        }
        if let Ok(entries) = std::fs::read_dir(&gmm_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{stem}_")) && name.ends_with(".gmm") {
                    return Some(entry.path());
                }
            }
        }

        // DSPRE: expanded/textArchives/{id:04}.json (zero-padded 4-digit name)
        let archives_dir = self.project_path.join("expanded/textArchives");
        let json = archives_dir.join(format!("{archive_id:04}.json"));
        if json.exists() {
            return Some(json);
        }

        None
    }

    /// Derive a text archive ID from a file stem (include-scan result).
    ///
    /// Platinum: stem `"acuity_cavern"` → `TEXT_BANK_ACUITY_CAVERN` → ID.
    /// HGSS:     stem `"msg_0115_D36R0101"` → parses `0115` → ID.
    /// DSPRE:    stem is already the numeric ID.
    pub fn text_archive_id(&self, stem: &str) -> Option<u16> {
        let upper = format!("TEXT_BANK_{}", stem.to_uppercase().replace('-', "_"));
        if let Some(id) = self.text_banks.get_id(&upper) {
            return Some(id as u16);
        }
        if let Some(num) = stem.strip_prefix("msg_") {
            if let Ok(id) = num.split('_').next()?.parse::<u16>() {
                return Some(id);
            }
        }
        stem.parse::<u16>().ok()
    }

    /// Return the menu entry text archive ID for a command, or `None`.
    pub fn menu_entry_id(command: &str, family: GameFamily) -> Option<u16> {
        match command {
            "AddMenuEntryImm" | "AddMultiOption" | "MenuItemAdd" | "CreateMultiTouchBox" => {
                Some(match family {
                    GameFamily::HGSS => 191,
                    GameFamily::DP => 321,
                    _ => 361,
                })
            }
            _ => None,
        }
    }

    /// Create a lightweight workspace for `project_path` without loading ROM data or symbols.
    ///
    /// `family` determines archive directory layouts and game-specific behaviour
    /// (e.g. menu-entry archive IDs). `project_type` is auto-detected from the
    /// path, so the caller only needs to know the game family. Text archive path
    /// resolution works normally; `text_archive_for_script_file` returns `None`
    /// because no map-header provider is loaded — DSPRE scripts work anyway since
    /// their archive ID is parsed directly from the numeric file stem.
    ///
    /// Use [`Workspace::open`] when full symbol/script-table support is required.
    pub fn new(project_path: PathBuf, game: Game) -> Self {
        let family = game.family();
        let project_type = Self::detect_project_type(&project_path);
        let sm = SourceManager::new();
        Self {
            project_path,
            project_type,
            game,
            family,
            language: GameLanguage::English,
            provider: Box::new(NoopProvider),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        }
    }

    /// Return the index of `text` in archive `archive_id`, inserting it if absent.
    ///
    /// On first access the archive is read from disk to populate the in-memory
    /// message list (used for deduplication). New messages are appended to that
    /// list in memory only; call [`flush_pending_messages`] after compilation to
    /// persist them to disk.
    pub fn find_or_add_message(&self, archive_id: u16, text: &str) -> std::io::Result<u16> {
        self.load_archive_into_cache(archive_id).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "no text archive found for ID {archive_id}; \
                         string literals require a project workspace"
                    ),
                )
            } else {
                e
            }
        })?;
        let mut entry = self.message_cache.get_mut(&archive_id).unwrap();
        if let Some(pos) = entry.iter().position(|m| m == text) {
            return Ok(pos as u16);
        }
        // Reuse the first garbage (empty/whitespace-only) slot rather than appending.
        if let Some(pos) = entry
            .iter()
            .position(|m| m.chars().all(char::is_whitespace))
        {
            entry[pos] = text.to_string();
            return Ok(pos as u16);
        }
        let idx = entry.len();
        entry.push(text.to_string());
        Ok(idx as u16)
    }

    /// Write all newly accumulated messages back to their JSON archives on disk.
    ///
    /// For each archive touched during compilation, reads the current JSON,
    /// appends any messages added since the archive was first loaded, and
    /// writes the file. Archives stored as GMM/binary are skipped (unsupported
    /// for write-back). This should be called once after all files in a
    /// compilation batch have been lowered.
    pub fn flush_pending_messages(&self) -> std::io::Result<()> {
        for kv in &self.message_cache {
            let archive_id = *kv.key();
            let all_messages = kv.value();

            let path = match self.cached_text_archive_path(archive_id) {
                Some(p) => p,
                None => continue,
            };

            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = std::fs::read_to_string(&path)?;
            let mut json: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            let lang_key = self.language.locale_key();

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let existing_count = json["messages"].as_array().map(|a| a.len()).unwrap_or(0);
            let new_messages = &all_messages[existing_count..];

            let messages_arr = json["messages"].as_array_mut().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "missing messages array")
            })?;

            // Overwrite any garbage slots that were reused: the pending entry is non-empty
            // while the on-disk entry is whitespace-only.
            let mut any_garbage_replaced = false;
            for idx in 0..existing_count.min(all_messages.len()) {
                let pending = &all_messages[idx];
                if pending.chars().all(char::is_whitespace) {
                    continue;
                }
                let on_disk_is_garbage = message_entry_is_garbage(&messages_arr[idx], lang_key);
                if on_disk_is_garbage {
                    let content = split_message_into_lines(pending);
                    let content_value = if content.len() == 1 {
                        serde_json::json!(content[0])
                    } else {
                        serde_json::json!(content)
                    };
                    messages_arr[idx][&lang_key] = content_value;
                    any_garbage_replaced = true;
                }
            }

            if new_messages.is_empty() && !any_garbage_replaced {
                continue;
            }

            for (i, text) in new_messages.iter().enumerate() {
                let idx = existing_count + i;
                let content = split_message_into_lines(text);
                let content_value = if content.len() == 1 {
                    serde_json::json!(content[0])
                } else {
                    serde_json::json!(content)
                };
                messages_arr.push(serde_json::json!({
                    "id": format!("msg_{stem}_{idx:05}"),
                    lang_key: content_value,
                }));
            }

            let updated = serde_json::to_string_pretty(&json)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            std::fs::write(&path, updated)?;
        }
        Ok(())
    }
}

/// No-op provider used by [`Workspace::empty`].
struct NoopProvider;

impl DataProvider for NoopProvider {
    fn get_map_header(&self, _id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
        Err(crate::error::UxieError::not_found("Map header", "noop"))
    }
    fn get_map_header_count(&self) -> crate::error::Result<usize> {
        Ok(0)
    }
    fn get_text_archive_for_script_file(&self, _id: u16) -> crate::error::Result<Option<u16>> {
        Ok(None)
    }
    fn find_maps_by_script_file_id(&self, _id: u16) -> crate::error::Result<Vec<u16>> {
        Ok(vec![])
    }
    fn find_maps_by_level_script_file_id(&self, _id: u16) -> crate::error::Result<Vec<u16>> {
        Ok(vec![])
    }
}

/// Split a chatot-encoded message string at line-break escape sequences.
///
/// Each segment retains the trailing `\n`, `\r`, or `\f` escape. Returns a
/// single-element vec for messages with no internal breaks so callers can
/// store them as a plain JSON string rather than an array.
fn split_message_into_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                if matches!(next, 'n' | 'r' | 'f') {
                    chars.next();
                    current.push('\\');
                    current.push(next);
                    lines.push(std::mem::take(&mut current));
                    continue;
                }
            }
        }
        current.push(ch);
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Returns true when a message archive entry has no meaningful text after the
/// same locale fallback as [`read_all_messages`] (primary → `en_US` → `ja_JP` →
/// any non-`id` key).
fn pick_message_value<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    primary_locale: &str,
) -> Option<&'a serde_json::Value> {
    obj.get(primary_locale)
        .or_else(|| obj.get("en_US"))
        .or_else(|| obj.get("ja_JP"))
        .or_else(|| obj.iter().find(|(k, _)| *k != "id").map(|(_, v)| v))
}

/// Returns true when a message archive entry has no meaningful text after the
/// same locale fallback as [`read_all_messages`] (primary → `en_US` → `ja_JP` →
/// any non-`id` key).
fn message_entry_is_garbage(entry: &serde_json::Value, primary_locale: &str) -> bool {
    let Some(obj) = entry.as_object() else {
        return true;
    };
    let value = pick_message_value(obj, primary_locale);

    match value {
        Some(serde_json::Value::String(s)) => s.chars().all(char::is_whitespace),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|e| e.as_str())
            .collect::<String>()
            .chars()
            .all(char::is_whitespace),
        _ => true,
    }
}

/// Read all message strings from a text archive file, dispatching by extension.
fn read_all_messages(path: &Path, language: GameLanguage) -> std::io::Result<Vec<String>> {
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let arr = json
            .get("messages")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "missing messages array")
            })?;
        let lang_key = language.locale_key();
        Ok(arr
            .iter()
            .map(|entry| {
                // Every array element yields exactly one String, so the result
                // stays index-aligned with the JSON `messages` array. Entries
                // with no usable text (id-only placeholder slots, malformed
                // objects) become empty strings rather than being dropped:
                // `message_cache` and `flush_pending_messages` rely on this
                // 1:1 correspondence (a shorter Vec panics the flush slice and
                // misaligns its garbage-slot loop).
                let Some(obj) = entry.as_object() else {
                    return String::new();
                };
                // Try the workspace language first, then English, then Japanese,
                // then whatever key is present — so every entry is counted regardless
                // of which locale the archive was exported with.
                let value = pick_message_value(obj, lang_key);

                // Messages are stored as either a plain string or an array of
                // line segments (chatot multi-line format). Arrays are joined
                // without a separator — chatot does the same when encoding.
                match value {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Array(parts)) => {
                        parts.iter().filter_map(|e| e.as_str()).collect::<String>()
                    }
                    _ => String::new(),
                }
            })
            .collect())
    } else {
        let content = std::fs::read_to_string(path)?;
        crate::c_parser::SymbolTable::extract_gmm_messages(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }
}

/// Return the arm9 map-header table offset and header count for a DSPRE project.
///
/// Offsets are per-language, sourced from DSPRE `RomInfo.cs SetHeaderTableOffset`.
/// Falls back to the English offset for unknown language codes.
fn map_header_table_offset(
    family: crate::game::GameFamily,
    language: crate::game::GameLanguage,
    game_code: &str,
) -> (u64, usize) {
    use crate::game::{GameFamily, GameLanguage};
    let offset: u64 = match family {
        GameFamily::DP => match language {
            GameLanguage::Japanese => {
                // Diamond vs Pearl differ by 4 bytes
                if game_code.starts_with("ADAJ") {
                    0xF0D68
                } else {
                    0xF0D6C
                }
            }
            GameLanguage::English => 0xEEDBC,
            GameLanguage::French => 0xEEDFC,
            GameLanguage::German => 0xEEDCC,
            GameLanguage::Italian => 0xEED70,
            GameLanguage::Spanish => 0xEEE08,
            GameLanguage::Korean => 0xEEDBC,
        },
        GameFamily::Platinum => match language {
            GameLanguage::Japanese => 0xE56F0,
            GameLanguage::English => 0xE601C,
            GameLanguage::French => 0xE60A4,
            GameLanguage::German => 0xE6074,
            GameLanguage::Italian => 0xE6038,
            GameLanguage::Spanish => 0xE60B0,
            GameLanguage::Korean => 0xE601C,
        },
        GameFamily::HGSS => match language {
            GameLanguage::Japanese => 0xF6390,
            GameLanguage::English => 0xF6BE0,
            GameLanguage::French => 0xF6BC4,
            GameLanguage::German => 0xF6B94,
            GameLanguage::Italian => 0xF6B58,
            GameLanguage::Spanish => {
                // HG (IPKx) and SS (IPGx) differ
                if game_code.starts_with("IPK") {
                    0xF6BC8
                } else {
                    0xF6BD0
                }
            }
            GameLanguage::Korean => 0xF6BE0,
        },
    };
    let count = match family {
        GameFamily::HGSS => 540,
        _ => 559,
    };
    (offset, count)
}

impl Workspace {
    pub fn get_symbols_for_map(&self, map_id: u16) -> Vec<String> {
        use crate::c_parser::SymbolTag;
        self.symbols.get_symbols_by_tag(&SymbolTag::Map(map_id))
    }

    pub fn resolve_script_id_to_name(&self, script_id: u16) -> Option<String> {
        if is_common_script_id(script_id) {
            self.symbols.resolve_name(script_id as i64, "CommonScript_")
        } else {
            self.symbols.resolve_name(script_id as i64, "")
        }
    }

    pub fn resolve_script(
        &self,
        script_id: u16,
        map_id: Option<u16>,
    ) -> crate::error::Result<Option<ScriptResolution>> {
        crate::script_file::resolve_script_id(
            script_id,
            map_id,
            &self.global_script_table,
            self.provider.as_ref(),
        )
    }

    pub fn resolve_level_script(
        &self,
        map_id: u16,
    ) -> crate::error::Result<Option<ScriptResolution>> {
        crate::script_file::resolve_level_script(
            map_id,
            &self.global_script_table,
            self.provider.as_ref(),
        )
    }

    pub fn get_map_script_info(&self, map_id: u16) -> crate::error::Result<MapScriptInfo> {
        crate::script_file::get_script_file_info_for_map(map_id, self.provider.as_ref())
    }

    /// Open an hg-engine project. Requires `rom.nds` in the project root for
    /// game detection. Constants (C headers, armips `.equ`) are not loaded
    /// here yet — call `load_hg_engine_constants` afterward.
    fn open_hg_engine(path: PathBuf) -> std::io::Result<Self> {
        // rom.nds must exist in the project root for game detection.
        let rom_path = path.join("rom.nds");
        if !rom_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "rom.nds not found in HgEngine project at {}",
                    path.display()
                ),
            ));
        }
        let header = RomHeader::open(&rom_path)?;
        let game = header.detect_game().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not detect game from rom.nds header",
            )
        })?;
        let family = game.family();
        let language = header.detect_language();

        let sm = SourceManager::new();
        let p = path.clone();
        Ok(Self {
            project_path: path,
            project_type: ProjectType::HgEngine,
            game,
            family,
            language,
            provider: Box::new(DecompProvider::new(&p, SymbolTable::new(), family)),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        })
    }

    /// Load HgEngine constants into this workspace's symbol table.
    ///
    /// 1. C headers from `include/constants/` (parsed first — takes priority).
    /// 2. Armips `.equ`/`equ` directives from `armips/include/` and
    ///    `asm/include/` (skips names already defined by C headers).
    pub fn load_hg_engine_constants(&mut self) -> std::io::Result<()> {
        let root = &self.project_path;
        let symbols = std::sync::Arc::make_mut(&mut self.symbols);

        let c_dir = root.join("include/constants");
        if c_dir.exists() {
            symbols.load_headers_from_dir(c_dir)?;
        }

        // Load armips .equ directives, skipping names already known.
        // Process all files together so cross-file forward references resolve.
        let armips_dirs: Vec<PathBuf> = [root.join("armips/include"), root.join("asm/include")]
            .into_iter()
            .filter(|d| d.exists())
            .collect();
        if !armips_dirs.is_empty() {
            let refs: Vec<&Path> = armips_dirs.iter().map(|d| d.as_path()).collect();
            crate::c_parser::armips_equ::parse_armips_equ_dirs(&refs, symbols)?;
        }

        Ok(())
    }

    fn open_dspre(path: PathBuf) -> std::io::Result<Self> {
        let header = RomHeader::open(&path)?;
        let game = header.detect_game().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not detect game from ROM header",
            )
        })?;
        let family = game.family();

        let arm9_path = [path.join("arm9/arm9.bin"), path.join("arm9.bin")]
            .into_iter()
            .find(|candidate| candidate.exists())
            .unwrap_or_else(|| path.join("arm9/arm9.bin"));

        if !arm9_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "arm9.bin not found in ds-rom project at {} (tried arm9/arm9.bin, arm9.bin)",
                    path.display()
                ),
            ));
        }

        let language = header.detect_language();
        let (offset, count) = map_header_table_offset(family, language, &header.game_code);

        let game_strings = GameStrings::load_from_dspre(&path, family, language)?;

        let global_script_table = match family {
            GameFamily::HGSS => {
                GlobalScriptTable::from_hgss_binary_file_for_language(&arm9_path, game, language)
                    .unwrap_or_else(|_| GlobalScriptTable::new())
            }
            GameFamily::Platinum => match language {
                crate::game::GameLanguage::Japanese => {
                    GlobalScriptTable::platinum_japanese_hardcoded()
                }
                crate::game::GameLanguage::Korean => GlobalScriptTable::platinum_korean_hardcoded(),
                _ => GlobalScriptTable::platinum_western_hardcoded(),
            },
            GameFamily::DP => match language {
                crate::game::GameLanguage::Japanese => GlobalScriptTable::dp_japanese_hardcoded(),
                crate::game::GameLanguage::Korean => GlobalScriptTable::dp_korean_hardcoded(),
                _ => GlobalScriptTable::dp_western_hardcoded(),
            },
        };

        let script_dir = path.join("expanded/scripts");
        let mut scripts = ScriptTable::new();
        if script_dir.exists() {
            scripts.load_dspre_script_dir(&script_dir)?;
        }

        let sm = SourceManager::new();
        Ok(Self {
            project_path: path,
            project_type: ProjectType::Dspre,
            game,
            family,
            language,
            provider: Box::new(Arm9Provider::new(arm9_path, offset, count, family)),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts,
            text_banks: TextBankTable::new(),
            game_strings,
            global_script_table,
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        })
    }

    pub fn open_decomp(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();

        let (game, family) = Self::detect_decomp_game(&root);

        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());

        // Broad recursive parse of the entire project
        Self::load_project_symbols_broad(&root, &mut symbols, family)?;

        let mut scripts = ScriptTable::new();
        let scripts_order = root.join("res/field/scripts/scripts.order");
        if scripts_order.exists() {
            scripts.load_order_file(scripts_order)?;
        } else if family == GameFamily::HGSS {
            let hgss_script_dirs = [
                root.join("files/fielddata/script/scr_seq"),
                root.join("files/fielddata/script"),
            ];
            for script_dir in hgss_script_dirs {
                if script_dir.exists() {
                    scripts.load_hgss_script_dir(script_dir)?;
                }
            }
        }
        for (id, script_name) in scripts.get_all_names().iter().enumerate() {
            symbols.insert_define(script_name.clone(), id as i64);
        }

        let mut text_banks = TextBankTable::new();
        let text_banks_list = root.join("generated/text_banks.txt");
        if text_banks_list.exists() {
            text_banks.load_list_file(text_banks_list)?;
        }

        symbols.resolve_all();
        let symbols = Arc::new(symbols);

        let global_script_table = match family {
            GameFamily::HGSS => {
                let fieldmap_path = root.join("src/fieldmap.c");
                if fieldmap_path.exists() {
                    GlobalScriptTable::from_hgss_decomp_file(&fieldmap_path, &symbols)?
                } else {
                    GlobalScriptTable::new()
                }
            }
            GameFamily::Platinum => {
                let script_manager_path = root.join("src/script_manager.c");
                if script_manager_path.exists() {
                    GlobalScriptTable::from_platinum_decomp_file(&script_manager_path, &symbols)?
                } else {
                    GlobalScriptTable::platinum_western_hardcoded()
                }
            }
            GameFamily::DP => GlobalScriptTable::dp_western_hardcoded(),
        };

        Ok(Self {
            project_path: root.clone(),
            project_type: ProjectType::Decomp,
            game,
            family,
            language: GameLanguage::English,
            provider: Box::new(crate::provider::DecompProvider::new(
                root,
                (*symbols).clone(),
                family,
            )),
            symbols,
            scripts,
            text_banks,
            game_strings: GameStrings::new(),
            global_script_table,
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        })
    }

    pub fn load_cached_symbols(
        cache_dir: &Path,
        project_root: &Path,
        include_roots: &[PathBuf],
        game_family: GameFamily,
    ) -> std::io::Result<(Arc<SymbolTable>, bool)> {
        std::fs::create_dir_all(cache_dir)?;
        let cache_path = cache_dir.join(format!(
            "uxie-constants-{}.bin",
            match game_family {
                GameFamily::DP => "dp",
                GameFamily::Platinum => "platinum",
                GameFamily::HGSS => "hgss",
            }
        ));
        let input_files = Self::collect_cached_symbol_inputs(project_root, include_roots)?;
        let command_database = Self::command_database_path(project_root, game_family);
        let mut cache_input_files = input_files.clone();
        if let Some(command_database) = &command_database {
            cache_input_files.push(command_database.clone());
            cache_input_files.sort();
        }

        if cache_path.is_file() {
            match ConstantCache::load(&cache_path) {
                Ok(cache) if cache.is_current(project_root, game_family, &cache_input_files)? => {
                    return Ok((Arc::new(SymbolTable::from_snapshot(&cache.snapshot)), false));
                }
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {}
                Err(err) => {
                    return Err(std::io::Error::new(
                        err.kind(),
                        format!(
                            "Failed to read constant cache {}: {err}",
                            cache_path.display()
                        ),
                    ));
                }
            }
        }

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        for path in &input_files {
            Self::load_cached_symbol_file(&mut symbols, path)?;
        }
        if let Some(command_database) = &command_database {
            symbols.load_database_var_flag_constants(command_database)?;
        }
        symbols.resolve_all();

        ConstantCache::from_symbols(project_root, game_family, &cache_input_files, &symbols)?
            .save(&cache_path)?;

        Ok((Arc::new(symbols), true))
    }

    fn collect_cached_symbol_inputs(
        project_root: &Path,
        include_roots: &[PathBuf],
    ) -> std::io::Result<Vec<PathBuf>> {
        let mut inputs = BTreeSet::new();
        for root in [
            project_root.join("include/constants"),
            project_root.join("generated"),
            project_root.join("res/field/scripts"),
            project_root.join("res/text"),
            project_root.join("build/res/text/bank"),
            project_root.join("files"),
        ] {
            inputs.extend(Self::collect_cached_symbol_root_files(&root)?);
        }
        for root in include_roots {
            inputs.extend(Self::collect_cached_symbol_root_files(root)?);
        }
        for path in [
            project_root.join("include/script_manager.h"),
            project_root.join("build/res/field/scripts/scr_seq.naix.h"),
            project_root.join("build/debug/res/field/scripts/scr_seq.naix.h"),
            project_root.join("build/release/res/field/scripts/scr_seq.naix.h"),
            project_root.join("res/field/scripts/scr_seq.naix.h"),
            project_root.join("files/fielddata/script/scr_seq.naix"),
            project_root.join("fielddata/script/scr_seq.naix"),
            project_root.join("files/msgdata/msg.naix"),
            project_root.join("msgdata/msg.naix"),
        ] {
            if path.is_file() {
                inputs.insert(path);
            }
        }
        Ok(inputs.into_iter().collect())
    }

    /// Returns the path to the command database for the given game family, if it exists in the current project.
    fn command_database_path(project_root: &Path, game_family: GameFamily) -> Option<PathBuf> {
        let file_name = match game_family {
            GameFamily::DP => "diamond_pearl_v2.json",
            GameFamily::Platinum => "platinum_v2.json",
            GameFamily::HGSS => "hgss_v2.json",
        };
        let path = project_root.join(".rotom/command_database").join(file_name);
        if path.is_file() { Some(path) } else { None }
    }

    fn collect_cached_symbol_root_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
        if root.is_file() {
            return Ok(if Self::is_cached_symbol_source_file(root) {
                vec![root.to_path_buf()]
            } else {
                Vec::new()
            });
        }
        if !root.is_dir() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some(".git") {
                    files.extend(Self::collect_cached_symbol_root_files(&path)?);
                }
                continue;
            }

            if Self::is_cached_symbol_source_file(&path) {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn is_cached_symbol_source_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .is_some_and(|ext| matches!(ext.as_str(), "h" | "hpp" | "txt" | "py" | "json"))
    }

    fn load_cached_symbol_file(symbols: &mut SymbolTable, path: &Path) -> std::io::Result<()> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        match extension.as_deref() {
            Some("txt") => {
                // Skip template files (e.g. Jinja2/Handlebars) that happen to
                // have a .txt extension but are not constant list files.
                let content = std::fs::read_to_string(path)?;
                if content.contains("{%") || content.contains("{{") {
                    return Ok(());
                }
                symbols.load_list_file(path)
            }
            Some("py") => symbols.load_python_enum(path),
            Some("json") => {
                // Quick sniff-test: only treat JSON as a text bank if it has
                // a top-level "messages" or "object_events" array. This skips
                // levelscripts and other non-text-bank JSON files that may live
                // in the include tree.
                let content = std::fs::read_to_string(path)?;
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let messages = json.get("messages").and_then(|v| v.as_array());
                    let object_events = json.get("object_events").and_then(|v| v.as_array());
                    if messages.is_some() || object_events.is_some() {
                        return symbols.load_text_bank_json(path).map(|_| ());
                    }
                }
                Ok(())
            }
            _ => symbols.load_header(path),
        }
    }

    fn detect_decomp_game(root: &Path) -> (Game, GameFamily) {
        let platinum_markers = [
            root.join("res/field/scripts/scripts.order"),
            root.join("include/data/map_headers.h"),
            root.join("src/script_manager.c"),
        ];
        let hgss_markers = [
            root.join("src/data/map_headers.h"),
            root.join("src/fieldmap.c"),
            root.join("files/fielddata/script/scr_seq"),
            root.join("files/msgdata/msg"),
        ];

        let has_platinum_marker = platinum_markers.iter().any(|path| path.exists());
        let has_hgss_marker = hgss_markers.iter().any(|path| path.exists());

        if has_hgss_marker && !has_platinum_marker {
            (Game::HeartGold, GameFamily::HGSS)
        } else {
            (Game::Platinum, GameFamily::Platinum)
        }
    }

    fn load_project_symbols_broad(
        root: &Path,
        symbols: &mut SymbolTable,
        family: GameFamily,
    ) -> std::io::Result<()> {
        // 1. Load all constants from include/constants
        let include_constants = root.join("include/constants");
        if include_constants.exists() {
            symbols.load_headers_from_dir(&include_constants)?;
        }

        // 2. Load generated constants from source-generated files.
        let generated = root.join("generated");
        if generated.exists() {
            symbols.load_headers_from_dir(&generated)?;
        }

        let text_dirs = [root.join("res/text"), root.join("build/res/text/bank")];
        for text_dir in text_dirs {
            if text_dir.exists() {
                symbols.load_headers_from_dir(&text_dir)?;
            }
        }

        let hgss_symbol_dirs = [
            root.join("files/msgdata/msg"),
            root.join("files/fielddata/script/scr_seq"),
        ];
        for symbol_dir in hgss_symbol_dirs {
            if !symbol_dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&symbol_dir)? {
                let path = entry?.path();
                if !path.is_file() {
                    continue;
                }

                let is_header = path.extension().and_then(|s| s.to_str()) == Some("h");
                let file_name = path.file_name().and_then(|s| s.to_str());
                let is_hgss_symbol_header = file_name
                    .is_some_and(|name| name.starts_with("msg_") || name.starts_with("event_"));
                if is_header && is_hgss_symbol_header {
                    match symbols.load_header(&path) {
                        Ok(()) => {}
                        Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
                            let bytes = std::fs::read(&path)?;
                            let content = String::from_utf8_lossy(&bytes);
                            symbols.load_header_str_with_tag(
                                &content,
                                &path,
                                crate::c_parser::SymbolTag::Global,
                            )?;
                        }
                        Err(err) => return Err(err),
                    }
                }
            }
        }

        // 7. Load script/message index constants needed by global script table parsing.
        // These files define symbols like `scripts_common`, `NARC_scr_seq_*`, and `NARC_msg_*`.
        let index_files = [
            root.join("include/script_manager.h"),
            root.join("build/res/field/scripts/scr_seq.naix.h"),
            root.join("build/debug/res/field/scripts/scr_seq.naix.h"),
            root.join("build/release/res/field/scripts/scr_seq.naix.h"),
            root.join("res/field/scripts/scr_seq.naix.h"),
            root.join("files/fielddata/script/scr_seq.naix"),
            root.join("fielddata/script/scr_seq.naix"),
            root.join("files/msgdata/msg.naix"),
            root.join("msgdata/msg.naix"),
        ];
        for index_file in index_files {
            if index_file.exists() {
                symbols.load_header(index_file)?;
            }
        }

        // NOTE: We intentionally do NOT load res/field/events or
        // build/res/field/events headers here. Those contain per-map
        // LOCALID_* definitions that conflict across maps. Each script should
        // load its own events header via #include resolution in
        // collect_constants_for_file() or collect_constants_for_source().

        if let Some(command_database) = Self::command_database_path(root, family) {
            symbols.load_database_var_flag_constants(command_database)?;
        }

        Ok(())
    }

    pub fn collect_constants_for_file(
        &self,
        path: impl AsRef<Path>,
    ) -> std::io::Result<SymbolTable> {
        let mut include_dirs = Vec::new();
        if self.project_type == ProjectType::Decomp {
            include_dirs.push(self.project_path.clone());
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(self.symbols.clone());
        self.load_project_constants_recursive(&mut table, path.as_ref(), &include_dirs)?;
        Ok(table)
    }

    pub fn collect_constants_for_source(
        &self,
        source: &str,
        current_file_dir: impl AsRef<Path>,
    ) -> std::io::Result<SymbolTable> {
        let mut include_dirs = Vec::new();
        if self.project_type == ProjectType::Decomp {
            include_dirs.push(self.project_path.clone());
            include_dirs.push(self.project_path.join("include"));
            include_dirs.push(self.project_path.join("res/field/scripts"));
        }

        let mut table = SymbolTable::with_parent(self.symbols.clone());
        self.load_project_constants_recursive_str(
            &mut table,
            source,
            current_file_dir.as_ref(),
            &include_dirs,
        )?;
        Ok(table)
    }

    fn load_project_constants_recursive(
        &self,
        table: &mut SymbolTable,
        path: &Path,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        if self.project_type == ProjectType::Decomp {
            let mut unresolved_include_handler = |table: &mut SymbolTable,
                                                  parent_dir: &Path,
                                                  include_dirs: &[PathBuf],
                                                  include_path: &str|
             -> std::io::Result<bool> {
                Self::try_load_decomp_events_include_json(
                    table,
                    parent_dir,
                    include_dirs,
                    include_path,
                )
            };
            table.load_recursive_with_handler(
                path,
                include_dirs,
                Some(&mut unresolved_include_handler),
            )
        } else {
            table.load_recursive(path, include_dirs)
        }
    }

    fn load_project_constants_recursive_str(
        &self,
        table: &mut SymbolTable,
        source: &str,
        current_file_dir: &Path,
        include_dirs: &[PathBuf],
    ) -> std::io::Result<()> {
        if self.project_type == ProjectType::Decomp {
            let mut unresolved_include_handler = |table: &mut SymbolTable,
                                                  parent_dir: &Path,
                                                  include_dirs: &[PathBuf],
                                                  include_path: &str|
             -> std::io::Result<bool> {
                Self::try_load_decomp_events_include_json(
                    table,
                    parent_dir,
                    include_dirs,
                    include_path,
                )
            };
            table.load_recursive_str_with_handler(
                source,
                current_file_dir,
                include_dirs,
                Some(&mut unresolved_include_handler),
            )
        } else {
            table.load_recursive_str(source, current_file_dir, include_dirs)
        }
    }

    fn try_load_decomp_events_include_json(
        table: &mut SymbolTable,
        parent_dir: &Path,
        include_dirs: &[PathBuf],
        include_path: &str,
    ) -> std::io::Result<bool> {
        if !include_path.contains("res/field/events/") || !include_path.ends_with(".h") {
            return Ok(false);
        }

        let json_path_str = include_path.replace(".h", ".json");
        let json_rel = parent_dir.join(&json_path_str);
        if json_rel.exists() {
            table.load_events_json(&json_rel)?;
            return Ok(true);
        }

        for dir in include_dirs {
            let json_path = dir.join(&json_path_str);
            if json_path.exists() {
                table.load_events_json(&json_path)?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resolve a symbolic constant from the workspace symbol table.
    ///
    /// This is commonly passed as a callback into decomp conversion helpers,
    /// for example: `to_move_data(|name| workspace.resolve_constant(name))`.
    pub fn resolve_constant(&self, name: &str) -> Option<i64> {
        self.symbols.resolve_constant(name)
    }

    pub fn resolve_name(&self, value: i64, prefix: &str) -> Option<String> {
        self.symbols.resolve_name(value, prefix)
    }

    pub fn resolve_names(&self, value: i64, prefixes: &[&str]) -> Vec<String> {
        self.symbols.resolve_names(value, prefixes)
    }

    pub fn resolve_script_symbols(&self, script: &str) -> String {
        self.resolve_script_symbols_with(script, &self.symbols)
    }

    pub fn resolve_script_symbols_with(&self, script: &str, symbols: &SymbolTable) -> String {
        let mut result = String::with_capacity(script.len());
        let mut start_idx = 0;

        for (i, c) in script.char_indices() {
            let is_token_char = c.is_alphanumeric() || c == '_';

            if !is_token_char {
                if i > start_idx {
                    let token = &script[start_idx..i];
                    self.append_resolved_token_with(&mut result, token, symbols);
                }
                result.push(c);
                start_idx = i + c.len_utf8();
            }
        }

        if start_idx < script.len() {
            let token = &script[start_idx..];
            self.append_resolved_token_with(&mut result, token, symbols);
        }

        result
    }

    fn append_resolved_token_with(&self, result: &mut String, token: &str, symbols: &SymbolTable) {
        if let Some(val) = symbols.resolve_constant(token) {
            result.push_str(&val.to_string());
        } else if let Ok(val) = token.parse::<i64>() {
            if let Some(name) = symbols.resolve_name(val, "") {
                result.push_str(&name);
            } else if let Some(name) = self.resolve_name(val, "") {
                result.push_str(&name);
            } else {
                result.push_str(token);
            }
        } else {
            result.push_str(token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_parser::ConstantCache;
    use crate::map_header::MapHeader;
    use crate::rom_header::ROM_HEADER_SIZE;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_test_header_bin(path: &Path, game_title: &str, game_code: &str) {
        let mut data = vec![0_u8; ROM_HEADER_SIZE];
        data[..game_title.len()].copy_from_slice(game_title.as_bytes());
        data[12..16].copy_from_slice(game_code.as_bytes());
        data[16..18].copy_from_slice(b"01");
        fs::write(path, data).unwrap();
    }

    fn parse_hgss_script_filename_for_test(file_name: &str) -> Option<(usize, String)> {
        let stem = file_name.strip_suffix(".s")?;
        let mut parts = stem.split('_');

        if parts.next()? != "scr" || parts.next()? != "seq" {
            return None;
        }

        let id_part = parts.next()?;
        if id_part.len() != 4 || !id_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let id = id_part.parse::<usize>().ok()?;
        Some((id, stem.to_string()))
    }

    fn expected_hgss_script_from_fixture(root: &Path) -> std::io::Result<Option<(usize, String)>> {
        // Workspace loads `scr_seq` first and `script` second, so `script` wins on collisions.
        let precedence_dirs = [
            root.join("files/fielddata/script"),
            root.join("files/fielddata/script/scr_seq"),
        ];

        for dir in precedence_dirs {
            if !dir.exists() {
                continue;
            }

            let mut parsed = Vec::new();
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }

                let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
                    continue;
                };

                if let Some((id, script_name)) = parse_hgss_script_filename_for_test(&file_name) {
                    parsed.push((id, script_name));
                }
            }

            parsed.sort_by_key(|(id, _)| *id);
            if let Some(first) = parsed.into_iter().next() {
                return Ok(Some(first));
            }
        }

        Ok(None)
    }

    fn write_command_database_var_flag_fixture(root: &Path) -> PathBuf {
        let db_dir = root.join(".rotom/command_database");
        fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join("hgss_v2.json");
        fs::write(
            &db_path,
            r#"{
                "vars": {
                    "VARS_END": { "id": 16751 }
                },
                "flags": {
                    "FLAG_MAPTEMP_001": { "id": 1 }
                }
            }"#,
        )
        .unwrap();
        db_path
    }

    #[test]
    fn test_load_cached_symbols_round_trips_and_reuses_valid_cache() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::write(
            root.join("include/constants/test.h"),
            "#define TEST_CONST 42\n",
        )
        .unwrap();
        let cache_dir = root.join(".rotom/cache");

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("TEST_CONST"), Some(42));

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(!rebuilt);
        assert_eq!(symbols.resolve_constant("TEST_CONST"), Some(42));
    }

    #[test]
    fn test_load_cached_symbols_uses_command_database_as_var_flag_fallback() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write_command_database_var_flag_fixture(root);
        let cache_dir = root.join(".rotom/cache");

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::HGSS).unwrap();

        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("VARS_END"), Some(16751));
        assert_eq!(symbols.resolve_constant("FLAG_MAPTEMP_001"), Some(1));

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::HGSS).unwrap();

        assert!(!rebuilt);
        assert_eq!(symbols.resolve_constant("VARS_END"), Some(16751));
        assert_eq!(symbols.resolve_constant("FLAG_MAPTEMP_001"), Some(1));
    }

    #[test]
    fn test_load_cached_symbols_keeps_header_vars_and_flags_before_command_database_fallback() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::write(
            root.join("include/constants/vars.h"),
            "#define VAR_FROM_HEADER 16384\n",
        )
        .unwrap();
        fs::write(
            root.join("include/constants/flags.h"),
            "#define FLAG_FROM_HEADER 1\n",
        )
        .unwrap();
        write_command_database_var_flag_fixture(root);
        let cache_dir = root.join(".rotom/cache");

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::HGSS).unwrap();

        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("VAR_FROM_HEADER"), Some(16384));
        assert_eq!(symbols.resolve_constant("FLAG_FROM_HEADER"), Some(1));
        assert_eq!(symbols.resolve_constant("VARS_END"), None);
        assert_eq!(symbols.resolve_constant("FLAG_MAPTEMP_001"), None);
    }

    #[test]
    fn test_load_cached_symbols_rebuilds_on_corrupt_or_stale_cache() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("include/constants")).unwrap();
        let header = root.join("include/constants/test.h");
        fs::write(&header, "#define TEST_CONST 42\n").unwrap();
        let cache_dir = root.join(".rotom/cache");
        let cache_path = cache_dir.join("uxie-constants-platinum.bin");

        Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();

        fs::write(&cache_path, b"broken-cache").unwrap();
        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("TEST_CONST"), Some(42));

        let mut cache = ConstantCache::load(&cache_path).unwrap();
        cache
            .file_hashes
            .insert("include/constants/test.h".to_string(), 0);
        cache.save(&cache_path).unwrap();

        let (_, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(rebuilt);
    }

    #[test]
    fn test_load_cached_symbols_uses_configured_include_roots() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let custom_root = root.join("custom/constants");
        fs::create_dir_all(&custom_root).unwrap();
        fs::write(custom_root.join("test.h"), "#define TEST_CONST 42\n").unwrap();
        let cache_dir = root.join(".rotom/cache");

        let (symbols, rebuilt) = Workspace::load_cached_symbols(
            &cache_dir,
            root,
            std::slice::from_ref(&custom_root),
            GameFamily::Platinum,
        )
        .unwrap();

        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("TEST_CONST"), Some(42));
    }

    #[test]
    fn test_load_cached_symbols_rebuilds_when_new_input_file_appears() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let include_dir = root.join("include/constants");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(include_dir.join("test.h"), "#define TEST_CONST 42\n").unwrap();
        let cache_dir = root.join(".rotom/cache");

        let (_, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(rebuilt);

        fs::write(include_dir.join("new.h"), "#define NEW_CONST 99\n").unwrap();

        let (symbols, rebuilt) =
            Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum).unwrap();
        assert!(rebuilt);
        assert_eq!(symbols.resolve_constant("NEW_CONST"), Some(99));
    }

    #[test]
    fn test_load_cached_symbol_file_supports_txt_py_and_json_sources() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let txt_path = root.join("custom.txt");
        let py_path = root.join("custom.py");
        let json_path = root.join("custom.json");

        fs::write(&txt_path, "TXT_CONST = 7\n").unwrap();
        fs::write(&py_path, "PY_CONST = 9\n").unwrap();
        fs::write(&json_path, r#"{ "messages": [ { "id": "MSG_HELLO" } ] }"#).unwrap();

        let mut symbols = SymbolTable::with_source_manager(SourceManager::new());
        Workspace::load_cached_symbol_file(&mut symbols, &txt_path).unwrap();
        Workspace::load_cached_symbol_file(&mut symbols, &py_path).unwrap();
        Workspace::load_cached_symbol_file(&mut symbols, &json_path).unwrap();
        symbols.resolve_all();

        assert_eq!(symbols.resolve_constant("TXT_CONST"), Some(7));
        assert_eq!(symbols.resolve_constant("PY_CONST"), Some(9));
        assert_eq!(symbols.resolve_constant("MSG_HELLO"), Some(0));
    }

    #[test]
    fn test_load_cached_symbols_propagates_non_decode_cache_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::write(
            root.join("include/constants/test.h"),
            "#define TEST_CONST 42\n",
        )
        .unwrap();
        let cache_dir = root.join(".rotom/cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_path = cache_dir.join("uxie-constants-platinum.bin");
        fs::write(&cache_path, b"cached").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&cache_path).unwrap().permissions();
            permissions.set_mode(0o222);
            fs::set_permissions(&cache_path, permissions).unwrap();
        }

        let error = Workspace::load_cached_symbols(&cache_dir, root, &[], GameFamily::Platinum)
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn test_resolve_script_symbols_with_constants() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("FLAG_START".to_string(), 100);
        symbols.insert_define("VAR_TEMP".to_string(), 16384);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        let result = ws.resolve_script_symbols("SetFlag FLAG_START");
        assert_eq!(result, "SetFlag 100");

        let result = ws.resolve_script_symbols("GetVar VAR_TEMP + FLAG_START");
        assert_eq!(result, "GetVar 16384 + 100");
    }

    #[test]
    fn test_resolve_script_symbols_numeric_to_name() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("FLAG_TEST".to_string(), 42);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        let result = ws.resolve_script_symbols("SetFlag 42");
        assert_eq!(result, "SetFlag FLAG_TEST");
    }

    #[test]
    fn test_resolve_script_symbols_passthrough() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        let result = ws.resolve_script_symbols("UnknownCmd UNKNOWN_FLAG");
        assert_eq!(result, "UnknownCmd UNKNOWN_FLAG");
    }

    #[test]
    fn test_resolve_script_symbols_with_prefers_file_local_constants() {
        let sm = SourceManager::new();

        let mut global_symbols = SymbolTable::with_source_manager(sm.clone());
        global_symbols.insert_define("LOCALID_HIKER".to_string(), 3);
        global_symbols.insert_define("FLAG_START".to_string(), 100);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(global_symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        let mut file_symbols = SymbolTable::with_parent(ws.symbols.clone());
        file_symbols.insert_define("LOCALID_HIKER".to_string(), 0);

        let result = ws.resolve_script_symbols_with(
            "ApplyMovement LOCALID_HIKER\nSetFlag FLAG_START\n",
            &file_symbols,
        );
        assert_eq!(result, "ApplyMovement 0\nSetFlag 100\n");
    }

    #[test]
    fn test_collect_constants_for_file_loads_map_local_event_ids() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        std::fs::create_dir_all(root.join("res/field/events")).unwrap();

        std::fs::write(
            root.join("res/field/scripts/scripts.order"),
            "test_script\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/scripts/test_script.s"),
            concat!(
                "#include \"res/field/events/events_test_map.h\"\n",
                "ApplyMovement LOCALID_HIKER, TestMovement\n",
            ),
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/events/events_test_map.json"),
            concat!(
                "{\n",
                "  \"object_events\": [\n",
                "    { \"id\": \"LOCALID_HIKER\" },\n",
                "    { \"id\": \"LOCALID_TWIN\" }\n",
                "  ]\n",
                "}\n"
            ),
        )
        .unwrap();

        let ws = Workspace::open_decomp(root).unwrap();

        assert_eq!(ws.resolve_constant("LOCALID_HIKER"), None);
        assert_eq!(ws.resolve_constant("LOCALID_TWIN"), None);

        let symbols = ws
            .collect_constants_for_file(root.join("res/field/scripts/test_script.s"))
            .unwrap();

        assert_eq!(symbols.resolve_constant("LOCALID_HIKER"), Some(0));
        assert_eq!(symbols.resolve_constant("LOCALID_TWIN"), Some(1));

        let resolved =
            ws.resolve_script_symbols_with("ApplyMovement LOCALID_HIKER, TestMovement", &symbols);
        assert_eq!(resolved, "ApplyMovement 0, TestMovement");
    }

    #[test]
    fn test_collect_constants_for_file_propagates_event_json_parse_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        std::fs::create_dir_all(root.join("res/field/events")).unwrap();

        std::fs::write(
            root.join("res/field/scripts/scripts.order"),
            "test_script\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/scripts/test_script.s"),
            "#include \"res/field/events/events_test_map.h\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/events/events_test_map.json"),
            "{ this is not valid json }",
        )
        .unwrap();

        let ws = Workspace::open_decomp(root).unwrap();
        let err = ws
            .collect_constants_for_file(root.join("res/field/scripts/test_script.s"))
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_collect_constants_for_file_propagates_event_json_schema_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        std::fs::create_dir_all(root.join("res/field/events")).unwrap();

        std::fs::write(
            root.join("res/field/scripts/scripts.order"),
            "test_script\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/scripts/test_script.s"),
            "#include \"res/field/events/events_test_map.h\"\n",
        )
        .unwrap();
        std::fs::write(root.join("res/field/events/events_test_map.json"), "{}").unwrap();

        let ws = Workspace::open_decomp(root).unwrap();
        let err = ws
            .collect_constants_for_file(root.join("res/field/scripts/test_script.s"))
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("object_events"));
    }

    #[test]
    fn test_collect_constants_for_file_propagates_event_json_entry_id_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        std::fs::create_dir_all(root.join("res/field/events")).unwrap();

        std::fs::write(
            root.join("res/field/scripts/scripts.order"),
            "test_script\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/scripts/test_script.s"),
            "#include \"res/field/events/events_test_map.h\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("res/field/events/events_test_map.json"),
            r#"{ "object_events": [ {} ] }"#,
        )
        .unwrap();

        let ws = Workspace::open_decomp(root).unwrap();
        let err = ws
            .collect_constants_for_file(root.join("res/field/scripts/test_script.s"))
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("object_events[0].id"));
    }

    #[test]
    fn test_get_map_internal_name() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let mut ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: Some(vec!["D01R0101".to_string(), "D02R0102".to_string()]),
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        assert_eq!(ws.get_map_internal_name(0), Some("D01R0101".to_string()));
        assert_eq!(ws.get_map_internal_name(1), Some("D02R0102".to_string()));
        assert_eq!(ws.get_map_internal_name(99), None);

        ws.internal_names = None;
        assert_eq!(ws.get_map_internal_name(0), None);
    }

    #[test]
    fn test_get_map_location_name() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let mut ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: Some(vec![
                "Twinleaf Town".to_string(),
                "Sandgem Town".to_string(),
            ]),
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        assert_eq!(
            ws.get_map_location_name(0),
            Some("Twinleaf Town".to_string())
        );
        assert_eq!(
            ws.get_map_location_name(1),
            Some("Sandgem Town".to_string())
        );
        assert_eq!(ws.get_map_location_name(99), None);

        ws.location_names = None;
        assert_eq!(ws.get_map_location_name(0), None);
    }

    #[test]
    fn test_resolve_constant() {
        let sm = SourceManager::new();
        let mut symbols = SymbolTable::with_source_manager(sm.clone());
        symbols.insert_define("TEST_CONST".to_string(), 12345);

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(MockProvider),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        assert_eq!(ws.resolve_constant("TEST_CONST"), Some(12345));
        assert_eq!(ws.resolve_constant("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_script_file_for_map_hgss_decomp_missing_name_returns_none() {
        let sm = SourceManager::new();
        let symbols = SymbolTable::with_source_manager(sm.clone());

        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Decomp,
            language: crate::game::GameLanguage::English,
            game: Game::HeartGold,
            family: GameFamily::HGSS,
            provider: Box::new(HgssScriptProvider { script_file_id: 81 }),
            symbols: Arc::new(symbols),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        assert_eq!(ws.get_script_file_for_map(0), None);
    }

    #[test]
    fn test_open_decomp_detection() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        let mut const_file = fs::File::create(root.join("include/constants/test.h")).unwrap();
        writeln!(const_file, "#define TEST_VALUE 42").unwrap();

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
        assert_eq!(ws.resolve_constant("TEST_VALUE"), Some(42));
    }

    #[test]
    fn test_open_decomp_detection_without_include_constants_uses_platinum_markers() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("totally_generic_project");

        fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        fs::write(
            root.join("res/field/scripts/scripts.order"),
            "script_main\nscript_event\n",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
        assert_eq!(ws.scripts.get_name(0), Some("script_main"));
        assert_eq!(ws.scripts.get_name(1), Some("script_event"));
    }

    #[test]
    fn test_open_decomp_detection_without_include_constants_uses_hgss_markers() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("totally_generic_project");

        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::HeartGold);
        assert_eq!(ws.family, GameFamily::HGSS);
    }

    #[test]
    fn test_open_decomp_uses_command_database_as_var_flag_fallback() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("totally_generic_project");

        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();
        write_command_database_var_flag_fixture(&root);

        let ws = Workspace::open_decomp(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.resolve_constant("VARS_END"), Some(16751));
        assert_eq!(ws.resolve_constant("FLAG_MAPTEMP_001"), Some(1));
    }

    #[test]
    fn test_open_dspre_detection_does_not_depend_on_root_name() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("pokeheartgold");

        fs::create_dir_all(root.join("unpacked")).unwrap();
        write_test_header_bin(&root.join("header.bin"), "POKEMON PL", "CPUE");
        fs::write(root.join("arm9.bin"), vec![0_u8; 4]).unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Dspre);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
    }

    #[test]
    fn test_open_dspre_accepts_arm9_subdirectory_layout() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_dspre_project");

        fs::create_dir_all(root.join("arm9")).unwrap();
        write_test_header_bin(&root.join("header.bin"), "POKEMON PL", "CPUE");
        fs::write(root.join("arm9/arm9.bin"), vec![0_u8; 4]).unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Dspre);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
    }

    #[test]
    fn test_open_decomp_with_scripts_order() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("res/field/scripts")).unwrap();

        let mut order_file =
            fs::File::create(root.join("res/field/scripts/scripts.order")).unwrap();
        writeln!(order_file, "script_main").unwrap();
        writeln!(order_file, "script_event").unwrap();

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.scripts.get_name(0), Some("script_main"));
        assert_eq!(ws.scripts.get_name(1), Some("script_event"));
    }

    #[test]
    fn test_open_decomp_location_names_uses_ja_jp_when_en_us_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("res/text")).unwrap();
        fs::write(
            root.join("res/text/location_names.json"),
            r#"{"messages":[{"ja_JP":"jp_only_location"}]}"#,
        )
        .unwrap();

        let ws = Workspace::open(root).unwrap();
        assert_eq!(
            ws.get_map_location_name(0),
            Some("jp_only_location".to_string())
        );
    }

    #[test]
    fn test_open_decomp_invalid_location_names_entry_returns_invalid_data() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("res/text")).unwrap();
        fs::write(
            root.join("res/text/location_names.json"),
            r#"{"messages":[{"id":"LOCATION_ONLY_ID"}]}"#,
        )
        .unwrap();

        match Workspace::open(root) {
            Ok(_) => panic!("Expected open to fail for invalid location_names.json entry"),
            Err(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
                assert!(err.to_string().contains("message[0]"));
                assert!(err.to_string().contains("en_US"));
                assert!(err.to_string().contains("ja_JP"));
            }
        }
    }

    #[test]
    fn test_open_decomp_hgss_loads_id_named_script_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0003_D01R0101.s"),
            "",
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0081_D32R0102.s"),
            "",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.game, Game::HeartGold);
        assert_eq!(ws.scripts.get_name(3), Some("scr_seq_0003_D01R0101"));
        assert_eq!(ws.scripts.get_name(81), Some("scr_seq_0081_D32R0102"));
        assert_eq!(ws.scripts.get_name(4), None);
    }

    #[test]
    fn test_open_decomp_hgss_loads_msg_header_symbols() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/msgdata/msg")).unwrap();
        fs::write(
            root.join("files/msgdata/msg/msg_0139_D49R0102.h"),
            concat!(
                "#ifndef MSG_0139_D49R0102_H\n",
                "#define MSG_0139_D49R0102_H\n",
                "#define msg_0139_D49R0102_00000 0\n",
                "#define msg_0139_D49R0102_00004 4\n",
                "#endif\n"
            ),
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.resolve_constant("msg_0139_D49R0102_00000"), Some(0));
        assert_eq!(ws.resolve_constant("msg_0139_D49R0102_00004"), Some(4));
    }

    #[test]
    fn test_open_decomp_hgss_loads_event_header_symbols() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/event_D02R0101.h"),
            concat!(
                "#ifndef SCR_SEQ_D02R0101_H_\n",
                "#define SCR_SEQ_D02R0101_H_\n",
                "#define _EV_scr_seq_D02R0101_000 0\n",
                "#define obj_D02R0101_player 7\n",
                "#define obj_D02R0101_gsrivel 0\n",
                "#endif\n"
            ),
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.resolve_constant("obj_D02R0101_player"), Some(7));
        assert_eq!(ws.resolve_constant("obj_D02R0101_gsrivel"), Some(0));
    }

    #[test]
    fn test_open_decomp_hgss_fieldmap_parses_without_generated_narc_headers() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();

        fs::write(
            root.join("include/constants/std_script.h"),
            concat!(
                "#define _std_misc 2000\n",
                "#define _std_scratch_card 10500\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join("src/fieldmap.c"),
            concat!(
                "const struct ScriptBankMapping sScriptBankMapping[30] = {\n",
                "    { _std_scratch_card, NARC_scr_seq_scr_seq_0263_bin, NARC_msg_msg_0433_bin },\n",
                "    { _std_misc, NARC_scr_seq_scr_seq_0003_bin, NARC_msg_msg_0040_bin },\n",
                "};\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0003.s"),
            "",
        )
        .unwrap();

        let ws = Workspace::open_decomp(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);

        let entry = ws.global_script_table.lookup(10500).unwrap();
        assert_eq!(entry.min_script_id, 10500);
        assert_eq!(entry.script_file_id, 263);
        assert_eq!(entry.text_archive_id, 433);

        let entry = ws.global_script_table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 3);
        assert_eq!(entry.text_archive_id, 40);
    }

    #[test]
    fn test_open_decomp_hgss_still_loads_global_constants_from_include_constants() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::write(
            root.join("include/constants/test_flags.h"),
            "#define STICKS_ACTIVE 123\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("files/msgdata/msg")).unwrap();
        fs::write(
            root.join("files/msgdata/msg/msg_0014.h"),
            "#define msg_0014_00000 0\n",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.resolve_constant("STICKS_ACTIVE"), Some(123));
        assert_eq!(ws.resolve_constant("msg_0014_00000"), Some(0));
    }

    #[test]
    fn test_open_decomp_hgss_symbol_scan_ignores_non_utf8_non_header_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("generic_hgss_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/msgdata/msg")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();

        fs::write(
            root.join("files/msgdata/msg/msg_0139_D49R0102.h"),
            concat!(
                "#define msg_0139_D49R0102_00000 0\n",
                "#define msg_0139_D49R0102_00004 4\n"
            ),
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/event_D02R0101.h"),
            concat!(
                "#define obj_D02R0101_player 7\n",
                "#define obj_D02R0101_gsrivel 0\n"
            ),
        )
        .unwrap();

        fs::write(
            root.join("files/msgdata/msg/msg_0139_D49R0102.bin"),
            [0xFF_u8, 0xFE_u8, 0x00_u8, 0x01_u8],
        )
        .unwrap();
        fs::write(
            root.join("files/msgdata/msg/msg_0139_D49R0102.gmm"),
            [0xFF_u8, 0xFE_u8, 0x00_u8, 0x01_u8],
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0007_D02R0101.bin"),
            [0xFF_u8, 0xFE_u8, 0x00_u8, 0x01_u8],
        )
        .unwrap();
        fs::write(
            root.join("files/fielddata/script/scr_seq/scr_seq_0007_D02R0101.s"),
            "apply_movement obj_D02R0101_gsrivel, _00D0\n",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.resolve_constant("msg_0139_D49R0102_00000"), Some(0));
        assert_eq!(ws.resolve_constant("msg_0139_D49R0102_00004"), Some(4));
        assert_eq!(ws.resolve_constant("obj_D02R0101_player"), Some(7));
        assert_eq!(ws.resolve_constant("obj_D02R0101_gsrivel"), Some(0));
    }

    #[test]
    fn test_open_decomp_hgss_detection_does_not_depend_on_root_name() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("totally_generic_project");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("files/fielddata/script/scr_seq")).unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::HeartGold);
        assert_eq!(ws.family, GameFamily::HGSS);
    }

    #[test]
    fn test_open_decomp_platinum_detection_does_not_flip_from_hgss_like_root_name() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("pokeheartgold");

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("res/field/scripts")).unwrap();
        fs::write(
            root.join("res/field/scripts/scripts.order"),
            "script_main\n",
        )
        .unwrap();

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
        assert_eq!(ws.scripts.get_name(0), Some("script_main"));
    }

    #[test]
    fn test_open_hg_engine_detection() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("armips")).unwrap();
        fs::write(root.join("narcs.mk"), "").unwrap();
        write_test_header_bin(&root.join("rom.nds"), "POKEMON HG", "IPKE");

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.project_type, ProjectType::HgEngine);
        assert_eq!(ws.game, Game::HeartGold);
        assert_eq!(ws.family, GameFamily::HGSS);
    }

    #[test]
    fn test_open_hg_engine_detection_wins_over_decomp() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Both HgEngine and Decomp markers present — HgEngine must win.
        fs::create_dir_all(root.join("armips")).unwrap();
        fs::write(root.join("narcs.mk"), "").unwrap();
        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::write(root.join("include/constants/test.h"), "#define X 1\n").unwrap();
        write_test_header_bin(&root.join("rom.nds"), "POKEMON HG", "IPKE");

        let ws = Workspace::open(root).unwrap();

        assert_eq!(ws.project_type, ProjectType::HgEngine);
    }

    #[test]
    fn test_open_hg_engine_missing_rom_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("armips")).unwrap();
        fs::write(root.join("narcs.mk"), "").unwrap();

        let result = Workspace::open(root);
        match result {
            Err(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
                assert!(err.to_string().contains("rom.nds"));
            }
            Ok(_) => panic!("expected NotFound error"),
        }
    }

    #[test]
    fn test_open_hg_engine_invalid_rom_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("armips")).unwrap();
        fs::write(root.join("narcs.mk"), "").unwrap();
        // Write a file that is too short to contain a valid NDS header.
        fs::write(root.join("rom.nds"), b"too short").unwrap();

        let result = Workspace::open(root);
        match result {
            Err(err) => assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof),
            Ok(_) => panic!("expected UnexpectedEof error"),
        }
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_open_platinum_decomp_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "workspace Platinum decomp integration test",
        ) else {
            return;
        };

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.game, Game::Platinum);
        assert_eq!(ws.family, GameFamily::Platinum);
        assert_eq!(ws.provider.get_map_header_count().unwrap(), 559);

        let scripts_order = root.join("res/field/scripts/scripts.order");
        if scripts_order.exists() {
            let expected_first = fs::read_to_string(&scripts_order)
                .unwrap()
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .expect("scripts.order exists but contains no script names");

            assert_eq!(ws.scripts.get_name(0), Some(expected_first.as_str()));
            assert_eq!(ws.scripts.get_id(&expected_first), Some(0));
        }

        let map_count = ws.provider.get_map_header_count().unwrap();
        let sample_map_ids = [0_u16, 5, 36, 200, (map_count.saturating_sub(1)) as u16];
        for map_id in sample_map_ids {
            let header = ws.provider.get_map_header(map_id).unwrap();
            let expected_name = ws
                .scripts
                .get_name(header.script_file_id() as usize)
                .map(str::to_string);
            assert_eq!(
                ws.get_script_file_for_map(map_id),
                expected_name,
                "workspace script lookup mismatch for map {}",
                map_id
            );
        }

        if root.join("generated/maps.txt").exists() {
            assert!(
                ws.get_map_internal_name(0).is_some(),
                "generated/maps.txt exists but map internal name[0] is missing"
            );
        }

        if root.join("res/text/location_names.json").exists() {
            let location_id = match ws.provider.get_map_header(0).unwrap() {
                MapHeader::Pt(h) => h.location_name,
                _ => panic!("expected Platinum map header for Platinum fixture"),
            };
            assert!(
                ws.get_map_location_name(location_id).is_some(),
                "res/text/location_names.json exists but location_name[{}] is missing",
                location_id
            );
        }
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_open_hgss_decomp_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "workspace HGSS decomp integration test",
        ) else {
            return;
        };

        let ws = Workspace::open(&root).unwrap();

        assert_eq!(ws.project_type, ProjectType::Decomp);
        assert_eq!(ws.family, GameFamily::HGSS);
        assert_eq!(ws.provider.get_map_header_count().unwrap(), 540);

        let Some((script_id, script_name)) = expected_hgss_script_from_fixture(&root).unwrap()
        else {
            panic!(
                "expected at least one scr_seq_XXXX*.s script file under {}",
                root.display()
            );
        };

        assert_eq!(ws.scripts.get_name(script_id), Some(script_name.as_str()));
        assert_eq!(ws.scripts.get_id(&script_name), Some(script_id));

        let map_count = ws.provider.get_map_header_count().unwrap();
        let sample_map_ids = [0_u16, 5, 36, 200, (map_count.saturating_sub(1)) as u16];
        for map_id in sample_map_ids {
            let header = ws.provider.get_map_header(map_id).unwrap();
            let expected_name = ws
                .scripts
                .get_name(header.script_file_id() as usize)
                .map(str::to_string);
            assert_eq!(
                ws.get_script_file_for_map(map_id),
                expected_name,
                "workspace script lookup mismatch for map {}",
                map_id
            );
        }

        if root.join("generated/maps.txt").exists() {
            assert!(
                ws.get_map_internal_name(0).is_some(),
                "generated/maps.txt exists but map internal name[0] is missing"
            );
        }

        if root.join("res/text/location_names.json").exists() {
            let location_id = match ws.provider.get_map_header(0).unwrap() {
                MapHeader::HGSS(h) => h.location_name,
                _ => panic!("expected HGSS map header for HGSS fixture"),
            };
            assert!(
                ws.get_map_location_name(location_id).is_some(),
                "res/text/location_names.json exists but location_name[{}] is missing",
                location_id
            );
        }
    }

    #[test]
    fn test_open_decomp_missing_script_manager_uses_hardcoded_global_script_table() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();

        let ws = Workspace::open(root).unwrap();
        let hardcoded = GlobalScriptTable::platinum_hardcoded();

        assert_eq!(ws.global_script_table.len(), hardcoded.len());
        assert_eq!(ws.global_script_table.lookup(2000), hardcoded.lookup(2000));
    }

    #[test]
    fn test_open_decomp_invalid_script_manager_returns_invalid_data() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("include/constants")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();

        let mut script_manager = fs::File::create(root.join("src/script_manager.c")).unwrap();
        writeln!(script_manager, "not a SCRIPT_RANGE_TABLE file").unwrap();

        match Workspace::open(root) {
            Ok(_) => panic!("Expected open to fail for invalid script_manager.c"),
            Err(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
                assert!(err.to_string().contains("SCRIPT_RANGE_TABLE"));
            }
        }
    }

    struct MockProvider;

    struct HgssScriptProvider {
        script_file_id: u16,
    }

    impl DataProvider for MockProvider {
        fn get_map_header(&self, _id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
            Err(crate::error::UxieError::not_found("Map header", "mock"))
        }

        fn get_map_header_count(&self) -> crate::error::Result<usize> {
            Ok(0)
        }

        fn get_text_archive_for_script_file(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_map_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }

        fn find_map_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }
    }

    impl DataProvider for HgssScriptProvider {
        fn get_map_header(&self, id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
            if id != 0 {
                return Err(crate::error::UxieError::not_found(
                    "Map header",
                    id.to_string(),
                ));
            }

            let header = crate::map_header::MapHeaderHGSS {
                script_file_id: self.script_file_id,
                ..crate::map_header::MapHeaderHGSS::default()
            };

            Ok(crate::map_header::MapHeader::HGSS(header))
        }

        fn get_map_header_count(&self) -> crate::error::Result<usize> {
            Ok(1)
        }

        fn get_text_archive_for_script_file(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_script_file_id(
            &self,
            script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            if script_file_id == self.script_file_id {
                Ok(vec![0])
            } else {
                Ok(Vec::new())
            }
        }

        fn find_maps_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }
    }

    /// Returns archive ID = script_file_id for any input.
    struct DspreArchiveProvider;

    impl DataProvider for DspreArchiveProvider {
        fn get_map_header(&self, id: u16) -> crate::error::Result<crate::map_header::MapHeader> {
            Err(crate::error::UxieError::not_found(
                "Map header",
                id.to_string(),
            ))
        }

        fn get_map_header_count(&self) -> crate::error::Result<usize> {
            Ok(0)
        }

        fn get_text_archive_for_script_file(
            &self,
            script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(Some(script_file_id))
        }

        fn find_map_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_script_file_id(
            &self,
            _script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }

        fn find_map_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Option<u16>> {
            Ok(None)
        }

        fn find_maps_by_level_script_file_id(
            &self,
            _level_script_file_id: u16,
        ) -> crate::error::Result<Vec<u16>> {
            Ok(Vec::new())
        }
    }

    #[test]
    #[ignore = "requires local HGSS DSPRE fixture via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_hgss_dspre_text_archive_for_script_file_0003() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            "HGSS DSPRE workspace integration test",
        ) else {
            return;
        };
        let ws = Workspace::open(&root).expect("open HGSS DSPRE workspace");
        assert_eq!(ws.project_type, ProjectType::Dspre);
        assert_eq!(ws.family, GameFamily::HGSS);
        let script_id = ws.scripts.get_id("0003");
        println!("scripts.get_id('0003') = {script_id:?}");
        let archive_id = ws.text_archive_for_script_file("0003");
        println!("text_archive_for_script_file('0003') = {archive_id:?}");
        let archive_id = archive_id.expect("expected Some archive ID for script 0003");
        let msg = ws.read_message(archive_id, 3);
        println!("read_message({archive_id}, 3) = {msg:?}");
        assert!(
            msg.is_some(),
            "expected a message at index 3 in archive {archive_id}"
        );
    }

    #[test]
    #[ignore = "requires local Platinum DSPRE fixture via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_platinum_dspre_text_archive_for_map_bound_script_0224() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "Platinum DSPRE workspace integration test",
        ) else {
            return;
        };
        let ws = Workspace::open(&root).expect("open Platinum DSPRE workspace");
        assert_eq!(ws.project_type, ProjectType::Dspre);
        assert_eq!(ws.family, GameFamily::Platinum);
        let header_count = ws.provider.get_map_header_count().unwrap_or(0);
        println!("map header count = {header_count}");
        let raw_provider = ws.provider.get_text_archive_for_script_file(224);
        println!("provider.get_text_archive_for_script_file(224) = {raw_provider:?}");
        let global = ws.global_script_table.find_by_script_file_id(224);
        println!("global_script_table.find_by_script_file_id(224) = {global:?}");
        // Also check a known-good script (211 = common strings)
        println!(
            "provider(211) = {:?}",
            ws.provider.get_text_archive_for_script_file(211)
        );
        println!(
            "global(211)   = {:?}",
            ws.global_script_table.find_by_script_file_id(211)
        );
        let archive_id = ws.text_archive_for_script_file("0224");
        println!("text_archive_for_script_file('0224') = {archive_id:?}");
        let archive_id = archive_id.expect("expected Some archive ID for map-bound script 0224");
        println!(
            "read_message({archive_id}, 0) = {:?}",
            ws.read_message(archive_id, 0)
        );
        assert!(
            ws.read_message(archive_id, 0).is_some(),
            "expected a message in archive {archive_id}"
        );
    }

    #[test]
    fn test_text_archive_for_script_file_dspre_numeric_name() {
        let sm = SourceManager::new();
        let ws = Workspace {
            project_path: PathBuf::from("/test"),
            project_type: ProjectType::Dspre,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(DspreArchiveProvider),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        };

        // DSPRE scripts are named by NARC index ("0213" → file_id 213).
        assert_eq!(ws.text_archive_for_script_file("0213"), Some(213));
        assert_eq!(ws.text_archive_for_script_file("0"), Some(0));
        // Non-numeric names that aren't in the ScriptTable must return None.
        assert_eq!(ws.text_archive_for_script_file("script_main"), None);
    }

    fn make_dspre_workspace(project_path: std::path::PathBuf) -> Workspace {
        let sm = SourceManager::new();
        Workspace {
            project_path,
            project_type: ProjectType::Dspre,
            language: crate::game::GameLanguage::English,
            game: Game::Platinum,
            family: GameFamily::Platinum,
            provider: Box::new(DspreArchiveProvider),
            symbols: Arc::new(SymbolTable::with_source_manager(sm.clone())),
            scripts: ScriptTable::new(),
            text_banks: TextBankTable::new(),
            game_strings: GameStrings::new(),
            global_script_table: GlobalScriptTable::new(),
            source_manager: sm,
            location_names: None,
            internal_names: None,
            text_archive_paths: DashMap::new(),
            message_cache: DashMap::new(),
            message_ids: Arc::new(DashMap::new()),
        }
    }

    #[test]
    fn find_or_add_message_deduplicates_and_appends() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        std::fs::write(
            archives_dir.join("0000.json"),
            r#"{"messages": [{"id": "msg_0000_00000", "en_US": "existing"}]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());

        // First call loads from disk and appends "new message" at index 1.
        let idx1 = ws.find_or_add_message(0, "new message").unwrap();
        assert_eq!(idx1, 1);

        // Second call for the same text must return the same index (dedup).
        let idx2 = ws.find_or_add_message(0, "new message").unwrap();
        assert_eq!(idx2, 1);

        // Third call for the existing on-disk text must return index 0.
        let idx0 = ws.find_or_add_message(0, "existing").unwrap();
        assert_eq!(idx0, 0);

        // A distinct second message gets the next slot.
        let idx3 = ws.find_or_add_message(0, "another").unwrap();
        assert_eq!(idx3, 2);
    }

    #[test]
    fn find_or_add_message_reuses_garbage_slots() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        // Archive with one real entry and one garbage (all-spaces) entry.
        std::fs::write(
            archives_dir.join("0002.json"),
            r#"{"messages": [{"id": "msg_0002_00000", "en_US": "real"}, {"id": "msg_0002_00001", "en_US": "   "}]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());

        // Asking for a new string should reuse the garbage slot (index 1) instead of appending.
        let idx = ws.find_or_add_message(2, "replacement").unwrap();
        assert_eq!(idx, 1);

        // The slot is now taken; a second new string gets appended.
        let idx2 = ws.find_or_add_message(2, "appended").unwrap();
        assert_eq!(idx2, 2);
    }

    #[test]
    fn flush_pending_messages_overwrites_garbage_slots() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        let archive_path = archives_dir.join("0003.json");
        std::fs::write(
            &archive_path,
            r#"{"messages": [{"id": "msg_0003_00000", "en_US": "real"}, {"id": "msg_0003_00001", "en_US": "   "}]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        ws.find_or_add_message(3, "replacement").unwrap(); // reuses slot 1
        ws.flush_pending_messages().unwrap();

        let content = std::fs::read_to_string(&archive_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        let messages = json["messages"].as_array().unwrap();
        // Still 2 entries — no new slot appended.
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["en_US"], "real");
        assert_eq!(messages[1]["en_US"], "replacement");
    }

    #[test]
    fn flush_pending_messages_overwrites_garbage_slots_non_primary_locale() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        let archive_path = archives_dir.join("0004.json");
        std::fs::write(
            &archive_path,
            r#"{"messages": [{"id": "msg_0004_00000", "en_US": "real"}, {"id": "msg_0004_00001", "ja_JP": "   "}]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        ws.find_or_add_message(4, "replacement").unwrap();
        ws.flush_pending_messages().unwrap();

        let content = std::fs::read_to_string(&archive_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["en_US"], "real");
        assert_eq!(messages[1]["en_US"], "replacement");
    }

    #[test]
    fn flush_pending_messages_writes_new_entries_to_json() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        let archive_path = archives_dir.join("0005.json");
        std::fs::write(
            &archive_path,
            r#"{"messages": [{"id": "msg_0005_00000", "en_US": "seed"}]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        ws.find_or_add_message(5, "seed").unwrap(); // existing — no new entry
        ws.find_or_add_message(5, "hello").unwrap();
        ws.find_or_add_message(5, "world").unwrap();

        ws.flush_pending_messages().unwrap();

        let content = std::fs::read_to_string(&archive_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["en_US"], "seed");
        assert_eq!(messages[1]["en_US"], "hello");
        assert_eq!(messages[2]["en_US"], "world");
        // IDs should be zero-padded with the archive stem.
        assert_eq!(messages[1]["id"], "msg_0005_00001");
        assert_eq!(messages[2]["id"], "msg_0005_00002");
    }

    #[test]
    fn read_message_lazy_populates_cache() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        std::fs::write(
            archives_dir.join("0199.json"),
            r#"{"messages": [
                {"id": "msg_0199_00000", "en_US": "Hello"},
                {"id": "msg_0199_00001", "en_US": "World"},
                {"id": "msg_0199_00002", "en_US": "Foo"}
            ]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        assert!(!ws.message_cache.contains_key(&199));

        let text = ws.read_message(199, 1).unwrap();
        assert_eq!(text, "World");

        assert!(ws.message_cache.contains_key(&199));
        assert_eq!(ws.message_cache.get(&199).unwrap().len(), 3);
    }

    #[test]
    fn read_message_returns_none_for_garbage_slot() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        std::fs::write(
            archives_dir.join("0100.json"),
            r#"{"messages": [
                {"id": "msg_0100_00000", "en_US": "real text"},
                {"id": "msg_0100_00001", "en_US": "   "}
            ]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        assert_eq!(ws.read_message(100, 0), Some("real text".to_string()));
        assert_eq!(ws.read_message(100, 1), None);
    }

    #[test]
    fn load_archive_populates_message_ids() {
        let dir = tempfile::tempdir().unwrap();
        let archives_dir = dir.path().join("expanded/textArchives");
        std::fs::create_dir_all(&archives_dir).unwrap();
        std::fs::write(
            archives_dir.join("0199.json"),
            r#"{"messages": [
                {"id": "msg_0199_00000", "en_US": "Hello"},
                {"id": "msg_0199_00001", "en_US": "World"}
            ]}"#,
        )
        .unwrap();

        let ws = make_dspre_workspace(dir.path().to_path_buf());
        ws.ensure_archive_loaded(199).unwrap();

        assert_eq!(
            ws.message_ids.get("msg_0199_00001").map(|e| *e.value()),
            Some((199, 1))
        );
        assert_eq!(ws.message_cache.get(&199).unwrap().len(), 2);
    }
}
