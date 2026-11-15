//! #829 body facts of the macro census (DESIGN-v2 section 5.1): does a replacement list nest in
//! `()`, `{}` and `[]`, can it emit a comma outside its own parentheses, and which of its paren
//! groups can become a macro argument list.
//!
//! One ordered scan over the body tokens of `tokenize_seq` keeps a stack of `Paren{potential}`,
//! `VaOpt` (the syntax parentheses of `__VA_OPT__`, which the expansion does not emit, so they
//! are transparent for tightness), `Brace` and `Bracket`. A `(` is a POTENTIAL group when the
//! token before it is ANY identifier or parameter (`__VA_ARGS__` included), or the `)` that
//! closed a potential group or a `__VA_OPT__`: no identifier is assumed to stay what it is
//! spelled (an external macro may erase it, alias it or return it), so no group is exempt by its
//! root. A comma or `__VA_ARGS__` is TIGHT when its innermost non-`__VA_OPT__` bracket is a
//! parenthesis, LOOSE otherwise, and HELD when that bracket is a `{` or `[` while some potential
//! group is open. Every identifier inside an open potential group that is not a parameter is
//! FORWARDED (`group_ids`): a macro named there that is itself loose splits the argument list on
//! a rescan. A second, parentheses-only stack (the `__VA_OPT__` parentheses included) answers
//! `paren_open`: a `(` that no `)` of the body closes can open an argument list the following
//! source text closes (a stray `)` alone does not set it).
use super::Tok;
use std::collections::BTreeSet;

/// The raw facts of one body (before closure over the macros it names).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct BodyFacts {
    pub unbalanced: bool,
    pub loose_comma: bool,
    pub group_held: bool,
    pub paren_open: bool,
    pub group_ids: BTreeSet<String>,
}

impl BodyFacts {
    /// A body no pass could read: every hazard is assumed (fail closed).
    pub(super) fn unreadable() -> Self {
        Self {
            unbalanced: true,
            loose_comma: true,
            group_held: true,
            paren_open: true,
            group_ids: BTreeSet::new(),
        }
    }

    pub(super) fn merge(&mut self, o: Self) {
        self.unbalanced |= o.unbalanced;
        self.loose_comma |= o.loose_comma;
        self.group_held |= o.group_held;
        self.paren_open |= o.paren_open;
        self.group_ids.extend(o.group_ids);
    }
}

enum Entry {
    Paren { potential: bool },
    VaOpt,
    Brace,
    Bracket,
}

/// Facts of the token sequence of one body. `params` are the declared parameters (their
/// identifiers are not forwarded names); `permissive` reads a census text that is not parsed
/// C/C++, where the parameter identities are unknown, so every identifier of a potential group
/// counts. `named_variadic` (a GNU `args...` parameter, whose substituted separators are
/// unknown) fails closed: loose and group-held.
pub(super) fn scan_body(
    seq: &[Tok],
    params: &BTreeSet<String>,
    permissive: bool,
    named_variadic: bool,
) -> BodyFacts {
    let mut f = BodyFacts::default();
    let mut stack: Vec<Entry> = Vec::new();
    let mut potential_open = 0usize;
    let mut parens_only = 0usize;
    // Index of the `)` that closed a potential group or a `__VA_OPT__` last: a `(` right after
    // it is a chained group.
    let mut last_close: Option<usize> = None;
    let mut pending_vaopt = false;
    // The innermost entry that is not a `__VA_OPT__`: is it a parenthesis, a brace or bracket?
    let innermost = |stack: &[Entry]| -> Option<bool> {
        stack.iter().rev().find_map(|e| match e {
            Entry::VaOpt => None,
            Entry::Paren { .. } => Some(true),
            Entry::Brace | Entry::Bracket => Some(false),
        })
    };
    for (i, t) in seq.iter().enumerate() {
        // A digraph is the bracket it spells.
        let t = match t {
            Tok::Digraph(b'{') => &Tok::LBrace,
            Tok::Digraph(b'}') => &Tok::RBrace,
            Tok::Digraph(b'[') => &Tok::LBracket,
            Tok::Digraph(_) => &Tok::RBracket,
            t => t,
        };
        match t {
            Tok::Ident(x) => {
                if x == "__VA_OPT__" && seq.get(i + 1) == Some(&Tok::Open) {
                    pending_vaopt = true;
                } else if x == "__VA_ARGS__" && innermost(&stack) != Some(true) {
                    f.loose_comma = true;
                    if innermost(&stack) == Some(false) && potential_open > 0 {
                        f.group_held = true;
                    }
                }
                if potential_open > 0
                    && x != "__VA_ARGS__"
                    && x != "__VA_OPT__"
                    && (permissive || !params.contains(x))
                {
                    f.group_ids.insert(x.clone());
                }
            }
            Tok::Open => {
                parens_only += 1;
                if pending_vaopt {
                    pending_vaopt = false;
                    stack.push(Entry::VaOpt);
                } else {
                    let potential =
                        i > 0 && (matches!(seq[i - 1], Tok::Ident(_)) || last_close == Some(i - 1));
                    if potential {
                        potential_open += 1;
                    }
                    stack.push(Entry::Paren { potential });
                }
            }
            Tok::Close => {
                parens_only = parens_only.saturating_sub(1);
                match stack.last() {
                    Some(Entry::Paren { potential }) => {
                        if *potential {
                            potential_open -= 1;
                            last_close = Some(i);
                        }
                        stack.pop();
                    }
                    Some(Entry::VaOpt) => {
                        stack.pop();
                        last_close = Some(i);
                    }
                    _ => f.unbalanced = true,
                }
            }
            Tok::LBrace => stack.push(Entry::Brace),
            Tok::LBracket => stack.push(Entry::Bracket),
            Tok::RBrace => match stack.last() {
                Some(Entry::Brace) => {
                    stack.pop();
                }
                _ => f.unbalanced = true,
            },
            Tok::RBracket => match stack.last() {
                Some(Entry::Bracket) => {
                    stack.pop();
                }
                _ => f.unbalanced = true,
            },
            Tok::Comma => {
                if innermost(&stack) != Some(true) {
                    f.loose_comma = true;
                    if innermost(&stack) == Some(false) && potential_open > 0 {
                        f.group_held = true;
                    }
                }
            }
            Tok::Digraph(_) => unreachable!("mapped to its bracket above"),
            Tok::Other => {}
        }
    }
    if !stack.is_empty() {
        f.unbalanced = true;
    }
    f.paren_open = parens_only > 0;
    if named_variadic {
        f.loose_comma = true;
        f.group_held = true;
    }
    f
}

