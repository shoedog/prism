//! Seed resolution type definitions for the Tier-2 reasoning layer.

use petgraph::graph::NodeIndex;
use tree_sitter::Node;

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{CpgNode, StmtKind, VarAccess};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::navigation::types::{
    Location, QueryError, ReasoningWarning, SymbolRef, Warning, WarningKind,
};
use crate::navigation::NavigationSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSpec {
    Loc { file: String, line: usize },
    Symbol { name: String, file: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ResolvedSeed {
    pub locations: Vec<VarLocation>,
    pub nodes: Vec<NodeIndex>,
    pub symbol: Option<SymbolRef>,
    pub origin: SeedSpec,
}

#[derive(Debug, Clone, Default)]
pub struct SeedSet {
    pub seeds: Vec<ResolvedSeed>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedRole {
    Source,
    Sink,
}

pub fn resolve(
    session: &NavigationSession,
    specs: &[SeedSpec],
    role: SeedRole,
) -> Result<SeedSet, QueryError> {
    if specs.is_empty() {
        return Err(QueryError::SymbolNotFound {
            seed: match role {
                SeedRole::Source => "sources".into(),
                SeedRole::Sink => "sinks".into(),
            },
        });
    }

    let mut out = SeedSet::default();
    let mut first_error = None;
    for spec in specs {
        match resolve_one(session, spec, role) {
            Ok(seed) => out.seeds.push(seed),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error.clone());
                }
                if role == SeedRole::Sink {
                    return Err(error);
                }
                out.warnings.push(seed_unresolved_warning(spec, &error));
            }
        }
    }

    if out.seeds.is_empty() {
        return Err(first_error.unwrap_or_else(|| QueryError::SymbolNotFound {
            seed: "sources".into(),
        }));
    }
    Ok(out)
}

fn resolve_one(
    session: &NavigationSession,
    spec: &SeedSpec,
    role: SeedRole,
) -> Result<ResolvedSeed, QueryError> {
    match spec {
        SeedSpec::Loc { file, line } => resolve_loc(session, file, *line, role, spec.clone()),
        SeedSpec::Symbol { name, file } => {
            resolve_symbol(session, name, file.as_deref(), spec.clone())
        }
    }
}

fn resolve_loc(
    session: &NavigationSession,
    file: &str,
    line: usize,
    role: SeedRole,
    origin: SeedSpec,
) -> Result<ResolvedSeed, QueryError> {
    let Some(parsed) = session.repo.files.get(file) else {
        return Err(QueryError::UnsupportedFile { file: file.into() });
    };
    if line == 0 || line > line_count(&parsed.source) {
        return Err(QueryError::LocationOutOfRange {
            file: file.into(),
            line,
        });
    }

    let mut nodes = Vec::new();
    let mut locations = Vec::new();
    for node in loc_seed_nodes(session, file, line, role) {
        if let Some(loc) = session.index.cpg.to_var_location(node) {
            nodes.push(node);
            locations.push(loc);
        }
    }
    if nodes.is_empty() {
        return Err(QueryError::SymbolNotFound {
            seed: seed_label(&origin),
        });
    }

    Ok(ResolvedSeed {
        symbol: locations.first().map(symbol_for_var_location),
        locations,
        nodes,
        origin,
    })
}

fn resolve_symbol(
    session: &NavigationSession,
    name: &str,
    file: Option<&str>,
    origin: SeedSpec,
) -> Result<ResolvedSeed, QueryError> {
    let mut candidates = Vec::new();
    match file {
        Some(file) => {
            let Some(parsed) = session.repo.files.get(file) else {
                return Err(QueryError::UnsupportedFile { file: file.into() });
            };
            candidates.extend(
                functions_named(parsed, name)
                    .into_iter()
                    .map(|node| (file, parsed, node)),
            );
        }
        None => {
            for (file, parsed) in &session.repo.files {
                candidates.extend(
                    functions_named(parsed, name)
                        .into_iter()
                        .map(|node| (file.as_str(), parsed, node)),
                );
            }
        }
    }

    if candidates.is_empty() {
        return Err(QueryError::SymbolNotFound {
            seed: seed_label(&origin),
        });
    }
    if candidates.len() > 1 {
        return Err(QueryError::AmbiguousSymbol {
            candidates: candidates
                .iter()
                .map(|(file, parsed, node)| symbol_for_function(parsed, file, *node))
                .collect(),
        });
    }

    let (file, parsed, function) = candidates[0];
    let function_name = parsed
        .language
        .function_name(&function)
        .map(|node| parsed.node_text(&node).to_string())
        .unwrap_or_else(|| name.to_string());
    let function_start_line = function.start_position().row + 1;

    let mut nodes = Vec::new();
    let mut locations = Vec::new();
    for (param, start_byte, end_byte) in parsed.function_parameter_occurrences(&function) {
        if !parsed.has_bare_references(&function, &param) {
            continue;
        }
        let output_loc = VarLocation {
            file: file.to_string(),
            function: function_name.clone(),
            function_start_line,
            line: parsed.line_for_byte(start_byte),
            path: AccessPath::simple(&param),
            start_byte,
            end_byte,
            kind: VarAccessKind::Def,
        };
        let lookup_loc = VarLocation {
            line: function_start_line,
            ..output_loc.clone()
        };
        if let Some(node) = session.index.cpg.var_node_for_location(&lookup_loc) {
            nodes.push(node);
            locations.push(output_loc);
        }
    }
    if nodes.is_empty() {
        return Err(QueryError::SymbolNotFound {
            seed: seed_label(&origin),
        });
    }

    Ok(ResolvedSeed {
        locations,
        nodes,
        symbol: Some(symbol_for_function(parsed, file, function)),
        origin,
    })
}

