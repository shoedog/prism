use crate::cpg::CpgNode;
use crate::navigation::types::{QueryError, SymbolRef};
use crate::navigation::NavigationSession;
use petgraph::graph::NodeIndex;

#[derive(Debug)]
pub struct ResolvedFn {
    pub idx: NodeIndex,
    pub symbol: SymbolRef,
}

pub(crate) fn fn_symbol(s: &NavigationSession, idx: NodeIndex) -> Option<SymbolRef> {
    match s.index.cpg.node(idx) {
        CpgNode::Function {
            name,
            file,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => Some(SymbolRef::Function {
            file: file.clone(),
            name: name.clone(),
            start_line: *start_line,
            end_line: *end_line,
            start_byte: *start_byte,
            end_byte: *end_byte,
            ordinal: 0,
        }),
        _ => None,
    }
}

/// Anonymous callables (`<anon@L:C>`) are reached by location only. A symbol seed spelling an
/// anonymous owner name is refused up front, with an error that says what to use instead.
pub(crate) fn reject_anonymous_symbol(name: &str) -> Result<(), QueryError> {
    if crate::anon_owner::is_anonymous_owner_name(name) {
        return Err(QueryError::SymbolNotFound {
            seed: format!(
                "symbol:{name} is an anonymous callable; use a location seed (file:line)"
            ),
        });
    }
    Ok(())
}

/// A trailing newline terminates the last line; it does not add a seedable line.
pub(crate) fn location_in_range(source: &str, line: usize) -> bool {
    line > 0 && source.lines().nth(line - 1).is_some()
}

/// The shared boundary list also includes wrappers around non-callable declarations.
/// Class wrappers are not enclosures themselves; the walk still visits their methods.
fn is_callable_enclosure_node(node: tree_sitter::Node<'_>, kinds: &[&str]) -> bool {
    match node.kind() {
        "decorated_definition" | "template_declaration" => node
            .named_children(&mut node.walk())
            .any(|child| child.kind() == "function_definition"),
        kind => kinds.contains(&kind),
    }
}

/// Recognize lexical callables even when they have no CPG Function identity (e.g. Go
/// func literals). Walk only nodes spanning the line, including whitespace between children.
/// A module evaluation region is deliberately not a lexical callable.
pub(crate) fn has_syntax_callable_enclosure(parsed: &crate::ast::ParsedFile, line: usize) -> bool {
    let kinds = parsed.language.callable_boundary_node_types();
    let mut pending = vec![parsed.tree.root_node()];
    while let Some(node) = pending.pop() {
        let (start, end) = parsed.node_line_range(&node);
        if line < start || line > end {
            continue;
        }
        if is_callable_enclosure_node(node, &kinds) {
            return true;
        }
        pending.extend(node.named_children(&mut node.walk()));
    }
    false
}

/// Classify a failed callable lookup against the indexed source, never the live file.
pub(crate) fn missing_callable_error(s: &NavigationSession, file: &str, line: usize) -> QueryError {
    if let Some(parsed) = s
        .repo
        .files
        .get(file)
        .filter(|parsed| location_in_range(&parsed.source, line))
    {
        if s.index
            .enclosing_function_including_anonymous(file, line)
            .is_some_and(|(_, name)| !crate::anon_owner::is_module_owner_name(&name))
            || has_syntax_callable_enclosure(parsed, line)
        {
            return QueryError::NoNamedEnclosingCallable {
                file: file.into(),
                line,
            };
        }
        QueryError::NoEnclosingCallable {
            file: file.into(),
            line,
        }
    } else {
        QueryError::LocationOutOfRange {
            file: file.into(),
            line,
        }
    }
}

/// Resolve a seed to exactly one function node. Precedence: location > symbol.
pub fn resolve_fn(
    s: &NavigationSession,
    symbol: Option<&str>,
    file: Option<&str>,
    location: Option<&str>,
) -> Result<ResolvedFn, QueryError> {
    if let Some(loc) = location {
        let (f, line) = loc
            .rsplit_once(':')
            .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
            .ok_or_else(|| QueryError::SymbolNotFound {
                seed: format!("loc:{loc}"),
            })?;
        let (idx, _) = s
            .index
            .enclosing_function(&f, line)
            .ok_or_else(|| missing_callable_error(s, &f, line))?;
        let symbol = fn_symbol(s, idx).ok_or(QueryError::LocationOutOfRange { file: f, line })?;
        return Ok(ResolvedFn { idx, symbol });
    }
    let name = symbol.ok_or_else(|| QueryError::SymbolNotFound {
        seed: "<empty>".into(),
    })?;
    reject_anonymous_symbol(name)?;
    let mut hits: Vec<NodeIndex> = s
        .index
        .name_index
        .iter()
        .filter(|((f, n), _)| n == name && file.map_or(true, |ff| ff == f))
        .flat_map(|(_, v)| v.iter().copied())
        .collect();
    hits.sort_by_key(|i| i.index());
    hits.dedup();
    match hits.len() {
        0 => Err(QueryError::SymbolNotFound {
            seed: format!("symbol:{name}"),
        }),
        1 => Ok(ResolvedFn {
            idx: hits[0],
            symbol: fn_symbol(s, hits[0]).ok_or(QueryError::SymbolNotFound {
                seed: format!("symbol:{name}"),
            })?,
        }),
        _ => Err(QueryError::AmbiguousSymbol {
            candidates: hits.iter().filter_map(|&i| fn_symbol(s, i)).collect(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::ParsedFile, languages::Language};

    mod callable_wrappers {
        include!("../../tests/common/callable_wrappers.rs");
    }

    #[test]
    fn syntax_wrapper_classes_are_not_callables() {
        let mut failures = Vec::new();
        for &(file, source, lines) in callable_wrappers::CLASS_CASES {
            let language = if file.ends_with(".py") {
                Language::Python
            } else {
                Language::Cpp
            };
            let parsed = ParsedFile::parse(file, source, language).unwrap();
            for &line in lines {
                if has_syntax_callable_enclosure(&parsed, line) {
                    failures.push(format!("{file}:{line}: {source}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn syntax_wrapper_functions_and_nested_methods_are_callables() {
        for &(file, source, lines, _) in callable_wrappers::CALLABLE_CONTROLS {
            let language = if file.ends_with(".py") {
                Language::Python
            } else {
                Language::Cpp
            };
            let parsed = ParsedFile::parse(file, source, language).unwrap();
            for &line in lines {
                assert!(
                    has_syntax_callable_enclosure(&parsed, line),
                    "{file}:{line}: {source}"
                );
            }
        }
    }
}
