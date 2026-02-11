//! Global script ID to script file mapping table.
//!
//!
//! Maps global script IDs (used by `CallCommonScript` and similar commands)
//! to their corresponding script file and text bank indices.
//!
//! ## Overview
//!
//! In Gen 4 games, scripts can be called by global IDs that span multiple files.
//! Each ID range maps to a specific script file (NARC index) and text bank.
//! For example, in Platinum:
//! - IDs 2000-2499 → `scripts_common` (file 211)
//! - IDs 3000-4999 → single battle scripts (file 1114)
//! - IDs 5000-6999 → double battle scripts (file 1114)
//!
//! ## Loading
//!
//! The table can be loaded from:
//! - **HGSS binary**: Read from arm9.bin (pointer at 0x40164)
//! - **HGSS decomp**: Parse `src/fieldmap.c` `sScriptBankMapping` table
//! - **Platinum decomp**: Parse `src/script_manager.c` `SCRIPT_RANGE_TABLE` macro
//! - **Platinum binary**: Hardcoded (no clean table in binary)

use crate::c_parser::SymbolTable;
use crate::script_file::COMMON_SCRIPT_THRESHOLD;
use byteorder::{LittleEndian, ReadBytesExt};
use regex::Regex;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::LazyLock;

/// HGSS arm9.bin offset where the table pointer is stored
const HGSS_TABLE_POINTER_OFFSET: u64 = 0x40164;

/// HGSS table entry count
const HGSS_TABLE_ENTRY_COUNT: usize = 30;

/// HGSS memory base address (subtracted to get arm9 offset)
const HGSS_MEMORY_BASE: u32 = 0x02000000;

/// Platinum table entry count
#[cfg(test)]
const PLATINUM_TABLE_ENTRY_COUNT: usize = 30;

/// Regex to match HGSS sScriptBankMapping array entries:
/// `{ _std_scratch_card, NARC_scr_seq_scr_seq_0263_bin, NARC_msg_msg_0433_bin },`
static RE_HGSS_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*\}").unwrap()
});

/// Regex to match SCRIPT_RANGE_TABLE macro entries:
/// `Entry(10490, scripts_unk_0499, TEXT_BANK_SCRATCH_OFF_CARDS) \`
static RE_PLATINUM_TABLE_ENTRY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Entry\s*\(\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*\)")
        .unwrap()
});

fn resolve_value(s: &str, symbols: &SymbolTable) -> Option<i64> {
    if let Ok(v) = s.parse::<i64>() {
        return Some(v);
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        if let Ok(v) = i64::from_str_radix(&s[2..], 16) {
            return Some(v);
        }
    }
    symbols.resolve_constant(s)
}

/// Entry mapping a global script ID range to a script file.
///
/// Scripts with IDs >= `min_script_id` (and < the next entry's min_script_id)
/// are loaded from `script_file_id` with text from `text_archive_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalScriptEntry {
    /// Minimum global script ID for this range (inclusive)
    pub min_script_id: u16,
    /// Script file NARC index (in fielddata/script/scr_seq)
    pub script_file_id: u16,
    /// Text archive NARC index (in msgdata/msg)
    pub text_archive_id: u16,
}

impl GlobalScriptEntry {
    /// Create a new entry
    pub const fn new(min_script_id: u16, script_file_id: u16, text_archive_id: u16) -> Self {
        Self {
            min_script_id,
            script_file_id,
            text_archive_id,
        }
    }

    /// Read an entry from a reader (6 bytes, little-endian)
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let min_script_id = reader.read_u16::<LittleEndian>()?;
        let script_file_id = reader.read_u16::<LittleEndian>()?;
        let text_archive_id = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            min_script_id,
            script_file_id,
            text_archive_id,
        })
    }
}

/// Table mapping global script IDs to script files and text banks.
///
/// Entries are stored in descending order by `min_script_id` for efficient lookup.
#[derive(Debug, Clone, Default)]
pub struct GlobalScriptTable {
    /// Entries sorted by min_script_id descending
    entries: Vec<GlobalScriptEntry>,
}

