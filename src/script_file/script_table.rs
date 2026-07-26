//! Script name to file ID mapping table.
//!
//! Parses `scripts.order` files from decompilation projects to map
//! symbolic script names to their numeric file IDs.
//!
//! HGSS decomp projects don't use `scripts.order`; script files are named by
//! NARC index in `files/fielddata/script` (for example,
//! `scr_seq_0081_D32R0102.s`). Those names can be loaded with
//! [`ScriptTable::load_hgss_script_dir`].

use rustc_hash::FxHashMap;
use std::io;
use std::path::{Path, PathBuf};

/// Mapping between script names and their file IDs.
///
/// In decompilation projects, scripts are referenced by symbolic names
/// (e.g., `scripts_jubilife_city`). The `scripts.order` file maps these
/// names to numeric file IDs based on line number (0-indexed).
///
/// # Example
///
/// ```
/// use uxie::script_file::ScriptTable;
///
/// let mut table = ScriptTable::new();
/// table.load_order_str("scripts_unk_0000\nscripts_jubilife_city\n").unwrap();
///
/// assert_eq!(table.get_id("scripts_jubilife_city"), Some(1));
/// assert_eq!(table.get_name(0), Some("scripts_unk_0000"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ScriptTable {
    pub(crate) names: Vec<String>,
    pub(crate) name_to_id: FxHashMap<String, usize>,
    pub(crate) sparse_names: FxHashMap<usize, String>,
    /// Order file (scripts.order) used to populate the dense script-name table.
    order_path: Option<PathBuf>,
    /// On-disk source paths observed while loading, keyed by file ID.
    ///
    /// Populated by the directory-enumerating loaders
    /// ([`load_dspre_script_dir`](Self::load_dspre_script_dir) and
    /// [`load_hgss_script_dir`](Self::load_hgss_script_dir)) and by
    /// [`load_order_file`](Self::load_order_file) for `scripts.order` names
    /// that have a matching source file in the same directory.
    pub(crate) paths: FxHashMap<usize, PathBuf>,
}

