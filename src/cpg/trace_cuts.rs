use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::ast::ParsedFile;
use crate::call_graph::{CallSiteOrigin, FunctionId};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;
use crate::resolution::ResolutionConfidence;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use super::{
    BoundaryKind, CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, LossReason, Relation,
    ReturnFlowMode, SameLineOrderView, VarAccess,
};

/// A def-use crossing recorded without traversing the refused hop.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundaryEdge {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub kind: BoundaryKind,
    pub reason: LossReason,
    pub relation: Relation,
    /// Resolved site evidence collected when the crossing ledger is enabled.
    pub site: Option<CrossingSite>,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossingSite {
    pub start_byte: usize,
    pub end_byte: usize,
    pub slot: Option<usize>,
}

pub(super) struct SiteEvidence {
    pub location: CrossingSite,
    grades: Vec<ResolutionConfidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceCut {
    pub root: NodeIndex,
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub reason: LossReason,
    pub relation: Relation,
    pub site: Option<CrossingSite>,
    pub depth: usize,
}

#[derive(Clone, Copy)]
pub(super) struct CrossingLedger<'a> {
    walk: &'static str,
    id: usize,
    /// Navigation prints the crossing's site, slot and grades; review mode prints only the reason.
    fields: bool,
    pub site_view: Option<&'a dyn SameLineOrderView>,
}

impl<'a> CrossingLedger<'a> {
    pub(super) fn start(
        walk: &'static str,
        site_view: Option<&'a dyn SameLineOrderView>,
        fields: bool,
    ) -> Option<Self> {
        static NEXT_WALK: AtomicUsize = AtomicUsize::new(0);
        std::env::var_os("PRISM_TAINT_CROSSING_TRACE")
            .is_some_and(|value| value == "1")
            .then(|| Self {
                walk,
                id: NEXT_WALK.fetch_add(1, Ordering::Relaxed),
                fields,
                site_view,
            })
    }
}

pub(super) struct LegacyCrossingLedger<'a> {
    ledger: CrossingLedger<'a>,
    root: NodeIndex,
    depths: BTreeMap<NodeIndex, usize>,
}

impl<'a> LegacyCrossingLedger<'a> {
    pub(super) fn new(ledger: CrossingLedger<'a>, root: NodeIndex) -> Self {
        Self {
            ledger,
            root,
            depths: BTreeMap::from([(root, 0)]),
        }
    }

    pub(super) fn reached(
        &mut self,
        cpg: &CodePropertyGraph,
        node: NodeIndex,
        (parent, label): (NodeIndex, Option<FlowConfidence>),
    ) {
        let crossed = label.is_some_and(|label| {
            cpg.crossing_kind(parent, node, Relation::DataFlow, Some(label))
                .is_some()
        });
        self.depths
            .insert(node, self.depths[&parent] + usize::from(crossed));
    }

    pub(super) fn attempt(
        &self,
        cpg: &CodePropertyGraph,
        from: NodeIndex,
        to: NodeIndex,
        label: FlowConfidence,
        admitted: bool,
    ) {
        if cpg
            .crossing_kind(from, to, Relation::DataFlow, Some(label))
            .is_some()
        {
            let relation = Relation::CallDescent;
            let site = cpg.ledger_site(from, to, relation, self.ledger.site_view);
            cpg.log_trace_attempt(
                self.ledger,
                self.root,
                from,
                to,
                relation,
                site.as_ref(),
                Some(label),
                self.depths[&from],
                (!admitted).then_some(LossReason::PolicyCeiling),
            );
        }
    }
}

