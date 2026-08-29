//! `gen-contracts`：从 `docs/architecture/abi/` 镜像生成
//! `crates/lumio-contract-types/src/generated_data.rs`。
//!
//! 镜像是唯一生成源（钉住的上游 revision 见镜像 README）；生成物不得手改，
//! 只能经本命令更新并与镜像一起提交。`registry_values_are_unique` 等
//! crate 测试与本 crate 的回归测试共同断言生成物与镜像零漂移。
//!
//! 解析器是面向这两个机器生成 JSON 文件的严格最小实现：任何意外字节直接
//! 报错退出，不做宽容恢复。

use std::fmt::Write as _;
use std::path::Path;

// ---------- 最小 JSON 解析 ----------

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
    fn get(&self, key: &str) -> Result<&Json, String> {
        match self {
            Json::Obj(pairs) => pairs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v)
                .ok_or_else(|| format!("missing key `{key}`")),
            other => Err(format!("get(`{key}`) on non-object {other:?}")),
        }
    }

    fn as_str(&self) -> Result<&str, String> {
        match self {
            Json::Str(s) => Ok(s),
            other => Err(format!("expected string, got {other:?}")),
        }
    }

    fn as_i64(&self) -> Result<i64, String> {
        match self {
            Json::Num(n) => {
                let v = *n as i64;
                if (v as f64 - n).abs() >= f64::EPSILON {
                    return Err(format!("non-integer {n}"));
                }
                Ok(v)
            }
            other => Err(format!("expected number, got {other:?}")),
        }
    }

    fn as_arr(&self) -> Result<&[Json], String> {
        match self {
            Json::Arr(items) => Ok(items),
            other => Err(format!("expected array, got {other:?}")),
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let bytes = text.as_bytes();
    let mut pos = 0usize;
    let value = parse_value(bytes, &mut pos)?;
    skip_ws(bytes, &mut pos);
    if pos != bytes.len() {
        return Err(format!("trailing bytes at {pos}"));
    }
    Ok(value)
}

fn skip_ws(b: &[u8], pos: &mut usize) {
    while *pos < b.len() && matches!(b[*pos], b' ' | b'\t' | b'\n' | b'\r') {
        *pos += 1;
    }
}

fn expect(b: &[u8], pos: &mut usize, byte: u8) -> Result<(), String> {
    if *pos >= b.len() || b[*pos] != byte {
        return Err(format!("expected `{}` at byte {}", byte as char, *pos));
    }
    *pos += 1;
    Ok(())
}

fn parse_value(b: &[u8], pos: &mut usize) -> Result<Json, String> {
    skip_ws(b, pos);
    match b.get(*pos) {
        Some(b'{') => parse_obj(b, pos),
        Some(b'[') => parse_arr(b, pos),
        Some(b'"') => Ok(Json::Str(parse_string(b, pos)?)),
        Some(b't') => parse_lit(b, pos, "true", Json::Bool(true)),
        Some(b'f') => parse_lit(b, pos, "false", Json::Bool(false)),
        Some(b'n') => parse_lit(b, pos, "null", Json::Null),
        Some(_) => parse_num(b, pos),
        None => Err("unexpected end of JSON".to_string()),
    }
}

fn parse_lit(b: &[u8], pos: &mut usize, lit: &str, value: Json) -> Result<Json, String> {
    if !b[*pos..].starts_with(lit.as_bytes()) {
        return Err(format!("bad literal at byte {}", *pos));
    }
    *pos += lit.len();
    Ok(value)
}

fn parse_obj(b: &[u8], pos: &mut usize) -> Result<Json, String> {
    expect(b, pos, b'{')?;
    let mut pairs = Vec::new();
    skip_ws(b, pos);
    if b.get(*pos) == Some(&b'}') {
        *pos += 1;
        return Ok(Json::Obj(pairs));
    }
    loop {
        skip_ws(b, pos);
        let key = parse_string(b, pos)?;
        skip_ws(b, pos);
        expect(b, pos, b':')?;
        pairs.push((key, parse_value(b, pos)?));
        skip_ws(b, pos);
        match b.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b'}') => {
                *pos += 1;
                return Ok(Json::Obj(pairs));
            }
            other => return Err(format!("expected `,` or `}}`, got {other:?} at {}", *pos)),
        }
    }
}