impl GlobalScriptTable {
    /// Create an empty table
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a table from a list of entries.
    ///
    /// Entries will be sorted by min_script_id descending.
    pub fn from_entries(mut entries: Vec<GlobalScriptEntry>) -> Self {
        entries.sort_by(|a, b| b.min_script_id.cmp(&a.min_script_id));
        Self { entries }
    }

    /// Load from HGSS arm9.bin binary.
    ///
    /// Reads the table pointer from offset 0x40164, then reads 30 entries
    /// from that address.
    pub fn from_hgss_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        reader.seek(SeekFrom::Start(HGSS_TABLE_POINTER_OFFSET))?;
        let table_addr = reader.read_u32::<LittleEndian>()?;
        let table_offset = table_addr.saturating_sub(HGSS_MEMORY_BASE) as u64;

        reader.seek(SeekFrom::Start(table_offset))?;
        let mut entries = Vec::with_capacity(HGSS_TABLE_ENTRY_COUNT);
        for _ in 0..HGSS_TABLE_ENTRY_COUNT {
            entries.push(GlobalScriptEntry::read_from(reader)?);
        }

        Ok(Self::from_entries(entries))
    }

    /// Load from HGSS arm9.bin file path.
    pub fn from_hgss_binary_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let data = std::fs::read(path)?;
        let mut cursor = Cursor::new(data);
        Self::from_hgss_binary(&mut cursor)
    }

    /// Parse HGSS `sScriptBankMapping` table from fieldmap.c source.
    ///
    /// Requires a `SymbolTable` to resolve symbolic constants like
    /// `_std_scratch_card` and `NARC_scr_seq_scr_seq_0263_bin`.
    pub fn from_hgss_decomp(content: &str, symbols: &SymbolTable) -> Option<Self> {
        let start = content.find("sScriptBankMapping")?;
        let block_start = content[start..].find('{')?;
        let block = &content[start + block_start..];

        let mut entries = Vec::new();
        for caps in RE_HGSS_ENTRY.captures_iter(block) {
            let script_id_sym = caps.get(1)?.as_str();
            let script_file_sym = caps.get(2)?.as_str();
            let text_archive_sym = caps.get(3)?.as_str();

            let min_script_id = symbols.resolve_constant(script_id_sym)? as u16;
            let script_file_id = symbols.resolve_constant(script_file_sym)? as u16;
            let text_archive_id = symbols.resolve_constant(text_archive_sym)? as u16;

            entries.push(GlobalScriptEntry::new(
                min_script_id,
                script_file_id,
                text_archive_id,
            ));
        }

        if entries.is_empty() {
            return None;
        }

        Some(Self::from_entries(entries))
    }

    /// Parse HGSS `sScriptBankMapping` from fieldmap.c file path.
    pub fn from_hgss_decomp_file(
        path: impl AsRef<Path>,
        symbols: &SymbolTable,
    ) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_hgss_decomp(&content, symbols).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to parse sScriptBankMapping",
            )
        })
    }

    /// Parse Platinum `SCRIPT_RANGE_TABLE` macro from script_manager.c source.
    ///
    /// Parses the macro table format:
    /// ```c
    /// #define SCRIPT_RANGE_TABLE(Entry) \
    ///     Entry(10490, scripts_unk_0499, TEXT_BANK_SCRATCH_OFF_CARDS) \
    ///     Entry(10450, scripts_unk_0500, TEXT_BANK_UNK_0016) \
    ///     ...
    /// ```
    pub fn from_platinum_decomp(content: &str, symbols: &SymbolTable) -> Option<Self> {
        // Find the SCRIPT_RANGE_TABLE macro definition
        let start = content.find("SCRIPT_RANGE_TABLE")?;
        let block_start = content[start..].find('(')?;
        let block = &content[start + block_start..];

        let mut entries = Vec::new();
        for caps in RE_PLATINUM_TABLE_ENTRY.captures_iter(block) {
            let script_id_sym = caps.get(1)?.as_str();
            let script_file_sym = caps.get(2)?.as_str();
            let text_archive_sym = caps.get(3)?.as_str();

            let min_script_id = resolve_value(script_id_sym, symbols)? as u16;
            let script_file_id = resolve_value(script_file_sym, symbols)? as u16;
            let text_archive_id = resolve_value(text_archive_sym, symbols)? as u16;

            entries.push(GlobalScriptEntry::new(
                min_script_id,
                script_file_id,
                text_archive_id,
            ));
        }

        if entries.is_empty() {
            return None;
        }

        Some(Self::from_entries(entries))
    }

    /// Parse Platinum `SCRIPT_RANGE_TABLE` macro from script_manager.c file path.
    pub fn from_platinum_decomp_file(
        path: impl AsRef<Path>,
        symbols: &SymbolTable,
    ) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_platinum_decomp(&content, symbols).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to parse SCRIPT_RANGE_TABLE macro",
            )
        })
    }

    /// Get hardcoded Platinum table.
    ///
    /// Extracted from pokeplatinum `src/script_manager.c` `ScriptContext_LoadAndOffsetID`.
    /// Used for DSPRE projects without decomp source access.
    pub fn platinum_hardcoded() -> Self {
        let entries = vec![
            GlobalScriptEntry::new(10490, 499, 0x21D), // scripts_unk_0499
            GlobalScriptEntry::new(10450, 500, 0x010), // scripts_unk_0500
            GlobalScriptEntry::new(10400, 419, 0x0CB), // scripts_pokemon_center_daily_trainers
            GlobalScriptEntry::new(10300, 1051, 0x17B), // scripts_unk_1051
            GlobalScriptEntry::new(10200, 407, 0x17B), // scripts_unk_0407
            GlobalScriptEntry::new(10150, 460, 0x26D), // scripts_tv_reporter_interviews
            GlobalScriptEntry::new(10100, 459, 0x26E), // scripts_tv_broadcast
            GlobalScriptEntry::new(10000, 410, 0x17D), // scripts_field_moves
            GlobalScriptEntry::new(9950, 412, 0x17D),  // scripts_pokedex_ratings
            GlobalScriptEntry::new(9900, 397, 0x0D5),  // scripts_unk_0397
            GlobalScriptEntry::new(9800, 212, 0x0D9),  // scripts_unk_0212
            GlobalScriptEntry::new(9700, 423, 0x1AD),  // scripts_follower_partners
            GlobalScriptEntry::new(9600, 413, 0x0D5),  // scripts_init_new_game
            GlobalScriptEntry::new(9500, 501, 0x223),  // scripts_unk_0501
            GlobalScriptEntry::new(9400, 426, 0x1B0),  // scripts_unk_0426
            GlobalScriptEntry::new(9300, 406, 0x176),  // scripts_unk_0406
            GlobalScriptEntry::new(9200, 422, 0x1AE),  // scripts_unk_0423
            GlobalScriptEntry::new(9100, 0, 0x00B),    // scripts_unk_0000
            GlobalScriptEntry::new(9000, 213, 0x0DD),  // scripts_unk_0213
            GlobalScriptEntry::new(8970, 425, 0x007),  // scripts_unk_0425
            GlobalScriptEntry::new(8950, 498, 0x21B),  // scripts_unk_0498
            GlobalScriptEntry::new(8900, 424, 0x1AF),  // scripts_unk_0424
            GlobalScriptEntry::new(8800, 405, 0x175),  // scripts_safari_game
            GlobalScriptEntry::new(8000, 408, 0x17C),  // scripts_unk_0408 (hidden items)
            GlobalScriptEntry::new(7000, 404, 0x171),  // scripts_unk_0404
            GlobalScriptEntry::new(5000, 1114, 0x0D5), // scripts_unk_1114 (double battles)
            GlobalScriptEntry::new(3000, 1114, 0x0D5), // scripts_unk_1114 (single battles)
            GlobalScriptEntry::new(2800, 414, 0x18D),  // scripts_berry_tree_interaction
            GlobalScriptEntry::new(2500, 1, 0x011),    // scripts_unk_0001
            GlobalScriptEntry::new(2000, 211, 0x0D5),  // scripts_common
        ];

        Self::from_entries(entries)
    }

    /// Look up the entry for a global script ID.
    ///
    /// Returns the entry where `script_id >= entry.min_script_id`.
    pub fn lookup(&self, script_id: u16) -> Option<&GlobalScriptEntry> {
        self.entries.iter().find(|e| script_id >= e.min_script_id)
    }

    /// Check if a script ID is a global/common script (not a map script).
    ///
    /// Map scripts use IDs 1-1999, global scripts use 2000+.
    pub fn is_global_script(&self, script_id: u16) -> bool {
        script_id >= COMMON_SCRIPT_THRESHOLD
    }

    /// Get all entries in the table.
    pub fn entries(&self) -> &[GlobalScriptEntry] {
        &self.entries
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;
    use proptest::prelude::*;

    #[test]
    fn test_platinum_hardcoded_table() {
        let table = GlobalScriptTable::platinum_hardcoded();
        assert_eq!(table.len(), PLATINUM_TABLE_ENTRY_COUNT);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 211);

        let entry = table.lookup(2050).unwrap();
        assert_eq!(entry.min_script_id, 2000);

        let entry = table.lookup(3500).unwrap();
        assert_eq!(entry.min_script_id, 3000);
        assert_eq!(entry.script_file_id, 1114);

        let entry = table.lookup(5500).unwrap();
        assert_eq!(entry.min_script_id, 5000);
        assert_eq!(entry.script_file_id, 1114);
    }

    #[test]
    fn test_global_script_check() {
        let table = GlobalScriptTable::platinum_hardcoded();
        assert!(!table.is_global_script(1));
        assert!(!table.is_global_script(1999));
        assert!(table.is_global_script(2000));
        assert!(table.is_global_script(10000));
    }

    #[test]
    fn test_entry_read() {
        let data: [u8; 6] = [0xD0, 0x07, 0xD3, 0x00, 0xD5, 0x00];
        let mut cursor = Cursor::new(data);
        let entry = GlobalScriptEntry::read_from(&mut cursor).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 211);
        assert_eq!(entry.text_archive_id, 213);
    }

    #[test]
    fn test_hgss_decomp_parsing() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("_std_scratch_card".to_string(), 10500);
        symbols.insert_define("NARC_scr_seq_scr_seq_0263_bin".to_string(), 263);
        symbols.insert_define("NARC_msg_msg_0433_bin".to_string(), 0x1B1);
        symbols.insert_define("_std_misc".to_string(), 2000);
        symbols.insert_define("NARC_scr_seq_scr_seq_0003_bin".to_string(), 3);
        symbols.insert_define("NARC_msg_msg_0040_bin".to_string(), 0x28);

        let content = r#"
