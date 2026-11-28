//! Shared tree-sitter readings of a call site, so the crossing ledger and the walk proofs
//! use one reading of an argument slot, a function node, a formal and a returned operand.

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;
use tree_sitter::Node;

use crate::cpg::trace_cuts::byte_span;
use crate::cpg::{CpgNode, VarAccess};

pub(in crate::cpg) fn argument_slot(
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

pub(in crate::cpg) fn function_at<'a>(
    parsed: &'a ParsedFile,
    target: &FunctionId,
) -> Option<tree_sitter::Node<'a>> {
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

pub(in crate::cpg) fn formal_matches(
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

pub(in crate::cpg) fn returned_operand(
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

// ---------------------------------------------------------------------------------------------
// Call-site readings shared by the real proofs (a port of the offline classifier's `syntax.py`;
// kind names are the classifier's, so the replay joins one to one).
// ---------------------------------------------------------------------------------------------

const CALLS: [&str; 4] = [
    "call",
    "call_expression",
    "new_expression",
    "type_conversion_expression",
];
const FUNCTIONS: [&str; 11] = [
    "function_definition",
    "lambda",
    "function_declaration",
    "function_expression",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
    "method_declaration",
    "func_literal",
    "function_literal",
];
const WRAPPERS: [&str; 5] = [
    "parenthesized_expression",
    "as_expression",
    "satisfies_expression",
    "non_null_expression",
    "type_assertion",
];
const IDENTS: [&str; 3] = [
    "identifier",
    "shorthand_property_identifier",
    "shorthand_property_identifier_pattern",
];
const LHS_LISTS: [&str; 6] = [
    "expression_list",
    "pattern_list",
    "tuple_pattern",
    "list_pattern",
    "array_pattern",
    "object_pattern",
];
const REST: [&str; 4] = [
    "list_splat_pattern",
    "dictionary_splat_pattern",
    "rest_pattern",
    "variadic_parameter_declaration",
];
const CONDITIONALS: [&str; 19] = [
    "if_statement",
    "if_expression",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "try_statement",
    "switch_statement",
    "select_statement",
    "expression_switch_statement",
    "type_switch_statement",
    "match_statement",
    "with_statement",
    "conditional_expression",
    "ternary_expression",
    "catch_clause",
    "case_clause",
    "comprehension",
    "list_comprehension",
    "generator_expression",
];

/// `(file, function name, function start line)`: the owner identity a graph node carries.
pub(in crate::cpg) type OwnerId = (String, String, usize);

pub(in crate::cpg) fn is_call(node: Node<'_>) -> bool {
    CALLS.contains(&node.kind())
}

pub(in crate::cpg) fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !child.is_extra())
        .collect()
}

/// The first of the named fields the node has.
pub(in crate::cpg) fn field<'a>(node: Option<Node<'a>>, names: &[&str]) -> Option<Node<'a>> {
    let node = node?;
    names.iter().find_map(|name| node.child_by_field_name(name))
}

/// The node itself, then its parents up to the root.
pub(in crate::cpg) fn ancestors(node: Option<Node<'_>>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node, |node| node.parent())
}

pub(in crate::cpg) fn covers(node: Node<'_>, (start, end): (usize, usize)) -> bool {
    node.start_byte() <= start && start <= end && end <= node.end_byte()
}

pub(in crate::cpg) fn span(node: Node<'_>) -> (usize, usize) {
    (node.start_byte(), node.end_byte())
}

fn line(node: Node<'_>) -> usize {
    node.start_position().row + 1
}

pub(in crate::cpg) fn txt<'a>(parsed: &'a ParsedFile, node: Option<Node<'_>>) -> Option<&'a str> {
    node.map(|node| parsed.node_text(&node))
}

/// Peel parentheses and erased TS wrappers; `None` for a wrapper with no operand.
pub(in crate::cpg) fn unwrap_wrappers(mut node: Option<Node<'_>>) -> Option<Node<'_>> {
    while let Some(wrapper) = node.filter(|node| WRAPPERS.contains(&node.kind())) {
        let parts = children(wrapper);
        node = if wrapper.kind() == "type_assertion" {
            parts.last().copied()
        } else {
            parts.first().copied()
        };
    }
    node
}

/// The smallest named node over non-empty `bounds`; `None` for a zero-width range or one the
/// file does not hold.
pub(in crate::cpg) fn at<'a>(parsed: &'a ParsedFile, bounds: (usize, usize)) -> Option<Node<'a>> {
    let root = parsed.tree.root_node();
    if bounds.0 >= bounds.1 || !covers(root, bounds) {
        return None;
    }
    root.named_descendant_for_byte_range(bounds.0, bounds.1)
}