fn parse_arr(b: &[u8], pos: &mut usize) -> Result<Json, String> {
    expect(b, pos, b'[')?;
    let mut items = Vec::new();
    skip_ws(b, pos);
    if b.get(*pos) == Some(&b']') {
        *pos += 1;
        return Ok(Json::Arr(items));
    }
    loop {
        items.push(parse_value(b, pos)?);
        skip_ws(b, pos);
        match b.get(*pos) {
            Some(b',') => *pos += 1,
            Some(b']') => {
                *pos += 1;
                return Ok(Json::Arr(items));
            }
            other => return Err(format!("expected `,` or `]`, got {other:?} at {}", *pos)),
        }
    }
}

fn parse_string(b: &[u8], pos: &mut usize) -> Result<String, String> {
    expect(b, pos, b'"')?;
    let mut out = String::new();
    loop {
        match b.get(*pos) {
            Some(b'"') => {
                *pos += 1;
                return Ok(out);
            }
            Some(b'\\') => {
                *pos += 1;
                match b.get(*pos) {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'r') => out.push('\r'),
                    other => return Err(format!("unsupported escape {other:?} at {}", *pos)),
                }
                *pos += 1;
            }
            Some(_) => {
                let ch = std::str::from_utf8(&b[*pos..])
                    .map_err(|e| format!("utf-8: {e}"))?
                    .chars()
                    .next()
                    .ok_or("empty")?;
                out.push(ch);
                *pos += ch.len_utf8();
            }
            None => return Err("unterminated string".to_string()),
        }
    }
}

fn parse_num(b: &[u8], pos: &mut usize) -> Result<Json, String> {
    let start = *pos;
    while *pos < b.len() && matches!(b[*pos], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
        *pos += 1;
    }
    let text = std::str::from_utf8(&b[start..*pos]).map_err(|e| format!("utf-8: {e}"))?;
    text.parse()
        .map(Json::Num)
        .map_err(|e| format!("bad number `{text}`: {e}"))
}

// ---------- 生成 ----------

pub const GENERATED_DATA_REL: &str = "crates/lumio-contract-types/src/generated_data.rs";
const BUNDLE_MIRROR_REL: &str = "docs/architecture/abi/root-abi-bundle.json";
const IDS_MIRROR_REL: &str = "docs/architecture/abi/ids-index.json";
const HEADER_MIRROR_REL: &str = "docs/architecture/abi/lumio_core.h";
/// ADR-046：`ErrorCode` numeric 必须放得进 `lumio_status_t`（int32）。
const STATUS_NUMERIC_MAX: i64 = 2_147_483_647;
/// Header 里 capability 常量的前缀（ADR-040 §7.1 的键空间投影）。
const CAPABILITY_DEFINE_PREFIX: &str = "#define LUMIO_CAPABILITY_";

fn read_mirror(root: &Path, rel: &str) -> Result<Json, String> {
    let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("{rel}: {e}"))
}

