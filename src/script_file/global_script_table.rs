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
const HGSS_MEMORY_BASE: u32 = 0x0200_0000;

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
    Regex::new(r"\bEntry\s*\(\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*,\s*([A-Za-z0-9_]+)\s*\)")
        .unwrap()
});

fn parse_hgss_narc_member_id(s: &str) -> Option<i64> {
    let digits = s
        .strip_prefix("NARC_scr_seq_scr_seq_")
        .or_else(|| s.strip_prefix("NARC_msg_msg_"))?
        .strip_suffix("_bin")?;

    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    digits.parse::<i64>().ok()
}

fn resolve_value(s: &str, symbols: &SymbolTable) -> Option<i64> {
    if let Ok(v) = s.parse::<i64>() {
        return Some(v);
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        if let Ok(v) = i64::from_str_radix(&s[2..], 16) {
            return Some(v);
        }
    }
    symbols
        .resolve_constant(s)
        .or_else(|| parse_hgss_narc_member_id(s))
}

/// Semantic identity for a global script range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalScriptRange {
    ScratchOffCards,
    BattleFrontierRecords,
    PokemonCenterDailyTrainers,
    CounterpartTalk,
    MysteryGiftDeliveryman,
    MysteryGift,
    TvReporterInterviews,
    TvBroadcast,
    FieldMoves,
    PokedexRatings,
    CommonStrings9900,
    Contests,
    FollowerPartners,
    InitNewGame,
    DayCareCommon,
    PoffinCommon,
    GroupConnection,
    PokemonCenterB1fAttendants,
    CommunicationClub,
    PokemonCenter2fAttendants,
    PokeRadar,
    VsSeeker,
    RecordChatotCry,
    SafariGame,
    HiddenItems,
    VisibleItems,
    Trainer {
        double_battle_id: u8,
    },
    BerryTreeInteractions,
    BgEvents,
    CommonScripts,
    FrontierMoveTutor,
    BugContest,
    TrainerHouse,
    Pokeathlon,
    WiFiReception,
    Colosseum,
    CommunicationReception,
    ApricornTree,
    Bookshelves,
    /// for script ranges that are undocumented in the decomps
    ScriptBank(u16),
}

impl GlobalScriptRange {
    pub fn display_name(self) -> String {
        match self {
            Self::ScratchOffCards => "Scratch-Off Cards".into(),
            Self::BattleFrontierRecords => "Battle Frontier Records".into(),
            Self::PokemonCenterDailyTrainers => "Pokemon Center Daily Trainers".into(),
            Self::CounterpartTalk => "Counterpart Talk".into(),
            Self::MysteryGiftDeliveryman => "Mystery Gift Deliveryman".into(),
            Self::MysteryGift => "Mystery Gift".into(),
            Self::TvReporterInterviews => "TV Reporter Interviews".into(),
            Self::TvBroadcast => "TV Broadcast".into(),
            Self::FieldMoves => "Field Moves".into(),
            Self::PokedexRatings => "Pokedex Ratings".into(),
            Self::CommonStrings9900 => "Common Strings (9900)".into(),
            Self::Contests => "Contests".into(),
            Self::FollowerPartners => "Follower Partners".into(),
            Self::InitNewGame => "Init New Game".into(),
            Self::DayCareCommon => "Day Care Common".into(),
            Self::PoffinCommon => "Poffin Common".into(),
            Self::GroupConnection => "Group Connection".into(),
            Self::PokemonCenterB1fAttendants => "Pokemon Center B1F Attendants".into(),
            Self::CommunicationClub => "Communication Club".into(),
            Self::PokemonCenter2fAttendants => "Pokemon Center 2F Attendants".into(),
            Self::PokeRadar => "Poke Radar".into(),
            Self::VsSeeker => "Vs Seeker".into(),
            Self::RecordChatotCry => "Record Chatot Cry".into(),
            Self::SafariGame => "Safari Game".into(),
            Self::HiddenItems => "Hidden Items".into(),
            Self::VisibleItems => "Visible Items".into(),
            Self::Trainer {
                double_battle_id: 1,
            } => "Single Battles".into(),
            Self::Trainer {
                double_battle_id: 2,
            } => "Double Battles".into(),
            Self::Trainer { double_battle_id } => format!("Trainer Battles ({double_battle_id})"),
            Self::BerryTreeInteractions => "Berry Tree Interactions".into(),
            Self::BgEvents => "BG Events".into(),
            Self::CommonScripts => "Common Scripts".into(),
            Self::FrontierMoveTutor => "Frontier Move Tutor".into(),
            Self::BugContest => "Bug Contest".into(),
            Self::TrainerHouse => "Trainer House".into(),
            Self::Pokeathlon => "Pokeathlon".into(),
            Self::WiFiReception => "Wi-Fi Reception".into(),
            Self::Colosseum => "Colosseum".into(),
            Self::CommunicationReception => "Communication Reception".into(),
            Self::ApricornTree => "Apricorn Tree".into(),
            Self::Bookshelves => "Bookshelves".into(),
            Self::ScriptBank(min_script_id) => format!("Script Range {min_script_id}"),
        }
    }

