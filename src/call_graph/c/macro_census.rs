//! Repo-wide `#define` census for C and C++ (#630 slice 6a, behaviour-neutral).
//!
//! Slice 6's rules U and D (design: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/`)
//! read unexpanded source, so a macro can hide a read, a write, an address-taking or a jump
//! target that no identifier token on the line witnesses. The census records, per macro name
//! defined in any parsed C/C++ file of the repository:
//!
//! * `params`: the declared parameters (`__VA_ARGS__` for a variadic one);
//! * `free`: the transitive free-identifier set: identifier tokens of the body minus the
//!   parameters, closed over the in-repo macros the body names (keywords stay in: extra
//!   names only refuse more);
//! * `control`: the transitive body holds `goto`, `case`, `default`, a returns-twice name or
//!   an `asm` spelling;
//! * `poisoned`: `##` or its digraph `%:%:`, a body that fails to tokenize, or recursion
//!   through the closure (propagated to every macro that reaches a poisoned one).
//!
//! Definitions are collected textually, per file, not from the syntax tree: backslash-newline
//! continuations are joined, comments are stripped (string, character and C++ raw-string
//! literals kept), and every logical line whose first token is `#` (or `%:`) followed by
//! `define` is a definition. That covers defines the grammar does not expose as define nodes
//! (inside an `ERROR` node, an enum body or an initializer list). The census also carries
//! `incomplete`: some `#define` line (or an unterminated comment, an uncertain raw string, an
//! unreadable parameter) could not be read, or the loader skipped an in-repo C/C++ file
//! (non-UTF-8, unparsable, unreadable, too large, a symlink: stamped on every parsed C/C++
//! file, so full, partial and scoped builds agree). `incomplete` means slice 6b must
//! disable rules U and D for ALL C/C++; it is part of the digest.
//!
//! The textual pass is not the compiler's lexer, so it is belted: phase 1 is normalized
//! (BOM, CRLF and lone CR, trigraphs, splices with any blank before the newline), definition
//! lines are the union of the comment-stripped and the unstripped text (only the stripped
//! primary pass sets `incomplete`), and one identifier rule (ASCII, non-ASCII bytes, UCNs)
//! serves the name, parameter and body readers.
//!
//! Several definitions of one name are unioned and every judgement over-approximates: a
//! name that is a macro is judged by the union of all its definitions. Nothing consumes the
//! census yet; slice 6a only builds it and persists its digest with the cached DFG.
use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Names whose call can return a second time (G4 of the design) plus `asm` spellings.
/// Prefixed spellings are also matched by stem (see [`is_control_name`]).
const CONTROL_NAMES: &[&str] = &[
    "goto",
    "case",
    "default",
    "setjmp",
    "_setjmp",
    "_setjmpex",
    "__setjmp",
    "sigsetjmp",
    "_sigsetjmp",
    "__sigsetjmp",
    "__builtin_setjmp",
    "qsetjmp",
    "savectx",
    "vfork",
    "__vfork",
    "getcontext",
    "asm",
    "_asm",
    "__asm",
    "__asm__",
];

/// GCC's returns-twice stems (after stripping one leading `__` or `_`).
const CONTROL_STEMS: &[&str] = &[
    "setjmp",
    "setjmpex",
    "setjmp_syscall",
    "sigsetjmp",
    "savectx",
    "qsetjmp",
    "vfork",
    "getcontext",
];

fn is_control_name(t: &str) -> bool {
    if CONTROL_NAMES.contains(&t) {
        return true;
    }
    let stem = t
        .strip_prefix("__")
        .or_else(|| t.strip_prefix('_'))
        .unwrap_or(t);
    CONTROL_STEMS.contains(&stem)
}

/// The facts about one macro name (all definitions unioned, closure applied).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacroFacts {
    pub params: BTreeSet<String>,
    pub free: BTreeSet<String>,
    pub control: bool,
    pub poisoned: bool,
}

/// One definition before the closure.
#[derive(Default)]
struct RawDef {
    params: BTreeSet<String>,
    free: BTreeSet<String>,
    control: bool,
    poisoned: bool,
}

/// The repo-wide census. Cheap to clone; compare with [`CMacroCensus::digest`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CMacroCensus {
    macros: BTreeMap<String, MacroFacts>,
    /// A `#define` line could not be read into a name, or an in-repo C/C++ file never
    /// reached the census (the loader skipped it): 6b must disable U and D for ALL C/C++.
    incomplete: bool,
}