fn read_mirror_text(root: &Path, rel: &str) -> Result<String, String> {
    let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// 注册 id 的 SCREAMING_SNAKE 投影，与发布 Header 的宏拼写一致
/// （`HybridCLR` -> `HYBRID_CLR`）。
pub fn screaming_snake(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0
            && c.is_ascii_uppercase()
            && (!chars[i - 1].is_ascii_uppercase()
                || chars.get(i + 1).is_some_and(char::is_ascii_lowercase))
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

/// 镜像 Header 里的 capability 键投影。
pub struct CapabilityDefines {
    /// `(宏名去前缀, 数值)`，例如 `("VOXEL_SPATIAL", 6)`。
    pub keys: Vec<(String, i64)>,
    /// `LUMIO_CAPABILITY_COUNT`，缺失为 `None`。
    pub count: Option<i64>,
}

/// 解析镜像 Header 的 capability 常量。
///
/// `LUMIO_CAPABILITY_BITS` 共用同一前缀但**不是键**——它是 `capability_bits`
/// 标量，掩码还是计数、以及任何 bit 位指派 V1 均未冻结（D-015 只裁了键空间），
/// 因此在这里被显式排除，不参与任何键推导。
pub fn parse_capability_defines(header: &str) -> Result<CapabilityDefines, String> {
    let mut keys = Vec::new();
    let mut count = None;
    for line in header.lines() {
        let Some(rest) = line.strip_prefix(CAPABILITY_DEFINE_PREFIX) else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            return Err(format!("malformed capability define `{line}`"));
        };
        let numeric: i64 = value
            .trim_end_matches('u')
            .parse()
            .map_err(|e| format!("bad capability define `{line}`: {e}"))?;
        match name {
            "BITS" => {}
            "COUNT" => count = Some(numeric),
            _ => keys.push((name.to_string(), numeric)),
        }
    }
    Ok(CapabilityDefines { keys, count })
}

/// 从镜像推导 `generated_data.rs` 的完整内容（确定性输出）。
pub fn derive_generated_data(root: &Path) -> Result<String, String> {
    let bundle = read_mirror(root, BUNDLE_MIRROR_REL)?;

    let mut out = String::new();
    out.push_str(
        "//! @generated by `cargo xtask gen-contracts` — DO NOT EDIT BY HAND.\n\
         //!\n\
         //! Source of truth: the byte-pinned mirrors under `docs/architecture/abi/`\n\
         //! (upstream revision in that directory's README). Regenerate with\n\
         //! `cargo xtask gen-contracts` after a mirror update; commit together.\n\n\
         use crate::generated::{ArchitectureCapabilityKey, ArchitectureErrorCode};\n\
         use crate::layout::{AbiStructGolden, AbiTypeGolden};\n\n",
    );

    // layoutProfile 标量。
    let profile = bundle.get("layoutProfile")?;
    let pointer_bytes = profile.get("pointerBytes")?.as_i64()?;
    let max_alignment = profile.get("maxAlignment")?.as_i64()?;
    writeln!(
        out,
        "pub(crate) const ABI_POINTER_BYTES: u32 = {pointer_bytes};\n\
         pub(crate) const ABI_MAX_ALIGNMENT: u32 = {max_alignment};"
    )
    .unwrap();
    out.push('\n');

    // typeMapping 中的命名 C 类型（lumio_ 前缀），按 C 名去重并要求各行一致。
    let mut named: Vec<(String, i64, i64)> = Vec::new();
    for row in bundle.get("typeMapping")?.as_arr()? {
        let c_name = row.get("c")?.as_str()?;
        if !c_name.starts_with("lumio_") || c_name.contains('*') {
            continue;
        }
        let size = row.get("size")?.as_i64()?;
        let align = row.get("align")?.as_i64()?;
        if let Some(existing) = named.iter().find(|(n, _, _)| n == c_name) {
            if existing.1 != size || existing.2 != align {
                return Err(format!("typeMapping rows disagree for {c_name}"));
            }
            continue;
        }
        named.push((c_name.to_string(), size, align));
    }
    out.push_str("#[rustfmt::skip]\npub(crate) const ABI_TYPE_GOLDEN: &[AbiTypeGolden] = &[\n");
    for (name, size, align) in &named {
        writeln!(
            out,
            "    AbiTypeGolden {{ name: \"{name}\", size: {size}, align: {align} }},"
        )
        .unwrap();
    }
    out.push_str("];\n\n");

    // root + 各 API table 的结构 Golden（成员 = 头部字段与槽位/表指针偏移）。
    // (name, declared_size, minimum_size, members[(name, offset)])
    type StructRow = (String, i64, i64, Vec<(String, i64)>);
    let mut structs: Vec<StructRow> = Vec::new();
    {
        let root_obj = bundle.get("root")?;
        let mut members = Vec::new();
        for field in root_obj.get("fields")?.as_arr()? {
            members.push((
                field.get("name")?.as_str()?.to_string(),
                field.get("offset")?.as_i64()?,
            ));
        }
        for table in root_obj.get("tables")?.as_arr()? {
            members.push((
                table.get("name")?.as_str()?.to_string(),
                table.get("offset")?.as_i64()?,
            ));
        }
        structs.push((
            "lumio_root_api".to_string(),
            root_obj.get("declaredStructSize")?.as_i64()?,
            root_obj.get("minimumStructSize")?.as_i64()?,
            members,
        ));
    }
    for table in bundle.get("tables")?.as_arr()? {
        let mut members = Vec::new();
        for field in table.get("fields")?.as_arr()? {
            members.push((
                field.get("name")?.as_str()?.to_string(),
                field.get("offset")?.as_i64()?,
            ));
        }
        for slot in table.get("slots")?.as_arr()? {
            members.push((
                slot.get("name")?.as_str()?.to_string(),
                slot.get("offset")?.as_i64()?,
            ));
        }
        structs.push((
            table.get("name")?.as_str()?.to_string(),
            table.get("declaredStructSize")?.as_i64()?,
            table.get("minimumStructSize")?.as_i64()?,
            members,
        ));
    }
    out.push_str("#[rustfmt::skip]\npub(crate) const ABI_STRUCT_GOLDEN: &[AbiStructGolden] = &[\n");
    for (name, declared, minimum, members) in &structs {
        writeln!(
            out,
            "    AbiStructGolden {{ name: \"{name}\", declared_size: {declared}, minimum_size: {minimum}, members: &["
        )
        .unwrap();
        for (member, offset) in members {
            writeln!(out, "        (\"{member}\", {offset}),").unwrap();
        }
        out.push_str("    ] },\n");
    }
    out.push_str("];\n\n");

    // 各 API table 的发布版本号（tables[].version）。
    out.push_str("#[rustfmt::skip]\npub(crate) const ABI_TABLE_VERSIONS: &[(&str, u32)] = &[\n");
    for table in bundle.get("tables")?.as_arr()? {
        writeln!(
            out,
            "    (\"{}\", {}),",
            table.get("name")?.as_str()?,
            table.get("version")?.as_i64()?
        )
        .unwrap();
    }
    out.push_str("];\n\n");

    // ids/index.json 的 ErrorCode 命名空间（Architecture 所有；唯一 numeric 权威）。
    let ids = read_mirror(root, IDS_MIRROR_REL)?;
    let error_ns = ids
        .get("namespaces")?
        .as_arr()?
        .iter()
        .find(|ns| ns.get("namespace").and_then(Json::as_str).ok() == Some("ErrorCode"))
        .ok_or("ids-index.json missing ErrorCode namespace")?;
    if error_ns.get("owner")?.as_str()? != "Architecture" {
        return Err("ErrorCode namespace owner is not Architecture".to_string());
    }
    let mut seen_ids: Vec<String> = Vec::new();
    let mut seen_numerics: Vec<i64> = Vec::new();
    out.push_str("#[rustfmt::skip]\npub(crate) const ERROR_CODES: &[ArchitectureErrorCode] = &[\n");
    for value in error_ns.get("values")?.as_arr()? {
        let id = value.get("id")?.as_str()?;
        let numeric = value.get("numeric")?.as_i64()?;
        let status = value.get("status")?.as_str()?;
        if status != "Active" {
            return Err(format!("ErrorCode {id} has unexpected status {status}"));
        }
        if !(1..=STATUS_NUMERIC_MAX).contains(&numeric) {
            return Err(format!(
                "ErrorCode {id} numeric {numeric} out of status range"
            ));
        }
        if seen_ids.iter().any(|s| s == id) || seen_numerics.contains(&numeric) {
            return Err(format!("ErrorCode duplicate id/numeric: {id}/{numeric}"));
        }
        seen_ids.push(id.to_string());
        seen_numerics.push(numeric);
        writeln!(out, "    ArchitectureErrorCode::new(\"{id}\", {numeric}),").unwrap();
    }
    out.push_str("];\n\n");

    // ids/index.json 的 Capability 命名空间（Architecture 所有）。D-015 裁决
    // （ADR-040 §7.1）：注册表是键空间唯一权威，架构生成器是唯一发射方，下游
    // 消费投影；仓内私有键值表即违规。numeric 是枚举序号，不是 bit 位。
    let capability_ns = ids
        .get("namespaces")?
        .as_arr()?
        .iter()
        .find(|ns| ns.get("namespace").and_then(Json::as_str).ok() == Some("Capability"))
        .ok_or("ids-index.json missing Capability namespace")?;
    if capability_ns.get("owner")?.as_str()? != "Architecture" {
        return Err("Capability namespace owner is not Architecture".to_string());
    }

    // Header 是同一权威的另一形态；两侧不一致说明镜像半新半旧，直接失败。
    let header = read_mirror_text(root, HEADER_MIRROR_REL)?;
    let header_defines = parse_capability_defines(&header)?;
    let header_keys = &header_defines.keys;

    let mut seen_ids: Vec<String> = Vec::new();
    let mut seen_numerics: Vec<i64> = Vec::new();
    out.push_str(
        "#[rustfmt::skip]\npub(crate) const CAPABILITY_KEYS: &[ArchitectureCapabilityKey] = &[\n",
    );
    for value in capability_ns.get("values")?.as_arr()? {
        let id = value.get("id")?.as_str()?;
        let numeric = value.get("numeric")?.as_i64()?;
        let status = value.get("status")?.as_str()?;
        if status != "Active" && status != "Reserved" {
            return Err(format!("Capability {id} has unexpected status {status}"));
        }
        if numeric <= 0 {
            return Err(format!(
                "Capability {id} numeric {numeric} is not a 1-based ordinal"
            ));
        }
        if seen_ids.iter().any(|s| s == id) || seen_numerics.contains(&numeric) {
            return Err(format!("Capability duplicate id/numeric: {id}/{numeric}"));
        }
        let macro_name = screaming_snake(id);
        match header_keys.iter().find(|(name, _)| *name == macro_name) {
            Some((_, header_numeric)) if *header_numeric == numeric => {}
            Some((_, header_numeric)) => {
                return Err(format!(
                    "header LUMIO_CAPABILITY_{macro_name} = {header_numeric} disagrees with registry {id} = {numeric}"
                ));
            }
            None => {
                return Err(format!(
                    "header is missing LUMIO_CAPABILITY_{macro_name} for registered Capability {id}"
                ));
            }
        }
        seen_ids.push(id.to_string());
        seen_numerics.push(numeric);
        writeln!(
            out,
            "    ArchitectureCapabilityKey::new(\"{id}\", {numeric}, \"{status}\"),"
        )
        .unwrap();
    }
    out.push_str("];\n");

    for (name, _) in header_keys {
        if !seen_ids.iter().any(|id| screaming_snake(id) == *name) {
            return Err(format!(
                "header publishes LUMIO_CAPABILITY_{name} with no registered Capability value"
            ));
        }
    }
    match header_defines.count {
        Some(count) if count == seen_ids.len() as i64 => {}
        Some(count) => {
            return Err(format!(
                "header LUMIO_CAPABILITY_COUNT = {count} != {} registered Capability values",
                seen_ids.len()
            ));
        }
        None => return Err("header is missing LUMIO_CAPABILITY_COUNT".to_string()),
    }

    Ok(out)
}

/// 发布的符号面策略：`(entrySymbol, symbolPrefix)`，来自 bundle 镜像。
pub fn abi_symbol_policy(root: &Path) -> Result<(String, String), String> {
    let bundle = read_mirror(root, BUNDLE_MIRROR_REL)?;
    let abi = bundle.get("abi")?;
    Ok((
        abi.get("entrySymbol")?.as_str()?.to_string(),
        abi.get("symbolPrefix")?.as_str()?.to_string(),
    ))
}

pub fn generated_data_path(root: &Path) -> std::path::PathBuf {
    root.join(GENERATED_DATA_REL.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// 生成并写盘；内容与既有文件一致时不动文件。
pub fn write_generated_data(root: &Path) -> Result<bool, String> {
    let derived = derive_generated_data(root)?;
    let path = generated_data_path(root);
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current == derived {
        return Ok(false);
    }
    std::fs::write(&path, derived).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}