    fn from_display_name(name: &str, min_script_id: u16) -> Self {
        match name {
            "Scratch-Off Cards" => Self::ScratchOffCards,
            "Battle Frontier Records" => Self::BattleFrontierRecords,
            "Pokemon Center Daily Trainers" => Self::PokemonCenterDailyTrainers,
            "Counterpart Talk" => Self::CounterpartTalk,
            "Mystery Gift Deliveryman" => Self::MysteryGiftDeliveryman,
            "Mystery Gift" => Self::MysteryGift,
            "TV Reporter Interviews" => Self::TvReporterInterviews,
            "TV Broadcast" => Self::TvBroadcast,
            "Field Moves" => Self::FieldMoves,
            "Pokedex Ratings" => Self::PokedexRatings,
            "Common Strings (9900)" => Self::CommonStrings9900,
            "Contests" => Self::Contests,
            "Follower Partners" => Self::FollowerPartners,
            "Init New Game" => Self::InitNewGame,
            "Day Care Common" => Self::DayCareCommon,
            "Poffin Common" => Self::PoffinCommon,
            "Group Connection" => Self::GroupConnection,
            "Pokemon Center B1F Attendants" => Self::PokemonCenterB1fAttendants,
            "Communication Club" => Self::CommunicationClub,
            "Pokemon Center 2F Attendants" => Self::PokemonCenter2fAttendants,
            "Poke Radar" => Self::PokeRadar,
            "Vs Seeker" => Self::VsSeeker,
            "Record Chatot Cry" => Self::RecordChatotCry,
            "Safari Game" => Self::SafariGame,
            "Hidden Items" => Self::HiddenItems,
            "Visible Items" => Self::VisibleItems,
            "Single Battles" => Self::Trainer {
                double_battle_id: 1,
            },
            "Double Battles" => Self::Trainer {
                double_battle_id: 2,
            },
            "Berry Tree Interactions" => Self::BerryTreeInteractions,
            "BG Events" => Self::BgEvents,
            "Common Scripts" => Self::CommonScripts,
            "Frontier Move Tutor" => Self::FrontierMoveTutor,
            "Bug Contest" => Self::BugContest,
            "Trainer House" => Self::TrainerHouse,
            "Pokeathlon" => Self::Pokeathlon,
            "Wi-Fi Reception" => Self::WiFiReception,
            "Colosseum" => Self::Colosseum,
            "Communication Reception" => Self::CommunicationReception,
            "Apricorn Tree" => Self::ApricornTree,
            "Bookshelves" => Self::Bookshelves,
            _ => Self::ScriptBank(min_script_id),
        }
    }
}

/// Entry mapping a global script ID range to a script file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalScriptEntry {
    /// Minimum global script ID for this range (inclusive)
    pub min_script_id: u16,
    /// Script file NARC index (in fielddata/script/scr_seq)
    pub script_file_id: u16,
    /// Text archive NARC index (in msgdata/msg)
    pub text_archive_id: u16,
    /// Semantic range identity and display label.
    pub range: GlobalScriptRange,
}

impl GlobalScriptEntry {
    /// Create a new entry with a range name.
    pub fn new(
        min_script_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
        name: impl AsRef<str>,
    ) -> Self {
        Self {
            min_script_id,
            script_file_id,
            text_archive_id,
            range: GlobalScriptRange::from_display_name(name.as_ref(), min_script_id),
        }
    }

    /// Create a new entry with an explicit semantic range.
    pub fn with_range(
        min_script_id: u16,
        script_file_id: u16,
        text_archive_id: u16,
        range: GlobalScriptRange,
    ) -> Self {
        Self {
            min_script_id,
            script_file_id,
            text_archive_id,
            range,
        }
    }

    /// Read an entry from HGSS arm9 binary (6 bytes, little-endian).
    pub fn read_from_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let min_script_id = reader.read_u16::<LittleEndian>()?;
        let script_file_id = reader.read_u16::<LittleEndian>()?;
        let text_archive_id = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            min_script_id,
            script_file_id,
            text_archive_id,
            range: GlobalScriptRange::ScriptBank(min_script_id),
        })
    }
}

fn entries_from_ids(
    ranges: &[GlobalScriptRange],
    ids: &[(u16, u16, u16)],
) -> Vec<GlobalScriptEntry> {
    assert_eq!(ranges.len(), ids.len());
    ranges
        .iter()
        .zip(ids.iter())
        .map(|(range, (min, script, text))| {
            GlobalScriptEntry::with_range(*min, *script, *text, *range)
        })
        .collect()
}

