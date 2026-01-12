use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CDefine {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<i64>,
}

pub fn parse_defines(source: &str) -> Vec<CDefine> {
    let mut defines = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if !line.starts_with("#define") { continue; }
        let rest = line[7..].trim();
        if rest.is_empty() { continue; }
        
        let mut name = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '(' { break; }
            name.push(c);
            chars.next();
        }
        if name.is_empty() { continue; }
        if chars.peek() == Some(&'(') { continue; }
        
        let mut value = chars.collect::<String>().trim().to_string();
        if let Some(pos) = value.find("//") { value = value[..pos].trim().to_string(); }
        if let Some(pos) = value.find("/*") { value = value[..pos].trim().to_string(); }
        
        if !value.is_empty() {
            defines.push(CDefine { name, value, resolved: None });
        }
    }
    defines
}

pub fn parse_and_resolve_defines(source: &str) -> Vec<CDefine> {
    let mut defines = parse_defines(source);
    let exprs: HashMap<String, String> = defines.iter().map(|d| (d.name.clone(), d.value.clone())).collect();
    let mut cache = HashMap::new();
    let res = HashMap::new();
    for i in 0..defines.len() {
        defines[i].resolved = eval_expr_with_context(&defines[i].value, &exprs, &res, &mut cache);
    }
    defines
}

pub fn parse_value(value: &str, defines: &[CDefine]) -> Option<i64> {
    let exprs: HashMap<String, String> = defines.iter().map(|d| (d.name.clone(), d.value.clone())).collect();
    let mut cache = HashMap::new();
    let res = HashMap::new();
    eval_expr_with_context(value.trim(), &exprs, &res, &mut cache)
}

pub fn eval_expr_with_context(
    expr: &str, 
    expressions: &HashMap<String, String>, 
    resolved: &HashMap<String, i64>,
    cache: &mut HashMap<String, i64>
) -> Option<i64> {
    let expr = expr.trim();
    if expr.is_empty() { return None; }
    if let Some(&val) = resolved.get(expr) { return Some(val); }
    if let Some(&cached) = cache.get(expr) { return Some(cached); }
    let result = eval_expr_internal(expr, expressions, resolved, cache);
    if let Some(val) = result { cache.insert(expr.to_string(), val); }
    result
}

fn eval_expr_internal(
    expr: &str, 
    expressions: &HashMap<String, String>, 
    resolved: &HashMap<String, i64>,
    cache: &mut HashMap<String, i64>
) -> Option<i64> {
    let expr = expr.trim();
    if expr.starts_with('(') && expr.ends_with(')') {
        let inner = &expr[1..expr.len() - 1];
        if is_balanced(inner) { return eval_expr_with_context(inner, expressions, resolved, cache); }
    }
    if let Some(result) = try_eval_call(expr, expressions, resolved, cache) { return Some(result); }
    if let Some(result) = try_eval_or(expr, expressions, resolved, cache) { return Some(result); }
    if let Some(result) = try_eval_xor(expr, expressions, resolved, cache) { return Some(result); }
    if let Some(result) = try_eval_bitwise(expr, expressions, resolved, cache) { return Some(result); }
    if let Some(result) = try_eval_add_sub(expr, expressions, resolved, cache) { return Some(result); }
    if let Some(result) = try_eval_shift(expr, expressions, resolved, cache) { return Some(result); }
    if expr.starts_with("0x") || expr.starts_with("0X") { return i64::from_str_radix(&expr[2..], 16).ok(); }
    if let Ok(n) = expr.parse::<i64>() { return Some(n); }
    if let Some(other) = expressions.get(expr) { return eval_expr_with_context(other, expressions, resolved, cache); }
    None
}

fn is_balanced(s: &str) -> bool {
    let mut d = 0i32;
    for c in s.chars() {
        if c == '(' { d += 1; }
        else if c == ')' { d -= 1; if d < 0 { return false; } }
    }
    d == 0
}

fn try_eval_or(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let parts = split_on(e, '|');
    if parts.len() < 2 { return None; }
    let mut val = 0i64;
    for p in parts { val |= eval_expr_with_context(p, exprs, res, cache)?; }
    Some(val)
}

fn try_eval_xor(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let parts = split_on(e, '^');
    if parts.len() < 2 { return None; }
    let mut val = 0i64;
    for (i, p) in parts.iter().enumerate() {
        let v = eval_expr_with_context(p, exprs, res, cache)?;
        if i == 0 { val = v; } else { val ^= v; }
    }
    Some(val)
}

