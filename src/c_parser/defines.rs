use dashmap::DashMap;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::BuildHasher;

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
        if !line.starts_with("#define") {
            continue;
        }
        let rest = line[7..].trim();
        if rest.is_empty() {
            continue;
        }

        let mut name = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '(' {
                break;
            }
            name.push(c);
            chars.next();
        }
        if name.is_empty() {
            continue;
        }
        if chars.peek() == Some(&'(') {
            continue;
        }

        let mut value = chars.collect::<String>().trim().to_string();
        if let Some(pos) = value.find("//") {
            value = value[..pos].trim().to_string();
        }
        if let Some(pos) = value.find("/*") {
            value = value[..pos].trim().to_string();
        }

        if !value.is_empty() {
            defines.push(CDefine {
                name,
                value,
                resolved: None,
            });
        }
    }
    defines
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Number(i64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    And,
    Or,
    Xor,
    Tilde,
    Not,
    LShift,
    RShift,
    LParen,
    RParen,
    Comma,
    Invalid,
    Eof,
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Token::Eof;
        }

        let b = self.input[self.pos];
        match b {
            b'0'..=b'9' => self.read_number(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.read_ident(),
            b'+' => {
                self.pos += 1;
                Token::Plus
            }
            b'-' => {
                self.pos += 1;
                Token::Minus
            }
            b'*' => {
                self.pos += 1;
                Token::Star
            }
            b'/' => {
                self.pos += 1;
                Token::Slash
            }
            b'%' => {
                self.pos += 1;
                Token::Percent
            }
            b'&' => {
                self.pos += 1;
                Token::And
            }
            b'|' => {
                self.pos += 1;
                Token::Or
            }
            b'^' => {
                self.pos += 1;
                Token::Xor
            }
            b'~' => {
                self.pos += 1;
                Token::Tilde
            }
            b'!' => {
                self.pos += 1;
                Token::Not
            }
            b'<' => {
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == b'<' {
                    self.pos += 2;
                    Token::LShift
                } else {
                    self.pos += 1;
                    Token::Invalid
                }
            }
            b'>' => {
                if self.pos + 1 < self.input.len() && self.input[self.pos + 1] == b'>' {
                    self.pos += 2;
                    Token::RShift
                } else {
                    self.pos += 1;
                    Token::Invalid
                }
            }
            b'(' => {
                self.pos += 1;
                Token::LParen
            }
            b')' => {
                self.pos += 1;
                Token::RParen
            }
            b',' => {
                self.pos += 1;
                Token::Comma
            }
            _ => {
                self.pos += 1;
                Token::Invalid
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        if self.pos + 2 < self.input.len()
            && self.input[self.pos] == b'0'
            && (self.input[self.pos + 1] == b'x' || self.input[self.pos + 1] == b'X')
        {
            self.pos += 2;
            let hex_start = self.pos;
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_hexdigit() {
                self.pos += 1;
            }
            if hex_start == self.pos {
                return Token::Invalid;
            }

            let Ok(hex_str) = std::str::from_utf8(&self.input[hex_start..self.pos]) else {
                return Token::Invalid;
            };
            let Ok(value) = i64::from_str_radix(hex_str, 16) else {
                return Token::Invalid;
            };
            return Token::Number(value);
        }

        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let Ok(num_str) = std::str::from_utf8(&self.input[start..self.pos]) else {
            return Token::Invalid;
        };
        let Ok(value) = num_str.parse::<i64>() else {
            return Token::Invalid;
        };
        Token::Number(value)
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_ascii_alphanumeric() || self.input[self.pos] == b'_')
        {
            self.pos += 1;
        }
        let Ok(ident) = std::str::from_utf8(&self.input[start..self.pos]) else {
            return Token::Invalid;
        };
        Token::Ident(ident.to_string())
    }
}

pub fn eval_expr_with_context<SExpr, SResolved>(
    expr: &str,
    expressions: &HashMap<String, String, SExpr>,
    resolved: &HashMap<String, i64, SResolved>,
    cache: &DashMap<String, i64>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let mut visiting = FxHashSet::default();
    eval_expr_recursive(
        expr,
        expressions,
        resolved,
        cache,
        &mut visiting,
        0,
        &|_| None,
    )
}