// These range identities are intentionally hand-maintained labels for decomp
// script-bank rows. Refresh them if pokeplatinum/pokeheartgold grow official
// names for `SCRIPT_RANGE_TABLE` / `sScriptBankMapping`.
const PLATINUM_RANGES: &[GlobalScriptRange] = &[
    GlobalScriptRange::ScratchOffCards,
    GlobalScriptRange::BattleFrontierRecords,
    GlobalScriptRange::PokemonCenterDailyTrainers,
    GlobalScriptRange::CounterpartTalk,
    GlobalScriptRange::MysteryGiftDeliveryman,
    GlobalScriptRange::TvReporterInterviews,
    GlobalScriptRange::TvBroadcast,
    GlobalScriptRange::FieldMoves,
    GlobalScriptRange::PokedexRatings,
    GlobalScriptRange::CommonStrings9900,
    GlobalScriptRange::Contests,
    GlobalScriptRange::FollowerPartners,
    GlobalScriptRange::InitNewGame,
    GlobalScriptRange::DayCareCommon,
    GlobalScriptRange::PoffinCommon,
    GlobalScriptRange::GroupConnection,
    GlobalScriptRange::PokemonCenterB1fAttendants,
    GlobalScriptRange::CommunicationClub,
    GlobalScriptRange::PokemonCenter2fAttendants,
    GlobalScriptRange::PokeRadar,
    GlobalScriptRange::VsSeeker,
    GlobalScriptRange::RecordChatotCry,
    GlobalScriptRange::SafariGame,
    GlobalScriptRange::HiddenItems,
    GlobalScriptRange::VisibleItems,
    GlobalScriptRange::Trainer {
        double_battle_id: 2,
    },
    GlobalScriptRange::Trainer {
        double_battle_id: 1,
    },
    GlobalScriptRange::BerryTreeInteractions,
    GlobalScriptRange::BgEvents,
    GlobalScriptRange::CommonScripts,
];

const HGSS_RANGES: &[GlobalScriptRange] = &[
    GlobalScriptRange::ScratchOffCards,
    GlobalScriptRange::BattleFrontierRecords,
    GlobalScriptRange::FrontierMoveTutor,
    GlobalScriptRange::BugContest,
    GlobalScriptRange::TrainerHouse,
    GlobalScriptRange::ScriptBank(10300),
    GlobalScriptRange::MysteryGift,
    GlobalScriptRange::ScriptBank(10150),
    GlobalScriptRange::TvBroadcast,
    GlobalScriptRange::FieldMoves,
    GlobalScriptRange::PokedexRatings,
    GlobalScriptRange::CommonStrings9900,
    GlobalScriptRange::Pokeathlon,
    GlobalScriptRange::ScriptBank(9800),
    GlobalScriptRange::FollowerPartners,
    GlobalScriptRange::InitNewGame,
    GlobalScriptRange::DayCareCommon,
    GlobalScriptRange::GroupConnection,
    GlobalScriptRange::WiFiReception,
    GlobalScriptRange::Colosseum,
    GlobalScriptRange::CommunicationReception,
    GlobalScriptRange::RecordChatotCry,
    GlobalScriptRange::SafariGame,
    GlobalScriptRange::HiddenItems,
    GlobalScriptRange::VisibleItems,
    GlobalScriptRange::Trainer {
        double_battle_id: 2,
    },
    GlobalScriptRange::Trainer {
        double_battle_id: 1,
    },
    GlobalScriptRange::ApricornTree,
    GlobalScriptRange::Bookshelves,
    GlobalScriptRange::CommonScripts,
];

const DP_RANGES: &[GlobalScriptRange] = &[
    GlobalScriptRange::CounterpartTalk,
    GlobalScriptRange::MysteryGiftDeliveryman,
    GlobalScriptRange::TvReporterInterviews,
    GlobalScriptRange::TvBroadcast,
    GlobalScriptRange::FieldMoves,
    GlobalScriptRange::PokedexRatings,
    GlobalScriptRange::CommonStrings9900,
    GlobalScriptRange::Contests,
    GlobalScriptRange::FollowerPartners,
    GlobalScriptRange::DayCareCommon,
    GlobalScriptRange::PoffinCommon,
    GlobalScriptRange::PokemonCenterB1fAttendants,
    GlobalScriptRange::CommunicationClub,
    GlobalScriptRange::PokemonCenter2fAttendants,
    GlobalScriptRange::PokeRadar,
    GlobalScriptRange::VsSeeker,
    GlobalScriptRange::RecordChatotCry,
    GlobalScriptRange::SafariGame,
    GlobalScriptRange::HiddenItems,
    GlobalScriptRange::VisibleItems,
    GlobalScriptRange::Trainer {
        double_battle_id: 2,
    },
    GlobalScriptRange::Trainer {
        double_battle_id: 1,
    },
    GlobalScriptRange::BerryTreeInteractions,
    GlobalScriptRange::BgEvents,
    GlobalScriptRange::CommonScripts,
];

