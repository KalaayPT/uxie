use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CDefine {
    pub name: String,
    pub value: String,
}

pub fn parse_defines(source: &str) -> Vec<CDefine> {
    let mut defines = Vec::new();
    
    let pattern = Regex::new(
        r"#define\s+([A-Za-z_][A-Za-z0-9_]*)\s+(.+?)(?:\s*(?://|/\*).*)?$"
    ).unwrap();

    for line in source.lines() {
        let line = line.trim();
        if let Some(caps) = pattern.captures(line) {
            defines.push(CDefine {
                name: caps[1].to_string(),
                value: caps[2].trim().to_string(),
            });
        }
    }

    defines
}

#[allow(dead_code)]
pub fn resolve_define_value(defines: &[CDefine], name: &str) -> Option<i64> {
    let def = defines.iter().find(|d| d.name == name)?;
    parse_value(&def.value, defines)
}

#[allow(dead_code)]
fn parse_value(value: &str, defines: &[CDefine]) -> Option<i64> {
    let v = value.trim();
    
    if v.starts_with("0x") || v.starts_with("0X") {
        return i64::from_str_radix(&v[2..], 16).ok();
    }
    
    if let Ok(n) = v.parse::<i64>() {
        return Some(n);
    }
    
    if let Some(other_def) = defines.iter().find(|d| d.name == v) {
        return parse_value(&other_def.value, defines);
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_defines() {
        let source = r#"
#define ENCOUNTERS_NONE 0xFFFF
#define MAX_HEADERS 593
#define SOME_VALUE 0x10
        "#;

        let defines = parse_defines(source);
        assert_eq!(defines.len(), 3);
        assert_eq!(defines[0].name, "ENCOUNTERS_NONE");
        assert_eq!(defines[0].value, "0xFFFF");
    }

    #[test]
    fn test_resolve_define() {
        let defines = vec![
            CDefine { name: "A".into(), value: "10".into() },
            CDefine { name: "B".into(), value: "A".into() },
        ];

        assert_eq!(resolve_define_value(&defines, "A"), Some(10));
        assert_eq!(resolve_define_value(&defines, "B"), Some(10));
    }
}
