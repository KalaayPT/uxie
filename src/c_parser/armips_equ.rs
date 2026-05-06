use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use regex::Regex;

use super::SymbolTable;

static RE_GNU_EQU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\.equ\s+([A-Za-z_][A-Za-z0-9_]*)\s*,\s*(.+)$").unwrap()
});

static RE_ARMIPS_EQU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s+equ\s+(.+)$").unwrap()
});

struct PendingEqu {
    name: String,
    expr: String,
    source: PathBuf,
    line_number: usize,
}

/// Parse armips `.equ` / `equ` directives from a file and insert them into the
/// symbol table. Skips names that are already defined (C headers take priority).
///
/// Handles:
/// - `.equ NAME, value` (GNU asm)
/// - `NAME equ value`   (armips native)
/// - `;` and `//` line comments
/// - Forward references (multi-pass resolution)
///
/// Values are evaluated through the symbol table's expression parser, so they
/// can reference previously-defined symbols.
pub fn parse_armips_equ_file(
    path: &Path,
    symbols: &mut SymbolTable,
) -> std::io::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    parse_armips_equ_str(&content, path, symbols)
}

/// Parse armips `.equ` / `equ` directives from a string (for testing).
///
/// Uses multi-pass resolution to handle forward references (a constant defined
/// later in the file referencing an earlier definition).
pub fn parse_armips_equ_str(
    content: &str,
    source: &Path,
    symbols: &mut SymbolTable,
) -> std::io::Result<usize> {
    let mut pending = Vec::new();
    collect_pending_from_str(content, source, symbols, &mut pending);
    resolve_all_pending(&mut pending, symbols)
}

/// Parse all armips `.equ` / `equ` directives from every file in one or more
/// directories (non-recursive). Files with extensions `.s` and `.inc` are
/// processed. All definitions are resolved together in a single multi-pass so
/// cross-file forward references work.
pub fn parse_armips_equ_dirs(
    dirs: &[&Path],
    symbols: &mut SymbolTable,
) -> std::io::Result<usize> {
    let mut all_pending: Vec<PendingEqu> = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !matches!(ext.to_ascii_lowercase().as_str(), "s" | "inc") {
                continue;
            }

            let content = std::fs::read_to_string(&path)?;
            collect_pending_from_str(&content, &path, symbols, &mut all_pending);
        }
    }

    resolve_all_pending(&mut all_pending, symbols)
}

/// Parse all armips `.equ` / `equ` directives from every file in a directory
/// (non-recursive). Files with extensions `.s` and `.inc` are processed.
/// Returns the total number of constants inserted.
pub fn parse_armips_equ_dir(
    dir: &Path,
    symbols: &mut SymbolTable,
) -> std::io::Result<usize> {
    parse_armips_equ_dirs(&[dir], symbols)
}

// --- internal helpers ---

fn collect_pending_from_str(
    content: &str,
    source: &Path,
    symbols: &SymbolTable,
    pending: &mut Vec<PendingEqu>,
) {
    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;

        let line = line.split(';').next().unwrap_or(line);
        let line = line.split("//").next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }

        let (name, expr) = if let Some(caps) = RE_GNU_EQU.captures(line) {
            (caps.get(1).unwrap().as_str().to_string(), caps.get(2).unwrap().as_str().trim().to_string())
        } else if let Some(caps) = RE_ARMIPS_EQU.captures(line) {
            (caps.get(1).unwrap().as_str().to_string(), caps.get(2).unwrap().as_str().trim().to_string())
        } else {
            continue;
        };

        if symbols.resolve_constant(&name).is_some() {
            continue;
        }

        pending.push(PendingEqu {
            name,
            expr,
            source: source.to_path_buf(),
            line_number,
        });
    }
}

