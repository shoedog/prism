//! Demand-only parameter targets shared across callers within one assembly.
use super::*;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
#[path = "callee_param_cache_tests.rs"]
mod tests;

/// Coordinates in the immutable input graph, never a retained graph index.
#[derive(Clone)]
pub(super) struct ParamTarget {
    key: LegacyVarKey,
    bytes: (usize, usize),
}

impl ParamTarget {
    pub(super) fn resolve(
        &self,
        index: &BTreeMap<LegacyVarKey, NodeIndex>,
        graph: &DiGraph<CpgNode, CpgEdge>,
    ) -> Option<NodeIndex> {
        let idx = *index.get(&self.key)?;
        matches!(graph.node_weight(idx), Some(CpgNode::Variable { start_byte, end_byte, .. })
            if (*start_byte, *end_byte) == self.bytes)
        .then_some(idx)
    }
}

type Facts = Option<Vec<Option<ParamTarget>>>;
pub(super) type FactCell = Arc<OnceLock<Facts>>;

pub(super) struct ParamCache<'a> {
    pub(super) files: &'a BTreeMap<String, ParsedFile>,
    entries: Mutex<BTreeMap<FunctionId, FactCell>>,
}

impl<'a> ParamCache<'a> {
    pub(super) fn new(files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self {
            files,
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    pub(super) fn get(
        &self,
        callee: &FunctionId,
        parsed: &ParsedFile,
        index: &BTreeMap<LegacyVarKey, NodeIndex>,
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
                    .map(|node| {
                        node.map(|idx| {
                            let CpgNode::Variable {
                                file,
                                function,
                                function_start_line,
                                line,
                                path,
                                access,
                                start_byte,
                                end_byte,
                            } = &graph[idx]
                            else {
                                unreachable!("parameter index must address a variable")
                            };
                            ParamTarget {
                                key: (
                                    file.clone(),
                                    function.clone(),
                                    *function_start_line,
                                    *line,
                                    path.clone(),
                                    *access,
                                ),
                                bytes: (*start_byte, *end_byte),
                            }
                        })
                    })
                    .collect()
            })
        });
        cell
    }
}

/// Per-build positional targets. Holes are unsupported slots, not removed args.
/// The graph and source inputs are immutable for both Step-5b traversals.
pub(super) fn compute_param_def_nodes(
    parsed: &ParsedFile,
    callee: &FunctionId,
    var_index: &BTreeMap<(String, String, usize, usize, AccessPath, VarAccess), NodeIndex>,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Option<Vec<Option<NodeIndex>>> {
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
                    (callee.start_line..=callee.end_line).find_map(|line| {
                        var_index
                            .get(&(
                                callee.file.clone(),
                                callee.name.clone(),
                                callee.start_line,
                                line,
                                path.clone(),
                                VarAccess::Def,
                            ))
                            .copied()
                    })
                })
                .collect(),
        );
    }

    let mut functions = parsed.all_functions().into_iter().filter(|node| {
        parsed.node_line_range(node) == (callee.start_line, callee.end_line)
            && parsed
                .language
                .function_name(node)
                .is_some_and(|name| parsed.node_text(&name) == callee.name)
    });
    let function = functions.next()?;
    if functions.next().is_some() {
        return None;
    }
    // Slots and occurrences have distinct contracts. Defaults and refused
    // optional forms may occupy slots without a supported Def. Never compress
    // those holes, nor fall back to FunctionInfo names or body definitions.
    let slots = parsed.function_parameter_slot_occurrences(&function)?;
    // js-param-defs PR-A (SPEC D12): the bare arrow formal (`x => …`) has a
    // Def but stays a Step-5b hole. The call ladder admits name-inferred
    // callables (`{ write: s => … }`, `o.f = x => …`) as `free_single` Exact
    // targets of unrelated bare calls, so filling this slot minted Exact
    // argument edges the checker refutes (MEASUREMENTS-prA §3).
    let bare_formal = parsed
        .js_ts_bare_arrow_parameter(&function)
        .map(|param| (param.start_byte(), param.end_byte()));
    let supported: BTreeSet<_> = parsed
        .function_parameter_occurrences(&function)
        .into_iter()
        .filter(|occurrence| Some((occurrence.1, occurrence.2)) != bare_formal)
        .collect();
    Some(
        slots
            .into_iter()
            .map(|slot| {
                if !supported.contains(&slot) {
                    return None;
                }
                let path = AccessPath::simple(&slot.0);
                let idx = *var_index.get(&(
                    callee.file.clone(),
                    callee.name.clone(),
                    callee.start_line,
                    callee.start_line,
                    path.clone(),
                    VarAccess::Def,
                ))?;
                matches!(graph.node_weight(idx), Some(CpgNode::Variable {
                    file, function, function_start_line, line, path: actual_path,
                    access: VarAccess::Def, start_byte, end_byte,
                }) if file == &callee.file && function == &callee.name
                    && *function_start_line == callee.start_line
                    && *line == callee.start_line && actual_path == &path
                    && (*start_byte, *end_byte) == (slot.1, slot.2)
                )
                .then_some(idx)
            })
            .collect(),
    )
}
