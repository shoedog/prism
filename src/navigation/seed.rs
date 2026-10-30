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

/// Classify a failed callable lookup against the indexed source, never the live file.
pub(crate) fn missing_callable_error(s: &NavigationSession, file: &str, line: usize) -> QueryError {
    if s.repo
        .files
        .get(file)
        .is_some_and(|parsed| location_in_range(&parsed.source, line))
    {
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
