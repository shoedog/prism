//! Operation and field identity. Typed paths retain evidence that is absent from the spelling.

use std::fmt::{self, Write};
use std::sync::LazyLock;

use regex::Regex;

mod pointer;
pub mod route;

pub use pointer::{decode_fragment_pointer, PointerEncodingError, PointerToken};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OperationId {
    method: String,
    path: String,
}

impl OperationId {
    /// The caller applies the method policy of the source dialect.
    pub fn new(method: &str, path: &CanonicalPath) -> Self {
        Self {
            method: method.into(),
            path: path.to_string(),
        }
    }

    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "op:{} {}", self.method, self.path)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SuccessKey {
    Exact(u16),
    Range,
}

impl fmt::Display for SuccessKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(code) => write!(f, "{code}"),
            Self::Range => f.write_str("2XX"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageRoot {
    Request(OperationId),
    Response(OperationId, SuccessKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldId {
    pub root: MessageRoot,
    pub pointer: Vec<PointerToken>,
}

impl FieldId {
    pub fn new(root: MessageRoot, pointer: Vec<PointerToken>) -> Self {
        Self { root, pointer }
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.root {
            MessageRoot::Request(op) => write!(f, "field:req:{} {}", op.method(), op.path())?,
            MessageRoot::Response(op, status) => {
                write!(f, "field:resp:{} {}:{status}", op.method(), op.path())?
            }
        }
        f.write_char('#')?;
        for token in &self.pointer {
            write!(f, "/{token}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureWidth {
    Required,
    Optional,
    OneOrMore,
    ZeroOrMore,
    Exactly(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    pub name: String,
    pub width: CaptureWidth,
    pub optional_separator: bool,
    pub constraint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathToken {
    Literal(String),
    Capture(Capture),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalPath {
    pub segments: Vec<Vec<PathToken>>,
    pub ascii_fold: bool,
    pub strip_slash: bool,
    pub bare_parent: bool,
}

impl CanonicalPath {
    pub fn captures(&self) -> impl Iterator<Item = &Capture> {
        self.segments
            .iter()
            .flatten()
            .filter_map(|token| match token {
                PathToken::Capture(capture) => Some(capture),
                PathToken::Literal(_) => None,
            })
    }
}

impl fmt::Display for CanonicalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            f.write_char('/')?;
            for token in segment {
                match token {
                    PathToken::Literal(text) => f.write_str(text)?,
                    PathToken::Capture(capture) => match capture.width {
                        CaptureWidth::Required => f.write_str("{}")?,
                        CaptureWidth::Optional => f.write_str("{?}")?,
                        CaptureWidth::OneOrMore => f.write_str("{*}")?,
                        CaptureWidth::ZeroOrMore => f.write_str("{*0+}")?,
                        CaptureWidth::Exactly(count) => write!(f, "{{*{count}}}")?,
                    },
                }
            }
        }
        Ok(())
    }
}

pub fn schema_path(raw: &str, base: &str, labels: &[&str]) -> CanonicalPath {
    static BRACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^{}]+)\}").unwrap());
    let raw = raw.strip_prefix('/').unwrap_or(raw);
    let prefix = base.strip_prefix('/').unwrap_or(base);
    let paths = if prefix.is_empty() {
        raw.into()
    } else {
        format!("{prefix}/{raw}")
    };
    // A server base is already a resolved URI path. Only the declared suffix has captures.
    let literal_segments = if prefix.is_empty() {
        0
    } else {
        prefix.split('/').count()
    };
    let segments = paths
        .split('/')
        .enumerate()
        .map(|(i, segment)| {
            if i < literal_segments {
                return vec![PathToken::Literal(segment.into())];
            }
            tokenize(segment, &BRACES, |m| {
                let name = &m[1];
                let capture = Capture {
                    name: name.into(),
                    width: CaptureWidth::Required,
                    optional_separator: false,
                    constraint: None,
                };
                let mut tokens = Vec::with_capacity(2);
                if labels.contains(&name) {
                    tokens.push(PathToken::Literal(".".into()));
                }
                tokens.push(PathToken::Capture(capture));
                tokens
            })
        })
        .collect();
    CanonicalPath {
        segments,
        ascii_fold: false,
        strip_slash: false,
        bare_parent: false,
    }
}

fn tokenize(
    segment: &str,
    pattern: &Regex,
    mut capture: impl FnMut(regex::Captures<'_>) -> Vec<PathToken>,
) -> Vec<PathToken> {
    let mut start = 0;
    let mut tokens = Vec::new();
    for matched in pattern.captures_iter(segment) {
        let range = matched.get(0).unwrap().range();
        if range.start > start {
            tokens.push(PathToken::Literal(segment[start..range.start].into()));
        }
        tokens.extend(capture(matched));
        start = range.end;
    }
    if start < segment.len() {
        tokens.push(PathToken::Literal(segment[start..].into()));
    }
    tokens
}
