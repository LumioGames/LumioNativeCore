//! Test-only JSON reader for the vendored Root ABI mirror files
//! (`docs/architecture/abi/`, see its README for the pinned revision).
//!
//! Strict recursive descent over the machine-generated mirrors; any
//! malformed byte panics the test. This is deliberately not a public or
//! reusable parser — production code must never parse the mirrors at
//! runtime, the crate binds generated constants instead.

#![allow(dead_code)]

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Obj(pairs) => pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("missing key `{key}`")),
            other => panic!("get(`{key}`) on non-object {other:?}"),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Json::Str(s) => s,
            other => panic!("expected string, got {other:?}"),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Json::Num(n) => {
                let v = *n as i64;
                assert!((v as f64 - n).abs() < f64::EPSILON, "non-integer {n}");
                v
            }
            other => panic!("expected number, got {other:?}"),
        }
    }

    pub fn as_arr(&self) -> &[Json] {
        match self {
            Json::Arr(items) => items,
            other => panic!("expected array, got {other:?}"),
        }
    }
}

pub fn mirror_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/architecture/abi")
        .join(file_name)
}

pub fn parse_mirror(file_name: &str) -> Json {
    let path = mirror_path(file_name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read mirror {}: {e}", path.display()));
    parse(&text)
}

pub fn parse(text: &str) -> Json {
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let value = parse_value(bytes, &mut pos);
    skip_ws(bytes, &mut pos);
    assert_eq!(pos, bytes.len(), "trailing bytes after JSON document");
    value
}

fn skip_ws(b: &[u8], pos: &mut usize) {
    while *pos < b.len() && matches!(b[*pos], b' ' | b'\t' | b'\n' | b'\r') {
        *pos += 1;
    }
}

fn expect(b: &[u8], pos: &mut usize, byte: u8) {
    assert!(
        *pos < b.len() && b[*pos] == byte,
        "expected `{}` at byte {}",
        byte as char,
        *pos
    );
    *pos += 1;
}

fn parse_value(b: &[u8], pos: &mut usize) -> Json {
    skip_ws(b, pos);
    assert!(*pos < b.len(), "unexpected end of JSON");
    match b[*pos] {
        b'{' => parse_obj(b, pos),
        b'[' => parse_arr(b, pos),
        b'"' => Json::Str(parse_string(b, pos)),
        b't' => parse_lit(b, pos, "true", Json::Bool(true)),
        b'f' => parse_lit(b, pos, "false", Json::Bool(false)),
        b'n' => parse_lit(b, pos, "null", Json::Null),
        _ => parse_num(b, pos),
    }
}

fn parse_lit(b: &[u8], pos: &mut usize, lit: &str, value: Json) -> Json {
    assert!(
        b[*pos..].starts_with(lit.as_bytes()),
        "bad literal at byte {}",
        *pos
    );
    *pos += lit.len();
    value
}

fn parse_obj(b: &[u8], pos: &mut usize) -> Json {
    expect(b, pos, b'{');
    let mut pairs = Vec::new();
    skip_ws(b, pos);
    if *pos < b.len() && b[*pos] == b'}' {
        *pos += 1;
        return Json::Obj(pairs);
    }
    loop {
        skip_ws(b, pos);
        let key = parse_string(b, pos);
        skip_ws(b, pos);
        expect(b, pos, b':');
        pairs.push((key, parse_value(b, pos)));
        skip_ws(b, pos);
        match b.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b'}') => {
                *pos += 1;
                return Json::Obj(pairs);
            }
            other => panic!("expected `,` or `}}`, got {other:?} at byte {}", *pos),
        }
    }
}

fn parse_arr(b: &[u8], pos: &mut usize) -> Json {
    expect(b, pos, b'[');
    let mut items = Vec::new();
    skip_ws(b, pos);
    if *pos < b.len() && b[*pos] == b']' {
        *pos += 1;
        return Json::Arr(items);
    }
    loop {
        items.push(parse_value(b, pos));
        skip_ws(b, pos);
        match b.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b']') => {
                *pos += 1;
                return Json::Arr(items);
            }
            other => panic!("expected `,` or `]`, got {other:?} at byte {}", *pos),
        }
    }
}

fn parse_string(b: &[u8], pos: &mut usize) -> String {
    expect(b, pos, b'"');
    let mut out = String::new();
    loop {
        assert!(*pos < b.len(), "unterminated string");
        match b[*pos] {
            b'"' => {
                *pos += 1;
                return out;
            }
            b'\\' => {
                *pos += 1;
                match b.get(*pos) {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'r') => out.push('\r'),
                    // The mirrors carry no other escapes; fail loud if one appears.
                    other => panic!("unsupported escape {other:?} at byte {}", *pos),
                }
                *pos += 1;
            }
            _ => {
                let ch = std::str::from_utf8(&b[*pos..])
                    .expect("utf-8 mirror")
                    .chars()
                    .next()
                    .expect("non-empty");
                out.push(ch);
                *pos += ch.len_utf8();
            }
        }
    }
}

fn parse_num(b: &[u8], pos: &mut usize) -> Json {
    let start = *pos;
    while *pos < b.len() && matches!(b[*pos], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
        *pos += 1;
    }
    let text = std::str::from_utf8(&b[start..*pos]).expect("utf-8 number");
    Json::Num(
        text.parse()
            .unwrap_or_else(|e| panic!("bad number `{text}`: {e}")),
    )
}
