use super::{Dialect, Limit, RouteDeclaration};
use crate::element::{tokenize, CanonicalPath, Capture, CaptureWidth, PathToken};
use regex::Regex;
use std::sync::LazyLock;

pub(super) fn paths(
    declaration: &RouteDeclaration,
    prefixes: &[String],
    suffix: &str,
) -> Vec<Result<CanonicalPath, Limit>> {
    let joined = join(declaration.dialect, prefixes, suffix);
    if joined.is_empty() || !joined.starts_with('/') || joined.chars().any(char::is_control) {
        return vec![Err(Limit::PathUnchecked)];
    }
    if declaration.dialect == Dialect::Gin
        && joined
            .rsplit_once('/')
            .is_some_and(|(parent, _)| parent.contains('*'))
    {
        return vec![Err(Limit::PathUnchecked)];
    }
    let expanded = if declaration.dialect == Dialect::Express5 {
        expand_optional_literals(&joined)
    } else {
        Ok(vec![joined])
    };
    match expanded {
        Ok(paths) => paths.iter().map(|path| parse(declaration, path)).collect(),
        Err(limit) => vec![Err(limit)],
    }
}

fn join(dialect: Dialect, prefixes: &[String], suffix: &str) -> String {
    if dialect == Dialect::Gin {
        return prefixes
            .iter()
            .map(String::as_str)
            .chain([suffix])
            .fold("/".into(), gin_join);
    }
    let mut base = String::new();
    for prefix in prefixes {
        let prefix = prefix.trim_matches('/');
        if !prefix.is_empty() {
            base.push('/');
            base.push_str(prefix);
        }
    }
    if base.is_empty() {
        return suffix.into();
    }
    if dialect == Dialect::Hapi && suffix == "/" {
        return base;
    }
    format!("{base}/{}", suffix.strip_prefix('/').unwrap_or(suffix))
}

fn gin_join(base: String, relative: &str) -> String {
    if relative.is_empty() {
        return base;
    }
    let mut parts = Vec::new();
    for part in base.split('/').chain(relative.split('/')) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    let mut path = format!("/{}", parts.join("/"));
    if path != "/" && relative.ends_with('/') {
        path.push('/');
    }
    path
}

fn expand_optional_literals(path: &str) -> Result<Vec<String>, Limit> {
    static GROUP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{([^{}:*?+()\[\]\\]+)\}").unwrap());
    let mut paths = vec![String::new()];
    let mut end = 0;
    for group in GROUP.captures_iter(path) {
        let whole = group.get(0).unwrap();
        if paths.len() >= 64 {
            return Err(Limit::PathUnchecked);
        }
        for partial in &mut paths {
            partial.push_str(&path[end..whole.start()]);
        }
        let with_group = paths
            .iter()
            .map(|p| format!("{p}{}", &group[1]))
            .collect::<Vec<_>>();
        paths.extend(with_group);
        end = whole.end();
    }
    for partial in &mut paths {
        partial.push_str(&path[end..]);
    }
    Ok(paths)
}

fn parse(declaration: &RouteDeclaration, path: &str) -> Result<CanonicalPath, Limit> {
    static BRACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^{}]+)\}").unwrap());
    static GIN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r":([^:*]+)|\*([^:*]+)$").unwrap());
    static EXPRESS4: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r":([A-Za-z0-9_]+)(?:\(([^()]*)\))?(\?)?|\*").unwrap());
    static EXPRESS5: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"\{\*([A-Za-z0-9_$]+)\}|\*([A-Za-z0-9_$]+)|:(?:"([^"]+)"|([A-Za-z0-9_$]+))"#)
            .unwrap()
    });
    static HAPI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\{([^{}?*]+)(\?|\*[0-9]*|)\}").unwrap());
    let pattern = match declaration.dialect {
        Dialect::Literal => None,
        Dialect::Brace => Some(&*BRACE),
        Dialect::Gin => Some(&*GIN),
        Dialect::Express4 => Some(&*EXPRESS4),
        Dialect::Express5 => Some(&*EXPRESS5),
        Dialect::Hapi => Some(&*HAPI),
    };
    let path = if declaration.strip_slash && path != "/" {
        path.trim_end_matches('/')
    } else {
        path
    };
    let mut segments = Vec::new();
    for segment in path.strip_prefix('/').unwrap_or(path).split('/') {
        let mut error = None;
        let mut tokens = match pattern {
            Some(pattern) => tokenize(segment, pattern, |matched| {
                match capture(declaration, segment, &matched) {
                    Ok(capture) => vec![PathToken::Capture(capture)],
                    Err(limit) => {
                        error = Some(limit);
                        Vec::new()
                    }
                }
            }),
            None => vec![PathToken::Literal(segment.into())],
        };
        if let Some(limit) = error {
            return Err(limit);
        }
        for token in &mut tokens {
            if let PathToken::Literal(text) = token {
                let unsupported = match declaration.dialect {
                    Dialect::Express4 | Dialect::Express5 => {
                        text.contains([':', '*', '{', '}', '?', '(', ')', '[', ']', '+', '\\'])
                    }
                    Dialect::Gin => text.contains([':', '*']),
                    Dialect::Hapi | Dialect::Brace => text.contains(['{', '}']),
                    Dialect::Literal => false,
                };
                if unsupported {
                    return Err(Limit::PathUnchecked);
                }
                if declaration.ascii_fold {
                    text.make_ascii_lowercase();
                }
            }
        }
        segments.push(tokens);
    }
    Ok(CanonicalPath {
        segments,
        ascii_fold: declaration.ascii_fold,
        strip_slash: declaration.strip_slash,
        bare_parent: declaration.bare_parent,
    })
}

fn capture(
    declaration: &RouteDeclaration,
    segment: &str,
    m: &regex::Captures<'_>,
) -> Result<Capture, Limit> {
    use CaptureWidth::{Exactly, OneOrMore, Optional, Required, ZeroOrMore};

    let whole = m.get(0).unwrap().as_str();
    let empty_tail = if declaration.empty_tail {
        ZeroOrMore
    } else {
        OneOrMore
    };
    let (name, width, constraint) = match declaration.dialect {
        Dialect::Gin if whole.starts_with('*') => (&m[2], empty_tail, None),
        Dialect::Express4 if whole == "*" => ("", empty_tail, None),
        Dialect::Express4 => (
            &m[1],
            if m.get(3).is_some() {
                Optional
            } else {
                Required
            },
            m.get(2).map(|v| v.as_str().to_owned()),
        ),
        Dialect::Express5 if m.get(1).is_some() => (&m[1], ZeroOrMore, None),
        Dialect::Express5 if m.get(2).is_some() => (&m[2], OneOrMore, None),
        Dialect::Express5 => (
            m.get(3).or_else(|| m.get(4)).unwrap().as_str(),
            Required,
            None,
        ),
        Dialect::Hapi => {
            let width = match &m[2] {
                "" => Required,
                "?" => Optional,
                "*" => empty_tail,
                fixed => Exactly(
                    fixed[1..]
                        .parse::<usize>()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or(Limit::PathUnchecked)?,
                ),
            };
            (&m[1], width, None)
        }
        _ => (&m[1], Required, None),
    };
    if width != Required && declaration.version.is_none() {
        return Err(if width == Optional {
            Limit::OptionalSeparatorUnchecked
        } else {
            Limit::TailWidthUnchecked
        });
    }
    if !matches!(width, Required | Optional) && whole != segment {
        return Err(Limit::PathUnchecked);
    }
    Ok(Capture {
        name: name.into(),
        width,
        optional_separator: width == Optional && whole == segment,
        constraint,
    })
}
