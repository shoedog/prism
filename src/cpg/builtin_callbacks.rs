//! Name-only data flow from a represented built-in receiver to its callback.

use super::*;
use tree_sitter::Node;

#[path = "builtin_callbacks/diagnostic.rs"]
pub(super) mod diagnostic;
#[path = "builtin_callbacks/identity.rs"]
mod identity;
#[path = "builtin_callbacks/syntax.rs"]
mod syntax;

pub(super) const TABLE: &[(&str, usize)] = &[
    ("forEach", 0),
    ("map", 0),
    ("filter", 0),
    ("find", 0),
    ("findIndex", 0),
    ("some", 0),
    ("every", 0),
    ("flatMap", 0),
    ("reduce", 1),
    ("reduceRight", 1),
    ("then", 0),
];

pub(super) const LABEL: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::BuiltinCallback);

fn span(node: Node<'_>) -> (usize, usize) {
    (node.start_byte(), node.end_byte())
}

pub(super) fn collect(
    out: &mut Vec<PendingArgumentFlow>,
    site: &CallSite,
    caller_id: &FunctionId,
    caller_parsed: &ParsedFile,
    var_index: &VarIndex,
    exact_var_index: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
    output: Option<diagnostic::Output>,
) {
    let mut trace = output.map(|output| diagnostic::Decision::new(site, output));
    if let Err(reason) = collect_site(
        out,
        site,
        caller_id,
        caller_parsed,
        var_index,
        exact_var_index,
        graph,
        &mut trace,
    ) {
        if let Some(trace) = trace {
            trace.report("refused", reason, None, None);
        }
    }
}

fn collect_site(
    out: &mut Vec<PendingArgumentFlow>,
    site: &CallSite,
    caller: &FunctionId,
    parsed: &ParsedFile,
    var_index: &VarIndex,
    exact: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
    trace: &mut Option<diagnostic::Decision<'_>>,
) -> Result<(), &'static str> {
    let (call, arguments) = syntax::call(site, caller, parsed)?;
    let callee = call
        .child_by_field_name("function")
        .and_then(|node| syntax::unwrap(parsed, node))
        .ok_or("detached_callee")?;
    let (receiver, name) = syntax::member(parsed, callee)?;
    if let Some(t) = trace {
        t.receiver = Some(span(receiver));
        t.method = Some(name.clone());
    }
    let (_, slot) = TABLE
        .iter()
        .find(|(method, _)| *method == name)
        .ok_or("unlisted_method")?;
    let receiver = syntax::unwrap(parsed, receiver).ok_or("receiver_syntax")?;
    let path = syntax::receiver_path(parsed, receiver).ok_or("receiver_syntax")?;
    if let Some(t) = trace {
        t.receiver_path = Some(path.to_string());
    }
    let (from, representation) = receiver_use(
        caller, receiver, callee, &path, &name, var_index, exact, graph,
    )?;
    if let Some(t) = trace {
        t.receiver_use = diagnostic::endpoint(&graph[from]);
        t.representation = Some(representation);
    }

    let (callback, seed) = syntax::runtime_args(parsed, arguments);
    let callback = callback?;
    if let Some(t) = trace {
        t.callback = Some(span(callback));
    }
    let function = identity::implementation(parsed, callback, trace)?;
    let owner_name = parsed.dfg_owner_name(&function).ok_or("callback_owner")?;
    let (start_line, end_line) = parsed.node_line_range(&function);
    let owner = FunctionId {
        file: caller.file.clone(),
        name: owner_name,
        start_line,
        end_line,
    };
    if let Some(t) = trace {
        t.declaration.get_or_insert(span(function));
        t.implementation = Some(span(function));
        t.owner = Some(owner.clone());
    }
    let fixed = callee_params::formal_def_nodes(parsed, &function, &owner, var_index, graph, true)
        .ok_or("formal_layout")?;
    let rest = rest_params::rest_def_for_implementation(parsed, &owner, function, var_index, graph);

    let mut emitted = None;
    let mut bind = |role, position| {
        let report = |disposition, reason, formal| {
            if let Some(t) = trace.as_ref() {
                t.report(disposition, reason, Some((role, position)), formal);
            }
        };
        let target = fixed.get(position).copied().flatten().or_else(|| {
            rest.filter(|(start, _)| *start <= position)
                .map(|(_, node)| node)
        });
        let Some(to) = target else {
            report("refused", "missing_formal_def", None);
            return;
        };
        if !has_read(parsed, graph, &owner, function, to) {
            report("refused", "unread_formal_def", Some(&graph[to]));
            return;
        }
        if emitted == Some(to) {
            report("deduplicated", "same_rest_def", Some(&graph[to]));
            return;
        }
        out.push(PendingArgumentFlow {
            edge: (from, to, CpgEdge::DataFlow(LABEL)),
            cross_file_free_single_exact: false,
            bare_literal: false,
        });
        emitted = Some(to);
        report("admitted", "table_role", Some(&graph[to]));
    };
    bind(
        if name == "then" {
            "fulfillment"
        } else {
            "element"
        },
        *slot,
    );
    if *slot == 1 {
        match seed {
            syntax::Seed::Omitted => bind("first_accumulator", 0),
            syntax::Seed::Supplied | syntax::Seed::Unknown => {
                if let Some(t) = trace {
                    let reason = match seed {
                        syntax::Seed::Supplied => "seed_supplied",
                        _ => "seed_presence_unknown",
                    };
                    t.report("omitted", reason, Some(("first_accumulator", 0)), None);
                }
            }
        }
    }
    Ok(())
}

