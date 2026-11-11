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
//! `incomplete`: some `#define` line (or an unterminated comment) could not be read into a
//! name. `incomplete` means slice 6b must disable rules U and D everywhere for C/C++; it is
//! part of the digest.
//!
//! Several definitions of one name are unioned and every judgement over-approximates: a
//! name that is a macro is judged by the union of all its definitions. Nothing consumes the
//! census yet; slice 6a only builds it and persists its digest with the cached DFG.
use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Names whose call can return a second time (G4 of the design) plus `asm` spellings.
const CONTROL_NAMES: &[&str] = &[
    "goto",
    "case",
    "default",
    "setjmp",
    "_setjmp",
    "sigsetjmp",
    "__sigsetjmp",
    "__builtin_setjmp",
    "savectx",
    "vfork",
    "getcontext",
    "asm",
    "__asm",
    "__asm__",
];

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
    /// A `#define` line could not be read into a name: 6b must disable U and D everywhere.
    incomplete: bool,
}

/// Result of tokenizing a macro body.
struct Body {
    idents: BTreeSet<String>,
    paste: bool,
    ok: bool,
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
            c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80 => {
                let start = i;
                while i < b.len()
                    && (b[i].is_ascii_alphanumeric()
                        || b[i] == b'_'
                        || b[i] == b'$'
                        || b[i] >= 0x80)
                {
                    i += 1;
                }
                // Slicing at ASCII/non-ASCII boundaries is on char boundaries: bytes >= 0x80
                // are consumed whole.
                let ident = &text[start..i];
                if b.get(i) == Some(&b'"') && is_raw_prefix(ident) {
                    // A C++ raw string: no identifier, and no escape or quote rule inside.
                    match raw_string_end(b, i) {
                        Some(end) => i = end,
                        None => {
                            out.ok = false;
                            break;
                        }
                    }
                    continue;
                }
                out.idents.insert(ident.to_owned());
            }
            _ => i += 1,
        }
    }
    out
}

const WS: &[char] = &[' ', '\t', '\r', '\x0b', '\x0c'];

