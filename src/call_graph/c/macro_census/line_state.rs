//! The status of the output line a block comment starts on (#630 slice 6d-1, fold 8).
//!
//! A block comment that holds a newline stays one space on a directive's logical line (the
//! directive goes on after it, C phase 3 runs before phase 4) and keeps its newlines
//! elsewhere (so a directive after `*/` starts its line). Getting that wrong loses facts in
//! either direction (a `#define` after `*/`, or the tail of a macro body), so the decision is
//! made only for a PROVEN line:
//!
//! * the line BEGAN outside any string, character or raw-string literal and outside any
//!   earlier comment, AND
//! * either its first preprocessing token, bytes <= 0x20 and stripped comments skipped, is
//!   `#`, `%:` or `??=` (a directive), or no such byte sequence appears anywhere on the line
//!   before the comment, in code, literals or earlier comments alike (not a directive).
//!
//! Every other line is UNPROVEN: the caller reads both variants and unions their facts and
//! refusals, so one of them is always the compiler's reading. The state is updated
//! incrementally over the output buffer (each byte scanned once, so a long line of comments
//! stays linear); it is only ever synchronised where the buffer cannot end inside a
//! two- or three-byte leader.

/// What is proven about the line a comment starts on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Status {
    /// Clean line start and a directive leader as its first token.
    Directive,
    /// Clean line start and no leader byte sequence before the comment.
    NotDirective,
    /// Anything else.
    Unproven,
}

#[derive(Default)]
pub(super) struct LineState {
    scanned: usize,
    /// The current output line began inside a literal or a comment.
    inside: bool,
    /// A `#`, `%:` or `??=` anywhere so far on the current line.
    leader: bool,
    first_seen: bool,
    /// The first token of the current line is `#`, `%:` or `??=`.
    first_dir: bool,
}

fn holds_leader(b: &[u8]) -> bool {
    b.contains(&b'#') || b.windows(2).any(|w| w == b"%:") || b.windows(3).any(|w| w == b"??=")
}

impl LineState {
    /// Scan the bytes of `out` pushed since the last call.
    pub(super) fn update(&mut self, out: &[u8]) {
        for k in self.scanned..out.len() {
            let c = out[k];
            if c == b'\n' {
                *self = LineState {
                    scanned: self.scanned,
                    ..LineState::default()
                };
                continue;
            }
            if !self.first_seen && c > 0x20 {
                self.first_seen = true;
                self.first_dir = c == b'#'
                    || (c == b'%' && out.get(k + 1) == Some(&b':'))
                    || (c == b'?'
                        && out.get(k + 1) == Some(&b'?')
                        && out.get(k + 2) == Some(&b'='));
            }
            match c {
                b'#' => self.leader = true,
                b':' if k > 0 && out[k - 1] == b'%' => self.leader = true,
                b'=' if k > 1 && out[k - 1] == b'?' && out[k - 2] == b'?' => self.leader = true,
                _ => {}
            }
        }
        self.scanned = out.len();
    }

    /// `out` now ends with bytes that include a newline taken from inside a literal or a
    /// comment: the line that follows began inside it.
    pub(super) fn began_inside(&mut self, out: &[u8]) {
        self.update(out);
        self.inside = true;
    }

    /// A comment was stripped from the current line: its text counts as "before" a later
    /// comment on the same line.
    pub(super) fn note_comment(&mut self, body: &[u8]) {
        if holds_leader(body) {
            self.leader = true;
        }
    }

    pub(super) fn status(&self) -> Status {
        if self.inside {
            Status::Unproven
        } else if self.first_dir {
            Status::Directive
        } else if !self.leader {
            Status::NotDirective
        } else {
            Status::Unproven
        }
    }
}