/// Result of tokenizing a macro body.
struct Body {
    idents: BTreeSet<String>,
    paste: bool,
    ok: bool,
}

// ---- The one identifier rule (name parser, parameter parser, body tokenizer). ----
// Identifier characters: ASCII `[A-Za-z0-9_$]`, any non-ASCII byte, and UCNs
// (`\uXXXX`, `\UXXXXXXXX`).

/// Length of a UCN at `i` (`\u` + 4 hex or `\U` + 8 hex), else 0.
fn ucn_len(b: &[u8], i: usize) -> usize {
    if b.get(i) != Some(&b'\\') {
        return 0;
    }
    let n = match b.get(i + 1) {
        Some(b'u') => 4,
        Some(b'U') => 8,
        _ => return 0,
    };
    if b.len() >= i + 2 + n && b[i + 2..i + 2 + n].iter().all(u8::is_ascii_hexdigit) {
        2 + n
    } else {
        0
    }
}

fn is_ident_start(b: &[u8], i: usize) -> bool {
    let c = b[i];
    c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80 || ucn_len(b, i) > 0
}

/// End of the identifier-character run starting at `i` (digits continue it).
fn ident_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80 {
            i += 1;
        } else {
            let n = ucn_len(b, i);
            if n == 0 {
                break;
            }
            i += n;
        }
    }
    i
}

/// A token holding a non-ASCII byte or a UCN (its ASCII fragments are recorded too).
fn is_complex(t: &str) -> bool {
    t.bytes().any(|c| c >= 0x80 || c == b'\\')
}

/// Insert `t` and, when it is complex, every maximal ASCII `[A-Za-z_$][A-Za-z0-9_$]*`
/// fragment of it (extra names only refuse more).
fn insert_ident(set: &mut BTreeSet<String>, t: &str) {
    set.insert(t.to_owned());
    if !is_complex(t) {
        return;
    }
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
                i += 1;
            }
            let frag = t[s..i].trim_start_matches(|c: char| c.is_ascii_digit());
            if !frag.is_empty() {
                set.insert(frag.to_owned());
            }
        } else {
            i += 1;
        }
    }
}

/// Identifier tokens of a `#define` body. Comments, string and character literals and
/// numbers carry no identifier; `##` and any unterminated literal or comment fail closed.
fn tokenize(text: &str) -> Body {
    let b = text.as_bytes();
    let mut out = Body {
        idents: BTreeSet::new(),
        paste: false,
        ok: true,
    };
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'/' if b.get(i + 1) == Some(&b'*') => match text[i + 2..].find("*/") {
                Some(end) => i += end + 4,
                None => {
                    out.ok = false;
                    break;
                }
            },
            b'/' if b.get(i + 1) == Some(&b'/') => {
                // A line comment runs to the end of the logical line (continuations join).
                while i < b.len() && b[i] != b'\n' {
                    if b[i] == b'\\' && i + 1 < b.len() {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'"' | b'\'' => {
                let quote = c;
                i += 1;
                let mut closed = false;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 2,
                        q if q == quote => {
                            closed = true;
                            i += 1;
                            break;
                        }
                        b'\n' => break,
                        _ => i += 1,
                    }
                }
                if !closed {
                    out.ok = false;
                    break;
                }
            }
            b'#' if b.get(i + 1) == Some(&b'#') => {
                out.paste = true;
                i += 2;
            }
            // `%:%:` is the digraph for `##`; `%:` alone is `#` (stringizing).
            b'%' if b.get(i + 1) == Some(&b':') => {
                if b.get(i + 2) == Some(&b'%') && b.get(i + 3) == Some(&b':') {
                    out.paste = true;
                    i += 4;
                } else {
                    i += 2;
                }
            }
            c if c.is_ascii_digit() => i = pp_number_end(b, i),
            _ if is_ident_start(b, i) => {
                let start = i;
                i = ident_end(b, i);
                // Slicing is on char boundaries: bytes >= 0x80 are consumed whole and a UCN
                // is ASCII.
                let ident = &text[start..i];
                if b.get(i) == Some(&b'"') && is_raw_prefix(ident) {
                    // A C++ raw string: no identifier, and no escape or quote rule inside.
                    match raw_string_end(b, i, &[]) {
                        Some(end) => i = end,
                        None => {
                            out.ok = false;
                            break;
                        }
                    }
                    continue;
                }
                insert_ident(&mut out.idents, ident);
            }
            _ => i += 1,
        }
    }
    out
}