const struct ScriptBankMapping sScriptBankMapping[30] = {
    { _std_scratch_card, NARC_scr_seq_scr_seq_0263_bin, NARC_msg_msg_0433_bin },
    { _std_misc, NARC_scr_seq_scr_seq_0003_bin, NARC_msg_msg_0040_bin },
};
"#;

        let table = GlobalScriptTable::from_hgss_decomp(content, &symbols).unwrap();
        assert_eq!(table.len(), 2);

        let entry = table.lookup(10500).unwrap();
        assert_eq!(entry.min_script_id, 10500);
        assert_eq!(entry.script_file_id, 263);
        assert_eq!(entry.text_archive_id, 0x1B1);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 3);
    }

    #[test]
    fn test_platinum_decomp_parsing() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("scripts_unk_0499".to_string(), 499);
        symbols.insert_define("TEXT_BANK_SCRATCH_OFF_CARDS".to_string(), 0x21D);
        symbols.insert_define("scripts_common".to_string(), 211);
        symbols.insert_define("TEXT_BANK_COMMON_STRINGS".to_string(), 0x0D5);
        symbols.insert_define("SCRIPT_ID_OFFSET_COMMON_SCRIPTS".to_string(), 2000);

        let content = r#"
// clang-format off
#define SCRIPT_RANGE_TABLE(Entry) \
    Entry(10490,                                    scripts_unk_0499,                       TEXT_BANK_SCRATCH_OFF_CARDS) \
    Entry(SCRIPT_ID_OFFSET_COMMON_SCRIPTS,          scripts_common,                         TEXT_BANK_COMMON_STRINGS)