pub fn eval_expr_with_parent<SExpr, SResolved>(
    expr: &str,
    expressions: &HashMap<String, String, SExpr>,
    resolved: &HashMap<String, i64, SResolved>,
    cache: &DashMap<String, i64>,
    parent_resolver: &dyn Fn(&str) -> Option<i64>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let mut visiting = FxHashSet::default();
    eval_expr_recursive(
        expr,
        expressions,
        resolved,
        cache,
        &mut visiting,
        0,
        parent_resolver,
    )
}

fn eval_expr_recursive<SExpr, SResolved>(
    expr: &str,
    expressions: &HashMap<String, String, SExpr>,
    resolved: &HashMap<String, i64, SResolved>,
    cache: &DashMap<String, i64>,
    visiting: &mut FxHashSet<String>,
    depth: usize,
    parent_resolver: &dyn Fn(&str) -> Option<i64>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    const MAX_DEPTH: usize = 128;
    if depth > MAX_DEPTH {
        return None;
    }

    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }
    if let Some(&val) = resolved.get(expr) {
        return Some(val);
    }
    if let Some(cached) = cache.get(expr) {
        return Some(*cached);
    }
    if let Some(val) = parent_resolver(expr) {
        return Some(val);
    }

    if let Some(val) = try_parse_numeric(expr) {
        return Some(val);
    }

    if !visiting.insert(expr.to_string()) {
        return None;
    }

    let mut tokenizer = Tokenizer::new(expr);
    let mut tokens = Vec::with_capacity(8);
    let mut has_invalid_token = false;
    loop {
        let tok = tokenizer.next_token();
        match tok {
            Token::Eof => break,
            Token::Invalid => {
                has_invalid_token = true;
                break;
            }
            _ => tokens.push(tok),
        }
    }

    let result = if has_invalid_token || tokens.is_empty() {
        None
    } else {
        let mut pos = 0;
        let mut ctx = ParserContext {
            expressions,
            resolved,
            cache,
            visiting,
            depth,
            parent_resolver,
        };
        let val = parse_expr(&tokens, &mut pos, 0, &mut ctx);
        if pos == tokens.len() { val } else { None }
    };

    visiting.remove(expr);

    if let Some(val) = result {
        cache.insert(expr.to_string(), val);
    }
    result
}

fn try_parse_numeric(s: &str) -> Option<i64> {
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse().ok()
    }
}

fn get_precedence(tok: &Token) -> u8 {
    match tok {
        Token::Or => 1,
        Token::Xor => 2,
        Token::And => 3,
        Token::LShift | Token::RShift => 4,
        Token::Plus | Token::Minus => 5,
        Token::Star | Token::Slash | Token::Percent => 6,
        _ => 0,
    }
}

struct ParserContext<'a, SExpr, SResolved>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    expressions: &'a HashMap<String, String, SExpr>,
    resolved: &'a HashMap<String, i64, SResolved>,
    cache: &'a DashMap<String, i64>,
    visiting: &'a mut FxHashSet<String>,
    depth: usize,
    parent_resolver: &'a dyn Fn(&str) -> Option<i64>,
}

fn apply_binary_op(op: &Token, left: i64, right: i64) -> Option<i64> {
    match op {
        Token::Plus => Some(left + right),
        Token::Minus => Some(left - right),
        Token::Star => Some(left * right),
        Token::Slash => (right != 0).then_some(left / right),
        Token::Percent => (right != 0).then_some(left % right),
        Token::And => Some(left & right),
        Token::Or => Some(left | right),
        Token::Xor => Some(left ^ right),
        Token::LShift => Some(left << right),
        Token::RShift => Some(left >> right),
        _ => Some(left),
    }
}

fn expect_token(tokens: &[Token], pos: &mut usize, expected: Token) -> bool {
    if *pos < tokens.len() && tokens[*pos] == expected {
        *pos += 1;
        true
    } else {
        false
    }
}