/// The nearest enclosing function-like node (the node itself counts).
pub(in crate::cpg) fn owner(node: Option<Node<'_>>) -> Option<Node<'_>> {
    ancestors(node).find(|node| FUNCTIONS.contains(&node.kind()))
}

fn function_name(function: Node<'_>) -> Option<Node<'_>> {
    field(Some(function), &["name"]).or_else(|| field(function.parent(), &["name", "left", "key"]))
}

/// Does `function` execute the occurrences a graph node attributes to `owner`? An anonymous
/// owner (`<anon@..>`) is matched by its start line alone, a named one by line and name.
pub(in crate::cpg) fn owned_by(
    parsed: &ParsedFile,
    function: Option<Node<'_>>,
    owner: &OwnerId,
) -> bool {
    let Some(function) = function else {
        return false;
    };
    let mut lines = vec![line(function)];
    if let Some(parent) = function
        .parent()
        .filter(|parent| parent.kind() == "decorated_definition")
    {
        lines.push(line(parent));
    }
    if !lines.contains(&owner.2) {
        return false;
    }
    owner.1.starts_with('<') || txt(parsed, function_name(function)) == Some(owner.1.as_str())
}

fn callee(call: Node<'_>) -> Option<Node<'_>> {
    field(Some(call), &["function", "constructor", "type"])
}

pub(in crate::cpg) fn selector(call: Node<'_>) -> Option<Node<'_>> {
    let node = unwrap_wrappers(callee(call));
    field(node, &["property", "attribute", "field"]).or(node)
}

pub(in crate::cpg) fn receiver(call: Node<'_>) -> Option<Node<'_>> {
    field(
        unwrap_wrappers(callee(call)),
        &["object", "operand", "value"],
    )
}

pub(in crate::cpg) fn args(call: Node<'_>) -> Vec<Node<'_>> {
    let list = field(Some(call), &["arguments", "operand"]).or_else(|| {
        (call.kind() == "type_conversion_expression")
            .then(|| field(Some(call), &["value"]))
            .flatten()
    });
    list.map(children).unwrap_or_default()
}

/// What an argument supplies: the value of a keyword argument, else the argument itself.
fn value_arg(arg: Node<'_>) -> Option<Node<'_>> {
    if arg.kind() == "keyword_argument" {
        arg.child_by_field_name("value")
    } else {
        Some(arg)
    }
}

pub(in crate::cpg) fn actual_covered_by(arg: Node<'_>, actual: (usize, usize)) -> bool {
    value_arg(arg).is_some_and(|value| covers(value, actual))
}

/// The call the reached bytes are an argument of, as `(call, slot, argument)` triples. A nested
/// call's result supplies its own outer actual; the bytes of its arguments do not. The refusal
/// names why the bytes supply no call actual of the owner the graph attributes them to.
pub(in crate::cpg) fn actual_sites<'a>(
    parsed: &'a ParsedFile,
    actual: (usize, usize),
    owner_id: &OwnerId,
) -> Result<Vec<(Node<'a>, usize, Node<'a>)>, ActualFailure> {
    let node = at(parsed, actual).ok_or(ActualFailure::Unbound("zero_width_or_unbound_bytes"))?;
    let function = owner(Some(node));
    if !owned_by(parsed, function, owner_id) {
        return Err(if function.is_some() {
            ActualFailure::Refused("intervening_callable")
        } else {
            ActualFailure::Unbound("execution_owner_unbound")
        });
    }
    let mut sites = Vec::new();
    for parent in ancestors(Some(node)) {
        if Some(parent) == function {
            break;
        }
        if !is_call(parent) {
            continue;
        }
        sites.extend(
            args(parent)
                .into_iter()
                .enumerate()
                .filter(|&(_, arg)| actual_covered_by(arg, actual))
                .map(|(slot, arg)| (parent, slot, arg)),
        );
        if sites.is_empty() {
            return Err(ActualFailure::Refused("receiver_callee_or_keyword_token"));
        }
        // The first call boundary is decisive: wider calls consume its result.
        break;
    }
    if sites.is_empty() {
        let jsx = ancestors(Some(node)).any(|node| node.kind().starts_with("jsx_"));
        return Err(if jsx {
            ActualFailure::Unbound("jsx_or_binding_read")
        } else {
            ActualFailure::Refused("no_ordinary_actual")
        });
    }
    Ok(sites)
}

/// Why the reached bytes supply no single call actual: provably none, or not decidable here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cpg) enum ActualFailure {
    Refused(&'static str),
    Unbound(&'static str),
}

/// A call node of exactly `site` bytes containing the byte range's smallest named node.
pub(in crate::cpg) fn call_at<'a>(
    parsed: &'a ParsedFile,
    site: (usize, usize),
) -> Option<Node<'a>> {
    ancestors(at(parsed, site)).find(|node| is_call(*node) && span(*node) == site)
}

/// `(selector, receiver)` text of a call.
fn call_labels<'a>(parsed: &'a ParsedFile, call: Node<'_>) -> (Option<&'a str>, Option<&'a str>) {
    (txt(parsed, selector(call)), txt(parsed, receiver(call)))
}

/// The calls a function executes itself (nested functions excluded).
pub(in crate::cpg) fn calls_of<'a>(function: Node<'a>) -> Vec<Node<'a>> {
    let mut calls = Vec::new();
    let mut stack = vec![function];
    while let Some(node) = stack.pop() {
        if is_call(node) {
            calls.push(node);
        }
        if node == function || !FUNCTIONS.contains(&node.kind()) {
            stack.extend(children(node).into_iter().rev());
        }
    }
    calls
}

/// Calls of the same function and line spelled like `call`, other than `call` itself.
pub(in crate::cpg) fn same_line_peers<'a>(
    parsed: &ParsedFile,
    call: Node<'a>,
    function: Option<Node<'a>>,
) -> Vec<Node<'a>> {
    let labels = call_labels(parsed, call);
    function
        .map(calls_of)
        .unwrap_or_default()
        .into_iter()
        .filter(|peer| {
            *peer != call
                && owner(Some(*peer)) == function
                && line(*peer) == line(call)
                && call_labels(parsed, *peer) == labels
        })
        .collect()
}

