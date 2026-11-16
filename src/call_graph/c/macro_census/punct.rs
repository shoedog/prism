//! Preprocessing punctuators by maximal munch (#829, [lex.pptoken]/3, C++ [lex.operators],
//! C 6.4.6): the next token is the longest sequence that can form one, so the second byte of a
//! longer token never starts another (`::>` is `::` `>`, never `:` `:>`; `<<%` is `<<` `%`).
//! The set is closed, so this is the whole table, not a list of instances; the digraphs
//! `<: :> <% %> %: %:%:` are entries of it like any other.
//!
//! Three readings differ: C++ (`::`, `.*`, `->*`, `<=>`, and the `<::` exception), C17 (none of
//! them) and C23 (`::`). The census unions the readings, so a hazard of any one counts.

/// The lexing reading of one pass.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Lex {
    Cxx,
    C17,
    C23,
}

impl Lex {
    pub(super) fn cxx(self) -> bool {
        self == Lex::Cxx
    }

    /// The readings a pass of the C dialect (`cxx` false) or the C++ dialect stands for.
    pub(super) fn readings(cxx: bool) -> &'static [Lex] {
        if cxx {
            &[Lex::Cxx]
        } else {
            &[Lex::C17, Lex::C23]
        }
    }
}

/// What a punctuator token is, for the structural stream.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Punct {
    /// `( ) { } [ ] ,` (the byte).
    Structural(u8),
    /// `<% %> <: :>`, carrying the bracket spelled.
    Digraph(u8),
    /// `##` or `%:%:`.
    Paste,
    /// Every other punctuator.
    Plain,
}

const FOUR: &[&[u8]] = &[b"%:%:"];
const THREE: &[(&[u8], Lex)] = &[
    (b"...", Lex::C17),
    (b"<<=", Lex::C17),
    (b">>=", Lex::C17),
    (b"->*", Lex::Cxx),
    (b"<=>", Lex::Cxx),
];
const TWO: &[&[u8]] = &[
    b"->", b"++", b"--", b"<<", b">>", b"<=", b">=", b"==", b"!=", b"&&", b"||", b"+=", b"-=",
    b"*=", b"/=", b"%=", b"^=", b"&=", b"|=", b"<:", b":>", b"<%", b"%>", b"%:", b"##",
];

/// The punctuator starting at `b[i]` and its length, `None` when the byte starts none (the
/// caller reads it as a one-byte other token).
pub(super) fn punct_at(b: &[u8], i: usize, lex: Lex) -> Option<(usize, Punct)> {
    let rest = &b[i..];
    // C++ [lex.pptoken]/3.2: `<::` not followed by `:` or `>` is `<` then `::`.
    if lex == Lex::Cxx && rest.starts_with(b"<::") && !matches!(b.get(i + 3), Some(b':' | b'>')) {
        return Some((1, Punct::Plain));
    }
    for t in FOUR {
        if rest.starts_with(t) {
            return Some((4, Punct::Paste));
        }
    }
    for (t, only) in THREE {
        // `...`, `<<=`, `>>=` are in every reading; `->*` and `<=>` are C++ only.
        let ok = *only == Lex::C17 || *only == lex;
        if ok && rest.starts_with(t) {
            return Some((3, Punct::Plain));
        }
    }
    // `::` and `.*` (C++, and `::` in C23).
    if lex != Lex::C17 && rest.starts_with(b"::") {
        return Some((2, Punct::Plain));
    }
    if lex == Lex::Cxx && rest.starts_with(b".*") {
        return Some((2, Punct::Plain));
    }
    for t in TWO {
        if rest.starts_with(t) {
            return Some((
                2,
                match *t {
                    b"##" => Punct::Paste,
                    b"<%" => Punct::Digraph(b'{'),
                    b"%>" => Punct::Digraph(b'}'),
                    b"<:" => Punct::Digraph(b'['),
                    b":>" => Punct::Digraph(b']'),
                    _ => Punct::Plain,
                },
            ));
        }
    }
    match rest[0] {
        b'(' | b')' | b'{' | b'}' | b'[' | b']' | b',' => Some((1, Punct::Structural(rest[0]))),
        b';' | b':' | b'?' | b'.' | b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'&' | b'|'
        | b'~' | b'!' | b'=' | b'<' | b'>' | b'#' => Some((1, Punct::Plain)),
        _ => None,
    }
}