fn parse_expr<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    min_prec: u8,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let mut left = parse_primary(tokens, pos, ctx)?;

    while *pos < tokens.len() {
        let prec = get_precedence(&tokens[*pos]);
        if prec < min_prec || prec == 0 {
            break;
        }

        let op = tokens[*pos].clone();
        *pos += 1;
        let right = parse_expr(tokens, pos, prec + 1, ctx)?;
        left = apply_binary_op(&op, left, right)?;
    }

    Some(left)
}

fn parse_rgb_call<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let r = parse_expr(tokens, pos, 0, ctx)?;
    if !expect_token(tokens, pos, Token::Comma) {
        return None;
    }
    let g = parse_expr(tokens, pos, 0, ctx)?;
    if !expect_token(tokens, pos, Token::Comma) {
        return None;
    }
    let b = parse_expr(tokens, pos, 0, ctx)?;
    if !expect_token(tokens, pos, Token::RParen) {
        return None;
    }
    Some((b << 10) | (g << 5) | r)
}

fn resolve_identifier<SExpr, SResolved>(
    id: &str,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    if let Some(&val) = ctx.resolved.get(id) {
        return Some(val);
    }
    if let Some(cached) = ctx.cache.get(id) {
        return Some(*cached);
    }
    if let Some(val) = (ctx.parent_resolver)(id) {
        return Some(val);
    }
    if let Some(expr) = ctx.expressions.get(id) {
        return eval_expr_recursive(
            expr,
            ctx.expressions,
            ctx.resolved,
            ctx.cache,
            ctx.visiting,
            ctx.depth + 1,
            ctx.parent_resolver,
        );
    }
    None
}

fn parse_identifier<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    id: &str,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    if id == "RGB" && *pos + 1 < tokens.len() && tokens[*pos + 1] == Token::LParen {
        *pos += 2;
        return parse_rgb_call(tokens, pos, ctx);
    }
    *pos += 1;
    resolve_identifier(id, ctx)
}

fn parse_parenthesized<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    *pos += 1;
    let val = parse_expr(tokens, pos, 0, ctx)?;
    if !expect_token(tokens, pos, Token::RParen) {
        return None;
    }
    Some(val)
}

fn parse_unary<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let op = tokens[*pos].clone();
    *pos += 1;
    let val = parse_primary(tokens, pos, ctx)?;
    match op {
        Token::Plus => Some(val),
        Token::Minus => Some(-val),
        Token::Tilde => Some(!val),
        Token::Not => Some(i64::from(val == 0)),
        _ => None,
    }
}

fn parse_primary<SExpr, SResolved>(
    tokens: &[Token],
    pos: &mut usize,
    ctx: &mut ParserContext<'_, SExpr, SResolved>,
) -> Option<i64>
where
    SExpr: BuildHasher,
    SResolved: BuildHasher,
{
    let token = tokens.get(*pos)?;
    match token {
        Token::Number(n) => {
            *pos += 1;
            Some(*n)
        }
        Token::Ident(id) => parse_identifier(tokens, pos, id, ctx),
        Token::LParen => parse_parenthesized(tokens, pos, ctx),
        Token::Plus | Token::Minus | Token::Tilde | Token::Not => parse_unary(tokens, pos, ctx),
        _ => None,
    }
}

pub fn parse_and_resolve_defines(source: &str) -> Vec<CDefine> {
    let mut defines = parse_defines(source);
    let exprs: FxHashMap<String, String> = defines
        .iter()
        .map(|d| (d.name.clone(), d.value.clone()))
        .collect();
    let cache = DashMap::new();
    let res = FxHashMap::default();
    for def in &mut defines {
        def.resolved = eval_expr_with_context(&def.value, &exprs, &res, &cache);
    }
    defines
}

