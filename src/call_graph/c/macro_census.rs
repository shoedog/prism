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
//! * `poisoned`: `##`, a body that fails to tokenize, or recursion through the closure
//!   (propagated to every macro that reaches a poisoned one).
//!
//! Several definitions of one name are unioned and every judgement over-approximates: a
//! name that is a macro is judged by the union of all its definitions. Nothing consumes the
//! census yet; slice 6a only builds it and persists its digest with the cached DFG.
use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

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
            c if c.is_ascii_digit() => {
                // A pp-number: digits, letters, `.`, `_` and signed exponents.
                while i < b.len() {
                    let d = b[i];
                    if d.is_ascii_alphanumeric() || d == b'.' || d == b'_' {
                        i += 1;
                    } else if (d == b'+' || d == b'-')
                        && matches!(b[i - 1], b'e' | b'E' | b'p' | b'P')
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
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
                out.idents.insert(text[start..i].to_owned());
            }
            _ => i += 1,
        }
    }
    out
}

/// Split `#define NAME[(params)] body` textually. The grammar's `name`/`value` fields are
/// not used: a comment inside a body makes tree-sitter-c emit an ERROR with a wrong `name`
/// (`#define K 1 /* c */ real`), whereas the node's source range stays whole.
fn parse_define(text: &str) -> Option<(String, Option<Vec<String>>, &str)> {
    let t = text.trim_start().strip_prefix('#')?.trim_start();
    let t = t.strip_prefix("define")?;
    if !t.starts_with(|c: char| c.is_whitespace() || c == '\\' || c == '/') {
        return None;
    }
    let t = t.trim_start_matches(|c: char| c.is_whitespace() || c == '\\');
    let end = t
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(t.len());
    if end == 0 {
        return None;
    }
    let (name, rest) = t.split_at(end);
    if let Some(inner) = rest.strip_prefix('(') {
        let close = inner.find(')')?;
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
        Some((name.to_owned(), Some(params), &inner[close + 1..]))
    } else {
        Some((name.to_owned(), None, rest))
    }
}

fn raw_def(parsed: &ParsedFile, node: Node<'_>) -> Option<(String, RawDef)> {
    let (name, params, body_text) = parse_define(parsed.node_text(&node))?;
    let mut def = RawDef::default();
    def.params = params.into_iter().flatten().collect();
    let body = tokenize(body_text);
    def.poisoned = body.paste || !body.ok;
    def.control = body
        .idents
        .iter()
        .any(|t| CONTROL_NAMES.contains(&t.as_str()));
    def.free = body.idents.difference(&def.params).cloned().collect();
    Some((name, def))
}

fn collect(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeMap<String, RawDef>) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if matches!(n.kind(), "preproc_def" | "preproc_function_def") {
            if let Some((name, def)) = raw_def(parsed, n) {
                // Union of definitions; an invocation can be any of them.
                let e = out.entry(name).or_default();
                e.params.extend(def.params);
                e.free.extend(def.free);
                e.control |= def.control;
                e.poisoned |= def.poisoned;
            }
        }
        let mut cursor = n.walk();
        stack.extend(n.children(&mut cursor));
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
        for parsed in files.values().filter(|p| is_c_family(p)) {
            collect(parsed, parsed.tree.root_node(), &mut raw);
        }
        Self::close(raw)
    }

    /// The one-file census: the seam for callers with no repository.
    pub fn of_file(parsed: &ParsedFile) -> Self {
        let mut raw = BTreeMap::new();
        if is_c_family(parsed) {
            collect(parsed, parsed.tree.root_node(), &mut raw);
        }
        Self::close(raw)
    }

    /// Close each definition over the in-repo macros its body names. A macro that reaches
    /// itself (recursion, a self-reference included) is poisoned, and poison, control and
    /// free names propagate to every macro that reaches it.
    fn close(raw: BTreeMap<String, RawDef>) -> Self {
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
        Self { macros }
    }

    /// Facts for a macro name, `None` when the name is not defined in a parsed file.
    pub fn get(&self, name: &str) -> Option<&MacroFacts> {
        self.macros.get(name)
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
        if self.macros.is_empty() {
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
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}