fn binding_leaves(node: Option<Node<'_>>) -> Vec<Node<'_>> {
    let Some(node) = node else {
        return Vec::new();
    };
    let kind = node.kind();
    if IDENTS.contains(&kind) {
        vec![node]
    } else if matches!(
        kind,
        "pair_pattern" | "assignment_pattern" | "object_assignment_pattern"
    ) {
        binding_leaves(field(Some(node), &["value", "left"]))
    } else if LHS_LISTS.contains(&kind) || REST.contains(&kind) {
        children(node)
            .into_iter()
            .flat_map(|child| binding_leaves(Some(child)))
            .collect()
    } else {
        Vec::new()
    }
}

fn lhs_rhs(node: Node<'_>) -> (Option<Node<'_>>, Option<Node<'_>>) {
    match node.kind() {
        "assignment"
        | "assignment_expression"
        | "augmented_assignment"
        | "augmented_assignment_expression"
        | "short_var_declaration"
        | "assignment_statement"
        | "named_expression" => (
            field(Some(node), &["left", "name"]),
            field(Some(node), &["right", "value"]),
        ),
        "variable_declarator" | "var_spec" | "const_spec" => {
            (field(Some(node), &["name"]), field(Some(node), &["value"]))
        }
        _ => (None, None),
    }
}

/// `(leaf, assigned value, statement)` of every lexical write of `name` before `cutoff`
/// directly in `scope` (nested functions excluded). Guarded writes are listed too; no caller
/// assumes they run.
pub(in crate::cpg) fn writes<'a>(
    parsed: &ParsedFile,
    scope: Node<'a>,
    name: &str,
    cutoff: usize,
) -> Vec<(Node<'a>, Option<Node<'a>>, Node<'a>)> {
    let mut found = Vec::new();
    let mut stack = vec![scope];
    while let Some(node) = stack.pop() {
        if node == scope || !FUNCTIONS.contains(&node.kind()) {
            stack.extend(children(node).into_iter().rev());
        }
        if node.start_byte() >= cutoff {
            continue;
        }
        let (lhs, value) = lhs_rhs(node);
        let lefts: Vec<Option<Node<'a>>> = if matches!(node.kind(), "var_spec" | "const_spec") {
            let mut cursor = node.walk();
            node.children_by_field_name("name", &mut cursor)
                .map(Some)
                .collect()
        } else if matches!(
            node.kind(),
            "delete_statement" | "inc_statement" | "dec_statement" | "update_expression"
        ) {
            children(node).into_iter().map(Some).collect()
        } else {
            vec![lhs]
        };
        for left in lefts {
            for leaf in binding_leaves(left) {
                if txt(parsed, Some(leaf)) == Some(name) && leaf.start_byte() < cutoff {
                    found.push((leaf, value, node));
                }
            }
        }
    }
    found
}