const DP_WESTERN_SCRIPT_IDS: &[(u16, u16, u16)] = &[
    (10300, 977, 496),
    (10200, 373, 332),
    (10150, 1042, 562),
    (10100, 1041, 563),
    (10000, 375, 334),
    (9950, 376, 335),
    (9900, 365, 199),
    (9800, 206, 203),
    (9700, 387, 378),
    (9500, 464, 464),
    (9400, 391, 381),
    (9200, 388, 379),
    (9100, 0, 9),
    (9000, 207, 207),
    (8970, 390, 7),
    (8950, 463, 463),
    (8900, 389, 380),
    (8800, 462, 462),
    (8000, 374, 333),
    (7000, 370, 325),
    (5000, 1040, 199),
    (3000, 1040, 199),
    (2800, 378, 350),
    (2500, 1, 13),
    (2000, 205, 199),
];

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
        entries.sort_by_key(|b| std::cmp::Reverse(b.min_script_id));
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
        for index in 0..HGSS_TABLE_ENTRY_COUNT {
            let mut entry = GlobalScriptEntry::read_from_binary(reader)?;
            entry.range = HGSS_RANGES
                .get(index)
                .copied()
                .unwrap_or(GlobalScriptRange::ScriptBank(entry.min_script_id));
            entries.push(entry);
        }

        Ok(Self::from_entries(entries))
    }

    /// Load from HGSS arm9.bin file path.
    pub fn from_hgss_binary_file(path: impl AsRef<Path>) -> io::Result<Self> {
        let data = std::fs::read(path)?;
        let mut cursor = Cursor::new(data);
        Self::from_hgss_binary(&mut cursor)
    }

    /// Load from HGSS arm9.bin using the correct pointer offset for the given game and language.
    ///
    /// Pointer offsets vary by locale and (for Spanish/Korean) by title.
    /// Falls back gracefully if the binary is unreadable (e.g. unexpected layout).
    pub fn from_hgss_binary_file_for_language(
        path: impl AsRef<Path>,
        game: crate::game::Game,
        language: crate::game::GameLanguage,
    ) -> io::Result<Self> {
        use crate::game::{Game, GameLanguage};
        let pointer_offset: u64 = match (game, language) {
            (Game::HeartGold | Game::SoulSilver, GameLanguage::Japanese) => 0x3FEB0,
            (Game::HeartGold, GameLanguage::Korean) => 0x403FC,
            (Game::SoulSilver, GameLanguage::Korean) => 0x403F4,
            (Game::HeartGold, GameLanguage::Spanish) => 0x4015C,
            _ => HGSS_TABLE_POINTER_OFFSET,
        };

        let data = std::fs::read(path)?;
        let mut cursor = Cursor::new(data);
        cursor.seek(SeekFrom::Start(pointer_offset))?;
        let table_addr = cursor.read_u32::<LittleEndian>()?;
        let table_offset = table_addr.saturating_sub(HGSS_MEMORY_BASE) as u64;
        cursor.seek(SeekFrom::Start(table_offset))?;
        let mut entries = Vec::with_capacity(HGSS_TABLE_ENTRY_COUNT);
        for index in 0..HGSS_TABLE_ENTRY_COUNT {
            let mut entry = GlobalScriptEntry::read_from_binary(&mut cursor)?;
            entry.range = HGSS_RANGES
                .get(index)
                .copied()
                .unwrap_or(GlobalScriptRange::ScriptBank(entry.min_script_id));
            entries.push(entry);
        }
        Ok(Self::from_entries(entries))
    }

    /// Parse HGSS `sScriptBankMapping` table from fieldmap.c source.
    ///
    /// Requires a `SymbolTable` to resolve symbolic constants like
    /// `_std_scratch_card`. Generated HGSS `NARC_*_XXXX_bin` member names are
    /// also accepted directly, so unbuilt decomp checkouts can still parse.
    pub fn from_hgss_decomp(content: &str, symbols: &SymbolTable) -> Option<Self> {
        let start = content.find("sScriptBankMapping")?;
        let block_start = content[start..].find('{')?;
        let block = &content[start + block_start..];

        let mut entries = Vec::new();
        for (index, caps) in RE_HGSS_ENTRY.captures_iter(block).enumerate() {
            let script_id_sym = caps.get(1)?.as_str();
            let script_file_sym = caps.get(2)?.as_str();
            let text_archive_sym = caps.get(3)?.as_str();

            let min_script_id = resolve_value(script_id_sym, symbols)? as u16;
            let script_file_id = resolve_value(script_file_sym, symbols)? as u16;
            let text_archive_id = resolve_value(text_archive_sym, symbols)? as u16;

            entries.push(GlobalScriptEntry::with_range(
                min_script_id,
                script_file_id,
                text_archive_id,
                HGSS_RANGES
                    .get(index)
                    .copied()
                    .unwrap_or(GlobalScriptRange::ScriptBank(min_script_id)),
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
        for (index, caps) in RE_PLATINUM_TABLE_ENTRY.captures_iter(block).enumerate() {
            let script_id_sym = caps.get(1)?.as_str();
            let script_file_sym = caps.get(2)?.as_str();
            let text_archive_sym = caps.get(3)?.as_str();

            let min_script_id = resolve_value(script_id_sym, symbols)? as u16;
            let script_file_id = resolve_value(script_file_sym, symbols)? as u16;
            let text_archive_id = resolve_value(text_archive_sym, symbols)? as u16;

            entries.push(GlobalScriptEntry::with_range(
                min_script_id,
                script_file_id,
                text_archive_id,
                PLATINUM_RANGES
                    .get(index)
                    .copied()
                    .unwrap_or(GlobalScriptRange::ScriptBank(min_script_id)),
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

    /// Hardcoded table for Platinum Western (US/EU/DE/FR/IT/ES).
    ///
    /// Extracted from `ScriptContext_LoadAndOffsetID` in arm9 (func 0x3EB20, pool 0x3EE08).
    pub fn platinum_hardcoded() -> Self {
        Self::platinum_western_hardcoded()
    }

    /// Hardcoded table for Platinum Western (US/EU/DE/FR/IT/ES).
    pub fn platinum_western_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(
            PLATINUM_RANGES,
            &[
                (10490, 499, 499),
                (10450, 500, 16),
                (10400, 400, 203),
                (10300, 1051, 552),
                (10200, 407, 379),
                (10150, 1116, 621),
                (10100, 1115, 622),
                (10000, 409, 381),
                (9950, 411, 383),
                (9900, 397, 213),
                (9800, 212, 217),
                (9700, 422, 422),
                (9600, 412, 213),
                (9500, 501, 501),
                (9400, 426, 426),
                (9300, 406, 374),
                (9200, 423, 423),
                (9100, 0, 11),
                (9000, 213, 221),
                (8970, 425, 7),
                (8950, 498, 498),
                (8900, 424, 424),
                (8800, 497, 497),
                (8000, 408, 380),
                (7000, 404, 369),
                (5000, 1114, 213),
                (3000, 1114, 213),
                (2800, 413, 397),
                (2500, 1, 17),
                (2000, 211, 213),
            ],
        ))
    }

    /// Hardcoded table for Platinum Japanese.
    ///
    /// Extracted from arm9 func 0x3E6D8, pool 0x3E9C0.
    pub fn platinum_japanese_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(
            PLATINUM_RANGES,
            &[
                (10490, 499, 499),
                (10450, 500, 15),
                (10400, 400, 202),
                (10300, 1051, 546),
                (10200, 407, 378),
                (10150, 1116, 613),
                (10100, 1115, 614),
                (10000, 409, 380),
                (9950, 411, 382),
                (9900, 397, 212),
                (9800, 212, 216),
                (9700, 422, 422),
                (9600, 412, 212),
                (9500, 501, 501),
                (9400, 426, 426),
                (9300, 406, 373),
                (9200, 423, 423),
                (9100, 0, 11),
                (9000, 213, 220),
                (8970, 425, 7),
                (8950, 498, 498),
                (8900, 424, 424),
                (8800, 497, 497),
                (8000, 408, 379),
                (7000, 404, 368),
                (5000, 1114, 212),
                (3000, 1114, 212),
                (2800, 413, 393),
                (2500, 1, 16),
                (2000, 211, 212),
            ],
        ))
    }

    /// Hardcoded table for Platinum Korean (Giratina build).
    ///
    /// Extracted from arm9 func 0x3F00C, pool 0x3F2F4.
    pub fn platinum_korean_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(
            PLATINUM_RANGES,
            &[
                (10490, 499, 499),
                (10450, 500, 15),
                (10400, 400, 202),
                (10300, 1051, 547),
                (10200, 407, 378),
                (10150, 1116, 614),
                (10100, 1115, 615),
                (10000, 409, 380),
                (9950, 411, 382),
                (9900, 397, 212),
                (9800, 212, 216),
                (9700, 422, 422),
                (9600, 412, 212),
                (9500, 501, 501),
                (9400, 426, 426),
                (9300, 406, 373),
                (9200, 423, 423),
                (9100, 0, 11),
                (9000, 213, 220),
                (8970, 425, 7),
                (8950, 498, 498),
                (8900, 424, 424),
                (8800, 497, 497),
                (8000, 408, 379),
                (7000, 404, 368),
                (5000, 1114, 212),
                (3000, 1114, 212),
                (2800, 413, 393),
                (2500, 1, 16),
                (2000, 211, 212),
            ],
        ))
    }

    /// Hardcoded table for Diamond/Pearl Western (US/EU Rev 5).
    ///
    /// Extracted from `LoadScriptsAndMessagesByMapId` in arm9 (func 0x38F18, pool 0x39210).
    pub fn dp_western_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(DP_RANGES, DP_WESTERN_SCRIPT_IDS))
    }

    /// Hardcoded table for Diamond/Pearl Japanese.
    ///
    /// Extracted from arm9 func 0x3B6B0, pool 0x3B95C. Includes two extra script
    /// banks at 9600 and 9300 not present in western DP; those use generic range names.
    pub fn dp_japanese_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(
            &[
                GlobalScriptRange::CounterpartTalk,
                GlobalScriptRange::MysteryGiftDeliveryman,
                GlobalScriptRange::TvReporterInterviews,
                GlobalScriptRange::TvBroadcast,
                GlobalScriptRange::FieldMoves,
                GlobalScriptRange::PokedexRatings,
                GlobalScriptRange::CommonStrings9900,
                GlobalScriptRange::Contests,
                GlobalScriptRange::FollowerPartners,
                GlobalScriptRange::ScriptBank(9600),
                GlobalScriptRange::DayCareCommon,
                GlobalScriptRange::PoffinCommon,
                GlobalScriptRange::ScriptBank(9300),
                GlobalScriptRange::PokemonCenterB1fAttendants,
                GlobalScriptRange::CommunicationClub,
                GlobalScriptRange::PokemonCenter2fAttendants,
                GlobalScriptRange::PokeRadar,
                GlobalScriptRange::VsSeeker,
                GlobalScriptRange::RecordChatotCry,
                GlobalScriptRange::SafariGame,
                GlobalScriptRange::HiddenItems,
                GlobalScriptRange::VisibleItems,
                GlobalScriptRange::Trainer {
                    double_battle_id: 2,
                },
                GlobalScriptRange::Trainer {
                    double_battle_id: 1,
                },
                GlobalScriptRange::BerryTreeInteractions,
                GlobalScriptRange::BgEvents,
                GlobalScriptRange::CommonScripts,
            ],
            &[
                (10300, 977, 488),
                (10200, 373, 330),
                (10150, 1042, 552),
                (10100, 1041, 553),
                (10000, 375, 332),
                (9950, 376, 333),
                (9900, 365, 198),
                (9800, 206, 202),
                (9700, 387, 370),
                (9600, 377, 198),
                (9500, 464, 484),
                (9400, 391, 373),
                (9300, 372, 327),
                (9200, 388, 371),
                (9100, 0, 9),
                (9000, 207, 206),
                (8970, 390, 7),
                (8950, 463, 478),
                (8900, 389, 372),
                (8800, 462, 477),
                (8000, 374, 331),
                (7000, 370, 323),
                (5000, 1040, 198),
                (3000, 1040, 198),
                (2800, 378, 344),
                (2500, 1, 12),
                (2000, 205, 198),
            ],
        ))
    }

    /// Hardcoded table for Diamond/Pearl Korean.
    ///
    /// Extracted from arm9 func 0x39388, pool 0x3967C.
    pub fn dp_korean_hardcoded() -> Self {
        Self::from_entries(entries_from_ids(
            DP_RANGES,
            &[
                (10300, 977, 490),
                (10200, 373, 331),
                (10150, 1042, 554),
                (10100, 1041, 555),
                (10000, 375, 333),
                (9950, 376, 334),
                (9900, 365, 198),
                (9800, 206, 202),
                (9700, 387, 372),
                (9500, 464, 464),
                (9400, 391, 375),
                (9200, 388, 373),
                (9100, 0, 9),
                (9000, 207, 206),
                (8970, 390, 7),
                (8950, 463, 463),
                (8900, 389, 374),
                (8800, 462, 462),
                (8000, 374, 332),
                (7000, 370, 324),
                (5000, 1040, 198),
                (3000, 1040, 198),
                (2800, 378, 345),
                (2500, 1, 12),
                (2000, 205, 198),
            ],
        ))
    }

    /// Look up the entry for a global script ID.
    ///
    /// Returns the entry where `script_id >= entry.min_script_id`.
    pub fn lookup(&self, script_id: u16) -> Option<&GlobalScriptEntry> {
        self.entries.iter().find(|e| script_id >= e.min_script_id)
    }

    /// Lowest global script ID in this table (map scripts are below this).
    pub fn min_global_script_id(&self) -> Option<u16> {
        self.entries.iter().map(|e| e.min_script_id).min()
    }

    /// Check if a script ID falls in a global script range from this table.
    pub fn is_global_script(&self, script_id: u16) -> bool {
        self.lookup(script_id).is_some()
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

    /// Find the entry whose `script_file_id` matches the given value.
    ///
    /// Returns the first matching entry, or `None` if no entry uses the
    /// given script file.
    pub fn find_by_script_file_id(&self, script_file_id: u16) -> Option<&GlobalScriptEntry> {
        self.entries
            .iter()
            .find(|e| e.script_file_id == script_file_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::WriteBytesExt;
    use proptest::prelude::*;

    #[test]
    fn test_platinum_western_hardcoded_table() {
        let table = GlobalScriptTable::platinum_western_hardcoded();
        assert_eq!(table.len(), 30);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 211);
        assert_eq!(entry.text_archive_id, 213);
        assert_eq!(entry.range, GlobalScriptRange::CommonScripts);

        let entry = table.lookup(2050).unwrap();
        assert_eq!(entry.min_script_id, 2000);

        let entry = table.lookup(3500).unwrap();
        assert_eq!(entry.min_script_id, 3000);
        assert_eq!(entry.script_file_id, 1114);
        assert_eq!(entry.text_archive_id, 213);
        assert_eq!(
            entry.range,
            GlobalScriptRange::Trainer {
                double_battle_id: 1
            }
        );
        assert_eq!(
            table.lookup(5500).unwrap().range,
            GlobalScriptRange::Trainer {
                double_battle_id: 2
            }
        );
    }

    #[test]
    fn test_dp_japanese_extra_banks_use_generic_names() {
        let table = GlobalScriptTable::dp_japanese_hardcoded();
        assert_eq!(
            table.lookup(9600).unwrap().range,
            GlobalScriptRange::ScriptBank(9600)
        );
        assert_eq!(
            table.lookup(9300).unwrap().range,
            GlobalScriptRange::ScriptBank(9300)
        );
        assert_eq!(
            table.lookup(3000).unwrap().range.display_name(),
            "Single Battles"
        );
    }

    #[test]
    fn test_dp_western_hardcoded_table() {
        let table = GlobalScriptTable::dp_western_hardcoded();
        assert_eq!(table.len(), 25);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 205);
        assert_eq!(entry.text_archive_id, 199);

        let entry = table.lookup(3500).unwrap();
        assert_eq!(entry.min_script_id, 3000);
        assert_eq!(entry.script_file_id, 1040);
        assert_eq!(
            entry.range,
            GlobalScriptRange::Trainer {
                double_battle_id: 1
            }
        );
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
        let entry = GlobalScriptEntry::read_from_binary(&mut cursor).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 211);
        assert_eq!(entry.text_archive_id, 213);
    }

    #[test]
    fn test_hgss_binary_assigns_semantic_ranges() {
        let table_offset = 0x40200usize;
        let mut data = vec![0; table_offset + HGSS_TABLE_ENTRY_COUNT * 6];
        (&mut data[HGSS_TABLE_POINTER_OFFSET as usize..])
            .write_u32::<LittleEndian>(HGSS_MEMORY_BASE + table_offset as u32)
            .unwrap();

        let min_script_ids = [
            10500, 10400, 10350, 10300, 10250, 10200, 10175, 10150, 10100, 10000, 9950, 9900,
            9850, 9800, 9700, 9600, 9500, 9400, 9300, 9200, 9100, 9000, 8970, 8950, 8900, 5000,
            3000, 2800, 2500, 2000,
        ];

        for (index, min_script_id) in min_script_ids.into_iter().enumerate() {
            let start = table_offset + index * 6;
            let mut entry = &mut data[start..start + 6];
            entry.write_u16::<LittleEndian>(min_script_id).unwrap();
            entry.write_u16::<LittleEndian>(index as u16).unwrap();
            entry
                .write_u16::<LittleEndian>((index as u16).saturating_add(100))
                .unwrap();
        }

        let table = GlobalScriptTable::from_hgss_binary(&mut Cursor::new(data)).unwrap();

        assert_eq!(
            table.lookup(5000).unwrap().range,
            GlobalScriptRange::Trainer {
                double_battle_id: 2
            }
        );
        assert_eq!(
            table.lookup(3000).unwrap().range,
            GlobalScriptRange::Trainer {
                double_battle_id: 1
            }
        );
        assert_eq!(table.lookup(2000).unwrap().range, GlobalScriptRange::CommonScripts);
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

        let content = r"
const struct ScriptBankMapping sScriptBankMapping[30] = {
    { _std_scratch_card, NARC_scr_seq_scr_seq_0263_bin, NARC_msg_msg_0433_bin },
    { _std_misc, NARC_scr_seq_scr_seq_0003_bin, NARC_msg_msg_0040_bin },
};
";

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
    fn test_hgss_decomp_parsing_accepts_numeric_script_id_literal() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("NARC_scr_seq_scr_seq_0734_bin".to_string(), 734);
        symbols.insert_define("NARC_msg_msg_0444_bin".to_string(), 444);

        let content = r"
const struct ScriptBankMapping sScriptBankMapping[30] = {
    { 10300, NARC_scr_seq_scr_seq_0734_bin, NARC_msg_msg_0444_bin },
};
";

        let table = GlobalScriptTable::from_hgss_decomp(content, &symbols).unwrap();
        assert_eq!(table.len(), 1);

        let entry = table.lookup(10300).unwrap();
        assert_eq!(entry.min_script_id, 10300);
        assert_eq!(entry.script_file_id, 734);
        assert_eq!(entry.text_archive_id, 444);
    }

    #[test]
    fn test_hgss_decomp_parsing_without_generated_narc_symbols() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("_std_scratch_card".to_string(), 10500);
        symbols.insert_define("_std_misc".to_string(), 2000);

        let content = r"