fn is_ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

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
fn raw_string_end(b: &[u8], q: usize) -> Option<usize> {
    let start = q + 1;
    let mut j = start;
    while j < b.len() && b[j] != b'(' {
        let c = b[j];
        if j - start >= 16 || c.is_ascii_whitespace() || matches!(c, b')' | b'\\' | b'"') {
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
    b[body..]
        .windows(close.len())
        .position(|w| w == close.as_slice())
        .map(|p| body + p + close.len())
}

/// Translation phases 1-3 for the census: join backslash-newline continuations (a
/// backslash, optional blanks, then CR LF or LF), then strip comments, keeping string,
/// character and raw-string literals intact. A block comment becomes one space; the
/// boolean is true for an unterminated block comment.
fn logical_text(src: &str) -> (String, bool) {
    let sb = src.as_bytes();
    let mut spliced = Vec::with_capacity(sb.len());
    let mut i = 0;
    while i < sb.len() {
        if sb[i] == b'\\' {
            let mut j = i + 1;
            while j < sb.len() && matches!(sb[j], b' ' | b'\t') {
                j += 1;
            }
            if sb.get(j) == Some(&b'\r') && sb.get(j + 1) == Some(&b'\n') {
                i = j + 2;
                continue;
            }
            if sb.get(j) == Some(&b'\n') {
                i = j + 1;
                continue;
            }
        }
        spliced.push(sb[i]);
        i += 1;
    }
    let b = &spliced[..];
    let mut out = Vec::with_capacity(b.len());
    let mut unterminated = false;
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
            c if is_ident_byte(c) => {
                let start = i;
                while i < b.len() && is_ident_byte(b[i]) {
                    i += 1;
                }
                out.extend_from_slice(&b[start..i]);
                if b.get(i) == Some(&b'"') {
                    if let Ok(id) = std::str::from_utf8(&b[start..i]) {
                        if is_raw_prefix(id) {
                            if let Some(end) = raw_string_end(b, i) {
                                out.extend_from_slice(&b[i..end]);
                                i = end;
                            }
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
    (String::from_utf8_lossy(&out).into_owned(), unterminated)
}

/// One parsed `#define`: name, parameters (function-like only), body, and whether the
/// parameter list was left unclosed (the macro is then poisoned and the census incomplete).
struct Parsed<'a> {
    name: String,
    params: Option<Vec<String>>,
    body: &'a str,
    malformed: bool,
}

/// Split the text after the `define` keyword (comments already stripped, continuations
/// already joined) into name, parameters and body. `None`: no name could be read.
fn parse_define(after: &str) -> Option<Parsed<'_>> {
    let t = after.trim_start_matches(WS);
    let end = t
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(t.len());
    if end == 0 || t.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let (name, rest) = t.split_at(end);
    let name = name.to_owned();
    if let Some(inner) = rest.strip_prefix('(') {
        let Some(close) = inner.find(')') else {
            return Some(Parsed {
                name,
                params: Some(Vec::new()),
                body: "",
                malformed: true,
            });
        };
        let params = inner[..close]
            .split(',')
            .map(|p| p.trim().trim_end_matches("...").trim().to_owned())
            .chain(
                inner[..close]
                    .contains("...")
                    .then(|| "__VA_ARGS__".to_owned()),
            )
            .filter(|p| !p.is_empty())
            .collect();
        Some(Parsed {
            name,
            params: Some(params),
            body: &inner[close + 1..],
            malformed: false,
        })
    } else {
        Some(Parsed {
            name,
            params: None,
            body: rest,
            malformed: false,
        })
    }
}

fn raw_def(p: &Parsed<'_>) -> RawDef {
    let mut def = RawDef::default();
    def.params = p.params.iter().flatten().cloned().collect();
    let body = tokenize(p.body);
    def.poisoned = body.paste || !body.ok || p.malformed;
    def.control = body
        .idents
        .iter()
        .any(|t| CONTROL_NAMES.contains(&t.as_str()));
    def.free = body.idents.difference(&def.params).cloned().collect();
    def
}

/// Every logical `#define` line of `source`, textually. A line that names no macro, or an
/// unterminated comment, sets `incomplete`.
fn collect(source: &str, out: &mut BTreeMap<String, RawDef>, incomplete: &mut bool) {
    let (text, unterminated) = logical_text(source);
    *incomplete |= unterminated;
    for line in text.split('\n') {
        let t = line.trim_start_matches(WS);
        let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
            continue;
        };
        let Some(after) = rest.trim_start_matches(WS).strip_prefix("define") else {
            continue;
        };
        if after.starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '$') {
            continue; // another directive spelling, e.g. `#definex`
        }
        let Some(p) = parse_define(after) else {
            *incomplete = true;
            continue;
        };
        *incomplete |= p.malformed;
        let def = raw_def(&p);
        // Union of definitions; an invocation can be any of them.
        let e = out.entry(p.name).or_default();
        e.params.extend(def.params);
        e.free.extend(def.free);
        e.control |= def.control;
        e.poisoned |= def.poisoned;
    }
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
            collect(&parsed.source, &mut raw, &mut incomplete);
        }
        Self::close(raw, incomplete)
    }

    /// The one-file census: the seam for callers with no repository.
    pub fn of_file(parsed: &ParsedFile) -> Self {
        let mut raw = BTreeMap::new();
        let mut incomplete = false;
        if is_c_family(parsed) {
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
        let mut h = Sha256::new();
        for (name, f) in &self.macros {
            h.update(b"m");
            h.update(name.as_bytes());
            h.update([0]);
            for (tag, set) in [(b'p', &f.params), (b'f', &f.free)] {
                h.update([tag]);
                for s in set {
                    h.update(s.as_bytes());
                    h.update([0]);
                }
                h.update([1]);
            }
            h.update([u8::from(f.control), u8::from(f.poisoned), 2]);
        }
        if self.incomplete {
            h.update(b"!incomplete");
        }
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}