const WS: &[char] = &[' ', '\t', '\r', '\x0b', '\x0c'];

/// End of the pp-number starting at `i`: digits, letters, `.`, `_`, signed exponents and
/// C++14/C23 digit separators (`1'000`: a `'` followed by an alphanumeric or `_`).
fn pp_number_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() {
        let d = b[i];
        if d.is_ascii_alphanumeric() || d == b'.' || d == b'_' {
            i += 1;
        } else if (d == b'+' || d == b'-') && matches!(b[i - 1], b'e' | b'E' | b'p' | b'P') {
            i += 1;
        } else if d == b'\''
            && b.get(i + 1)
                .is_some_and(|n| n.is_ascii_alphanumeric() || *n == b'_')
        {
            i += 1;
        } else {
            break;
        }
    }
    i
}

fn is_raw_prefix(ident: &str) -> bool {
    matches!(ident, "R" | "u8R" | "uR" | "UR" | "LR")
}

/// `q` indexes the opening `"` of `R"delim( .. )delim"`; the index after the closing quote.
/// The delimiter is up to 16 characters other than space, `(`, `)`, `\`, tab, VT, FF and
/// newline (`"` is allowed). `None` means uncertain: an invalid delimiter, an unterminated
/// literal, or a line splice inside the literal (`splices` holds the indexes of `b` at
/// which a splice was removed; the compiler reverts them inside a raw string).
fn raw_string_end(b: &[u8], q: usize, splices: &[usize]) -> Option<usize> {
    let start = q + 1;
    let mut j = start;
    while j < b.len() && b[j] != b'(' {
        let c = b[j];
        if j - start >= 16 || matches!(c, b' ' | b')' | b'\\' | b'\t' | b'\x0b' | b'\x0c' | b'\n') {
            return None;
        }
        j += 1;
    }
    if j >= b.len() {
        return None;
    }
    let mut close = vec![b')'];
    close.extend_from_slice(&b[start..j]);
    close.push(b'"');
    let body = j + 1;
    let end = b[body..]
        .windows(close.len())
        .position(|w| w == close.as_slice())
        .map(|p| body + p + close.len())?;
    let first = splices.partition_point(|&s| s < q);
    if splices.get(first).is_some_and(|&s| s < end) {
        return None;
    }
    Some(end)
}

// ---- Translation phases 1-2. ----