impl ScriptTable {
    /// Creates an empty script table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads script names from a `scripts.order` file.
    ///
    /// Each line in the file becomes a script name, with its line number
    /// (0-indexed) as the file ID. Empty lines and lines starting with `#`
    /// are skipped. Script sources are expected alongside the order file
    /// (`<name>.s` / `<name>.rotom` in the same directory); the path of any
    /// such file found is recorded for [`get_path`](Self::get_path). Having
    /// both formats for one script is rejected as malformed project state.
    pub fn load_order_file(&mut self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "Failed to read script order file {} as UTF-8: {err}",
                    path.display()
                ),
            )
        })?;
        let start = self.names.len();
        self.load_order_str(&content)?;
        self.order_path = Some(path.to_path_buf());

        if let Some(dir) = path.parent() {
            let found: Vec<(usize, PathBuf)> = self.names[start..]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let native = dir.join(format!("{name}.rotom"));
                    let legacy = dir.join(format!("{name}.s"));
                    match (native.is_file(), legacy.is_file()) {
                        (true, true) => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Multiple source files found for script '{name}': {} and {}",
                                native.display(),
                                legacy.display()
                            ),
                        )),
                        (true, false) => Ok(Some((start + i, native))),
                        (false, true) => Ok(Some((start + i, legacy))),
                        (false, false) => Ok(None),
                    }
                })
                .collect::<io::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect();
            for (id, p) in found {
                self.paths.insert(id, p);
            }
        }
        Ok(())
    }

    /// Loads script names from a string containing `scripts.order` content.
    ///
    /// Each line becomes a script name, with its line number (0-indexed)
    /// as the file ID. Empty lines and lines starting with `#` are skipped.
    pub fn load_order_str(&mut self, content: &str) -> std::io::Result<()> {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let name = line.to_string();
            self.name_to_id.insert(name.clone(), self.names.len());
            self.names.push(name);
        }
        Ok(())
    }

    /// Loads HGSS script names from an ID-based script directory.
    ///
    /// The expected filename format is `scr_seq_XXXX*.(s|rotom)`, where `XXXX`
    /// is the script file ID in decimal. IDs are used directly as table indices.
    pub fn load_hgss_script_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        let mut parsed: Vec<(usize, String, PathBuf)> = Vec::new();
        let dir = dir.as_ref();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let path = entry.path();
            let Some(file_name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };

            if let Some((id, script_name)) =
                parse_hgss_script_filename(&file_name).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Failed to parse HGSS script filename '{}' in {}: {e}",
                            file_name,
                            dir.display()
                        ),
                    )
                })?
            {
                parsed.push((id, script_name, path));
            }
        }

        parsed.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.2.cmp(&right.2)));
        if let Some(duplicate) = parsed.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Multiple source files found for script ID {}: {} and {}",
                    duplicate[0].0,
                    duplicate[0].2.display(),
                    duplicate[1].2.display()
                ),
            ));
        }

        for (id, script_name, path) in parsed {
            if let Some(old_name) = self.sparse_names.get(&id) {
                if old_name != &script_name {
                    self.name_to_id.remove(old_name);
                }
            } else if let Some(old_name) = self.names.get(id) {
                if old_name != &script_name {
                    self.name_to_id.remove(old_name);
                }
            }

            if let Some(old_name) = self.sparse_names.insert(id, script_name.clone()) {
                if old_name != script_name {
                    self.name_to_id.remove(&old_name);
                }
            }
            self.name_to_id.insert(script_name, id);
            self.paths.insert(id, path);
        }

        Ok(())
    }

    /// Loads script file names from a DSPRE `expanded/scripts/` directory.
    ///
    /// DSPRE names each script file by its NARC index (`0213.script` /
    /// `0213.rotom`), so the stem is both the name and the ID.  Non-numeric
    /// stems and non-`.script`/`.rotom` files are silently skipped. Having
    /// both formats for one script ID is rejected as malformed project state.
    pub fn load_dspre_script_dir(&mut self, dir: impl AsRef<Path>) -> std::io::Result<()> {
        let mut parsed: Vec<(usize, String, PathBuf)> = Vec::new();

        for entry in std::fs::read_dir(dir.as_ref())? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if !matches!(ext, Some("script" | "rotom")) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(id) = stem.parse::<usize>() else {
                continue;
            };
            parsed.push((id, stem.to_string(), path));
        }

        parsed.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.2.cmp(&right.2)));
        if let Some(duplicate) = parsed.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Multiple source files found for script ID {}: {} and {}",
                    duplicate[0].0,
                    duplicate[0].2.display(),
                    duplicate[1].2.display()
                ),
            ));
        }

        for (id, name, path) in parsed {
            if let Some(old) = self.sparse_names.insert(id, name.clone()) {
                if old != name {
                    self.name_to_id.remove(&old);
                }
            }
            self.name_to_id.insert(name, id);
            self.paths.insert(id, path);
        }

        Ok(())
    }

    /// Returns the script name for a given file ID.
    ///
    /// Returns `None` if the ID is out of range.
    pub fn get_name(&self, id: usize) -> Option<&str> {
        self.sparse_names
            .get(&id)
            .map(String::as_str)
            .or_else(|| self.names.get(id).map(String::as_str))
    }

    /// Returns the file ID for a given script name.
    ///
    /// Returns `None` if the name is not found.
    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.name_to_id.get(name).copied()
    }

    /// Returns the on-disk source path for a given file ID, if one was
    /// observed while loading the table.
    ///
    /// Covers directory-enumerated entries (DSPRE and HGSS loaders) and
    /// `scripts.order` names that matched a source file in the order file's
    /// directory. Returns `None` for IDs not in the table or names with no
    /// matching source file.
    pub fn get_path(&self, id: usize) -> Option<&Path> {
        self.paths.get(&id).map(PathBuf::as_path)
    }

    /// Returns the `scripts.order` file used to load this table, if any.
    pub fn order_path(&self) -> Option<&Path> {
        self.order_path.as_deref()
    }

    /// Returns all script names in order by file ID.
    pub fn get_all_names(&self) -> &[String] {
        &self.names
    }
}