fn resolve_all_pending(
    pending: &mut Vec<PendingEqu>,
    symbols: &mut SymbolTable,
) -> std::io::Result<usize> {
    if pending.is_empty() {
        return Ok(0);
    }

    let mut resolved = 0;
    loop {
        let before = resolved;
        pending.retain(|entry| {
            if symbols.resolve_constant(&entry.name).is_some() {
                return false;
            }

            match symbols.evaluate_expression(&entry.expr) {
                Some(value) => {
                    symbols.insert_define(entry.name.clone(), value);
                    resolved += 1;
                    false
                }
                None => true,
            }
        });

        if resolved == before || pending.is_empty() {
            break;
        }
    }

    if let Some(entry) = pending.first() {
        eprintln!(
            "Warning: unresolved .equ at {}:{}: {} = '{}' \
             (referenced symbol not defined in project sources)",
            entry.source.display(), entry.line_number, entry.name, entry.expr
        );
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c_parser::SymbolTable;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
        path
    }

    #[test]
    fn parse_gnu_equ_simple() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let n = parse_armips_equ_str(
            ".equ FLAG_UNK, 0x3C3\n.equ VAR_TEMP, 42\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("FLAG_UNK"), Some(0x3C3));
        assert_eq!(table.resolve_constant("VAR_TEMP"), Some(42));
    }

    #[test]
    fn parse_armips_equ_simple() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.inc");
        let n = parse_armips_equ_str(
            "ITEM_MASTER_BALL equ 1\nITEM_POTION equ 17\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("ITEM_MASTER_BALL"), Some(1));
        assert_eq!(table.resolve_constant("ITEM_POTION"), Some(17));
    }

    #[test]
    fn parse_mixed_formats() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let n = parse_armips_equ_str(
            ".equ CONST_A, 100\nCONST_B equ 200\n.equ CONST_C, 300\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 3);
        assert_eq!(table.resolve_constant("CONST_A"), Some(100));
        assert_eq!(table.resolve_constant("CONST_B"), Some(200));
        assert_eq!(table.resolve_constant("CONST_C"), Some(300));
    }

    #[test]
    fn skip_comments_and_blanks() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let n = parse_armips_equ_str(
            "; this is a comment\n\n.equ FLAG_A, 1 ; inline comment\n\nFLAG_B equ 2\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("FLAG_A"), Some(1));
        assert_eq!(table.resolve_constant("FLAG_B"), Some(2));
    }

    #[test]
    fn expression_evaluation() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        // First define a base constant
        table.insert_define("BASE".to_string(), 100);
        // Then use it in an .equ expression
        let n = parse_armips_equ_str(
            ".equ DERIVED, BASE + 50\n.equ SHIFTED, 1 << 3\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("DERIVED"), Some(150));
        assert_eq!(table.resolve_constant("SHIFTED"), Some(8));
    }

    #[test]
    fn skips_already_defined_constants() {
        let mut table = SymbolTable::new();
        table.insert_define("ALREADY".to_string(), 999);
        let path = std::path::Path::new("test.s");

        let n = parse_armips_equ_str(
            ".equ ALREADY, 1\n.equ NEW, 2\n",
            path,
            &mut table,
        )
        .unwrap();
        // Only NEW should be inserted; ALREADY is skipped.
        assert_eq!(n, 1);
        assert_eq!(table.resolve_constant("ALREADY"), Some(999));
        assert_eq!(table.resolve_constant("NEW"), Some(2));
    }

    #[test]
    fn bad_expression_returns_error() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        // UNDEFINED_SYMBOL has no definition anywhere, so it's skipped with a warning.
        let result = parse_armips_equ_str(
            ".equ BAD, UNDEFINED_SYMBOL + 1\n",
            path,
            &mut table,
        );
        // Should NOT error — unresolvable constants are skipped with a warning.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(table.resolve_constant("BAD"), None);
    }

    #[test]
    fn forward_references_resolve_in_multipass() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        // CONTESTANT_TYPE_PLAYER references BATTLER_TYPE_* which are defined
        // later in the same file — must resolve via multi-pass.
        let n = parse_armips_equ_str(
            "\
.equ CONTESTANT_TYPE_PLAYER, BATTLER_TYPE_MAX + BATTLER_TYPE_SOLO_PLAYER
.equ BATTLER_TYPE_SOLO_PLAYER, 1
.equ BATTLER_TYPE_TAG_PARTNER, 2
.equ BATTLER_TYPE_MAX, BATTLER_TYPE_TAG_PARTNER
",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 4);
        assert_eq!(table.resolve_constant("BATTLER_TYPE_SOLO_PLAYER"), Some(1));
        assert_eq!(table.resolve_constant("BATTLER_TYPE_MAX"), Some(2));
        // CONTESTANT_TYPE_PLAYER = 2 + 1 = 3
        assert_eq!(table.resolve_constant("CONTESTANT_TYPE_PLAYER"), Some(3));
    }

    #[test]
    fn circular_dependency_is_skipped_with_warning() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let result = parse_armips_equ_str(
            ".equ A, B + 1\n.equ B, A + 1\n",
            path,
            &mut table,
        );
        // Circular refs can't resolve — both are skipped with a warning.
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn parse_armips_equ_file_from_disk() {
        let dir = tempdir().unwrap();
        let path = write_file(
            dir.path(),
            "flags.s",
            ".equ FLAG_UNK, 0x3C3\n.equ FLAG_DEX, 0x3C4\n",
        );

        let mut table = SymbolTable::new();
        let n = parse_armips_equ_file(&path, &mut table).unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("FLAG_UNK"), Some(0x3C3));
        assert_eq!(table.resolve_constant("FLAG_DEX"), Some(0x3C4));
    }

    #[test]
    fn parse_armips_equ_dir_skips_wrong_extensions() {
        let dir = tempdir().unwrap();
        write_file(dir.path(), "constants.s", ".equ FOO, 1\n");
        write_file(dir.path(), "items.inc", "ITEM_X equ 2\n");
        write_file(dir.path(), "notes.txt", "not an assembly file\n");
        write_file(dir.path(), "data.bin", "\x00\x01");

        let mut table = SymbolTable::new();
        let n = parse_armips_equ_dir(dir.path(), &mut table).unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("FOO"), Some(1));
        assert_eq!(table.resolve_constant("ITEM_X"), Some(2));
    }

    #[test]
    fn parse_armips_equ_nonexistent_dir() {
        let mut table = SymbolTable::new();
        let n = parse_armips_equ_dir(
            std::path::Path::new("/tmp/nonexistent_armips_dir_12345"),
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn parse_armips_equ_hex_values() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let n = parse_armips_equ_str(
            ".equ START_ADDRESS, 0x10\n.equ OVERLAY_ID, 0x81\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("START_ADDRESS"), Some(16));
        assert_eq!(table.resolve_constant("OVERLAY_ID"), Some(129));
    }

    #[test]
    fn parse_armips_equ_negative_values() {
        let mut table = SymbolTable::new();
        let path = std::path::Path::new("test.s");
        let n = parse_armips_equ_str(
            ".equ NEG_VAL, -1\n.equ NEG_HEX, -0x80\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("NEG_VAL"), Some(-1));
        assert_eq!(table.resolve_constant("NEG_HEX"), Some(-128));
    }

    #[test]
    fn parse_armips_equ_with_expression_chaining() {
        let mut table = SymbolTable::new();
        table.insert_define("BASE_ADDR".to_string(), 0x02000000);
        let path = std::path::Path::new("test.s");

        let n = parse_armips_equ_str(
            ".equ HEAP_ADDR, BASE_ADDR + 0x1000\n.equ STACK_ADDR, HEAP_ADDR + 0x800\n",
            path,
            &mut table,
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(table.resolve_constant("HEAP_ADDR"), Some(0x02001000));
        assert_eq!(table.resolve_constant("STACK_ADDR"), Some(0x02001800));
    }

    #[test]
    fn cross_file_forward_references_resolve() {
        let dir = tempdir().unwrap();

        // File A defines a constant that references one in file B.
        write_file(
            dir.path(),
            "animscript.s",
            ".equ CONTESTANT_TYPE_PLAYER, BATTLER_TYPE_MAX + BATTLER_TYPE_SOLO_PLAYER\n",
        );
        // File B defines the referenced constants.
        write_file(
            dir.path(),
            "battle_constants.inc",
            "\
.equ BATTLER_TYPE_SOLO_PLAYER, 0x0
.equ BATTLER_TYPE_MAX, 0x2
",
        );

        let mut table = SymbolTable::new();
        let dirs = [dir.path()];
        let n = parse_armips_equ_dirs(&dirs, &mut table).unwrap();
        assert_eq!(n, 3);
        assert_eq!(table.resolve_constant("BATTLER_TYPE_SOLO_PLAYER"), Some(0));
        assert_eq!(table.resolve_constant("BATTLER_TYPE_MAX"), Some(2));
        assert_eq!(table.resolve_constant("CONTESTANT_TYPE_PLAYER"), Some(2));
    }

    #[test]
    fn parse_actual_scriptmacros_s() {
        let path = std::path::Path::new(
            "/home/kalaay/dev/slop-engine/armips/include/scriptmacros.s",
        );
        if !path.exists() {
            eprintln!("skipping: slop-engine not found at {}", path.display());
            return;
        }

        let mut table = SymbolTable::new();
        let n = parse_armips_equ_file(path, &mut table).unwrap();
        assert!(n > 0, "should parse at least some .equ directives");

        // Verify a few known constants from the file.
        assert_eq!(table.resolve_constant("SCRDEF_END_CONSTANT"), Some(0xFD13));
        assert_eq!(table.resolve_constant("DIR_NORTH"), Some(0));
        assert_eq!(table.resolve_constant("DIR_SOUTH"), Some(1));
        assert_eq!(table.resolve_constant("DIR_WEST"), Some(2));
        assert_eq!(table.resolve_constant("DIR_EAST"), Some(3));
    }
}