fn receiver_use(
    owner: &FunctionId,
    receiver: Node<'_>,
    callee: Node<'_>,
    path: &AccessPath,
    method: &str,
    var_index: &VarIndex,
    exact: &ExactVarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Result<(NodeIndex, &'static str), &'static str> {
    let key = |node: Node<'_>, path: AccessPath| {
        (
            owner.file.clone(),
            owner.name.clone(),
            owner.start_line,
            node.start_position().row + 1,
            path,
            VarAccess::Use,
        )
    };
    let mut projection = path.clone();
    projection
        .fields
        .push(if callee.kind() == "subscript_expression" {
            crate::access_path::ARRAY_FIELD.to_owned()
        } else {
            method.to_owned()
        });
    for (node, path, representation) in [
        (receiver, path.clone(), "receiver"),
        (callee, projection, "projection"),
    ] {
        if let Some(matches) = exact
            .get(&key(node, path))
            .and_then(|spans| spans.get(&span(node)))
        {
            let [candidate] = matches.as_slice() else {
                return Err("ambiguous_receiver_use");
            };
            return Ok((*candidate, representation));
        }
    }
    let key = key(receiver, AccessPath::simple(&path.base));
    var_index
        .get(graph, &key)
        .filter(|&idx| {
            matches!(
                graph.node_weight(idx),
                Some(CpgNode::Variable { start_byte, end_byte, .. })
                    if (*start_byte, *end_byte) == (receiver.start_byte(), receiver.start_byte())
            )
        })
        .map(|node| (node, "zero_width_base"))
        .ok_or("missing_receiver_use")
}

fn has_read(
    parsed: &ParsedFile,
    graph: &DiGraph<CpgNode, CpgEdge>,
    owner: &FunctionId,
    function: Node<'_>,
    def: NodeIndex,
) -> bool {
    graph.edges(def).any(|edge| {
        let Some(CpgNode::Variable {
            file,
            function: name,
            function_start_line,
            start_byte,
            end_byte,
            access: VarAccess::Use,
            ..
        }) = graph.node_weight(edge.target())
        else {
            return false;
        };
        edge.weight().is_data_flow()
            && *file == owner.file
            && *name == owner.name
            && *function_start_line == owner.start_line
            && function.start_byte() <= *start_byte
            // A line anchor can represent a read anywhere on that line,
            // including inside a deferred callable with the outer DFG owner.
            && *start_byte < *end_byte
            && *end_byte <= function.end_byte()
            && parsed
                .tree
                .root_node()
                .descendant_for_byte_range(*start_byte, *end_byte)
                .and_then(|node| syntax::execution_owner(parsed, node))
                == Some(function)
    })
}
