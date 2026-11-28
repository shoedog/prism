//! The pinned swaggo `@Router` grammar. Anything this module does not accept is a recorded gap.
//!
//! Canonical line: `//` (exactly two slashes), optional blanks, the literal `@Router`, blanks,
//! an absolute literal path without `?` or `#`, optional blanks, `[` method `]`, optional blanks.
//! The method is ASCII letters, compared case-insensitively against the seven verbs of the
//! schema reader, and its raw spelling is preserved. Block comments are never read.

const VERBS: [&str; 7] = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Directive {
    Canonical {
        method: String,
        path: String,
    },
    Invalid {
        reason: &'static str,
        method: Option<String>,
        path: Option<String>,
    },
}

const BLANKS: [char; 2] = [' ', '\t'];

/// Whether a line comment names `@Router` at all, in any spelling this reader recognises.
pub(super) fn recognise(comment: &str) -> Option<Directive> {
    let slashes = comment.len() - comment.trim_start_matches('/').len();
    if slashes < 2 {
        return None;
    }
    let after = comment[slashes..].trim_start_matches(BLANKS);
    let word = after.split(char::is_whitespace).next().unwrap_or("");
    if !word.eq_ignore_ascii_case("@router") {
        return None;
    }
    let invalid = |reason, method: Option<&str>, path: Option<&str>| {
        Some(Directive::Invalid {
            reason,
            method: method.map(Into::into),
            path: path.map(Into::into),
        })
    };
    if slashes != 2 || word != "@Router" {
        return invalid("malformed_directive", None, None);
    }
    let body = &after[word.len()..];
    let trimmed = body.trim_start_matches(BLANKS);
    if trimmed.len() == body.len() {
        return invalid("malformed_directive", None, None);
    }
    let path_end = trimmed.find(BLANKS).unwrap_or(trimmed.len());
    let (path, tail) = trimmed.split_at(path_end);
    let Some(inner) = tail
        .trim_start_matches(BLANKS)
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .filter(|(_, after)| after.trim_matches(BLANKS).is_empty())
        .map(|(inner, _)| inner)
    else {
        return invalid(
            "malformed_directive",
            None,
            Some(path).filter(|p| !p.is_empty()),
        );
    };
    if !path.starts_with('/') || path.contains(['?', '#']) {
        return invalid("invalid_path", Some(inner), Some(path));
    }
    if inner.is_empty() || !inner.bytes().all(|b| b.is_ascii_alphabetic()) {
        return invalid("malformed_directive", Some(inner), Some(path));
    }
    if !VERBS.contains(&inner.to_ascii_uppercase().as_str()) {
        return invalid("unsupported_method", Some(inner), Some(path));
    }
    Some(Directive::Canonical {
        method: inner.into(),
        path: path.into(),
    })
}

/// A block comment that mentions `@Router` is an unsupported form, never bound.
pub(super) fn mentions_in_block(comment: &str) -> bool {
    comment.to_ascii_lowercase().contains("@router")
}

/// The Go generated-file header: `// Code generated .* DO NOT EDIT.` before the package clause.
pub(super) fn is_generated_header(comment: &str) -> bool {
    const HEAD: &str = "// Code generated ";
    const TAIL: &str = " DO NOT EDIT.";
    comment.len() >= HEAD.len() + TAIL.len() && comment.starts_with(HEAD) && comment.ends_with(TAIL)
}
