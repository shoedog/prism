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
//! (unparsable or unreadable, or a symlink: stamped on every parsed C/C++
//! file, so full, partial and scoped builds agree). `incomplete` means slice 6b must
//! disable rules U and D for ALL C/C++; it is part of the digest.
//!
//! The textual pass is not the compiler's lexer, so it is belted: phase 1 is normalized
//! (BOM, CRLF and lone CR, trigraphs, splices with any blank before the newline); the
//! definition lines are the union of the comment-stripped and the unstripped text, each read
//! in the C++ dialect (raw strings, digit separators) and in the C dialect, with the
//! directive prefix read through blanks and in-line comments in every pass (only the
//! primary pass sets `incomplete` for lexer state; any pass sets it for a define-looking
//! line it cannot read); a soft-poisoned definition keeps its poison unless another pass
//! over the same text read that very definition; and any directive line holding a
//! non-ASCII byte or UCN outside string/char literals and comments, or a trigraph, sets
//! `incomplete` (checked on the raw logical line in both dialect passes), so a spelling this lexer cannot equate with the compiler's never
//! loses a fact silently.
//!
//! The census reads more than the parsed C/C++ files (`repo_loader::c_census`, the hybrid
//! rule): the loader hands it, as text only, the in-repo files a `#include` of any parsed
//! C/C++ file or census text names (found by lexical target normalization and suffix match
//! against every in-repo path) and every file with a C-like extension, read with the
//! PERMISSIVE scan (no lexer state, never `incomplete` from it; comment-kept and
//! comment-erased variants unioned). Only an unreadable or symlinked-out input, or an oversize
//! non-C input holding a directive leader, stamps `incomplete`. A header with an unusual
//! extension reached only through a form prism cannot resolve acts like an external header.
//! Its cache identity is one aggregate entry over the facts of those texts and the lists of
//! files read, contributing and lost, so an unrelated edit does not move it. Review mode
//! parses only the diff files and fails closed when the repository holds unread census inputs.
//!
//! Several definitions of one name are unioned and every judgement over-approximates: a
//! name that is a macro is judged by the union of all its definitions. Nothing consumes the
//! census yet; slice 6a only builds it and persists its digest with the cached DFG.
mod raw_string;
use crate::ast::ParsedFile;
use crate::languages::Language;
use raw_string::raw_string_end;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Exact control spellings: jump keywords and `asm` spellings.
const CONTROL_NAMES: &[&str] = &["goto", "case", "default", "asm", "_asm", "__asm", "__asm__"];

/// Returns-twice stems (G4 of the design), matched as a case-sensitive SUBSTRING of any
/// identifier, so a prefixed wrapper (`__mingw_setjmp`, `_setjmp3`) is covered too. The
/// over-approximation costs only rule D's recall.
const CONTROL_SUBSTRINGS: &[&str] = &["setjmp", "vfork", "getcontext", "savectx"];

fn is_control_name(t: &str) -> bool {
    CONTROL_NAMES.contains(&t) || CONTROL_SUBSTRINGS.iter().any(|s| t.contains(s))
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
pub(crate) struct RawDef {
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
    /// Why `incomplete` is set (diagnostic only: not in the digest, ignored by `==`).
    pub(crate) reasons: IncompleteReasons,
}

/// One trigger that made the census incomplete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompleteReason {
    pub kind: &'static str,
    pub path: String,
    /// 1-based line in the spliced buffer (0 when the trigger is file-level).
    pub line: usize,
    /// The offending source line, truncated.
    pub snippet: String,
}

/// The first [`REASON_CAP`] reasons plus a count per kind. Compares equal to every other
/// value, so it can never change a census comparison.
#[derive(Clone, Debug, Default)]
pub struct IncompleteReasons {
    pub counts: BTreeMap<&'static str, usize>,
    pub first: Vec<IncompleteReason>,
}

