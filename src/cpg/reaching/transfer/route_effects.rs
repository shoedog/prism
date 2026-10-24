//! Route-local Python import definitions and `del` kills.

use super::super::binding_table::{select_binding_row, DeclarationKind, Role, Ruling};
use super::super::graph::BitSet;
use super::super::{DefId, DefSite, Line};
use super::row_introduced_identifiers;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Debug)]
struct DiscoveredEffect {
    line: Line,
    path: AccessPath,
    identifier_byte: usize,
    definition: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Placement {
    Straight {
        from: usize,
        to: usize,
    },
    IfArm {
        owner: usize,
        header: usize,
        after: usize,
        arm: u8,
        has_alternative: bool,
    },
    TryBody {
        owner: usize,
        header: usize,
        after: usize,
    },
}

#[derive(Debug, Default)]
pub(in crate::cpg::reaching) struct RouteEffects {
    kills: Vec<(usize, AccessPath)>,
    synthetic_nodes: BTreeMap<usize, Line>,
}

pub(in crate::cpg::reaching) fn is_route_effect_node(language: Language, kind: &str) -> bool {
    language == Language::Python
        && matches!(
            kind,
            "delete_statement" | "import_statement" | "import_from_statement" | "aliased_import"
        )
}

fn is_route_import_node(language: Language, kind: &str) -> bool {
    language == Language::Python
        && matches!(
            kind,
            "import_statement" | "import_from_statement" | "aliased_import"
        )
}

pub(in crate::cpg::reaching) fn is_route_import_definition(
    parsed: &ParsedFile,
    node: Node<'_>,
    def: &DefSite,
) -> bool {
    def.synthetic
        && is_route_import_node(parsed.language, node.kind())
        && select_binding_row(parsed, node).is_some_and(|row| {
            row_introduced_identifiers(parsed, node, row.fields)
                .iter()
                .any(|identifier| identifier.start_byte() == def.start_byte)
        })
}

pub(in crate::cpg::reaching) fn materialize_route_effects(
    parsed: &ParsedFile,
    func: &Node<'_>,
    line_index: &BTreeMap<Line, usize>,
    successors: &mut Vec<Vec<(usize, bool)>>,
    predecessors: &mut Vec<Vec<usize>>,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
) -> RouteEffects {
    if parsed.language != Language::Python {
        return RouteEffects::default();
    }
    let discovered = discover(parsed, *func);
    let mut effects = RouteEffects::default();
    let mut unindexed = BTreeMap::<Line, Vec<DiscoveredEffect>>::new();
    for effect in discovered {
        if let Some(node) = line_index.get(&effect.line).copied() {
            materialize_effect(effect, node, defs, mapped_defs, &mut effects);
        } else if effect.definition {
            unindexed.entry(effect.line).or_default().push(effect);
        }
    }
    materialize_unindexed_imports(
        parsed,
        func,
        line_index,
        unindexed,
        successors,
        defs,
        mapped_defs,
        &mut effects,
    );
    rebuild_predecessors(successors, predecessors);
    effects
}

fn discover(parsed: &ParsedFile, func: Node<'_>) -> Vec<DiscoveredEffect> {
    fn walk(
        parsed: &ParsedFile,
        node: Node<'_>,
        root_function_id: usize,
        effects: &mut Vec<DiscoveredEffect>,
    ) {
        if node.id() != root_function_id
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
        {
            return;
        }
        if matches!(
            node.kind(),
            "delete_statement" | "import_statement" | "import_from_statement"
        ) {
            if let Some(row) = select_binding_row(parsed, node).filter(|row| {
                row.ruling == Ruling::Classified
                    && row.roles.has(Role::Binding)
                    && row.declaration == Some(DeclarationKind::Other)
            }) {
                let definition = is_route_import_node(parsed.language, node.kind());
                for identifier in row_introduced_identifiers(parsed, node, row.fields) {
                    let name = parsed.node_text(&identifier);
                    if name != "_" {
                        effects.push(DiscoveredEffect {
                            line: node.start_position().row + 1,
                            path: AccessPath::simple(name),
                            identifier_byte: identifier.start_byte(),
                            definition,
                        });
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(parsed, child, root_function_id, effects);
        }
    }

    let mut effects = Vec::new();
    walk(parsed, func, func.id(), &mut effects);
    effects
}

fn materialize_effect(
    effect: DiscoveredEffect,
    node: usize,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
    effects: &mut RouteEffects,
) {
    if effect.definition {
        let def_index = defs.len();
        defs.push(DefSite {
            id: DefId(u32::try_from(def_index).unwrap_or(u32::MAX)),
            path: effect.path,
            line: effect.line,
            start_byte: effect.identifier_byte,
            alias_derived: false,
            synthetic: true,
        });
        mapped_defs.push(Some(node));
    } else {
        effects.kills.push((node, effect.path));
    }
}

#[allow(clippy::too_many_arguments)]
fn materialize_unindexed_imports(
    parsed: &ParsedFile,
    func: &Node<'_>,
    line_index: &BTreeMap<Line, usize>,
    imports: BTreeMap<Line, Vec<DiscoveredEffect>>,
    successors: &mut Vec<Vec<(usize, bool)>>,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
    effects: &mut RouteEffects,
) {
    let mut sites = BTreeMap::<Placement, Vec<(Line, usize)>>::new();
    for (line, definitions) in imports {
        let Some(placement) = placement(parsed, func, line_index, &definitions[0]) else {
            continue;
        };
        let node = successors.len();
        successors.push(Vec::new());
        effects.synthetic_nodes.insert(node, line);
        for definition in definitions {
            materialize_effect(definition, node, defs, mapped_defs, effects);
        }
        sites.entry(placement).or_default().push((line, node));
    }

    let mut covered_arms = BTreeMap::<usize, BTreeSet<u8>>::new();
    for placement in sites.keys() {
        if let Placement::IfArm { owner, arm, .. } = placement {
            covered_arms.entry(*owner).or_default().insert(*arm);
        }
    }
    for (placement, mut chain) in sites {
        chain.sort_by_key(|(line, _)| *line);
        let nodes: Vec<usize> = chain.into_iter().map(|(_, node)| node).collect();
        match placement {
            Placement::Straight { from, to } => {
                let incomplete = successors[from]
                    .iter()
                    .filter(|(target, _)| *target == to)
                    .any(|(_, incomplete)| *incomplete);
                successors[from].retain(|(target, _)| *target != to);
                connect_chain(successors, from, to, &nodes, incomplete);
            }
            Placement::IfArm {
                owner,
                header,
                after,
                has_alternative,
                ..
            } => {
                if has_alternative && covered_arms.get(&owner).is_some_and(|arms| arms.len() == 2) {
                    successors[header].retain(|(target, _)| *target != after);
                }
                connect_chain(successors, header, after, &nodes, false);
            }
            Placement::TryBody { header, after, .. } => {
                connect_chain(successors, header, after, &nodes, false);
            }
        }
    }
}

fn placement(
    parsed: &ParsedFile,
    func: &Node<'_>,
    line_index: &BTreeMap<Line, usize>,
    effect: &DiscoveredEffect,
) -> Option<Placement> {
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(effect.identifier_byte, effect.identifier_byte + 1)?;
    while node.id() != func.id() {
        if node.kind() == "if_statement" {
            let header = line_index.get(&(node.start_position().row + 1)).copied()?;
            let after = node_after(line_index, node.end_position().row + 1)?;
            let consequence = node.child_by_field_name("consequence");
            let alternative = node.child_by_field_name("alternative");
            let arm = if consequence.is_some_and(|arm| contains(arm, effect.identifier_byte)) {
                0
            } else if alternative.is_some_and(|arm| contains(arm, effect.identifier_byte)) {
                1
            } else {
                node = node.parent()?;
                continue;
            };
            return Some(Placement::IfArm {
                owner: node.id(),
                header,
                after,
                arm,
                has_alternative: alternative.is_some(),
            });
        }
        if node.kind() == "try_statement"
            && node
                .child_by_field_name("body")
                .is_some_and(|body| contains(body, effect.identifier_byte))
        {
            return Some(Placement::TryBody {
                owner: node.id(),
                header: line_index.get(&(node.start_position().row + 1)).copied()?,
                after: node_after(line_index, node.end_position().row + 1)?,
            });
        }
        node = node.parent()?;
    }
    let from = line_index
        .range(..effect.line)
        .next_back()
        .map(|(_, node)| *node)
        .unwrap_or(0);
    let to = node_after(line_index, effect.line)?;
    Some(Placement::Straight { from, to })
}

fn contains(node: Node<'_>, byte: usize) -> bool {
    node.start_byte() <= byte && byte < node.end_byte()
}

fn node_after(line_index: &BTreeMap<Line, usize>, line: Line) -> Option<usize> {
    line_index.range((line + 1)..).next().map(|(_, node)| *node)
}

fn connect_chain(
    successors: &mut [Vec<(usize, bool)>],
    from: usize,
    to: usize,
    nodes: &[usize],
    incomplete: bool,
) {
    let Some(first) = nodes.first().copied() else {
        return;
    };
    if !successors[from].iter().any(|(target, _)| *target == first) {
        successors[from].push((first, incomplete));
    }
    for (index, node) in nodes.iter().copied().enumerate() {
        let next = nodes.get(index + 1).copied().unwrap_or(to);
        successors[node].push((next, false));
    }
}

fn rebuild_predecessors(successors: &[Vec<(usize, bool)>], predecessors: &mut Vec<Vec<usize>>) {
    *predecessors = vec![Vec::new(); successors.len()];
    for (from, edges) in successors.iter().enumerate() {
        for (to, _) in edges {
            predecessors[*to].push(from);
        }
    }
}

pub(in crate::cpg::reaching) fn merge_synthetic_nodes(
    effects: &RouteEffects,
    synthetic_nodes: &mut BTreeMap<usize, Line>,
) {
    synthetic_nodes.extend(&effects.synthetic_nodes);
}

pub(in crate::cpg::reaching) fn apply_explicit_kills(
    effects: &RouteEffects,
    defs: &[DefSite],
    kill: &mut [BitSet],
    flat_kill: &mut [BitSet],
) {
    for (node, path) in &effects.kills {
        for (index, def) in defs.iter().enumerate() {
            if !def.alias_derived && def.path == *path {
                kill[*node].insert(index);
                flat_kill[*node].insert(index);
            }
        }
    }
}
