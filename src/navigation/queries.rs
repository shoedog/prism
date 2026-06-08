use crate::cpg::CpgNode;
use crate::navigation::types::*;
use crate::navigation::NavigationSession;

/// Exact CPG nodes at `file:line` (Function/Variable only, spec §8 R3-M3) plus the
/// innermost enclosing function as `EnclosingFunction` evidence.
pub fn nodes_at(s: &NavigationSession, file: &str, line: usize) -> Evidence {
    let query = format!("nodes-at:{file}:{line}");
    if !s.repo.files.contains_key(file) {
        let message = s
            .repo
            .skipped
            .iter()
            .find(|skipped| skipped.path == file)
            .map(|skipped| format!("file excluded: {:?}: {file}", skipped.reason))
            .unwrap_or_else(|| format!("file not in nav index: {file}"));
        return Evidence {
            query,
            items: vec![],
            truncated: false,
            warnings: vec![Warning {
                kind: WarningKind::SkippedPath,
                message,
                location: Some(Location {
                    file: file.into(),
                    start_line: line,
                    end_line: line,
                }),
            }],
        };
    }
    let mut items = Vec::new();
    for idx in s.index.cpg.nodes_at(file, line) {
        match s.index.cpg.node(idx) {
            CpgNode::Function {
                name,
                file: f,
                start_line,
                end_line,
            } => items.push(item_fn(f, name, *start_line, *end_line)),
            CpgNode::Variable {
                path,
                file: f,
                function,
                line: l,
                access,
            } => items.push(EvidenceItem {
                symbol: Some(SymbolRef::Variable {
                    file: f.clone(),
                    function: function.clone(),
                    line: *l,
                    path: format!("{path:?}"),
                    access: format!("{access:?}"),
                    ordinal: 0,
                }),
                location: Location {
                    file: f.clone(),
                    start_line: *l,
                    end_line: *l,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![],
                snippet: None,
            }),
            CpgNode::Statement { .. } => {} // statements not first-class in v1 (spec §8 R3-M3)
        }
    }
    // Enclosing function (innermost), as evidence on the line.
    if let Some((eidx, _)) = s.index.enclosing_function(file, line) {
        if let CpgNode::Function {
            name,
            file: f,
            start_line,
            end_line,
        } = s.index.cpg.node(eidx)
        {
            let func = SymbolRef::Function {
                file: f.clone(),
                name: name.clone(),
                start_line: *start_line,
                end_line: *end_line,
                ordinal: 0,
            };
            items.push(EvidenceItem {
                symbol: Some(func.clone()),
                location: Location {
                    file: f.clone(),
                    start_line: line,
                    end_line: line,
                },
                score: 1.0,
                source: Source::PrismCpg,
                fallback: false,
                why: vec![Reason::EnclosingFunction { function: func }],
                snippet: None,
            });
        }
    }
    Evidence {
        query,
        items,
        truncated: false,
        warnings: vec![],
    }
}

fn item_fn(file: &str, name: &str, start_line: usize, end_line: usize) -> EvidenceItem {
    let sym = SymbolRef::Function {
        file: file.into(),
        name: name.into(),
        start_line,
        end_line,
        ordinal: 0,
    };
    EvidenceItem {
        symbol: Some(sym),
        location: Location {
            file: file.into(),
            start_line,
            end_line,
        },
        score: 1.0,
        source: Source::PrismCpg,
        fallback: false,
        why: vec![],
        snippet: None,
    }
}