impl PartialEq for IncompleteReasons {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for IncompleteReasons {}

const REASON_CAP: usize = 20;

/// The incompleteness accumulator threaded through collection.
#[derive(Default)]
struct Inc {
    on: bool,
    cur: String,
    pub(crate) reasons: IncompleteReasons,
}

impl Inc {
    fn flag(&mut self, kind: &'static str, line: usize, snippet: &str) {
        self.on = true;
        *self.reasons.counts.entry(kind).or_insert(0) += 1;
        if self.reasons.first.len() < REASON_CAP {
            self.reasons.first.push(IncompleteReason {
                kind,
                path: self.cur.clone(),
                line,
                snippet: snippet.chars().take(120).collect(),
            });
        }
    }
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

/// Length of a UCN at `i` (`\u` + 4 hex, `\U` + 8 hex, or a delimited/named `\u{..}`,
/// `\U{..}`, `\N{..}`), else 0.
fn ucn_len(b: &[u8], i: usize) -> usize {
    if b.get(i) != Some(&b'\\') {
        return 0;
    }
    // Delimited (`\u{e9}`, `\U{..}`) and named (`\N{LATIN SMALL LETTER E}`) UCNs: accepted by
    // current compilers in identifiers. Read through the closing brace on the same line (the
    // prefix alone when it is unclosed): the token is complex, so it is hazardous on a
    // define/undef line and poisons a macro, never decoded.
    if matches!(b.get(i + 1), Some(b'u' | b'U' | b'N')) && b.get(i + 2) == Some(&b'{') {
        let close = b[i + 3..]
            .iter()
            .position(|&c| c == b'}' || c == b'\n')
            .filter(|&p| b[i + 3 + p] == b'}');
        return close.map_or(3, |p| 4 + p);
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
fn tokenize(text: &str, cxx: bool) -> Body {
    tokenize_seq(text, cxx, None)
}

/// One token of the ordered stream [`tokenize_seq`] can record: an identifier, or a
/// parenthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Ident(String),
    Open,
    Close,
}

fn tokenize_seq(text: &str, cxx: bool, mut seq: Option<&mut Vec<Tok>>) -> Body {
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
            c if c.is_ascii_digit() => i = pp_number_end(b, i, cxx),
            _ if is_ident_start(b, i) => {
                let start = i;
                i = ident_end(b, i);
                // Slicing is on char boundaries: bytes >= 0x80 are consumed whole and a UCN
                // is ASCII.
                let ident = &text[start..i];
                if cxx && b.get(i) == Some(&b'"') && is_raw_prefix(ident) {
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
                if let Some(seq) = seq.as_deref_mut() {
                    seq.push(Tok::Ident(ident.to_owned()));
                }
            }
            b'(' | b')' => {
                if let Some(seq) = seq.as_deref_mut() {
                    seq.push(if c == b'(' { Tok::Open } else { Tok::Close });
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

const WS: &[char] = &[' ', '\t', '\r', '\x0b', '\x0c'];

/// End of the pp-number starting at `i`: digits, letters, `.`, `_`, signed exponents and
/// C++14/C23 digit separators (`1'000`: a `'` followed by an alphanumeric or `_`).
fn pp_number_end(b: &[u8], mut i: usize, cxx: bool) -> usize {
    while i < b.len() {
        let d = b[i];
        if d.is_ascii_alphanumeric() || d == b'.' || d == b'_' {
            i += 1;
        } else if (d == b'+' || d == b'-') && matches!(b[i - 1], b'e' | b'E' | b'p' | b'P') {
            i += 1;
        } else if cxx
            && d == b'\''
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
    /// `(output index after a stripped comment, original index after it)`: the only places
    /// where output and input offsets stop agreeing.
    shifts: Vec<(usize, usize)>,
    /// `[start, end)` of every stripped comment, in spliced-input offsets (sorted).
    spans: Vec<(usize, usize)>,
}

impl Stripped {
    /// The index in the spliced input of output offset `p`.
    fn origin(&self, p: usize) -> usize {
        let k = self.shifts.partition_point(|&(out, _)| out <= p);
        match k.checked_sub(1).map(|k| self.shifts[k]) {
            Some((out, orig)) => orig + (p - out),
            None => p,
        }
    }

    /// Did stripping remove the spliced-input byte at `pos` as part of a comment?
    fn removed(&self, pos: usize) -> bool {
        let k = self.spans.partition_point(|&(start, _)| start <= pos);
        k.checked_sub(1).is_some_and(|k| pos < self.spans[k].1)
    }
}

/// Comment stripping over spliced text, keeping string, character and (when `cxx`) raw-string
/// literals intact. A block comment becomes one space. An uncertain raw string sets
/// `raw_uncertain` and the rest of the text is kept unstripped (the unstripped pass also
/// reads it). `cxx` selects the dialect: C++ lexing (raw strings, digit separators) or C.
fn strip_comments(b: &[u8], splices: &[usize], cxx: bool) -> Stripped {
    let mut out = Vec::with_capacity(b.len());
    let mut shifts = Vec::new();
    let mut spans = Vec::new();
    let mut unterminated = false;
    let mut raw_uncertain = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let start = i;
                out.push(b' ');
                match b[i + 2..].windows(2).position(|w| w == b"*/") {
                    Some(p) => i += p + 4,
                    None => {
                        unterminated = true;
                        i = b.len();
                    }
                }
                shifts.push((out.len(), i));
                spans.push((start, i));
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let start = i;
                out.push(b' ');
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                shifts.push((out.len(), i));
                spans.push((start, i));
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
                let end = pp_number_end(b, i, cxx);
                out.extend_from_slice(&b[i..end]);
                i = end;
            }
            _ if is_ident_start(b, i) => {
                let start = i;
                i = ident_end(b, i);
                out.extend_from_slice(&b[start..i]);
                if cxx
                    && b.get(i) == Some(&b'"')
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
        shifts,
        spans,
    }
}

/// Source text after translation phases 1-2 and comment stripping (a block comment is one
/// space), in both lexing dialects: the logical lines a directive reader must see. Slice 6b
/// reads `#pragma omp` through it, so a comment or a continuation cannot hide the word.
pub(crate) fn normalized_texts(source: &str) -> Vec<String> {
    let t = translate(source, true);
    let mut out = Vec::new();
    for lenient in [true, false] {
        let (spliced, points) = splice(&t, lenient);
        for cxx in [false, true] {
            out.push(strip_comments(&spliced, &points, cxx).text);
        }
    }
    out
}

/// Every identifier token (the census's one identifier rule) spelled in `source`, read from
/// the normalized text of both dialects and both splice modes, independently of how a
/// parser classifies the spelling (`#define const x = 1;` makes `const` a macro use).
/// Comments, string and character literals and numbers carry none; `None` when a literal or
/// comment cannot be read to its end (the caller fails closed).
pub(crate) fn spelled_identifiers(source: &str) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for text in normalized_texts(source) {
        for cxx in [false, true] {
            let body = tokenize(&text, cxx);
            if !body.ok {
                // An unreadable literal or comment: the token set would be partial.
                return None;
            }
            out.extend(body.idents);
        }
    }
    Some(out)
}

/// Every identifier token between the parentheses that follow ANY spelling `is_macro`
/// accepts (an in-repo macro name, a varargs builtin), read from the owner's normalized token
/// stream in both dialects and splice modes, whatever node the parser made of it: a call
/// argument, a `macro_type_specifier` type (`BIND(x) dummy;`) or a declarator parameter
/// (`long M(x);`). An unclosed list runs to the end (over-approximation). `None` when a
/// literal or comment cannot be read (the caller fails closed).
pub(crate) fn macro_call_arguments(
    source: &str,
    is_macro: &dyn Fn(&str) -> bool,
) -> Option<BTreeSet<String>> {
    let mut out = BTreeSet::new();
    for text in normalized_texts(source) {
        for cxx in [false, true] {
            let mut seq = Vec::new();
            if !tokenize_seq(&text, cxx, Some(&mut seq)).ok {
                return None;
            }
            for (k, t) in seq.iter().enumerate() {
                let Tok::Ident(name) = t else { continue };
                if !is_macro(name) || seq.get(k + 1) != Some(&Tok::Open) {
                    continue;
                }
                // The group, then every group that follows it directly: an argument can
                // expand to a function-like macro name that takes the next `(` group
                // (`ID(TRACKED)(x)` is `TRACKED(x)`), repeated for chains.
                let mut start = k + 1;
                loop {
                    let mut depth = 0usize;
                    let mut end = None;
                    for (off, t) in seq[start..].iter().enumerate() {
                        match t {
                            Tok::Open => depth += 1,
                            Tok::Close => {
                                depth -= 1;
                                if depth == 0 {
                                    end = Some(start + off);
                                    break;
                                }
                            }
                            Tok::Ident(a) => insert_ident(&mut out, a),
                        }
                    }
                    match end {
                        Some(e) if seq.get(e + 1) == Some(&Tok::Open) => start = e + 1,
                        _ => break,
                    }
                }
            }
        }
    }
    Some(out)
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

/// `s` without leading blanks and complete in-line `/* .. */` comments (an unclosed one is
/// left in place). The directive prefix is read through it in every pass, so a comment
/// between `#` and `define` (or before the name) cannot hide a definition.
fn skip_blanks(mut s: &str) -> &str {
    loop {
        let t = s.trim_start_matches(WS);
        match t
            .strip_prefix("/*")
            .and_then(|r| r.find("*/").map(|e| &r[e + 2..]))
        {
            Some(rest) => s = rest,
            None => return t,
        }
    }
}

/// Split the text after the `define` keyword (continuations already joined, comments
/// stripped in the primary pass) into name, parameters and body. `None`: no name.
fn parse_define(after: &str) -> Option<Parsed<'_>> {
    let t = skip_blanks(after);
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
/// pass; `soft` (a body or parameter list the pass could not read) is dropped only when
/// another pass over the same text read that very definition (same `#` position) fully.
struct Entry {
    name: String,
    def: RawDef,
    hard: bool,
    soft: bool,
    /// Offset of the directive's `#` in the spliced text (original coordinates).
    pos: usize,
}

fn entry(p: &Parsed<'_>, cxx: bool, pos: usize) -> Entry {
    let mut def = RawDef {
        params: p.params.iter().flatten().cloned().collect(),
        ..RawDef::default()
    };
    let body = tokenize(p.body, cxx);
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
        pos,
    }
}

/// Every logical `#define` line of `text`, with the directive prefix read through blanks
/// and in-line comments. A line that looks like a define but names no macro sets
/// `incomplete` in every pass, except in a comment-kept pass (`kept` is `Some`): there the
/// line is a hazard only when `kept` says some comment-stripped variant keeps it (it takes
/// the line's `#` offset in the spliced input) or a variant could not read its lexer state.
/// `primary` passes also set it for an unreadable parameter or an unclosed parameter list.
/// `origin` maps a text offset to the spliced input.
fn defs_in(
    text: &str,
    origin: &dyn Fn(usize) -> usize,
    primary: bool,
    cxx: bool,
    kept: Option<&dyn Fn(usize) -> bool>,
    incomplete: &mut Inc,
) -> Vec<Entry> {
    let mut found = Vec::new();
    let mut line_start = 0;
    for (line_no, line) in text.split('\n').enumerate() {
        let start = line_start;
        line_start += line.len() + 1;
        let t = skip_blanks(line);
        let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
            continue;
        };
        let Some(after) = skip_blanks(rest).strip_prefix("define") else {
            continue;
        };
        let ab = after.as_bytes();
        if !ab.is_empty()
            && (ab[0].is_ascii_alphanumeric() || ab[0] == b'_' || ab[0] == b'$' || ab[0] >= 0x80)
        {
            continue; // another directive spelling, e.g. `#definex`
        }
        let Some(p) = parse_define(after) else {
            if kept.map_or(true, |k| k(origin(start + (line.len() - t.len())))) {
                incomplete.flag("unreadable_define", line_no + 1, line);
            }
            continue;
        };
        if primary {
            if p.malformed {
                incomplete.flag("malformed_define", line_no + 1, line);
            }
            if p.bad_params {
                incomplete.flag("bad_params_unclosed_param_list", line_no + 1, line);
            }
        }
        found.push(entry(&p, cxx, origin(start + (line.len() - t.len()))));
    }
    found
}

/// Does a directive line hold a non-ASCII byte or a UCN in a CODE position, read with the
/// tokenizer's own rules for one dialect? Bytes inside string and char literals (raw strings
/// included) and comments cannot change a name or a definition. `None` means the literal
/// boundaries are uncertain (an unterminated literal or comment, an uncertain raw string):
/// the caller then treats any non-ASCII byte of the line as a hazard.
fn code_hazard(line: &str, cxx: bool) -> Option<bool> {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'/' if b.get(i + 1) == Some(&b'*') => match line[i + 2..].find("*/") {
                Some(end) => i += end + 4,
                None => return None,
            },
            b'/' if b.get(i + 1) == Some(&b'/') => return Some(false),
            b'"' | b'\'' => {
                i += 1;
                let mut closed = false;
                while i < b.len() {
                    match b[i] {
                        b'\\' => i += 2,
                        q if q == c => {
                            closed = true;
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                if !closed {
                    return None;
                }
            }
            c if c.is_ascii_digit() => i = pp_number_end(b, i, cxx),
            _ if is_ident_start(b, i) => {
                let start = i;
                i = ident_end(b, i);
                let ident = &line[start..i];
                if ident.bytes().any(|d| d >= 0x80 || d == b'\\') {
                    return Some(true);
                }
                if cxx && b.get(i) == Some(&b'"') && is_raw_prefix(ident) {
                    i = raw_string_end(b, i, &[])?;
                }
            }
            // A UCN is `\u` + 4 hex or `\U` + 8 hex; a bare `\U` (`C:\Users`) is not one.
            b'\\' if ucn_len(b, i) > 0 => return Some(true),
            _ => i += 1,
        }
    }
    Some(false)
}

/// Can this directive line (starting at its `#`, `%:` or `??=`) create or remove a macro
/// fact? Only `define` and `undef` do. `false` only when the directive name is a plain ASCII
/// word that ends cleanly and is neither: any other spelling (a trigraph, UCN, non-ASCII byte
/// or `$` next to the word, a comment or `??=` prefix the reader cannot see through, an empty
/// name) stays a possible define.
fn may_define_or_undef(t: &str) -> bool {
    let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
        return true;
    };
    let rest = skip_blanks(rest);
    let b = rest.as_bytes();
    let n = b
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == b'_')
        .count();
    if n == 0 {
        return true;
    }
    if matches!(b.get(n), Some(c) if *c == b'\\' || *c == b'?' || *c == b'$' || *c >= 0x80) {
        return true;
    }
    matches!(&rest[..n], "define" | "undef")
}

/// Is this directive line (starting at its `#` or `%:`) an `include`, `include_next` or
/// `import` with a plainly ASCII, cleanly ended directive word?
fn is_include_directive(t: &str) -> bool {
    let Some(rest) = t.strip_prefix('#').or_else(|| t.strip_prefix("%:")) else {
        return false;
    };
    let rest = skip_blanks(rest);
    let b = rest.as_bytes();
    let n = b
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == b'_')
        .count();
    if matches!(b.get(n), Some(c) if *c == b'\\' || *c == b'?' || *c == b'$' || *c >= 0x80) {
        return false;
    }
    matches!(&rest[..n], "include" | "include_next" | "import")
}

/// Is `line` (a logical line of the raw, untranslated text) a directive that holds a
/// non-ASCII byte or UCN outside string/char literals and comments (in either dialect pass;
/// any non-ASCII byte when a pass cannot read the line's literal boundaries), or a trigraph
/// anywhere? Such a line may spell names the lexer here does not equate with the compiler's,
/// so the census fails closed.
fn exotic_directive(spliced: &[u8]) -> Option<(&'static str, usize, String)> {
    let text = String::from_utf8_lossy(spliced);
    text.split('\n').enumerate().find_map(|(no, line)| {
        let t = skip_blanks(line);
        if !(t.starts_with('#') || t.starts_with("%:") || t.starts_with("??=")) {
            return None;
        }
        let include_like = is_include_directive(t);
        if !include_like && !may_define_or_undef(t) {
            return None;
        }
        let b = line.as_bytes();
        // An include target's trigraph is not read here (a backslash target resolves to no
        // file); its non-ASCII or UCN spelling below is.
        let trigraph = !include_like
            && b.windows(3).any(|w| {
                w[0] == b'?'
                    && w[1] == b'?'
                    && matches!(
                        w[2],
                        b'=' | b'/' | b'\'' | b'(' | b')' | b'!' | b'<' | b'>' | b'-'
                    )
            });
        if trigraph {
            return Some(("trigraph_directive", no + 1, line.to_string()));
        }
        [true, false]
            .into_iter()
            .any(|cxx| match code_hazard(line, cxx) {
                Some(h) => h,
                None => b.iter().any(|&c| c >= 0x80),
            })
            .then(|| ("non_ascii_directive", no + 1, line.to_string()))
    })
}

/// Collect one file. Every pass reads the definitions of one spliced buffer: comments
/// stripped (C++ and C dialect: raw strings and digit separators on or off) and comments
/// kept. Only the primary pass (translated text, lenient splices, C++ dialect, comments
/// stripped) sets `incomplete` for lexer state; every other pass only adds definitions, so a
/// lexer divergence cannot lose one. A soft-poisoned definition keeps its poison unless
/// another pass over the same buffer read that very definition without trouble.
fn collect(source: &str, out: &mut BTreeMap<String, RawDef>, incomplete: &mut Inc) {
    let translated = translate(source, true);
    let plain = translate(source, false);
    // Names spelled with non-ASCII bytes, UCNs or trigraphs inside a directive: fail closed.
    for lenient in [true, false] {
        if let Some((kind, line, snippet)) = exotic_directive(&splice(&plain, lenient).0) {
            incomplete.flag(kind, line, &snippet);
            break;
        }
    }
    let mut texts = vec![translated];
    if texts[0] != plain {
        texts.push(plain);
    }
    // Does any variant (both texts, splice modes and dialects) lose its lexer state?
    let desync = texts.iter().any(|text| {
        [true, false].into_iter().any(|lenient| {
            let (spliced, points) = splice(text, lenient);
            [true, false].into_iter().any(|cxx| {
                let s = strip_comments(&spliced, &points, cxx);
                s.unterminated || s.raw_uncertain
            })
        })
    });
    let mut acc: BTreeMap<String, RawDef> = BTreeMap::new();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for (ti, text) in texts.iter().enumerate() {
        for lenient in [true, false] {
            let (spliced, points) = splice(text, lenient);
            let buffer_primary = ti == 0 && lenient;
            if !buffer_primary && seen.contains(&spliced) {
                continue;
            }
            let mut group: Vec<Entry> = Vec::new();
            let variants = [true, false].map(|cxx| strip_comments(&spliced, &points, cxx));
            // A comment-kept pass's unreadable define-looking line is a hazard only when a
            // stripped variant of this buffer keeps it, or some variant desynchronised.
            let kept = |pos: usize| desync || variants.iter().any(|v| !v.removed(pos));
            for (cxx, stripped) in [true, false].into_iter().zip(&variants) {
                let primary = buffer_primary && cxx;
                if primary {
                    if stripped.unterminated {
                        incomplete.flag("unterminated_comment", 0, "");
                    }
                    if stripped.raw_uncertain {
                        incomplete.flag("uncertain_raw_string", 0, "");
                    }
                }
                group.extend(defs_in(
                    &stripped.text,
                    &|p| stripped.origin(p),
                    primary,
                    cxx,
                    None,
                    incomplete,
                ));
                let unstripped = String::from_utf8_lossy(&spliced).into_owned();
                group.extend(defs_in(
                    &unstripped,
                    &|p| p,
                    false,
                    cxx,
                    Some(&kept),
                    incomplete,
                ));
            }
            let recovered: BTreeSet<(usize, String)> = group
                .iter()
                .filter(|e| !e.soft)
                .map(|e| (e.pos, e.name.clone()))
                .collect();
            for mut e in group {
                if e.soft && recovered.contains(&(e.pos, e.name.clone())) {
                    e.def.poisoned = e.hard;
                }
                merge(&mut acc, e.name, e.def);
            }
            seen.push(spliced);
        }
    }
    for (name, def) in acc {
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

// ---- Census text inputs that are not parsed C/C++ (slice 6b hybrid). ----
//
// A file reached by an include discovery, or by the C-like-extension net, is read with a
// PERMISSIVE scan: no comment or string tracking and no `incomplete` from lexer state, so a
// Makefile, a script or prose that happens to contain `/*` or a quote cannot make the census
// incomplete. Every shape it can read as a definition becomes a fact (over-approximation
// only adds facts).

/// The text variants a permissive scan reads: phase 1 with and without trigraphs, lenient
/// and strict splices, each comment-kept and comment-erased (all unioned), plus the NUL-stripped text when it holds NULs (a
/// UTF-16 file without a byte-order mark).
pub(crate) fn permissive_variants(source: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |t: String| {
        if !out.contains(&t) {
            out.push(t);
        }
    };
    let mut bases = vec![source.to_owned()];
    if source.contains('\0') {
        bases.push(source.replace('\0', ""));
    }
    for base in &bases {
        for trigraphs in [true, false] {
            let t = translate(base, trigraphs);
            for lenient in [true, false] {
                let spliced = splice(&t, lenient).0;
                push(String::from_utf8_lossy(&spliced).into_owned());
                // The comment-erased variant: `#/**/define`, `#define/**/N`, a block
                // comment spanning a newline inside a directive.
                push(String::from_utf8_lossy(&erase_comments(&spliced)).into_owned());
            }
        }
    }
    out
}

/// Best-effort comment erasure for the permissive scan: `/* .. */` (newlines inside
/// included) and `// ..` (to the end of the line) become one space, with no string or
/// character tracking and no lexer state (an unterminated block comment runs to the end).
/// Wrong erasures only drop text from this variant; the kept variants are unioned with it.
fn erase_comments(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            out.push(b' ');
            i = match b[i + 2..].windows(2).position(|w| w == b"*/") {
                Some(p) => i + p + 4,
                None => b.len(),
            };
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            out.push(b' ');
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// Text after every directive leader (`#`, `%:`, `??=`) on `line`, wherever it stands.
fn leader_rests(line: &str) -> impl Iterator<Item = &str> {
    let b = line.as_bytes();
    (0..b.len()).filter_map(move |i| {
        let n = if b[i] == b'#' {
            1
        } else if b[i] == b'%' && b.get(i + 1) == Some(&b':') {
            2
        } else if b[i] == b'?' && b[i..].starts_with(b"??=") {
            3
        } else {
            return None;
        };
        Some(&line[i + n..])
    })
}

fn is_ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

/// One permissive define: the name, the identifier-like runs of the rest of the line as free
/// names, `##` as poison. `None` when no identifier name follows.
fn permissive_define(after: &str) -> Option<(String, RawDef)> {
    let t = skip_blanks(after);
    let tb = t.as_bytes();
    if tb.is_empty() || !is_ident_start(tb, 0) {
        return None;
    }
    let end = ident_end(tb, 0);
    let (name, rest) = t.split_at(end);
    let mut def = RawDef::default();
    if is_complex(name) {
        def.poisoned = true;
    }
    if rest.contains("##") || rest.contains("%:%:") {
        def.poisoned = true;
    }
    let rb = rest.as_bytes();
    let mut i = 0;
    while i < rb.len() {
        if is_ident_byte(rb[i]) {
            let s = i;
            while i < rb.len() && is_ident_byte(rb[i]) {
                i += 1;
            }
            let run = &rest[s..i];
            insert_ident(&mut def.free, run);
            let trimmed = run.trim_start_matches(|c: char| c.is_ascii_digit());
            if !trimmed.is_empty() && trimmed != run {
                insert_ident(&mut def.free, trimmed);
            }
        } else {
            i += 1;
        }
    }
    def.control = def.free.iter().any(|t| is_control_name(t));
    Some((name.to_owned(), def))
}

/// Collect the definitions of a census text that is not a parsed C/C++ file.
pub(crate) fn collect_permissive(source: &str, out: &mut BTreeMap<String, RawDef>) {
    for text in permissive_variants(source) {
        for line in text.split('\n') {
            for rest in leader_rests(line) {
                let r = skip_blanks(rest);
                let (after, is_define) = match (r.strip_prefix("define"), r.strip_prefix("undef")) {
                    (Some(a), _) => (a, true),
                    (None, Some(a)) => (a, false),
                    _ => continue,
                };
                if after.as_bytes().first().is_some_and(|&c| is_ident_byte(c)) {
                    continue; // another directive spelling, e.g. `#definex`
                }
                if let Some((name, mut def)) = permissive_define(after) {
                    if !is_define {
                        // An `undef` names a macro without a body: a fact with no names.
                        def = RawDef {
                            poisoned: def.poisoned,
                            ..RawDef::default()
                        };
                    }
                    merge(out, name, def);
                }
            }
        }
    }
}

/// Does a permissive read of `text` yield any macro fact?
pub(crate) fn text_has_facts(text: &str) -> bool {
    let mut raw = BTreeMap::new();
    collect_permissive(text, &mut raw);
    !raw.is_empty()
}

/// The digest of the facts the census text inputs alone yield, for the aggregate `c-census:`
/// cache-identity entry. Parsed C/C++ files are covered by their own content hashes.
pub(crate) fn texts_digest(texts: &BTreeMap<String, String>) -> String {
    let mut raw = BTreeMap::new();
    for text in texts.values() {
        collect_permissive(text, &mut raw);
    }
    CMacroCensus::close(raw, Inc::default()).digest()
}

/// The include targets spelled in `variants`, `(target, quoted)`: `#include`,
/// `#include_next` and `#import` with a quoted or angle target, found after any directive
/// leader on a line.
pub(crate) fn include_targets_in(variants: &[String]) -> BTreeSet<(String, bool)> {
    let mut out = BTreeSet::new();
    for text in variants {
        if !text.contains("include") && !text.contains("import") {
            continue;
        }
        for line in text.split('\n') {
            if !line.contains("include") && !line.contains("import") {
                continue;
            }
            for rest in leader_rests(line) {
                let r = skip_blanks(rest);
                let Some(r) = r
                    .strip_prefix("include")
                    .map(|r| r.strip_prefix("_next").unwrap_or(r))
                    .or_else(|| r.strip_prefix("import"))
                else {
                    continue;
                };
                let t = skip_blanks(r);
                for (open, close, quoted) in [('"', '"', true), ('<', '>', false)] {
                    if let Some(q) = t.strip_prefix(open) {
                        if let Some(end) = q.find(close) {
                            out.insert((q[..end].to_owned(), quoted));
                        }
                    }
                }
            }
        }
    }
    out
}

/// The include targets of a census text (permissive variants only).
pub(crate) fn text_include_targets(source: &str) -> BTreeSet<(String, bool)> {
    include_targets_in(&permissive_variants(source))
}

/// The include targets of a parsed C/C++ source, read on the comment-kept and the
/// comment-stripped variants (both dialects, both splice modes, both trigraph modes).
pub(crate) fn source_include_targets(source: &str) -> BTreeSet<(String, bool)> {
    let mut variants: Vec<String> = Vec::new();
    for trigraphs in [true, false] {
        let t = translate(source, trigraphs);
        for lenient in [true, false] {
            let (spliced, points) = splice(&t, lenient);
            let mut push = |s: String| {
                if !variants.contains(&s) {
                    variants.push(s);
                }
            };
            push(String::from_utf8_lossy(&spliced).into_owned());
            for cxx in [false, true] {
                push(strip_comments(&spliced, &points, cxx).text);
            }
        }
    }
    include_targets_in(&variants)
}

fn is_c_family(parsed: &ParsedFile) -> bool {
    matches!(parsed.language, Language::C | Language::Cpp)
}

impl CMacroCensus {
    /// The census over every parsed C and C++ file of `files` (the whole repository, also
    /// for a scoped build, so a scoped build refuses exactly what a full build does), plus
    /// the loader's text-only include files stamped on them.
    pub fn from_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut raw = BTreeMap::new();
        let mut incomplete = Inc::default();
        let mut texts: BTreeMap<&str, &str> = BTreeMap::new();
        for parsed in files.values().filter(|p| is_c_family(p)) {
            if parsed.c_family_skipped_in_repo() && !incomplete.on {
                incomplete.cur = parsed.path.clone();
                incomplete.flag("loader_incomplete_stamp", 0, "");
            }
            incomplete.cur = parsed.path.clone();
            collect(&parsed.source, &mut raw, &mut incomplete);
            if let Some(t) = parsed.c_census_texts() {
                texts.extend(t.iter().map(|(k, v)| (k.as_str(), v.as_str())));
            }
        }
        for text in texts.values() {
            collect_permissive(text, &mut raw);
        }
        let census = Self::close(raw, incomplete);
        if std::env::var_os("PRISM_C_PRUNE_TRACE").is_some() {
            for l in census.trace_lines() {
                eprintln!("{l}");
            }
        }
        census
    }

    /// The one-file census: the seam for callers with no repository.
    pub fn of_file(parsed: &ParsedFile) -> Self {
        let mut raw = BTreeMap::new();
        let mut incomplete = Inc::default();
        if is_c_family(parsed) {
            incomplete.cur = parsed.path.clone();
            if parsed.c_family_skipped_in_repo() {
                incomplete.flag("loader_incomplete", 0, "");
            }
            collect(&parsed.source, &mut raw, &mut incomplete);
            for text in parsed.c_census_texts().into_iter().flat_map(|t| t.values()) {
                collect_permissive(text, &mut raw);
            }
        }
        Self::close(raw, incomplete)
    }

    /// Close each definition over the in-repo macros its body names. A macro that reaches
    /// itself (recursion, a self-reference included) is poisoned, and poison, control and
    /// free names propagate to every macro that reaches it.
    fn close(raw: BTreeMap<String, RawDef>, inc: Inc) -> Self {
        let incomplete = inc.on;
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
        Self {
            macros,
            incomplete,
            reasons: inc.reasons,
        }
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

    /// Why the census is incomplete (diagnostic; first reasons and counts per kind).
    pub fn incomplete_reasons(&self) -> &IncompleteReasons {
        &self.reasons
    }

    /// The `PRISM_C_PRUNE_TRACE` text: a header line and the first reasons.
    pub fn trace_lines(&self) -> Vec<String> {
        if !self.incomplete {
            return Vec::new();
        }
        let counts: Vec<String> = self
            .reasons
            .counts
            .iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect();
        let mut out = vec![format!("c-census incomplete: {}", counts.join(" "))];
        for r in &self.reasons.first {
            out.push(format!(
                "c-census reason\t{}\t{}:{}\t{}",
                r.kind, r.path, r.line, r.snippet
            ));
        }
        out
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
