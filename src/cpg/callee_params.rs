//! Demand-only parameter targets shared across callers within one assembly.
use super::*;
use crate::languages::js_inferred_name::{js_inferred_name_source, JsInferredNameSource};
use std::sync::{Arc, Mutex, OnceLock};
use tree_sitter::Node;

#[cfg(test)]
#[path = "callee_param_cache_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BareSlot {
    pub declaration: (usize, usize),
    pub formal: (usize, usize),
    pub inferred_from_enclosing_call: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ParamDef {
    pub node: NodeIndex,
    pub bare: Option<BareSlot>,
}

/// Coordinates in the immutable input graph, never a retained graph index.
#[derive(Clone)]
pub(super) struct ParamTarget {
    key: LegacyVarKey,
    bytes: (usize, usize),
    pub bare: Option<BareSlot>,
}

impl ParamTarget {
    fn from_node(node: &CpgNode, bare: Option<BareSlot>) -> Self {
        let CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            path,
            access,
            start_byte,
            end_byte,
        } = node
        else {
            unreachable!("parameter index must address a variable")
        };
        Self {
            key: (
                file.clone(),
                function.clone(),
                *function_start_line,
                *line,
                path.clone(),
                *access,
            ),
            bytes: (*start_byte, *end_byte),
            bare,
        }
    }

    pub(super) fn resolve(
        &self,
        index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Option<NodeIndex> {
        let idx = index.get(graph, &self.key)?;
        matches!(graph.node_weight(idx), Some(CpgNode::Variable { start_byte, end_byte, .. })
            if (*start_byte, *end_byte) == self.bytes)
        .then_some(idx)
    }
}

type Facts = Option<Vec<Option<ParamTarget>>>;
pub(super) type FactCell = Arc<OnceLock<Facts>>;
type RestCell = Arc<OnceLock<Option<(usize, ParamTarget)>>>;

/// The coordinate of an admitted binding, independent of graph numbering.
#[derive(Clone)]
pub(super) struct PatternTarget(ParamTarget);

impl PatternTarget {
    fn from_node(node: &CpgNode) -> Self {
        Self(ParamTarget::from_node(node, None))
    }

    pub(super) fn resolve(
        &self,
        index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Option<NodeIndex> {
        resolve_pattern_target(index, graph, &self.0.key, self.0.bytes)
    }
}

fn resolve_pattern_target(
    index: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
    key: &LegacyVarKey,
    bytes: (usize, usize),
) -> Option<NodeIndex> {
    let [node] = index.get(key)?.get(&bytes)?.as_slice() else {
        return None;
    };
    let CpgNode::Variable {
        file,
        function,
        function_start_line,
        line,
        path,
        access,
        start_byte,
        end_byte,
    } = graph.node_weight(*node)?
    else {
        return None;
    };
    ((file, function, function_start_line, line, path, access)
        == (&key.0, &key.1, &key.2, &key.3, &key.4, &key.5)
        && (*start_byte, *end_byte) == bytes)
        .then_some(*node)
}

pub(super) struct PatternPosition<T> {
    pub(super) syntax: crate::ast::js_ts::PatternParam,
    /// Aligned to syntax order, including absent Defs.
    pub(super) targets: Vec<Option<T>>,
}

pub(super) struct PatternFacts<T> {
    pub(super) callable: (usize, usize),
    pub(super) positions: Vec<PatternPosition<T>>,
}

pub(super) type PatternCell = Arc<OnceLock<Option<PatternFacts<PatternTarget>>>>;

pub(super) fn compute_pattern_formal_nodes(
    parsed: &ParsedFile,
    callee: &FunctionId,
    exact: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Option<PatternFacts<NodeIndex>> {
    let anonymous = crate::anon_owner::is_anonymous_owner_name(&callee.name);
    let mut functions = crate::anon_owner::candidate_nodes(parsed, anonymous)
        .into_iter()
        .filter(|node| {
            parsed.node_line_range(node) == (callee.start_line, callee.end_line)
                && node.child_by_field_name("body").is_some()
                && crate::anon_owner::identity(parsed, node, &callee.file).as_ref() == Some(callee)
        });
    let callable = functions.next()?;
    if functions.next().is_some() || callable.has_error() {
        return None;
    }
    let positions = parsed
        .js_ts_pattern_params(&callable)
        .into_iter()
        .map(|syntax| {
            let targets = syntax
                .elements
                .iter()
                .map(|element| {
                    if !element.admitted {
                        return None;
                    }
                    let key = (
                        callee.file.clone(),
                        callee.name.clone(),
                        callee.start_line,
                        callee.start_line,
                        AccessPath::simple(&element.name),
                        VarAccess::Def,
                    );
                    resolve_pattern_target(
                        exact,
                        graph,
                        &key,
                        (element.bytes.start, element.bytes.end),
                    )
                })
                .collect();
            PatternPosition { syntax, targets }
        })
        .collect();
    Some(PatternFacts {
        callable: (callable.start_byte(), callable.end_byte()),
        positions,
    })
}

#[cfg(test)]
impl ParamCache<'_> {
    fn pattern_positions(
        &self,
        callee: &FunctionId,
        parsed: &ParsedFile,
        exact: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Vec<(usize, bool, Vec<PatternTarget>)> {
        let cell = self.patterns(callee, parsed, exact, graph);
        cell.get()
            .and_then(Option::as_ref)
            .into_iter()
            .flat_map(|facts| &facts.positions)
            .map(|position| {
                (
                    position.syntax.position,
                    position.syntax.alias_skipped,
                    position.targets.iter().flatten().cloned().collect(),
                )
            })
            .collect()
    }
}

pub(super) struct ParamCache<'a> {
    pub(super) files: &'a BTreeMap<String, ParsedFile>,
    entries: Mutex<BTreeMap<FunctionId, FactCell>>,
    rest_entries: Mutex<BTreeMap<FunctionId, RestCell>>,
    pattern_entries: Mutex<BTreeMap<FunctionId, PatternCell>>,
}

impl<'a> ParamCache<'a> {
    pub(super) fn new(files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self {
            files,
            entries: Mutex::new(BTreeMap::new()),
            rest_entries: Mutex::new(BTreeMap::new()),
            pattern_entries: Mutex::new(BTreeMap::new()),
        }
    }

