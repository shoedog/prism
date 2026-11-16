//! #829 G10 (DESIGN-v2 5.3): the exposed-owner test. An owner must not be captured by a macro
//! argument list that begins outside it, because the compiler then splits the owner's own
//! tokens at the commas of that list: SP1 of the design. Two halves:
//!
//! * INTERVAL. The file's own parentheses are counted over every branch of conditional
//!   compilation (the interval `[lo, hi]` of the running balance, unclamped; a `#if` without a
//!   plain `#else` may take no branch). The owner is exposed when a literal `(` before it can
//!   still be open at its start (`hi(a) > min(0, least lo before a)`), or a literal `)` after it
//!   can close something no literal `(` after it opened (`least lo after b < hi(b)`).
//! * HIDDEN OPENER (owner decision OD-C, the two-file form of A-span). The file spells in code,
//!   before the owner, a macro whose expansion can open a parenthesis (`paren_open`): the list it
//!   opens may be closed by a `)` in the file that includes this one (an include can sit in the
//!   middle of an argument list), where no test on this file sees it.
//!
//! The file is cut at every owner start and end, each piece is normalized separately (phases 1
//! and 2, comment stripping: the census's own code) and read by a lenient scanner: a string or
//! character literal ends at its quote or at the line end (skipped text such as `#if 0` prose
//! may hold an apostrophe), a line whose first token is `#` is a directive and yields only
//! `If`/`Elif`/`Else`/`Endif` (its parentheses, a `#define` body, `#error` text count for
//! nothing). Cutting at the owners gives each an exact position in the event list without any
//! offset map. Every lexing variant is checked (trigraphs on and off, lenient and strict
//! splices, C and C++ dialects of a `.c` file, C++ only for a C++ file) and any variant that
//! exposes the owner exposes it. A range that is not a cut, or an unreadable piece, is exposed
//! (fail closed).
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::{
    distinct_norms, lex_ident_end, lex_is_raw_prefix, lex_number_end, lex_raw_string_end,
    strip_piece, Norm,
};
use crate::languages::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ev {
    Open,
    Close,
    If,
    /// `#elif`, `#elifdef`, `#elifndef`: a branch that may not be taken.
    Elif,
    /// A plain `#else`: the branches are exhaustive.
    Else,
    Endif,
}

/// One lexing variant of the whole file, reduced to what `exposed` asks.
struct Variant {
    /// Event index after the piece that ends at cut `k`.
    cut_event: Vec<usize>,
    /// `hi[i]`: the upper bound of the balance after the first `i` events.
    hi: Vec<i64>,
    /// `pre_min[i]`: `min(0, least lo)` over the states `0..=i`.
    pre_min: Vec<i64>,
    /// `suf_min[i]`: the least lo after any `(` or `)` event with index at or beyond `i`.
    suf_min: Vec<i64>,
    /// Ordinal of the first piece that spells (in code) a macro with `paren_open`.
    hidden_piece: Option<usize>,
}

pub(super) struct CFileParens {
    /// Owner offsets (starts and ends), sorted: the cuts.
    cuts: Vec<usize>,
    variants: Vec<Variant>,
    /// Some variant could not read a piece: every owner is exposed.
    unreadable: bool,
}

const INF: i64 = i64::MAX / 4;

impl CFileParens {
    /// The test for a whole file: the cuts are the byte ranges of its owner nodes
    /// (`parsed.all_functions()`), `opens(name)` says the census has `name` with `paren_open`.
    pub(super) fn for_file(parsed: &ParsedFile, opens: &dyn Fn(&str) -> bool) -> Self {
        let mut cuts: Vec<usize> = parsed
            .all_functions()
            .iter()
            .flat_map(|n| [n.start_byte(), n.end_byte()])
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        Self::build(parsed, cuts, opens)
    }

    pub(super) fn build(
        parsed: &ParsedFile,
        cuts: Vec<usize>,
        opens: &dyn Fn(&str) -> bool,
    ) -> Self {
        let cxx_file = parsed.language == Language::Cpp;
        let dialects: &[bool] = if cxx_file { &[true] } else { &[false, true] };
        let source = parsed.source.as_str();
        let mut pieces: Vec<&str> = Vec::with_capacity(cuts.len() + 1);
        let mut at = 0;
        for &c in &cuts {
            let c = c.min(source.len());
            pieces.push(source.get(at..c).unwrap_or(""));
            at = c;
        }
        pieces.push(source.get(at..).unwrap_or(""));
        let mut variants = Vec::new();
        let mut unreadable = false;
        for norm in distinct_norms(source) {
            for &cxx in dialects {
                match build_variant(&pieces, norm, cxx, cxx_file, opens) {
                    Some(v) => variants.push(v),
                    None => unreadable = true,
                }
            }
        }
        Self {
            cuts,
            variants,
            unreadable,
        }
    }

    /// Can the owner `[start, end)` be captured by an argument list that starts outside it?
    pub(super) fn exposed(&self, start: usize, end: usize) -> bool {
        if self.unreadable {
            return true;
        }
        let (Ok(s), Ok(t)) = (
            self.cuts.binary_search(&start),
            self.cuts.binary_search(&end),
        ) else {
            return true;
        };
        self.variants.iter().any(|v| {
            let (a, b) = (v.cut_event[s], v.cut_event[t]);
            v.hi[a] > v.pre_min[a]
                || v.suf_min[b] < v.hi[b]
                || v.hidden_piece.is_some_and(|k| k <= s)
        })
    }
}

