//! C++ raw string literals for the macro census (#630 slice 6d-1).
//!
//! C++ [lex.pptoken]/3: between the opening `"` and the closing delimiter, line splicing
//! (phase 2) is reverted (the standard also reverts trigraphs, which the census cannot do: a
//! text with a raw-string candidate and a trigraph fails closed, see `first_trigraph`), so a backslash-newline inside the
//! literal is two characters of its text and the closing `)delim"` is found on the text as
//! written. The census reads the SPLICED buffer, so the closing delimiter is searched there
//! and a candidate is accepted only when no splice point falls inside it (a splice point
//! inside the match means the delimiter is not contiguous as written). The delimiter itself
//! cannot hold a backslash, so a splice between the opening quote and the `(` is ill-formed
//! once reverted: uncertain. A splice right before the quote (`R\` newline `"(..)"`) is
//! applied normally: the prefix is recognised after phase 2.

/// `q` indexes the opening `"` of `R"delim( .. )delim"`; the index after the closing quote,
/// in the spliced buffer. The delimiter is up to 16 characters other than space, `(`, `)`,
/// `\`, tab, VT, FF and newline (`"` is allowed). `None` means uncertain: an invalid
/// delimiter, a splice inside the delimiter or an unterminated literal. `splices` holds the
/// sorted indexes of `b` at which a splice was removed; an empty slice means the CALLER
/// ASSERTS the buffer had no splices (the readers of already-spliced text pass one, so a
/// closing delimiter that exists only after splicing is not detected there; the census and
/// the owners refuse such text through `interior_splice` and `has_spliced_raw_string`).
pub(super) fn raw_string_end(b: &[u8], q: usize, splices: &[usize]) -> Option<usize> {
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
    // Splice points in [q + 1, j] lie between the quote and the `(`.
    let first = splices.partition_point(|&s| s <= q);
    if splices.get(first).is_some_and(|&s| s <= j) {
        return None;
    }
    let mut close = vec![b')'];
    close.extend_from_slice(&b[start..j]);
    close.push(b'"');
    let body = j + 1;
    let mut from = body;
    while let Some(p) = b[from..]
        .windows(close.len())
        .position(|w| w == close.as_slice())
    {
        let m = from + p;
        let end = m + close.len();
        // A splice point strictly inside the match: not contiguous as written.
        let k = splices.partition_point(|&s| s <= m);
        if !splices.get(k).is_some_and(|&s| s < end) {
            return Some(end);
        }
        from = m + 1;
    }
    None
}

/// Does a splice point fall inside the literal `[q, end)` (strictly after the quote)?
pub(super) fn interior_splice(splices: &[usize], q: usize, end: usize) -> bool {
    let k = splices.partition_point(|&s| s <= q);
    splices.get(k).is_some_and(|&s| s < end)
}

/// Could this stripped output line (up to and including the raw prefix) be, or continue, a
/// directive? Any `#`, `%:` or `??=` before the literal counts, whatever blanks or stripped
/// comments precede it (over-approximation: the directive readers skip VT, FF and comments
/// the literal's own line check must not have to mirror).
pub(super) fn starts_directive(line: &[u8]) -> bool {
    line.contains(&b'#')
        || line.windows(2).any(|w| w == b"%:")
        || line.windows(3).any(|w| w == b"??=")
}

/// Does the text hold a trigraph sequence (`??` and one of `= / ' ( ) ! < > -`)? Returns the
/// byte offset of the first one.
pub(super) fn first_trigraph(b: &[u8]) -> Option<usize> {
    b.windows(3).position(|w| {
        w[0] == b'?'
            && w[1] == b'?'
            && matches!(
                w[2],
                b'=' | b'/' | b'\'' | b'(' | b')' | b'!' | b'<' | b'>' | b'-'
            )
    })
}