    pub(super) fn get(
        &self,
        callee: &FunctionId,
        parsed: &ParsedFile,
        index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> FactCell {
        let cell = self
            .entries
            .lock()
            .expect("parameter cache lock")
            .entry(callee.clone())
            .or_default()
            .clone();
        // Do not hold the map lock during discovery. The per-owner cell also
        // memoizes refusals; concurrent first demand cannot duplicate the walk.
        cell.get_or_init(|| {
            compute_param_def_nodes(parsed, callee, index, graph).map(|nodes| {
                nodes
                    .into_iter()
                    .map(|node| node.map(|def| ParamTarget::from_node(&graph[def.node], def.bare)))
                    .collect()
            })
        });
        cell
    }

    pub(super) fn patterns(
        &self,
        callee: &FunctionId,
        parsed: &ParsedFile,
        exact: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> PatternCell {
        let cell = self
            .pattern_entries
            .lock()
            .expect("pattern parameter cache lock")
            .entry(callee.clone())
            .or_default()
            .clone();
        cell.get_or_init(|| {
            let facts = compute_pattern_formal_nodes(parsed, callee, exact, graph)?;
            Some(PatternFacts {
                callable: facts.callable,
                positions: facts
                    .positions
                    .into_iter()
                    .map(|at| PatternPosition {
                        syntax: at.syntax,
                        targets: at
                            .targets
                            .into_iter()
                            .map(|target| target.map(|node| PatternTarget::from_node(&graph[node])))
                            .collect(),
                    })
                    .collect(),
            })
        });
        cell
    }

    pub(super) fn rest(
        &self,
        callee: &FunctionId,
        parsed: &ParsedFile,
        index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Option<(usize, ParamTarget)> {
        let cell = self
            .rest_entries
            .lock()
            .expect("rest parameter cache lock")
            .entry(callee.clone())
            .or_default()
            .clone();
        cell.get_or_init(|| {
            rest_params::compute_rest_def_node(parsed, callee, index, graph)
                .map(|(slot, node)| (slot, ParamTarget::from_node(&graph[node], None)))
        })
        .clone()
    }
}

/// Per-build positional targets. Holes are unsupported slots, not removed args.
/// The graph and source inputs are immutable for both Step-5b traversals.
pub(super) fn compute_param_def_nodes(
    parsed: &ParsedFile,
    callee: &FunctionId,
    var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Option<Vec<Option<ParamDef>>> {
    #[cfg(test)]
    super::callee_cost_tests::record("parameters", callee);
    if !matches!(
        parsed.language,
        crate::languages::Language::JavaScript
            | crate::languages::Language::TypeScript
            | crate::languages::Language::Tsx
    ) {
        // Preserve other languages' existing normalization and lookup contract.
        return Some(
            compute_param_names(parsed, callee)?
                .into_iter()
                .map(|name| {
                    let path = AccessPath::simple(name);
                    (callee.start_line..=callee.end_line)
                        .find_map(|line| {
                            var_index.get_ref(
                                graph,
                                VarKeyRef {
                                    file: &callee.file,
                                    function: &callee.name,
                                    function_start_line: callee.start_line,
                                    line,
                                    path: &path,
                                    access: VarAccess::Def,
                                },
                            )
                        })
                        .map(|node| ParamDef { node, bare: None })
                })
                .collect(),
        );
    }

    let function = locate_function(parsed, callee)?;
    let slots = parsed.function_parameter_slot_occurrences(&function)?;
    Some(lookup_formal_defs(
        parsed,
        callee,
        &function,
        slots.into_iter().map(Some),
        var_index,
        graph,
    ))
}

fn locate_function<'a>(parsed: &'a ParsedFile, callee: &FunctionId) -> Option<Node<'a>> {
    if crate::anon_owner::is_anonymous_owner_name(&callee.name) {
        // No fixed slots are opened by the newly reachable anonymous lookup.
        return crate::anon_owner::node_for_id(parsed, callee)
            .filter(|node| parsed.js_ts_bare_arrow_parameter(node).is_some());
    }
    let mut functions = parsed.all_functions().into_iter().filter(|node| {
        parsed.node_line_range(node) == (callee.start_line, callee.end_line)
            && parsed
                .language
                .function_name(node)
                .is_some_and(|name| parsed.node_text(&name) == callee.name)
    });
    let function = functions.next()?;
    functions.next().is_none().then_some(function)
}

/// Fixed runtime roles of a syntax-proved JS-family implementation.
/// Destructured and unsupported formals occupy their slots without a target.
pub(super) fn formal_def_nodes(
    parsed: &ParsedFile,
    function: &Node<'_>,
    owner: &FunctionId,
    var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
    admit_bare: bool,
) -> Option<Vec<Option<NodeIndex>>> {
    let parameters = parsed.parameter_binding_region(function)?;
    if !crate::parameter_slots::js_ts_parameter_list_is_clean(parsed, parameters) {
        return None;
    }
    let slots = if parameters.kind() == "identifier" {
        vec![admit_bare.then_some(parameters)]
    } else {
        rest_params::runtime_formals(parameters)
            .into_iter()
            .map(|pattern| {
                let binding = if pattern.kind() == "assignment_pattern" {
                    pattern.child_by_field_name("left")?
                } else {
                    pattern
                };
                (binding.kind() == "identifier").then_some(binding)
            })
            .collect()
    };
    let slots = slots.into_iter().map(|node| {
        node.map(|node| {
            (
                parsed.node_text(&node).to_owned(),
                node.start_byte(),
                node.end_byte(),
            )
        })
    });
    Some(
        lookup_formal_defs(parsed, owner, function, slots, var_index, graph)
            .into_iter()
            .map(|def| def.map(|def| def.node))
            .collect(),
    )
}

fn lookup_formal_defs(
    parsed: &ParsedFile,
    callee: &FunctionId,
    function: &Node<'_>,
    slots: impl Iterator<Item = Option<crate::ast::ParameterOccurrence>>,
    var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Vec<Option<ParamDef>> {
    // Unsupported occurrences retain their positional holes. The entry Def
    // must match the runtime slot, and a bare slot must match the identifier.
    let supported: BTreeSet<_> = parsed
        .function_parameter_occurrences(function)
        .into_iter()
        .collect();
    let bare = parsed
        .js_ts_bare_arrow_parameter(function)
        .map(|identifier| {
            let formal = (identifier.start_byte(), identifier.end_byte());
            let occurrence = (
                parsed.node_text(&identifier).to_string(),
                formal.0,
                formal.1,
            );
            let inferred_from_enclosing_call = js_inferred_name_source(*function)
                .is_some_and(|inferred| inferred.source == JsInferredNameSource::WrappedDeclarator);
            (
                occurrence,
                BareSlot {
                    declaration: (function.start_byte(), function.end_byte()),
                    formal,
                    inferred_from_enclosing_call,
                },
            )
        });
    slots
        .map(|slot| {
            let slot = slot?;
            if !supported.contains(&slot)
                || bare
                    .as_ref()
                    .is_some_and(|(occurrence, _)| occurrence != &slot)
            {
                return None;
            }
            let bare = bare.as_ref().map(|(_, marker)| *marker);
            let path = AccessPath::simple(&slot.0);
            let node = var_index.get_ref(
                graph,
                VarKeyRef {
                    file: &callee.file,
                    function: &callee.name,
                    function_start_line: callee.start_line,
                    line: callee.start_line,
                    path: &path,
                    access: VarAccess::Def,
                },
            )?;
            matches!(graph.node_weight(node), Some(CpgNode::Variable {
                file, function, function_start_line, line, path: actual_path,
                access: VarAccess::Def, start_byte, end_byte,
            }) if file == &callee.file && function == &callee.name
                && *function_start_line == callee.start_line
                && *line == callee.start_line && actual_path == &path
                && (*start_byte, *end_byte) == (slot.1, slot.2)
            )
            .then_some(ParamDef { node, bare })
        })
        .collect()
}