// clang-format on
"#;

        let table = GlobalScriptTable::from_platinum_decomp(content, &symbols).unwrap();
        assert_eq!(table.len(), 2);

        let entry = table.lookup(10490).unwrap();
        assert_eq!(entry.min_script_id, 10490);
        assert_eq!(entry.script_file_id, 499);
        assert_eq!(entry.text_archive_id, 0x21D);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 211);
        assert_eq!(entry.text_archive_id, 0x0D5);
    }

    #[test]
    #[ignore]
    fn test_hgss_binary_real_file() {
        let Some(path) = crate::test_env::existing_path_from_env_with_fallback(
            "UXIE_TEST_HGSS_ARM9_PATH",
            "HGSS_ARM9_PATH",
            "HGSS arm9 integration test",
        ) else {
            return;
        };

        let table = GlobalScriptTable::from_hgss_binary_file(path).unwrap();
        assert_eq!(table.len(), 30);

        let entry = table.lookup(10500).unwrap();
        assert!(entry.min_script_id >= 10000);

        let entry = table.lookup(2000).unwrap();
        assert!(entry.min_script_id <= 2000);
    }

    fn global_entries_strategy() -> impl Strategy<Value = Vec<GlobalScriptEntry>> {
        prop::collection::btree_map(any::<u16>(), (any::<u16>(), any::<u16>()), 0..48).prop_map(
            |mapping| {
                mapping
                    .into_iter()
                    .map(|(min_script_id, (script_file_id, text_archive_id))| {
                        GlobalScriptEntry::new(min_script_id, script_file_id, text_archive_id)
                    })
                    .collect()
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_from_entries_sorts_descending(entries in global_entries_strategy()) {
            let table = GlobalScriptTable::from_entries(entries.clone());
            let mins: Vec<u16> = table.entries().iter().map(|e| e.min_script_id).collect();
            prop_assert!(mins.windows(2).all(|w| w[0] >= w[1]));
            prop_assert_eq!(table.len(), entries.len());
        }

        #[test]
        fn prop_lookup_matches_manual_search(entries in global_entries_strategy(), script_id in any::<u16>()) {
            let mut expected_entries = entries.clone();
            expected_entries.sort_by(|a, b| b.min_script_id.cmp(&a.min_script_id));
            let expected = expected_entries
                .iter()
                .find(|e| script_id >= e.min_script_id)
                .copied();

            let table = GlobalScriptTable::from_entries(entries);
            let actual = table.lookup(script_id).copied();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_is_global_script_threshold(script_id in any::<u16>()) {
            let table = GlobalScriptTable::new();
            prop_assert_eq!(
                table.is_global_script(script_id),
                script_id >= COMMON_SCRIPT_THRESHOLD
            );
        }

        #[test]
        fn prop_entry_read_from_roundtrip(
            min_script_id in any::<u16>(),
            script_file_id in any::<u16>(),
            text_archive_id in any::<u16>()
        ) {
            let mut bytes = Vec::new();
            bytes.write_u16::<LittleEndian>(min_script_id).unwrap();
            bytes.write_u16::<LittleEndian>(script_file_id).unwrap();
            bytes.write_u16::<LittleEndian>(text_archive_id).unwrap();

            let mut cursor = Cursor::new(bytes);
            let parsed = GlobalScriptEntry::read_from(&mut cursor).unwrap();
            prop_assert_eq!(parsed.min_script_id, min_script_id);
            prop_assert_eq!(parsed.script_file_id, script_file_id);
            prop_assert_eq!(parsed.text_archive_id, text_archive_id);
        }
    }
}