fn parse_hgss_script_filename(file_name: &str) -> Result<Option<(usize, String)>, String> {
    if !file_name.starts_with("scr_seq_") {
        return Ok(None);
    }

    let stem = file_name
        .strip_suffix(".s")
        .or_else(|| file_name.strip_suffix(".rotom"));
    let Some(stem) = stem else {
        return Ok(None);
    };
    let mut parts = stem.split('_');

    if parts.next() != Some("scr") || parts.next() != Some("seq") {
        return Err(format!(
            "expected prefix format 'scr_seq_XXXX*.s', got '{}'",
            file_name
        ));
    }

    let Some(id_part) = parts.next() else {
        return Err(format!(
            "missing numeric script id in filename '{}'",
            file_name
        ));
    };
    if id_part.len() != 4 || !id_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "invalid script id '{}' in filename '{}'; expected 4 decimal digits",
            id_part, file_name
        ));
    }

    let id = id_part
        .parse::<usize>()
        .map_err(|e| format!("failed to parse script id '{}': {}", id_part, e))?;
    Ok(Some((id, stem.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn unique_names_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(any::<u16>(), 0..64).prop_map(|ids| {
            ids.into_iter()
                .enumerate()
                .map(|(idx, value)| format!("scripts_{}_{}", idx, value))
                .collect()
        })
    }

    fn names_with_duplicates_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(any::<u8>(), 0..64).prop_map(|ids| {
            ids.into_iter()
                .map(|value| format!("scripts_dup_{}", value % 16))
                .collect()
        })
    }

    #[test]
    fn test_load_order_skips_comments_and_blank_lines() {
        let mut table = ScriptTable::new();
        table
            .load_order_str(
                r"
                # comment
                scripts_unk_0000

                    scripts_jubilife_city
                # another comment
                scripts_oreburgh_city
                ",
            )
            .unwrap();

        assert_eq!(table.get_all_names().len(), 3);
        assert_eq!(table.get_name(0), Some("scripts_unk_0000"));
        assert_eq!(table.get_name(1), Some("scripts_jubilife_city"));
        assert_eq!(table.get_name(2), Some("scripts_oreburgh_city"));
    }

    #[test]
    fn test_load_hgss_script_dir_uses_id_from_filename() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("scr_seq_0081_D32R0102.s"), "").unwrap();
        fs::write(dir.path().join("scr_seq_0003_D01R0101.s"), "").unwrap();
        fs::write(dir.path().join("readme.txt"), "").unwrap();

        let mut table = ScriptTable::new();
        table.load_hgss_script_dir(dir.path()).unwrap();

        assert_eq!(table.get_name(3), Some("scr_seq_0003_D01R0101"));
        assert_eq!(table.get_name(81), Some("scr_seq_0081_D32R0102"));
        assert_eq!(table.get_name(4), None);
        assert_eq!(table.get_id("scr_seq_0003_D01R0101"), Some(3));
        assert_eq!(table.get_id("scr_seq_0004"), None);
    }

    #[test]
    fn test_load_hgss_script_dir_invalid_scr_seq_filename_returns_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("scr_seq_BAD_D01R0101.s"), "").unwrap();

        let mut table = ScriptTable::new();
        let err = table.load_hgss_script_dir(dir.path()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("invalid script id"));
    }

    #[test]
    fn test_get_path_for_dir_enumerated_loaders() {
        // HGSS loader rejects multiple source formats for one script ID.
        let hgss = tempdir().unwrap();
        fs::write(hgss.path().join("scr_seq_0003_D01R0101.s"), "").unwrap();
        fs::write(hgss.path().join("scr_seq_0003_D01R0101.rotom"), "").unwrap();
        let mut table = ScriptTable::new();
        let error = table.load_hgss_script_dir(hgss.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("Multiple source files"));
        fs::remove_file(hgss.path().join("scr_seq_0003_D01R0101.s")).unwrap();
        let mut table = ScriptTable::new();
        table.load_hgss_script_dir(hgss.path()).unwrap();
        assert_eq!(
            table.get_path(3),
            Some(hgss.path().join("scr_seq_0003_D01R0101.rotom").as_path())
        );
        assert_eq!(table.get_path(4), None);

        // DSPRE loader applies the same duplicate-source rule.
        let dspre = tempdir().unwrap();
        fs::write(dspre.path().join("0211.script"), "").unwrap();
        fs::write(dspre.path().join("0211.rotom"), "").unwrap();
        let mut table = ScriptTable::new();
        let error = table.load_dspre_script_dir(dspre.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("Multiple source files"));
        fs::remove_file(dspre.path().join("0211.script")).unwrap();
        let mut table = ScriptTable::new();
        table.load_dspre_script_dir(dspre.path()).unwrap();
        assert_eq!(
            table.get_path(211),
            Some(dspre.path().join("0211.rotom").as_path())
        );
    }

    #[test]
    fn test_get_path_for_order_file_reconstructs_source_path() {
        // A `scripts.order` name resolves to the source file in the same dir.
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("scripts.order"),
            "scripts_common\nscripts_jubilife_city\n",
        )
        .unwrap();
        fs::write(dir.path().join("scripts_common.s"), "").unwrap();
        fs::write(dir.path().join("scripts_common.rotom"), "").unwrap();
        // scripts_jubilife_city has no source file on disk -> no path.

        let mut table = ScriptTable::new();
        let error = table
            .load_order_file(dir.path().join("scripts.order"))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("Multiple source files"));
        fs::remove_file(dir.path().join("scripts_common.s")).unwrap();

        let mut table = ScriptTable::new();
        table
            .load_order_file(dir.path().join("scripts.order"))
            .unwrap();

        assert_eq!(
            table.get_path(0),
            Some(dir.path().join("scripts_common.rotom").as_path())
        );
        assert_eq!(table.get_path(1), None);
        assert_eq!(
            table.order_path(),
            Some(dir.path().join("scripts.order").as_path())
        );
    }

    fn parse_scripts_order_entries(path: &Path) -> std::io::Result<Vec<String>> {
        let content = fs::read_to_string(path)?;
        Ok(content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect())
    }

    fn collect_hgss_script_names_with_precedence(
        dirs: &[PathBuf],
    ) -> std::io::Result<BTreeMap<usize, String>> {
        let mut by_id = BTreeMap::new();

        for dir in dirs {
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

                if let Some((id, script_name)) =
                    parse_hgss_script_filename(&file_name).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Failed to parse HGSS script filename '{}' in {}: {e}",
                                file_name,
                                dir.display()
                            ),
                        )
                    })?
                {
                    parsed.push((id, script_name));
                }
            }

            parsed.sort_by_key(|(id, _)| *id);
            for (id, script_name) in parsed {
                by_id.insert(id, script_name);
            }
        }

        Ok(by_id)
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_order_file_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "script table integration test (Platinum decomp)",
        ) else {
            return;
        };

        let order_path = root.join("res/field/scripts/scripts.order");
        if !order_path.exists() {
            eprintln!(
                "Skipping script table integration test (Platinum decomp): missing {}",
                order_path.display()
            );
            return;
        }

        let expected = parse_scripts_order_entries(&order_path).unwrap();
        assert!(
            !expected.is_empty(),
            "expected non-empty scripts.order entries from {}",
            order_path.display()
        );

        let mut table = ScriptTable::new();
        table.load_order_file(&order_path).unwrap();

        assert_eq!(table.get_all_names().len(), expected.len());

        let mut sample_indices = vec![0, expected.len() / 2, expected.len().saturating_sub(1)];
        sample_indices.sort_unstable();
        sample_indices.dedup();
        for idx in sample_indices {
            let name = &expected[idx];
            assert_eq!(table.get_name(idx), Some(name.as_str()));
            assert_eq!(table.get_id(name), Some(idx));
        }
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_hgss_script_dir_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "script table integration test (HGSS decomp)",
        ) else {
            return;
        };

        // Keep the same precedence as workspace decomp bootstrap.
        let dirs = vec![
            root.join("files/fielddata/script/scr_seq"),
            root.join("files/fielddata/script"),
        ];

        let expected = collect_hgss_script_names_with_precedence(&dirs).unwrap();
        if expected.is_empty() {
            eprintln!(
                "Skipping script table integration test (HGSS decomp): no scr_seq_XXXX*.s files under {}",
                root.display()
            );
            return;
        }

        let mut table = ScriptTable::new();
        for dir in &dirs {
            if dir.exists() {
                table.load_hgss_script_dir(dir).unwrap();
            }
        }

        let (first_id, first_name) = expected.first_key_value().unwrap();
        assert_eq!(table.get_name(*first_id), Some(first_name.as_str()));
        assert_eq!(table.get_id(first_name), Some(*first_id));

        let (last_id, last_name) = expected.last_key_value().unwrap();
        assert_eq!(table.get_name(*last_id), Some(last_name.as_str()));
        assert_eq!(table.get_id(last_name), Some(*last_id));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_unique_names_roundtrip(names in unique_names_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push('\n');
            }

            let mut table = ScriptTable::new();
            table.load_order_str(&content).unwrap();

            prop_assert_eq!(table.get_all_names().len(), names.len());
            for (idx, name) in names.iter().enumerate() {
                prop_assert_eq!(table.get_name(idx), Some(name.as_str()));
                prop_assert_eq!(table.get_id(name), Some(idx));
            }
        }

        #[test]
        fn prop_trimmed_lines_and_comments_preserve_order(
            names in unique_names_strategy(),
            add_comment_before in prop::collection::vec(any::<bool>(), 0..64),
            add_blank_before in prop::collection::vec(any::<bool>(), 0..64)
        ) {
            let mut content = String::new();
            for (idx, name) in names.iter().enumerate() {
                if add_comment_before.get(idx).copied().unwrap_or(false) {
                    content.push_str("   # synthetic comment\n");
                }
                if add_blank_before.get(idx).copied().unwrap_or(false) {
                    content.push('\n');
                }
                content.push_str("   ");
                content.push_str(name);
                content.push_str("   \n");
            }

            let mut table = ScriptTable::new();
            table.load_order_str(&content).unwrap();

            prop_assert_eq!(table.get_all_names(), names.as_slice());
        }

        #[test]
        fn prop_duplicate_name_lookup_is_last_wins(names in names_with_duplicates_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push('\n');
            }

            let mut table = ScriptTable::new();
            table.load_order_str(&content).unwrap();

            for (idx, name) in names.iter().enumerate() {
                let expected_last = names.iter().rposition(|n| n == name).unwrap();
                prop_assert_eq!(table.get_name(idx), Some(name.as_str()));
                prop_assert_eq!(table.get_id(name), Some(expected_last));
            }
        }

        #[test]
        fn prop_unknown_or_oob_queries_return_none(names in unique_names_strategy()) {
            let mut content = String::new();
            for name in &names {
                content.push_str(name);
                content.push('\n');
            }

            let mut table = ScriptTable::new();
            table.load_order_str(&content).unwrap();

            let unknown = "scripts_definitely_missing";
            prop_assert_eq!(table.get_id(unknown), None);
            prop_assert_eq!(table.get_name(names.len()), None);
            prop_assert_eq!(table.get_name(names.len().saturating_add(100)), None);
        }
    }

    #[test]
    fn test_load_dspre_script_dir() {
        let dir = tempdir().unwrap();
        // Both .script (raw DSPRE export) and .rotom (post-conversion) are accepted.
        fs::write(dir.path().join("0213.script"), "").unwrap();
        fs::write(dir.path().join("0000.rotom"), "").unwrap();
        fs::write(dir.path().join("readme.txt"), "").unwrap();
        fs::write(dir.path().join("not_a_number.rotom"), "").unwrap();

        let mut table = ScriptTable::new();
        table.load_dspre_script_dir(dir.path()).unwrap();

        assert_eq!(table.get_name(213), Some("0213"));
        assert_eq!(table.get_name(0), Some("0000"));
        assert_eq!(table.get_id("0213"), Some(213));
        assert_eq!(table.get_id("0000"), Some(0));
        // Non-numeric stem skipped.
        assert_eq!(table.get_id("not_a_number"), None);
        // Wrong extension skipped.
        assert_eq!(table.get_id("readme"), None);
    }
}