fn build_variant(
    pieces: &[&str],
    norm: Norm,
    cxx: bool,
    precise: bool,
    opens: &dyn Fn(&str) -> bool,
) -> Option<Variant> {
    let mut events: Vec<Ev> = Vec::new();
    let mut cut_event = Vec::with_capacity(pieces.len());
    let mut hidden_piece = None;
    let mut at_line_start = true;
    for (k, piece) in pieces.iter().enumerate() {
        let text = strip_piece(piece, norm, cxx, precise)?;
        let mut hit = false;
        scan_piece(&text, cxx, at_line_start, &mut events, &mut |name| {
            hit |= opens(name)
        });
        if hit && hidden_piece.is_none() {
            hidden_piece = Some(k);
        }
        // The next piece starts a line only when this one ended with one.
        at_line_start = text.is_empty() && at_line_start || text.ends_with('\n');
        cut_event.push(events.len());
    }
    let (hi, pre_min, suf_min) = intervals(&events);
    Some(Variant {
        cut_event,
        hi,
        pre_min,
        suf_min,
        hidden_piece,
    })
}

/// The lenient scan of one stripped piece: parentheses and conditional directives into
/// `events`, every identifier of code into `ident`.
fn scan_piece(
    text: &str,
    cxx: bool,
    at_line_start: bool,
    events: &mut Vec<Ev>,
    ident: &mut dyn FnMut(&str),
) {
    let b = text.as_bytes();
    let mut i = 0;
    let mut bol = at_line_start;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            bol = true;
            i += 1;
            continue;
        }
        if matches!(c, b' ' | b'\t' | b'\r' | 0x0b | 0x0c) {
            i += 1;
            continue;
        }
        if bol {
            bol = false;
            let hash = if c == b'#' {
                Some(1)
            } else if c == b'%' && b.get(i + 1) == Some(&b':') {
                Some(2)
            } else {
                None
            };
            if let Some(n) = hash {
                let end = b[i..]
                    .iter()
                    .position(|&d| d == b'\n')
                    .map_or(b.len(), |p| i + p);
                if let Some(ev) = directive_event(&text[i + n..end]) {
                    events.push(ev);
                }
                i = end;
                continue;
            }
        }
        match c {
            b'(' => {
                events.push(Ev::Open);
                i += 1;
            }
            b')' => {
                events.push(Ev::Close);
                i += 1;
            }
            b'"' | b'\'' => {
                i += 1;
                while i < b.len() && b[i] != b'\n' {
                    let d = b[i];
                    i += 1;
                    if d == b'\\' && i < b.len() && b[i] != b'\n' {
                        i += 1;
                    } else if d == c {
                        break;
                    }
                }
            }
            c if c.is_ascii_digit() => i = lex_number_end(b, i, cxx),
            _ => match lex_ident_end(b, i) {
                Some(end) => {
                    let name = &text[i..end];
                    if cxx && b.get(end) == Some(&b'"') && lex_is_raw_prefix(name) {
                        // A raw string reads to its end, newlines included; an unreadable
                        // one ends at the line like any other literal.
                        i = lex_raw_string_end(b, end).unwrap_or(end + 1);
                    } else {
                        ident(name);
                        i = end;
                    }
                }
                None => i += 1,
            },
        }
    }
}

/// The conditional event of a directive line (the text after its `#`), if it is one.
fn directive_event(rest: &str) -> Option<Ev> {
    let word: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    match word.as_str() {
        "if" | "ifdef" | "ifndef" => Some(Ev::If),
        "elif" | "elifdef" | "elifndef" => Some(Ev::Elif),
        "else" => Some(Ev::Else),
        "endif" => Some(Ev::Endif),
        _ => None,
    }
}

/// The balance intervals over the events: `(hi, pre_min, suf_min)` indexed by the number of
/// events processed (see `Variant`).
fn intervals(events: &[Ev]) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
    struct Cond {
        base: (i64, i64),
        ends: Vec<(i64, i64)>,
        plain_else: bool,
    }
    let n = events.len();
    let (mut lo, mut hi) = (0i64, 0i64);
    let mut conds: Vec<Cond> = Vec::new();
    let mut his = Vec::with_capacity(n + 1);
    let mut pre = Vec::with_capacity(n + 1);
    let mut code_lo = vec![INF; n]; // lo after event m when it is a parenthesis, else INF
    let mut least = 0i64;
    his.push(hi);
    pre.push(0);
    for (m, ev) in events.iter().enumerate() {
        match ev {
            Ev::Open => {
                lo += 1;
                hi += 1;
                code_lo[m] = lo;
            }
            Ev::Close => {
                lo -= 1;
                hi -= 1;
                code_lo[m] = lo;
            }
            Ev::If => conds.push(Cond {
                base: (lo, hi),
                ends: Vec::new(),
                plain_else: false,
            }),
            Ev::Elif | Ev::Else => {
                if let Some(top) = conds.last_mut() {
                    top.ends.push((lo, hi));
                    top.plain_else |= *ev == Ev::Else;
                    (lo, hi) = top.base;
                }
            }
            Ev::Endif => {
                if let Some(top) = conds.pop() {
                    let mut ends = top.ends;
                    ends.push((lo, hi));
                    if !top.plain_else {
                        ends.push(top.base);
                    }
                    lo = ends.iter().map(|e| e.0).min().unwrap_or(lo);
                    hi = ends.iter().map(|e| e.1).max().unwrap_or(hi);
                }
            }
        }
        least = least.min(lo);
        his.push(hi);
        pre.push(least);
    }
    let mut suf = vec![INF; n + 1];
    for m in (0..n).rev() {
        suf[m] = suf[m + 1].min(code_lo[m]);
    }
    (his, pre, suf)
}