fn functions_named<'a>(parsed: &'a ParsedFile, name: &str) -> Vec<Node<'a>> {
    parsed
        .all_functions()
        .into_iter()
        .filter(|node| {
            parsed
                .language
                .function_name(node)
                .is_some_and(|name_node| parsed.node_text(&name_node) == name)
        })
        .collect()
}

fn symbol_for_function(parsed: &ParsedFile, file: &str, function: Node<'_>) -> SymbolRef {
    let name = parsed
        .language
        .function_name(&function)
        .map(|node| parsed.node_text(&node).to_string())
        .unwrap_or_else(|| "<anonymous>".into());
    let (start_line, end_line) = parsed.node_line_range(&function);
    SymbolRef::Function {
        file: file.into(),
        name,
        start_line,
        end_line,
        start_byte: function.start_byte(),
        end_byte: function.end_byte(),
        ordinal: 0,
    }
}

fn symbol_for_var_location(loc: &VarLocation) -> SymbolRef {
    SymbolRef::Variable {
        file: loc.file.clone(),
        function: loc.function.clone(),
        line: loc.line,
        path: loc.path.to_string(),
        access: match loc.kind {
            VarAccessKind::Def => "def",
            VarAccessKind::Use => "use",
        }
        .into(),
        start_byte: loc.start_byte,
        end_byte: loc.end_byte,
        ordinal: 0,
    }
}

fn loc_seed_nodes(
    session: &NavigationSession,
    file: &str,
    line: usize,
    role: SeedRole,
) -> Vec<NodeIndex> {
    let cpg = &session.index.cpg;
    let nodes = cpg.nodes_at(file, line);
    let mut callees: Vec<String> = nodes
        .iter()
        .filter_map(|&node| match cpg.node(node) {
            CpgNode::Statement {
                kind: StmtKind::Call { callee },
                ..
            } => Some(callee.clone()),
            _ => None,
        })
        .collect();
    let parsed = session.repo.files.get(file);
    let mut same_name_argument_bases = std::collections::BTreeSet::new();
    for sites in cpg.call_graph.calls.values() {
        for site in sites {
            if site.caller.file == file && site.line == line {
                callees.push(site.callee_name.clone());
                if let Some(parsed) = parsed {
                    for arg_text in
                        parsed.call_argument_texts_at(site.start_byte, &site.callee_name)
                    {
                        let path = AccessPath::from_expr(&arg_text);
                        if path.base == site.callee_name {
                            same_name_argument_bases.insert(path.base);
                        }
                    }
                }
            }
        }
    }
    callees.sort();
    callees.dedup();
    let variables: Vec<NodeIndex> = nodes
        .into_iter()
        .filter(|&node| match cpg.node(node) {
            CpgNode::Variable { access, path, .. } => {
                !(*access == VarAccess::Use
                    && callees.iter().any(|callee| callee == &path.base)
                    && !same_name_argument_bases.contains(&path.base))
            }
            _ => false,
        })
        .collect();

    if role == SeedRole::Source {
        return variables;
    }

    let uses: Vec<NodeIndex> = variables
        .iter()
        .copied()
        .filter(|&node| {
            matches!(
                cpg.node(node),
                CpgNode::Variable {
                    access: VarAccess::Use,
                    ..
                }
            )
        })
        .collect();
    if uses.is_empty() {
        return variables;
    }
    uses
}

fn seed_unresolved_warning(seed: &SeedSpec, error: &QueryError) -> Warning {
    Warning {
        kind: WarningKind::Reasoning(ReasoningWarning::SeedUnresolved {
            seed: seed_label(seed),
        }),
        message: format!("seed {} did not resolve: {error:?}", seed_label(seed)),
        location: seed_location(seed),
    }
}

fn seed_location(seed: &SeedSpec) -> Option<Location> {
    match seed {
        SeedSpec::Loc { file, line } => Some(Location {
            file: file.clone(),
            start_line: *line,
            end_line: *line,
            start_byte: 0,
            end_byte: 0,
        }),
        SeedSpec::Symbol { .. } => None,
    }
}

pub fn seed_label(seed: &SeedSpec) -> String {
    match seed {
        SeedSpec::Loc { file, line } => format!("{file}:{line}"),
        SeedSpec::Symbol { name, file } => match file {
            Some(file) => format!("{file}::{name}"),
            None => name.clone(),
        },
    }
}

fn line_count(source: &str) -> usize {
    source.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_types_construct() {
        let spec = SeedSpec::Loc {
            file: "a.py".into(),
            line: 3,
        };
        assert!(matches!(spec, SeedSpec::Loc { line: 3, .. }));
        assert!(SeedSet::default().seeds.is_empty());
    }
}
