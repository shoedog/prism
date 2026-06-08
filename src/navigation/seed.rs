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
        } => Some(SymbolRef::Function {
            file: file.clone(),
            name: name.clone(),
            start_line: *start_line,
            end_line: *end_line,
            ordinal: 0,
        }),
        _ => None,
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
        let (idx, _) =
            s.index
                .enclosing_function(&f, line)
                .ok_or(QueryError::LocationOutOfRange {
                    file: f.clone(),
                    line,
                })?;
        let symbol = fn_symbol(s, idx).ok_or(QueryError::LocationOutOfRange { file: f, line })?;
        return Ok(ResolvedFn { idx, symbol });
    }
    let name = symbol.ok_or_else(|| QueryError::SymbolNotFound {
        seed: "<empty>".into(),
    })?;
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