pub(super) fn byte_span(node: &CpgNode) -> (usize, usize) {
    match node {
        CpgNode::Function {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Statement {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::ReturnValue {
            start_byte,
            end_byte,
            ..
        } => (*start_byte, *end_byte),
    }
}

impl CodePropertyGraph {
    fn trace_identity(&self, index: NodeIndex) -> String {
        let node = self.node(index);
        let (start, end) = byte_span(node);
        let owner = self.node_file_fn(index);
        let occurrence = match node {
            CpgNode::Variable { path, access, .. } => format!(":{path}:{access:?}"),
            _ => String::new(),
        };
        format!(
            "{}:{}:{start}..{end}#{}@{owner:?}{occurrence}",
            node.file(),
            node.line(),
            index.index()
        )
    }
}

impl CodePropertyGraph {
    /// The ledger prints a site, slot and grades only if the same call expression contains
    /// the source operand and its result reaches the receiving occurrence. For a CallDescent,
    /// the call contains the source actual and its callee binds it to the receiving formal.
    /// Where either endpoint or a unique site cannot be established, all three fields are empty.
    pub(super) fn ledger_site(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        relation: Relation,
        view: Option<&dyn SameLineOrderView>,
    ) -> Option<SiteEvidence> {
        let view = view?;
        let receiver = self.to_var_location(to)?;
        if receiver.kind != VarAccessKind::Def {
            return None;
        }
        let call_owner = match relation {
            Relation::CallDescent => from,
            Relation::ReturnFlow | Relation::AssignmentPropagation => to,
            _ => return None,
        };
        let caller = view.ledger_file(self.node(call_owner).file())?;
        let callee_owner = match relation {
            Relation::CallDescent => Some(self.node_file_fn(to)?),
            Relation::ReturnFlow => Some(self.node_file_fn(from)?),
            _ => None,
        };
        let actual = self.to_var_location(from);
        let descent_site = if relation == Relation::CallDescent {
            Some(self.binding_site(from, to).ok()?)
        } else {
            None
        };
        let mut established = self.return_sites(call_owner).filter_map(|site| {
            if descent_site.is_some_and(|candidate| candidate != site) {
                return None;
            }
            let call = caller
                .tree
                .root_node()
                .descendant_for_byte_range(site.start_byte, site.end_byte)?;
            if call.byte_range() != (site.start_byte..site.end_byte)
                || !caller.language.is_call_node(call.kind())
                || call.has_error()
                || site.origin != CallSiteOrigin::Source
            {
                return None;
            }
            let (slot, arity) = if relation == Relation::CallDescent {
                (
                    argument_slot(caller, call, &site.caller, actual.as_ref()?, true)?,
                    0,
                )
            } else {
                let assignment =
                    caller.assignment_for_exact_call(site.start_byte, site.end_byte)?;
                let slot = assignment.lvalues.iter().position(|lhs| {
                    (lhs.start_byte, lhs.end_byte, &lhs.path)
                        == (receiver.start_byte, receiver.end_byte, &receiver.path)
                })?;
                (slot, assignment.lvalues.len())
            };
            let resolved = self.call_graph.resolve_call_site_full(site);
            let slot = match relation {
                Relation::AssignmentPropagation => {
                    super::build::singleton_exact(&resolved)?;
                    argument_slot(caller, call, &site.caller, actual.as_ref()?, false)?
                }
                _ => {
                    let (file, name, start) = callee_owner.as_ref()?;
                    let callee = view.ledger_file(file)?;
                    if !resolved.resolved.iter().any(|entry| {
                        let target = entry.target;
                        (&target.file, &target.name, target.start_line) == (file, name, *start)
                            && function_at(callee, target).is_some_and(|owner| {
                                if relation == Relation::CallDescent {
                                    formal_matches(callee, owner, slot, &receiver)
                                } else {
                                    returned_operand(callee, owner, self.node(from), slot, arity)
                                }
                            })
                    }) {
                        return None;
                    }
                    slot
                }
            };
            Some(SiteEvidence {
                location: CrossingSite {
                    start_byte: site.start_byte,
                    end_byte: site.end_byte,
                    slot: Some(slot),
                },
                grades: resolved
                    .resolved
                    .into_iter()
                    .map(|entry| entry.confidence)
                    .collect(),
            })
        });
        let site = established.next()?;
        established.next().is_none().then_some(site)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn log_trace_attempt(
        &self,
        ledger: CrossingLedger<'_>,
        root: NodeIndex,
        from: NodeIndex,
        to: NodeIndex,
        relation: Relation,
        site: Option<&SiteEvidence>,
        label: Option<FlowConfidence>,
        depth: usize,
        reason: Option<LossReason>,
    ) {
        let event = match (reason, relation) {
            (Some(_), _) => "taint-refusal",
            (None, Relation::ReturnInput) => "taint-return-input",
            (None, _) => "taint-crossing",
        };
        let slot = site
            .and_then(|site| site.location.slot)
            .map_or_else(|| "-".into(), |slot| slot.to_string());
        let component = match self.node(to) {
            CpgNode::ReturnValue {
                child_slot,
                return_start_byte,
                return_end_byte,
                ..
            } => format!(" component={child_slot} return={return_start_byte}..{return_end_byte}"),
            _ => String::new(),
        };
        let reason = reason.map_or_else(String::new, |reason| format!(" reason={reason}"));
        let site_grades = site
            .into_iter()
            .flat_map(|site| &site.grades)
            .map(|grade| format!("{grade:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let site = site.map_or_else(
            || "-".into(),
            |site| format!("{}..{}", site.location.start_byte, site.location.end_byte),
        );
        // The diagnostic has no bearing on the walk, including when stderr is closed.
        let site_fields = if ledger.fields {
            format!(" site={site} slot={slot} grades=[{site_grades}]")
        } else {
            String::new()
        };
        let _ = writeln!(io::stderr().lock(),
            "{event} walk={} walk_id={} root={:?} from={:?} to={:?} relation={relation:?}{site_fields} edge_grade={label:?} depth={depth}{component}{reason}",
            ledger.walk, ledger.id,
            self.trace_identity(root), self.trace_identity(from), self.trace_identity(to)
        );
    }
}

fn argument_slot(
    parsed: &ParsedFile,
    call: tree_sitter::Node<'_>,
    caller: &FunctionId,
    actual: &VarLocation,
    positional: bool,
) -> Option<usize> {
    if (&actual.file, &actual.function, actual.function_start_line)
        != (&caller.file, &caller.name, caller.start_line)
        || actual.kind != VarAccessKind::Use
        || actual.start_byte >= actual.end_byte
        || !parsed.is_semantic_value_use(actual.start_byte, actual.end_byte)
    {
        return None;
    }
    let arguments = parsed.language.call_arguments(&call)?;
    let mut cursor = arguments.walk();
    for (slot, arg) in arguments
        .named_children(&mut cursor)
        .filter(|arg| !arg.is_extra())
        .enumerate()
    {
        if positional
            && matches!(
                arg.kind(),
                "keyword_argument"
                    | "list_splat"
                    | "dictionary_splat"
                    | "spread_element"
                    | "variadic_argument"
            )
        {
            return None;
        }
        if arg.start_byte() <= actual.start_byte && actual.end_byte <= arg.end_byte() {
            let span = actual.start_byte..actual.end_byte;
            return (!positional || arg.byte_range() == span).then_some(slot);
        }
    }
    None
}

fn function_at<'a>(parsed: &'a ParsedFile, target: &FunctionId) -> Option<tree_sitter::Node<'a>> {
    let mut candidates = parsed.all_functions().into_iter().filter(|function| {
        parsed.node_line_range(function) == (target.start_line, target.end_line)
            && parsed
                .language
                .function_name(function)
                .is_some_and(|name| parsed.node_text(&name) == target.name)
    });
    let function = candidates.next()?;
    candidates.next().is_none().then_some(function)
}

fn formal_matches(
    parsed: &ParsedFile,
    owner: tree_sitter::Node<'_>,
    slot: usize,
    receiver: &VarLocation,
) -> bool {
    // Receiver-bearing signatures require call-form-specific argument mapping.
    if (parsed.language == Language::Go && owner.kind() == "method_declaration")
        || (parsed.language == Language::Python
            && owner.parent().is_some_and(|parent| {
                let block = if parent.kind() == "decorated_definition" {
                    parent.parent()
                } else {
                    Some(parent)
                };
                block
                    .and_then(|block| block.parent())
                    .is_some_and(|parent| parent.kind() == "class_definition")
            }))
    {
        return false;
    }
    receiver.path.fields.is_empty()
        && parsed
            .function_parameter_slot_occurrences(&owner)
            .is_some_and(|formals| {
                formals.get(slot).is_some_and(|(name, start, end)| {
                    (name, *start, *end)
                        == (&receiver.path.base, receiver.start_byte, receiver.end_byte)
                })
            })
}

fn returned_operand(
    parsed: &ParsedFile,
    owner: tree_sitter::Node<'_>,
    from: &CpgNode,
    slot: usize,
    arity: usize,
) -> bool {
    let span = byte_span(from);
    parsed.return_value_nodes(&owner).iter().any(|ret| {
        ret.values.len() == arity
            && ret.values.get(slot).is_some_and(|value| {
                span == (value.start_byte, value.end_byte)
                    && match from {
                        CpgNode::Variable {
                            access: VarAccess::Use,
                            ..
                        } => true,
                        CpgNode::ReturnValue {
                            return_start_byte,
                            return_end_byte,
                            child_slot,
                            ..
                        } => {
                            (*return_start_byte, *return_end_byte, *child_slot)
                                == (ret.start_byte, ret.end_byte, value.slot)
                        }
                        _ => false,
                    }
            })
    })
}

impl CodePropertyGraph {
    fn shortcut_suppressed(&self, use_idx: NodeIndex, def_idx: NodeIndex) -> bool {
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start,
            start_byte: use_start,
            end_byte: use_end,
            ..
        } = &self.graph[use_idx]
        else {
            return false;
        };

        self.graph
            .edges_directed(def_idx, Direction::Incoming)
            .filter(|edge| {
                matches!(
                    edge.weight(),
                    CpgEdge::ReturnFlow {
                        suppress_shortcut: true
                    }
                )
            })
            .any(|edge| {
                let Some((callee_file, callee_name, callee_start)) =
                    self.node_file_fn(edge.source())
                else {
                    return false;
                };
                self.call_graph
                    .calls
                    .iter()
                    .filter(|(id, _)| {
                        id.file == *caller_file
                            && id.name == *caller_name
                            && id.start_line == *caller_start
                    })
                    .flat_map(|(_, sites)| sites)
                    .any(|site| {
                        site.start_byte <= *use_start
                            && *use_end <= site.end_byte
                            && super::build::singleton_exact(
                                &self.call_graph.resolve_call_site_full(site),
                            )
                            .is_some_and(|resolved| {
                                resolved.target.file == callee_file
                                    && resolved.target.name == callee_name
                                    && resolved.target.start_line == callee_start
                            })
                    })
            })
    }

    pub(super) fn taint_neighbors(
        &self,
        node: NodeIndex,
        return_flow_mode: ReturnFlowMode,
    ) -> Vec<(NodeIndex, Relation, Option<FlowConfidence>, bool)> {
        let mut out: Vec<_> = self
            .graph
            .edges(node)
            .filter_map(|edge| match edge.weight() {
                CpgEdge::DataFlow(label) => {
                    Some((edge.target(), Relation::DataFlow, Some(*label), false))
                }
                _ => None,
            })
            .collect();
        // General DFG neighbors can cross lines; NodeIndex order is build-deterministic.
        out.sort_by_key(|(target, _, _, _)| target.index());

        if return_flow_mode == ReturnFlowMode::On {
            let mut returned: Vec<_> = self
                .graph
                .edges(node)
                .filter_map(|edge| match edge.weight() {
                    CpgEdge::ReturnInput => {
                        Some((edge.target(), Relation::ReturnInput, None, false))
                    }
                    CpgEdge::ReturnFlow { .. } => {
                        Some((edge.target(), Relation::ReturnFlow, None, false))
                    }
                    _ => None,
                })
                .collect();
            returned.sort_by_key(|(target, relation, _, _)| {
                (
                    match relation {
                        Relation::ReturnInput => 0,
                        Relation::ReturnFlow => 1,
                        _ => 2,
                    },
                    target.index(),
                )
            });
            out.extend(returned);
        }

        match &self.graph[node] {
            // Use → same-line same-function `Def`s (assignment propagation: `x = y`, the use of `y`
            // flows to the def of `x`). Must stay within the SAME function — keying on (file,line)
            // alone leaks across functions (minified one-line-per-file JS/TS degenerates to
            // everything-taints-everything). Statement/byte-range scoping within a function is a
            // tracked follow-up (line-granular over-approximation).
            CpgNode::Variable {
                access: VarAccess::Use,
                file,
                line,
                function,
                function_start_line,
                ..
            } => {
                if let Some(at) = self.location_index.get(&(file.clone(), *line)) {
                    let mut same: Vec<NodeIndex> = at
                        .iter()
                        .copied()
                        .filter(|&o| {
                            matches!(
                                &self.graph[o],
                                CpgNode::Variable {
                                    access: VarAccess::Def,
                                    function: def_fn,
                                    function_start_line: def_start_line,
                                    ..
                                } if def_fn == function && def_start_line == function_start_line
                            )
                        })
                        .collect();
                    same.sort_by_key(|&i| self.node_sort_key(i));
                    out.extend(same.into_iter().map(|t| {
                        let shortcut = return_flow_mode == ReturnFlowMode::On
                            && self.shortcut_suppressed(node, t);
                        (t, Relation::AssignmentPropagation, None, shortcut)
                    }));
                }
            }
            // `Def` → same-function SAME-PATH `Use`s, re-supplying def→use edges `data_flow.rs` drops:
            //  - SIMPLE path (`x`): the DFG carries cross-line def-use, but the variable's *same-line*
            //    edge is dropped (param `ref_line == start`; regular `ref_line == def_line`), so a
            //    same-line `y = u; sink(y)` / `q = p; sink(q)` would dead-end. Recover same-line uses.
            //  - FIELD path (`o.data`): the DFG filters a field's refs to lines AFTER the def
            //    (`collect_path_refs`, `line > def_line`), so a loop-carried `def@N → use@M (M<N)` edge
            //    never exists and the loop's earlier use is a false negative. Recover ALL same-path uses
            //    on ANY line; the back-edge-aware `cfg_valid` prunes the CFG-infeasible ones (loop
            //    back-edges keep the loop-carried use). Same composition simple paths get from the DFG.
            // Same-path only (the variable's own def-use chain, not an assignment), so no cross-variable
            // leak; the safe over-approximation is consistent with the line-granular stance.
            CpgNode::Variable {
                access: VarAccess::Def,
                path,
                ..
            } => {
                let uses = if path.fields.is_empty() {
                    self.same_line_same_path_uses(node)
                } else {
                    self.same_function_same_path_uses_any_line(node)
                };
                for u in uses {
                    out.push((u, Relation::RecoveredDefUse, None, false));
                }
            }
            _ => {}
        }
        out
    }
}