fn try_eval_shift(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let b = e.as_bytes();
    let mut d = 0;
    for i in (0..b.len().saturating_sub(1)).rev() {
        if b[i] == b')' { d += 1; }
        else if b[i] == b'(' { d -= 1; }
        else if d == 0 && i > 0 {
            if b[i] == b'>' && b[i-1] == b'>' {
                return Some(eval_expr_with_context(&e[..i-1], exprs, res, cache)? >> eval_expr_with_context(&e[i+1..], exprs, res, cache)?);
            } else if b[i] == b'<' && b[i-1] == b'<' {
                return Some(eval_expr_with_context(&e[..i-1], exprs, res, cache)? << eval_expr_with_context(&e[i+1..], exprs, res, cache)?);
            }
        }
    }
    None
}

fn try_eval_bitwise(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let b = e.as_bytes();
    let mut d = 0;
    for i in (0..b.len()).rev() {
        if b[i] == b')' { d += 1; }
        else if b[i] == b'(' { d -= 1; }
        else if d == 0 && b[i] == b'&' {
            return Some(eval_expr_with_context(&e[..i], exprs, res, cache)? & eval_expr_with_context(&e[i+1..], exprs, res, cache)?);
        }
    }
    None
}

fn try_eval_add_sub(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let b = e.as_bytes();
    let mut d = 0;
    for i in (0..b.len()).rev() {
        if b[i] == b')' { d += 1; }
        else if b[i] == b'(' { d -= 1; }
        else if d == 0 {
            if b[i] == b'+' {
                return Some(eval_expr_with_context(&e[..i], exprs, res, cache)? + eval_expr_with_context(&e[i+1..], exprs, res, cache)?);
            } else if b[i] == b'-' && i > 0 && !b"|&^+-*/%<<>>".contains(&b[i-1]) {
                return Some(eval_expr_with_context(&e[..i], exprs, res, cache)? - eval_expr_with_context(&e[i+1..], exprs, res, cache)?);
            }
        }
    }
    None
}

fn try_eval_call(e: &str, exprs: &HashMap<String, String>, res: &HashMap<String, i64>, cache: &mut HashMap<String, i64>) -> Option<i64> {
    let e = e.trim();
    if let Some(p) = e.find('(') {
        if e.ends_with(')') {
            let name = e[..p].trim();
            let args = split_on(&e[p+1..e.len()-1], ',');
            if name == "RGB" && args.len() == 3 {
                let r = eval_expr_with_context(args[0], exprs, res, cache)?;
                let g = eval_expr_with_context(args[1], exprs, res, cache)?;
                let b = eval_expr_with_context(args[2], exprs, res, cache)?;
                return Some((b << 10) | (g << 5) | r);
            }
        }
    }
    None
}

fn split_on(e: &str, op: char) -> Vec<&str> {
    let mut p = Vec::new();
    let (mut d, mut s) = (0, 0);
    for (i, c) in e.char_indices() {
        if c == '(' { d += 1; }
        else if c == ')' { d -= 1; }
        else if c == op && d == 0 { p.push(e[s..i].trim()); s = i + 1; }
    }
    p.push(e[s..].trim());
    p
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
    fn test_resolve_defines() {
        let source = r#"
#define A 10
#define B A + 5
#define C (A | B)
        "#;
        let defines = parse_and_resolve_defines(source);
        assert_eq!(defines.iter().find(|d| d.name == "B").and_then(|d| d.resolved), Some(15));
        assert_eq!(defines.iter().find(|d| d.name == "C").and_then(|d| d.resolved), Some(10 | 15));
    }

    #[test]
    fn test_bitwise_precedence() {
        let mut cache = HashMap::new();
        let exprs = HashMap::new();
        let res = HashMap::new();
        
        assert_eq!(eval_expr_with_context("1 | 2 << 1", &exprs, &res, &mut cache), Some(5));
        assert_eq!(eval_expr_with_context("1 & 2 << 1", &exprs, &res, &mut cache), Some(0));
        assert_eq!(eval_expr_with_context("1 << 1 | 2", &exprs, &res, &mut cache), Some(2));
    }

    #[test]
    fn test_rgb_macro() {
        let mut cache = HashMap::new();
        let exprs = HashMap::new();
        let res = HashMap::new();
        assert_eq!(eval_expr_with_context("RGB(31, 0, 0)", &exprs, &res, &mut cache), Some(31));
        assert_eq!(eval_expr_with_context("RGB(0, 31, 0)", &exprs, &res, &mut cache), Some(31 << 5));
        assert_eq!(eval_expr_with_context("RGB(0, 0, 31)", &exprs, &res, &mut cache), Some(31 << 10));
    }

    #[test]
    fn test_nested_rgb() {
        let mut cache = HashMap::new();
        let mut exprs = HashMap::new();
        exprs.insert("R".to_string(), "31".to_string());
        let res = HashMap::new();
        assert_eq!(eval_expr_with_context("RGB(R, 0, 0)", &exprs, &res, &mut cache), Some(31));
    }
}