pub fn parse_value(value: &str, defines: &[CDefine]) -> Option<i64> {
    let exprs: FxHashMap<String, String> = defines
        .iter()
        .map(|d| (d.name.clone(), d.value.clone()))
        .collect();
    let cache = DashMap::new();
    let res = FxHashMap::default();
    eval_expr_with_context(value.trim(), &exprs, &res, &cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_defines() {
        let source = r"
#define ENCOUNTERS_NONE 0xFFFF
#define MAX_HEADERS 593
#define SOME_VALUE 0x10
        ";
        let defines = parse_defines(source);
        assert_eq!(defines.len(), 3);
        assert_eq!(defines[0].name, "ENCOUNTERS_NONE");
        assert_eq!(defines[0].value, "0xFFFF");
    }

    #[test]
    fn test_resolve_defines() {
        let source = r"
#define A 10
#define B A + 5
#define C (A | B)
        ";
        let defines = parse_and_resolve_defines(source);
        assert_eq!(
            defines
                .iter()
                .find(|d| d.name == "B")
                .and_then(|d| d.resolved),
            Some(15)
        );
        assert_eq!(
            defines
                .iter()
                .find(|d| d.name == "C")
                .and_then(|d| d.resolved),
            Some(10 | 15)
        );
    }

    #[test]
    fn test_bitwise_precedence() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();

        assert_eq!(
            eval_expr_with_context("1 | 2 << 1", &exprs, &res, &cache),
            Some(1 | (2 << 1))
        );
        assert_eq!(
            eval_expr_with_context("1 & 2 << 1", &exprs, &res, &cache),
            Some(1 & (2 << 1))
        );
        assert_eq!(
            eval_expr_with_context("1 << 1 | 2", &exprs, &res, &cache),
            Some((1 << 1) | 2)
        );
    }

    #[test]
    fn test_rgb_macro() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();
        assert_eq!(
            eval_expr_with_context("RGB(31, 0, 0)", &exprs, &res, &cache),
            Some(31)
        );
        assert_eq!(
            eval_expr_with_context("RGB(0, 31, 0)", &exprs, &res, &cache),
            Some(31 << 5)
        );
        assert_eq!(
            eval_expr_with_context("RGB(0, 0, 31)", &exprs, &res, &cache),
            Some(31 << 10)
        );
    }

    #[test]
    fn test_nested_rgb() {
        let cache = DashMap::new();
        let mut exprs = FxHashMap::default();
        exprs.insert("R".to_string(), "31".to_string());
        let res = FxHashMap::default();
        assert_eq!(
            eval_expr_with_context("RGB(R, 0, 0)", &exprs, &res, &cache),
            Some(31)
        );
    }

    #[test]
    fn test_cycle_detection() {
        let cache = DashMap::new();
        let mut exprs = FxHashMap::default();
        exprs.insert("A".to_string(), "B".to_string());
        exprs.insert("B".to_string(), "A".to_string());
        let res = FxHashMap::default();
        assert_eq!(eval_expr_with_context("A", &exprs, &res, &cache), None);
    }

    #[test]
    fn test_complex_precedence() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();
        assert_eq!(
            eval_expr_with_context("1 + 2 * 3", &exprs, &res, &cache),
            Some(7)
        );
        assert_eq!(
            eval_expr_with_context("(1 + 2) * 3", &exprs, &res, &cache),
            Some(9)
        );
        assert_eq!(
            eval_expr_with_context("1 << 2 + 3", &exprs, &res, &cache),
            Some(32)
        );
    }

    #[test]
    fn test_unsupported_comparison_operators_return_none() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();

        assert_eq!(eval_expr_with_context("1 < 2", &exprs, &res, &cache), None);
        assert_eq!(eval_expr_with_context("1 > 2", &exprs, &res, &cache), None);
    }

    #[test]
    fn test_malformed_numeric_literals_return_none() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();

        assert_eq!(eval_expr_with_context("0x", &exprs, &res, &cache), None);
        assert_eq!(eval_expr_with_context("0xZZ", &exprs, &res, &cache), None);
    }

    #[test]
    fn test_unbalanced_parentheses_return_none() {
        let cache = DashMap::new();
        let exprs = FxHashMap::default();
        let res = FxHashMap::default();

        assert_eq!(eval_expr_with_context("(1 + 2", &exprs, &res, &cache), None);
    }
}
