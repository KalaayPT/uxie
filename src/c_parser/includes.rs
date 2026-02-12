use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static SYSTEM_INCLUDE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#include\s*<([^>]+)>").unwrap());

static LOCAL_INCLUDE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"#include\s*"([^"]+)""#).unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CInclude {
    pub path: String,
    pub is_system: bool,
}

pub fn parse_includes(source: &str) -> Vec<CInclude> {
    let mut includes = Vec::new();

    for line in source.lines() {
        let line = line.trim();

        if let Some(caps) = SYSTEM_INCLUDE_PATTERN.captures(line) {
            includes.push(CInclude {
                path: caps[1].to_string(),
                is_system: true,
            });
        } else if let Some(caps) = LOCAL_INCLUDE_PATTERN.captures(line) {
            includes.push(CInclude {
                path: caps[1].to_string(),
                is_system: false,
            });
        }
    }

    includes
}

pub fn resolve_includes(
    file_path: &Path,
    include_dirs: &[PathBuf],
    visited: &mut HashSet<PathBuf>,
) -> std::io::Result<Vec<PathBuf>> {
    let canonical = file_path.canonicalize()?;

    if visited.contains(&canonical) {
        return Ok(Vec::new());
    }
    visited.insert(canonical);

    let source = std::fs::read_to_string(file_path)?;
    let includes = parse_includes(&source);
    let parent_dir = file_path.parent().unwrap_or(Path::new("."));

    let mut resolved = Vec::new();

    for inc in includes {
        if inc.is_system {
            continue;
        }

        let mut found_path = None;

        let relative_path = parent_dir.join(&inc.path);
        if relative_path.exists() {
            found_path = Some(relative_path);
        }

        if found_path.is_none() {
            for dir in include_dirs {
                let path = dir.join(&inc.path);
                if path.exists() {
                    found_path = Some(path);
                    break;
                }
            }
        }

        if let Some(path) = found_path {
            resolved.push(path.clone());
            resolved.extend(resolve_includes(&path, include_dirs, visited)?);
        }
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_includes() {
        let source = r#"
#include <stdio.h>
#include "map_header.h"
#include "generated/map_headers.h"
        "#;

        let includes = parse_includes(source);
        assert_eq!(includes.len(), 3);
        assert!(includes[0].is_system);
        assert!(!includes[1].is_system);
        assert_eq!(includes[1].path, "map_header.h");
    }
}