/// Phase 1: strip a leading UTF-8 BOM, map CRLF and lone CR to LF and (when `trigraphs`)
/// translate trigraphs. Translating unconditionally over-approximates.
fn translate(src: &str, trigraphs: bool) -> String {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\r' => {
                out.push(b'\n');
                i += if b.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
            }
            b'?' if trigraphs && b.get(i + 1) == Some(&b'?') => {
                let t = match b.get(i + 2) {
                    Some(b'=') => Some(b'#'),
                    Some(b'/') => Some(b'\\'),
                    Some(b'\'') => Some(b'^'),
                    Some(b'(') => Some(b'['),
                    Some(b')') => Some(b']'),
                    Some(b'!') => Some(b'|'),
                    Some(b'<') => Some(b'{'),
                    Some(b'>') => Some(b'}'),
                    Some(b'-') => Some(b'~'),
                    _ => None,
                };
                match t {
                    Some(t) => {
                        out.push(t);
                        i += 3;
                    }
                    None => {
                        out.push(b'?');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    // Only ASCII bytes were removed or replaced.
    String::from_utf8_lossy(&out).into_owned()
}

/// Phase 2: join backslash-newline continuations, with any blanks (space, tab, VT, FF)
/// between the backslash and the newline when `lenient` (GCC, clang, C++23); a strict
/// splice (MSVC) takes only a backslash directly before the newline. Also returns the output indexes at which a
/// splice was removed.
fn splice(src: &str, lenient: bool) -> (Vec<u8>, Vec<usize>) {
    let sb = src.as_bytes();
    let mut out = Vec::with_capacity(sb.len());
    let mut points = Vec::new();
    let mut i = 0;
    while i < sb.len() {
        if sb[i] == b'\\' {
            let mut j = i + 1;
            while lenient && j < sb.len() && matches!(sb[j], b' ' | b'\t' | b'\x0b' | b'\x0c') {
                j += 1;
            }
            if sb.get(j) == Some(&b'\n') {
                points.push(out.len());
                i = j + 1;
                continue;
            }
        }
        out.push(sb[i]);
        i += 1;
    }
    (out, points)
}

struct Stripped {
    text: String,
    unterminated: bool,
    raw_uncertain: bool,
}

/// Comment stripping over spliced text, keeping string, character and raw-string literals
/// intact. A block comment becomes one space. An uncertain raw string sets `raw_uncertain`
/// and the rest of the text is kept unstripped (the unstripped pass also reads it).
fn strip_comments(b: &[u8], splices: &[usize]) -> Stripped {
    let mut out = Vec::with_capacity(b.len());
    let mut unterminated = false;
    let mut raw_uncertain = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'/' if b.get(i + 1) == Some(&b'*') => {
                out.push(b' ');
                match b[i + 2..].windows(2).position(|w| w == b"*/") {
                    Some(p) => i += p + 4,
                    None => {
                        unterminated = true;
                        i = b.len();
                    }
                }
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                out.push(b' ');
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' | b'\'' => {
                out.push(c);
                i += 1;
                while i < b.len() {
                    let d = b[i];
                    if d == b'\n' {
                        break;
                    }
                    out.push(d);
                    i += 1;
                    if d == b'\\' && i < b.len() && b[i] != b'\n' {
                        out.push(b[i]);
                        i += 1;
                    } else if d == c {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit() => {
                let end = pp_number_end(b, i);
                out.extend_from_slice(&b[i..end]);
                i = end;
            }
            _ if is_ident_start(b, i) => {
                let start = i;
                i = ident_end(b, i);
                out.extend_from_slice(&b[start..i]);
                if b.get(i) == Some(&b'"')
                    && std::str::from_utf8(&b[start..i]).is_ok_and(is_raw_prefix)
                {
                    match raw_string_end(b, i, splices) {
                        Some(end) => {
                            out.extend_from_slice(&b[i..end]);
                            i = end;
                        }
                        None => {
                            raw_uncertain = true;
                            out.extend_from_slice(&b[i..]);
                            i = b.len();
                        }
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Stripped {
        text: String::from_utf8_lossy(&out).into_owned(),
        unterminated,
        raw_uncertain,
    }
}

/// One parsed `#define`.
struct Parsed<'a> {
    name: String,
    params: Option<Vec<String>>,
    body: &'a str,
    /// The parameter list was left unclosed.
    malformed: bool,
    /// A parameter is not a valid identifier (nor `...`).
    bad_params: bool,
    /// The name or a parameter holds a non-ASCII byte or a UCN.
    complex: bool,
}

/// Split the text after the `define` keyword (continuations already joined, comments
/// stripped in the primary pass) into name, parameters and body. `None`: no name.
fn parse_define(after: &str) -> Option<Parsed<'_>> {
    let t = after.trim_start_matches(WS);
    let tb = t.as_bytes();
    if tb.is_empty() || !is_ident_start(tb, 0) {
        return None;
    }
    let end = ident_end(tb, 0);
    let (name, rest) = t.split_at(end);
    let name = name.to_owned();
    let mut complex = is_complex(&name);
    let Some(inner) = rest.strip_prefix('(') else {
        return Some(Parsed {
            name,
            params: None,
            body: rest,
            malformed: false,
            bad_params: false,
            complex,
        });
    };
    let Some(close) = inner.find(')') else {
        return Some(Parsed {
            name,
            params: Some(Vec::new()),
            body: "",
            malformed: true,
            bad_params: false,
            complex,
        });
    };
    let list = inner[..close].trim_matches(WS);
    let mut params = Vec::new();
    let mut bad = false;
    if !list.is_empty() {
        for p in list.split(',') {
            let p = p.trim_matches(WS);
            if p == "..." {
                params.push("__VA_ARGS__".to_owned());
                continue;
            }
            let (id, va) = match p.strip_suffix("...") {
                Some(id) => (id.trim_matches(WS), true),
                None => (p, false),
            };
            let ib = id.as_bytes();
            if ib.is_empty() || !is_ident_start(ib, 0) || ident_end(ib, 0) != ib.len() {
                bad = true;
                continue;
            }
            complex |= is_complex(id);
            params.push(id.to_owned());
            if va {
                params.push("__VA_ARGS__".to_owned());
            }
        }
    }
    Some(Parsed {
        name,
        params: Some(params),
        body: &inner[close + 1..],
        malformed: false,
        bad_params: bad,
        complex,
    })
}

/// A definition with the poison split: `hard` (paste, UCN/non-ASCII names) holds in every
/// pass; `soft` (a body or parameter list the pass could not read) only counts when the
/// primary pass never saw the name.
struct Entry {
    name: String,
    def: RawDef,
    hard: bool,
    soft: bool,
}

fn entry(p: &Parsed<'_>) -> Entry {
    let mut def = RawDef {
        params: p.params.iter().flatten().cloned().collect(),
        ..RawDef::default()
    };
    let body = tokenize(p.body);
    let hard = body.paste || p.complex;
    let soft = !body.ok || p.malformed || p.bad_params;
    def.poisoned = hard || soft;
    def.control = body.idents.iter().any(|t| is_control_name(t));
    def.free = body.idents.difference(&def.params).cloned().collect();
    Entry {
        name: p.name.clone(),
        def,
        hard,
        soft,
    }
}

/// Every logical `#define` line of `text`. `primary` passes set `incomplete` on a line
/// that names no macro, an unreadable parameter or an unclosed parameter list.
fn defs_in(text: &str, primary: bool, incomplete: &mut bool) -> Vec<Entry> {
    let mut found = Vec::new();
    for line in text.split('\n') {
        let t = line.trim_start_matches(WS);
        let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
            continue;
        };
        let Some(after) = rest.trim_start_matches(WS).strip_prefix("define") else {
            continue;
        };
        let ab = after.as_bytes();
        if !ab.is_empty()
            && (ab[0].is_ascii_alphanumeric() || ab[0] == b'_' || ab[0] == b'$' || ab[0] >= 0x80)
        {
            continue; // another directive spelling, e.g. `#definex`
        }
        let Some(p) = parse_define(after) else {
            *incomplete |= primary;
            continue;
        };
        if primary {
            *incomplete |= p.malformed || p.bad_params;
        }
        found.push(entry(&p));
    }
    found
}

/// Collect one file. The primary pass (translated text, comments stripped) is the only one
/// that sets `incomplete`; the other passes (comments kept, and the text without trigraph
/// translation) only add definitions, so a lexer divergence cannot lose one.
fn collect(source: &str, out: &mut BTreeMap<String, RawDef>, incomplete: &mut bool) {
    let translated = translate(source, true);
    let plain = translate(source, false);
    let mut texts = vec![translated];
    if texts[0] != plain {
        texts.push(plain);
    }
    let mut primary: BTreeMap<String, RawDef> = BTreeMap::new();
    let mut extra: Vec<Entry> = Vec::new();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (ti, text) in texts.iter().enumerate() {
        for lenient in [true, false] {
            let (spliced, points) = splice(text, lenient);
            let is_primary = ti == 0 && lenient;
            if !is_primary && seen.contains(&spliced) {
                continue;
            }
            let stripped = strip_comments(&spliced, &points);
            let mut flag = false;
            let entries = defs_in(&stripped.text, is_primary, &mut flag);
            if is_primary {
                *incomplete |= flag | stripped.unterminated | stripped.raw_uncertain;
                for e in entries {
                    merge(&mut primary, e.name, e.def);
                }
            } else {
                extra.extend(entries);
            }
            let unstripped = String::from_utf8_lossy(&spliced).into_owned();
            extra.extend(defs_in(&unstripped, false, &mut flag));
            seen.push(spliced);
        }
    }
    for e in extra {
        let mut def = e.def;
        if primary.contains_key(&e.name) {
            def.poisoned = e.hard;
        } else {
            def.poisoned = e.hard || e.soft;
        }
        merge(&mut primary, e.name, def);
    }
    for (name, def) in primary {
        merge(out, name, def);
    }
}

/// Union of definitions; an invocation can be any of them.
fn merge(out: &mut BTreeMap<String, RawDef>, name: String, def: RawDef) {
    let e = out.entry(name).or_default();
    e.params.extend(def.params);
    e.free.extend(def.free);
    e.control |= def.control;
    e.poisoned |= def.poisoned;
}

fn is_c_family(parsed: &ParsedFile) -> bool {
    matches!(parsed.language, Language::C | Language::Cpp)
}

impl CMacroCensus {
    /// The census over every parsed C and C++ file of `files` (the whole repository, also
    /// for a scoped build, so a scoped build refuses exactly what a full build does).
    pub fn from_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut raw = BTreeMap::new();
        let mut incomplete = false;
        for parsed in files.values().filter(|p| is_c_family(p)) {
            incomplete |= parsed.c_family_skipped_in_repo();
            collect(&parsed.source, &mut raw, &mut incomplete);
        }
        Self::close(raw, incomplete)
    }

    /// The one-file census: the seam for callers with no repository.
    pub fn of_file(parsed: &ParsedFile) -> Self {
        let mut raw = BTreeMap::new();
        let mut incomplete = false;
        if is_c_family(parsed) {
            incomplete |= parsed.c_family_skipped_in_repo();
            collect(&parsed.source, &mut raw, &mut incomplete);
        }
        Self::close(raw, incomplete)
    }

    /// Close each definition over the in-repo macros its body names. A macro that reaches
    /// itself (recursion, a self-reference included) is poisoned, and poison, control and
    /// free names propagate to every macro that reaches it.
    fn close(raw: BTreeMap<String, RawDef>, incomplete: bool) -> Self {
        let edges: BTreeMap<&String, BTreeSet<&String>> = raw
            .iter()
            .map(|(n, d)| {
                (
                    n,
                    d.free
                        .iter()
                        .filter_map(|f| raw.get_key_value(f))
                        .map(|(k, _)| k)
                        .collect(),
                )
            })
            .collect();
        // Everything reachable from a macro through at least one edge.
        let reach_of = |name: &String| -> BTreeSet<&String> {
            let mut reach: BTreeSet<&String> = BTreeSet::new();
            let mut work: Vec<&String> = edges[name].iter().copied().collect();
            while let Some(m) = work.pop() {
                if reach.insert(m) {
                    work.extend(edges[m].iter().copied());
                }
            }
            reach
        };
        let reaches: BTreeMap<&String, BTreeSet<&String>> =
            raw.keys().map(|n| (n, reach_of(n))).collect();
        // A macro that reaches itself is recursive.
        let cyclic: BTreeSet<&String> = reaches
            .iter()
            .filter(|(n, r)| r.contains(*n))
            .map(|(n, _)| *n)
            .collect();
        let mut macros = BTreeMap::new();
        for (name, def) in &raw {
            let mut facts = MacroFacts {
                params: def.params.clone(),
                free: def.free.clone(),
                control: def.control,
                poisoned: def.poisoned || cyclic.contains(name),
            };
            for m in &reaches[name] {
                let d = &raw[*m];
                facts.free.extend(d.free.iter().cloned());
                facts.control |= d.control;
                facts.poisoned |= d.poisoned || cyclic.contains(*m);
            }
            macros.insert(name.clone(), facts);
        }
        Self { macros, incomplete }
    }

    /// Facts for a macro name, `None` when the name is not defined in a parsed file.
    pub fn get(&self, name: &str) -> Option<&MacroFacts> {
        self.macros.get(name)
    }

    /// Some `#define` could not be read into a name: slice 6b must disable rules U and D
    /// everywhere for C/C++ (any macro fact may be missing).
    pub fn incomplete(&self) -> bool {
        self.incomplete
    }

    pub fn len(&self) -> usize {
        self.macros.len()
    }

    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// Number of macros flagged `control`.
    pub fn control_count(&self) -> usize {
        self.macros.values().filter(|f| f.control).count()
    }

    /// Number of macros flagged `poisoned`.
    pub fn poisoned_count(&self) -> usize {
        self.macros.values().filter(|f| f.poisoned).count()
    }

    /// A stable digest of exactly the recorded facts; empty for an empty census. The DFG
    /// storage persists it, and a partial cache hit that finds a different digest rebuilds
    /// in full.
    pub fn digest(&self) -> String {
        if self.macros.is_empty() && !self.incomplete {
            return String::new();
        }
        // Every string and every collection is length-prefixed, so no input text can
        // collide with the framing.
        fn put(h: &mut Sha256, bytes: &[u8]) {
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(bytes);
        }
        let mut h = Sha256::new();
        h.update(b"c-macro-census-v2");
        h.update((self.macros.len() as u64).to_le_bytes());
        for (name, f) in &self.macros {
            put(&mut h, name.as_bytes());
            for set in [&f.params, &f.free] {
                h.update((set.len() as u64).to_le_bytes());
                for s in set {
                    put(&mut h, s.as_bytes());
                }
            }
            h.update([u8::from(f.control), u8::from(f.poisoned)]);
        }
        h.update([u8::from(self.incomplete)]);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}
