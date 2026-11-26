use crate::element::{CanonicalPath, Capture, CaptureWidth, PathToken};
use std::collections::BTreeMap;

impl CanonicalPath {
    /// Test path geometry under this route's spelling policy. A dispatch selector is separate.
    pub fn accepts(&self, input: &str) -> bool {
        let Some(input) = input.strip_prefix('/') else {
            return false;
        };
        let normalized = if self.ascii_fold {
            input.to_ascii_lowercase()
        } else {
            input.into()
        };
        let normalized = if self.strip_slash {
            normalized.trim_end_matches('/')
        } else {
            &normalized
        };
        let segments = normalized.split('/').collect::<Vec<_>>();
        if self.bare_parent
            && matches!(
                self.segments.last().map(Vec::as_slice),
                Some([PathToken::Capture(Capture {
                    width: CaptureWidth::ZeroOrMore,
                    ..
                })])
            )
            && accepts_segments(
                &self.segments[..self.segments.len() - 1],
                &segments,
                &BTreeMap::new(),
            )
        {
            return true;
        }
        accepts_segments(&self.segments, &segments, &BTreeMap::new())
    }
}

fn accepts_segments<'a>(
    tokens: &[Vec<PathToken>],
    input: &[&'a str],
    names: &BTreeMap<String, &'a str>,
) -> bool {
    let Some((segment, rest)) = tokens.split_first() else {
        return input.is_empty();
    };
    if let [PathToken::Capture(capture)] = segment.as_slice() {
        if capture.optional_separator && accepts_segments(rest, input, names) {
            return true;
        }
        let count = match capture.width {
            CaptureWidth::OneOrMore => Some(1..=input.len()),
            CaptureWidth::ZeroOrMore => {
                // A retained separator with nothing after it represents an empty tail.
                if input == [""] && rest.is_empty() {
                    return true;
                }
                Some(1..=input.len())
            }
            CaptureWidth::Exactly(count) => Some(count..=count.min(input.len())),
            _ => None,
        };
        if let Some(count) = count {
            return count.into_iter().any(|count| {
                input[..count].iter().all(|part| !part.is_empty())
                    && accepts_segments(rest, &input[count..], names)
            });
        }
    }
    let Some((text, remaining)) = input.split_first() else {
        return false;
    };
    accepts_tokens(segment, text, names, &|names| {
        accepts_segments(rest, remaining, names)
    })
}

fn accepts_tokens<'a>(
    tokens: &[PathToken],
    input: &'a str,
    names: &BTreeMap<String, &'a str>,
    rest: &impl Fn(&BTreeMap<String, &'a str>) -> bool,
) -> bool {
    let Some((token, tokens)) = tokens.split_first() else {
        return input.is_empty() && rest(names);
    };
    match token {
        PathToken::Literal(literal) => input
            .strip_prefix(literal)
            .is_some_and(|suffix| accepts_tokens(tokens, suffix, names, rest)),
        PathToken::Capture(capture) => {
            if capture.constraint.is_some() {
                return false;
            }
            if let Some(known) = names.get(&capture.name) {
                return input
                    .strip_prefix(known)
                    .is_some_and(|suffix| accepts_tokens(tokens, suffix, names, rest));
            }
            input
                .char_indices()
                .map(|(offset, _)| offset)
                .chain([input.len()])
                .filter(|offset| *offset > 0 || capture.width == CaptureWidth::Optional)
                .any(|offset| {
                    let (value, suffix) = input.split_at(offset);
                    let mut names = names.clone();
                    names.insert(capture.name.clone(), value);
                    accepts_tokens(tokens, suffix, &names, rest)
                })
        }
    }
}