const JS_SCOPES: [&str; 6] = [
    "statement_block",
    "for_statement",
    "for_in_statement",
    "switch_body",
    "class_static_block",
    "program",
];
const GO_SCOPES: [&str; 11] = [
    "block",
    "if_statement",
    "for_statement",
    "expression_switch_statement",
    "type_switch_statement",
    "select_statement",
    "expression_case",
    "default_case",
    "type_case",
    "communication_case",
    "source_file",
];

/// Is the binding the write `statement` makes visible at `site`? Block-scoped declarations
/// (JS `let`/`const`, Go `:=`, `var`, `const`) are visible only inside their block; a JS `var`
/// and every plain assignment name a variable of the whole function. `None` when the
/// declaration's scope cannot be established.
pub(in crate::cpg) fn write_visible_at(statement: Node<'_>, site: Node<'_>) -> Option<bool> {
    let scopes: &[&str] = match statement.kind() {
        "variable_declarator" => match statement.parent()?.kind() {
            "variable_declaration" => return Some(true),
            "lexical_declaration" => &JS_SCOPES,
            _ => return None,
        },
        "short_var_declaration" | "var_spec" | "const_spec" => &GO_SCOPES,
        _ => return Some(true),
    };
    let scope = ancestors(statement.parent()).find(|node| scopes.contains(&node.kind()))?;
    Some(covers(scope, span(site)))
}

/// What a call is spelled as: member calls by selector and receiver; a bare callee by its text,
/// or by the identifier a single unconditional alias write visible at the call binds it to.
/// `Err` when the visibility of a candidate alias cannot be established.
pub(in crate::cpg) fn spelling_checked<'a>(
    parsed: &'a ParsedFile,
    call: Node<'a>,
    function: Option<Node<'a>>,
) -> Result<(Option<&'a str>, Option<&'a str>), &'static str> {
    let member = receiver(call);
    let node = unwrap_wrappers(callee(call));
    let Some(node) = node.filter(|node| member.is_none() && IDENTS.contains(&node.kind())) else {
        return Ok(call_labels(parsed, call));
    };
    if let Some(function) = function {
        let mut assignments = Vec::new();
        for write in writes(parsed, function, parsed.node_text(&node), node.start_byte()) {
            match write_visible_at(write.2, call) {
                Some(true) => assignments.push(write),
                Some(false) => {}
                None => return Err("alias_scope_unknown"),
            }
        }
        if let [(_, value, statement)] = assignments.as_slice() {
            let value = value.map(|value| match children(value).as_slice() {
                [only] if value.kind() == "expression_list" => *only,
                _ => value,
            });
            let conditional = ancestors(Some(*statement)).any(|ancestor| {
                CONDITIONALS.contains(&ancestor.kind()) && covers(function, span(ancestor))
            });
            if let (false, Some(value)) = (conditional, value) {
                if IDENTS.contains(&value.kind()) {
                    return Ok((txt(parsed, Some(value)), None));
                }
            }
        }
    }
    Ok((txt(parsed, Some(node)), None))
}

/// [`spelling_checked`] with an alias of unknown visibility left unfollowed.
#[cfg(test)]
pub(in crate::cpg) fn spelling<'a>(
    parsed: &'a ParsedFile,
    call: Node<'a>,
    function: Option<Node<'a>>,
) -> (Option<&'a str>, Option<&'a str>) {
    spelling_checked(parsed, call, function).unwrap_or_else(|_| call_labels(parsed, call))
}