const struct ScriptBankMapping sScriptBankMapping[30] = {
    { _std_scratch_card, NARC_scr_seq_scr_seq_0263_bin, NARC_msg_msg_0433_bin },
    { _std_misc, NARC_scr_seq_scr_seq_0003_bin, NARC_msg_msg_0040_bin },
};
";

        let table = GlobalScriptTable::from_hgss_decomp(content, &symbols).unwrap();
        assert_eq!(table.len(), 2);

        let entry = table.lookup(10500).unwrap();
        assert_eq!(entry.min_script_id, 10500);
        assert_eq!(entry.script_file_id, 263);
        assert_eq!(entry.text_archive_id, 433);

        let entry = table.lookup(2000).unwrap();
        assert_eq!(entry.min_script_id, 2000);
        assert_eq!(entry.script_file_id, 3);
        assert_eq!(entry.text_archive_id, 40);
    }

    #[test]
    fn test_platinum_decomp_parsing() {
        let mut symbols = SymbolTable::new();
        symbols.insert_define("scripts_unk_0499".to_string(), 499);
        symbols.insert_define("TEXT_BANK_SCRATCH_OFF_CARDS".to_string(), 0x21D);
        symbols.insert_define("scripts_common".to_string(), 211);
        symbols.insert_define("TEXT_BANK_COMMON_STRINGS".to_string(), 0x0D5);
        symbols.insert_define("SCRIPT_ID_OFFSET_COMMON_SCRIPTS".to_string(), 2000);

        let content = r"
// clang-format off
#define SCRIPT_RANGE_TABLE(Entry) \
    Entry(10490,                                    scripts_unk_0499,                       TEXT_BANK_SCRATCH_OFF_CARDS) \
    Entry(SCRIPT_ID_OFFSET_COMMON_SCRIPTS,          scripts_common,                         TEXT_BANK_COMMON_STRINGS)
// clang-format on
";

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
    #[ignore = "requires a real HGSS DSPRE project path via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn test_hgss_binary_real_file() {
        let Some(path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["arm9.bin", "unpacked/arm9.bin", "arm9/arm9.bin"],
            "HGSS arm9 integration test (from DSPRE project root)",
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

    #[test]
    #[ignore = "requires real HGSS fixture roots via UXIE_TEST_HGSS_DECOMP_PATH and UXIE_TEST_HGSS_DSPRE_PATH"]
    fn test_hgss_decomp_real_fixture_matches_hgss_binary() {
        let Some(fieldmap_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            &["src/fieldmap.c"],
            "HGSS decomp global-script-table integration test",
        ) else {
            return;
        };

        let Some(arm9_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["arm9.bin", "unpacked/arm9.bin", "arm9/arm9.bin"],
            "HGSS binary global-script-table integration test (from DSPRE project root)",
        ) else {
            return;
        };

        let decomp_root = fieldmap_path
            .parent()
            .and_then(std::path::Path::parent)
            .expect("fieldmap.c should be under <decomp>/src");

        let ws = crate::workspace::Workspace::open_decomp(decomp_root).unwrap();
        let decomp_table =
            GlobalScriptTable::from_hgss_decomp_file(&fieldmap_path, &ws.symbols).unwrap();
        let binary_table = GlobalScriptTable::from_hgss_binary_file(arm9_path).unwrap();

        assert_eq!(decomp_table.len(), HGSS_TABLE_ENTRY_COUNT);
        assert_eq!(binary_table.len(), HGSS_TABLE_ENTRY_COUNT);

        for entry in decomp_table.entries() {
            assert_eq!(binary_table.lookup(entry.min_script_id), Some(entry));
        }
    }

    #[test]
    #[ignore = "requires a real Platinum decomp path via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn test_platinum_decomp_real_fixture_matches_hardcoded_table() {
        let Some(script_manager_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            &["src/script_manager.c"],
            "Platinum decomp global-script-table integration test",
        ) else {
            return;
        };

        let decomp_root = script_manager_path
            .parent()
            .and_then(std::path::Path::parent)
            .expect("script_manager.c should be under <decomp>/src");

        let ws = crate::workspace::Workspace::open_decomp(decomp_root).unwrap();
        let decomp_table =
            GlobalScriptTable::from_platinum_decomp_file(&script_manager_path, &ws.symbols)
                .unwrap();
        let hardcoded_table = GlobalScriptTable::platinum_hardcoded();

        assert_eq!(decomp_table.len(), PLATINUM_TABLE_ENTRY_COUNT);
        assert_eq!(hardcoded_table.len(), PLATINUM_TABLE_ENTRY_COUNT);

        for entry in hardcoded_table.entries() {
            let decomp = decomp_table
                .lookup(entry.min_script_id)
                .expect("missing decomp range");
            assert_eq!(decomp.min_script_id, entry.min_script_id);
            assert_eq!(decomp.script_file_id, entry.script_file_id);
            assert_eq!(decomp.text_archive_id, entry.text_archive_id);
        }
    }

    fn global_entries_strategy() -> impl Strategy<Value = Vec<GlobalScriptEntry>> {
        prop::collection::btree_map(any::<u16>(), (any::<u16>(), any::<u16>()), 0..48).prop_map(
            |mapping| {
                mapping
                    .into_iter()
                    .map(|(min_script_id, (script_file_id, text_archive_id))| {
                        GlobalScriptEntry::new(
                            min_script_id,
                            script_file_id,
                            text_archive_id,
                            format!("Script Range {min_script_id}"),
                        )
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
                .cloned();

            let table = GlobalScriptTable::from_entries(entries);
            let actual = table.lookup(script_id).cloned();
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_is_global_script_matches_lookup(script_id in any::<u16>()) {
            let table = GlobalScriptTable::platinum_western_hardcoded();
            prop_assert_eq!(
                table.is_global_script(script_id),
                table.lookup(script_id).is_some()
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
            let parsed = GlobalScriptEntry::read_from_binary(&mut cursor).unwrap();
            prop_assert_eq!(parsed.min_script_id, min_script_id);
            prop_assert_eq!(parsed.script_file_id, script_file_id);
            prop_assert_eq!(parsed.text_archive_id, text_archive_id);
        }
    }
}
