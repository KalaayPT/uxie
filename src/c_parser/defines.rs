use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CDefine {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<i64>,
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
                resolved: None,
            });
        }
    }

    defines
}

pub fn parse_and_resolve_defines(source: &str) -> Vec<CDefine> {
    let mut defines = parse_defines(source);
    for i in 0..defines.len() {
        let val = parse_value(&defines[i].value, &defines);
        defines[i].resolved = val;
    }
    defines
}

#[allow(dead_code)]
pub fn resolve_define_value(defines: &[CDefine], name: &str) -> Option<i64> {
    let def = defines.iter().find(|d| d.name == name)?;
    parse_value(&def.value, defines)
}

/// Parse a C constant expression, resolving references and evaluating operators.
/// Supports: literals, hex, references, (A << B), (A | B | ...), parentheses
pub fn parse_value(value: &str, defines: &[CDefine]) -> Option<i64> {
    eval_expr(value.trim(), defines)
}

fn eval_expr(expr: &str, defines: &[CDefine]) -> Option<i64> {
    let expr = expr.trim();
    
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len()-1];
        if is_balanced(inner) {
            return eval_expr(inner, defines);
        }
    }
    
    if let Some(result) = try_eval_or(expr, defines) {
        return Some(result);
    }
    
    if let Some(result) = try_eval_shift(expr, defines) {
        return Some(result);
    }
    
    if expr.starts_with("0x") || expr.starts_with("0X") {
        return i64::from_str_radix(&expr[2..], 16).ok();
    }
    
    if let Ok(n) = expr.parse::<i64>() {
        return Some(n);
    }
    
    if let Some(other_def) = defines.iter().find(|d| d.name == expr) {
        return eval_expr(&other_def.value, defines);
    }
    
    None
}

fn is_balanced(s: &str) -> bool {
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 { return false; }
            }
            _ => {}
        }
    }
    depth == 0
}

fn try_eval_or(expr: &str, defines: &[CDefine]) -> Option<i64> {
    let parts = split_on_operator(expr, '|');
    if parts.len() < 2 {
        return None;
    }
    
    let mut result = 0i64;
    for part in parts {
        let val = eval_expr(part.trim(), defines)?;
        result |= val;
    }
    Some(result)
}

fn try_eval_shift(expr: &str, defines: &[CDefine]) -> Option<i64> {
    let mut depth = 0;
    let bytes = expr.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'<' if depth == 0 && bytes.get(i+1) == Some(&b'<') => {
                let left = &expr[..i];
                let right = &expr[i+2..];
                let lval = eval_expr(left.trim(), defines)?;
                let rval = eval_expr(right.trim(), defines)?;
                return Some(lval << rval);
            }
            _ => {}
        }
    }
    None
}

fn split_on_operator(expr: &str, op: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    
    for (i, c) in expr.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c == op && depth == 0 => {
                parts.push(&expr[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&expr[start..]);
    parts
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
            CDefine { name: "A".into(), value: "10".into(), resolved: None },
            CDefine { name: "B".into(), value: "A".into(), resolved: None },
        ];

        assert_eq!(resolve_define_value(&defines, "A"), Some(10));
        assert_eq!(resolve_define_value(&defines, "B"), Some(10));
    }

    #[test]
    fn test_bit_shift_expressions() {
        let defines = vec![];
        
        assert_eq!(parse_value("(1 << 0)", &defines), Some(1));
        assert_eq!(parse_value("(1 << 1)", &defines), Some(2));
        assert_eq!(parse_value("(1 << 10)", &defines), Some(1024));
        assert_eq!(parse_value("(0 << 0)", &defines), Some(0));
        assert_eq!(parse_value("(1 << 31)", &defines), Some(1 << 31));
    }

    #[test]
    fn test_bitwise_or_expressions() {
        let defines = vec![
            CDefine { name: "A".into(), value: "(1 << 0)".into(), resolved: None },
            CDefine { name: "B".into(), value: "(1 << 1)".into(), resolved: None },
            CDefine { name: "C".into(), value: "(1 << 2)".into(), resolved: None },
        ];
        
        assert_eq!(parse_value("(A | B)", &defines), Some(3));
        assert_eq!(parse_value("(A | B | C)", &defines), Some(7));
    }

    #[test]
    fn test_nested_references() {
        let defines = vec![
            CDefine { name: "TRAINER".into(), value: "(1 << 0)".into(), resolved: None },
            CDefine { name: "DOUBLES".into(), value: "(1 << 1)".into(), resolved: None },
            CDefine { name: "TRAINER_DOUBLES".into(), value: "(DOUBLES | TRAINER)".into(), resolved: None },
            CDefine { name: "LINK".into(), value: "(1 << 2)".into(), resolved: None },
            CDefine { name: "LINK_DOUBLES".into(), value: "(LINK | TRAINER_DOUBLES)".into(), resolved: None },
        ];
        
        assert_eq!(parse_value("TRAINER", &defines), Some(1));
        assert_eq!(parse_value("DOUBLES", &defines), Some(2));
        assert_eq!(parse_value("TRAINER_DOUBLES", &defines), Some(3));
        assert_eq!(parse_value("LINK_DOUBLES", &defines), Some(7));
    }
}