/// The facts of a body read in both lexing dialects: the union of the dialects that read it,
/// `None` when neither could (the caller decides what unreadable means).
pub(super) fn scan_text(
    text: &str,
    params: &BTreeSet<String>,
    permissive: bool,
    named_variadic: bool,
    dialects: &[bool],
) -> Option<BodyFacts> {
    let mut out: Option<BodyFacts> = None;
    for &cxx in dialects {
        let mut seq = Vec::new();
        if !super::tokenize_seq(text, cxx, Some(&mut seq)).ok {
            continue;
        }
        let f = scan_body(&seq, params, permissive, named_variadic);
        match &mut out {
            Some(acc) => acc.merge(f),
            None => out = Some(f),
        }
    }
    out
}

/// The hazard the owner pass found inside a potential callee group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupHazard {
    /// A comma held by `{}`/`[]`, a mismatched closer or a digraph bracket inside a group: the
    /// argument list would be cut apart.
    Split,
    /// An in-repo macro with `loose_comma` named inside a group: its expansion re-splits the
    /// list on a rescan.
    Loose,
}

enum Slot {
    Callee,
    Plain,
    Brace,
    Bracket,
}

/// G9 (DESIGN-v2 5.2), one bracket-stack pass over one token sequence. A `(` is a POTENTIAL
/// callee paren when it directly follows ANY identifier or the `)` of a potential paren (a
/// chained group); every other paren is plain. A token is CHECKED whenever any potential paren
/// is open, at any nesting depth: there is no shield (an unknown name, an identity wrapper or an
/// empty external macro may erase or return what stands before a group, so no group is exempt).
fn group_hazard_in(seq: &[Tok], macro_loose: &dyn Fn(&str) -> bool) -> Option<GroupHazard> {
    let mut stack: Vec<Slot> = Vec::new();
    let mut callee = 0usize;
    let mut closed_callee: Option<usize> = None;
    for (i, t) in seq.iter().enumerate() {
        match t {
            Tok::Open => {
                let potential =
                    i > 0 && (matches!(seq[i - 1], Tok::Ident(_)) || closed_callee == Some(i - 1));
                if potential {
                    callee += 1;
                }
                stack.push(if potential { Slot::Callee } else { Slot::Plain });
            }
            Tok::Close => match stack.last() {
                Some(Slot::Callee) => {
                    stack.pop();
                    callee -= 1;
                    closed_callee = Some(i);
                }
                Some(Slot::Plain) => {
                    stack.pop();
                }
                _ if callee > 0 => return Some(GroupHazard::Split),
                _ => {}
            },
            Tok::LBrace => stack.push(Slot::Brace),
            Tok::LBracket => stack.push(Slot::Bracket),
            Tok::RBrace => match stack.last() {
                Some(Slot::Brace) => {
                    stack.pop();
                }
                _ if callee > 0 => return Some(GroupHazard::Split),
                _ => {}
            },
            Tok::RBracket => match stack.last() {
                Some(Slot::Bracket) => {
                    stack.pop();
                }
                _ if callee > 0 => return Some(GroupHazard::Split),
                _ => {}
            },
            Tok::Comma => {
                if callee > 0 && matches!(stack.last(), Some(Slot::Brace | Slot::Bracket)) {
                    return Some(GroupHazard::Split);
                }
            }
            Tok::Digraph(_) => {
                if callee > 0 {
                    return Some(GroupHazard::Split);
                }
            }
            Tok::Ident(x) => {
                if callee > 0 && macro_loose(x) {
                    return Some(GroupHazard::Loose);
                }
            }
            Tok::Other => {}
        }
    }
    None
}

/// The owner pass of G9 over the owner's text, read in both lexing dialects and every
/// normalization variant the other owner readers use (any refusal refuses). `None`: some
/// literal or comment cannot be read (the caller fails closed, as for the other readers);
/// `Some(None)`: no hazard. `macro_loose` is asked only about identifiers that may name an
/// in-repo macro with `loose_comma`; it never classifies a root.
pub(crate) fn owner_group_hazard(
    text: &str,
    macro_loose: &dyn Fn(&str) -> bool,
) -> Option<Option<GroupHazard>> {
    let mut found = None;
    for t in super::normalized_texts(text) {
        for cxx in [false, true] {
            let mut seq = Vec::new();
            if !super::tokenize_seq(&t, cxx, Some(&mut seq)).ok {
                return None;
            }
            if found.is_none() {
                found = group_hazard_in(&seq, macro_loose);
            }
        }
    }
    Some(found)
}
