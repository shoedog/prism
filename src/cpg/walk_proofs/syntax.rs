//! Shared tree-sitter readings of a call site, so the crossing ledger and the walk proofs
//! use one reading of an argument slot, a function node, a formal and a returned operand.

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;

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
