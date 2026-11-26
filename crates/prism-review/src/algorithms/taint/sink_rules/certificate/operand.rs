use super::operation::Form;
use super::*;

pub(in super::super) fn certify_operand(
    parsed: &ParsedFile,
    operation: Node<'_>,
    shape: &OperationShape<'_>,
    target: Node<'_>,
    operand: Node<'_>,
    role: Role<'_>,
) -> Result<(), Reason> {
    if !same_node(operation, shape.operation)
        || operation.into_raw().tree != parsed.tree.root_node().into_raw().tree
        || !within(operand, operation)
    {
        return Err(Reason::RoleNotBound);
    }
    check_operand_position(parsed.language, operation, operand)?;
    let value = match (shape.form, role) {
        (_, Role::Argument(slot)) => positional(parsed, shape, operand, slot)?,
        (_, Role::KeywordArgument(name)) if parsed.language == Language::Python => {
            keyword(parsed, shape, operand, name)?
        }
        (_, Role::ReceiverValue) => {
            let path = callee::call_path(parsed, shape)?
                .filter(|path| path.matches(target))
                .ok_or(Reason::RoleNotBound)?;
            path.receiver.ok_or(Reason::RoleNotBound)?
        }
        (Form::Assignment | Form::HclAttribute, Role::AssignmentValue) => {
            let (lhs, rhs) = assignment(parsed.language, shape)?;
            if !assignment_target(parsed.language, lhs, target) {
                return Err(Reason::RoleNotBound);
            }
            rhs
        }
        (Form::JsxAttribute, Role::AttributeValue) if same_node(target, shape.head) => {
            js::attribute_value(parsed, operation, shape.head)?
        }
        _ => return Err(Reason::RoleNotBound),
    };
    require_exact(operand, value)?;
    check_value(parsed, operand)
}

fn check_operand_position(
    language: Language,
    operation: Node<'_>,
    operand: Node<'_>,
) -> Result<(), Reason> {
    for (child, parent) in ancestors(operand)
        .zip(ancestors(operand).skip(1))
        .take_while(|(child, _)| !same_node(*child, operation))
    {
        if comment(child) {
            return Err(Reason::OperandComment);
        }
        let label = match parent.kind() {
            "initializer_pair" => fields(parent, "designator").contains(&child),
            "field_initializer" if parent.child_by_field_name("field").is_some() => {
                parent.child_by_field_name("field") == Some(child)
            }
            "keyed_element" | "pair" | "object_elem" | "element_value_pair" => {
                parent.child_by_field_name("key") == Some(child)
            }
            "keyword_argument" | "field" => parent.child_by_field_name("name") == Some(child),
            "jsx_attribute" => {
                named(parent).first() == Some(&child)
                    && matches!(child.kind(), "property_identifier" | "jsx_namespace_name")
            }
            "attribute" if language == Language::Terraform => {
                child.kind() == "identifier" && named(parent).first() == Some(&child)
            }
            _ => false,
        };
        if label {
            return Err(Reason::OperandLabel);
        }
    }
    Ok(())
}

fn require_exact(operand: Node<'_>, value: Node<'_>) -> Result<(), Reason> {
    if same_node(operand, value) {
        Ok(())
    } else if within(operand, value) {
        Err(Reason::OperandNotDirectChild)
    } else {
        Err(Reason::RoleNotBound)
    }
}

fn positional<'tree>(
    parsed: &ParsedFile,
    shape: &OperationShape<'tree>,
    operand: Node<'_>,
    claimed: usize,
) -> Result<Node<'tree>, Reason> {
    let slots = shape.arguments.ok_or(Reason::RoleNotBound)?.slots();
    let index = slots
        .iter()
        .position(|&slot| within(operand, slot))
        .ok_or(Reason::RoleNotBound)?;
    let slot = slots[index];
    let through_slot = &slots[..=index];

    match parsed.language {
        Language::Rust
            if through_slot
                .iter()
                .any(|node| node.kind() == "attribute_item") =>
        {
            return Err(Reason::OperandAttribute);
        }
        Language::C | Language::Cpp if slot.kind() == "compound_statement" => {
            return Err(Reason::OperandMacroBlock);
        }
        Language::Terraform
            if slots
                .get(index + 1)
                .is_some_and(|node| node.kind() == "ellipsis") =>
        {
            return Err(Reason::OperandSpread);
        }
        _ => {}
    }
    if through_slot
        .iter()
        .any(|&node| positional_spread(parsed, node))
    {
        return Err(Reason::OperandSpread);
    }
    if slot.kind() == "keyword_argument" {
        return Err(Reason::SlotMismatch);
    }
    if slots[..index]
        .iter()
        .filter(|node| node.kind() != "keyword_argument")
        .count()
        != claimed
    {
        return Err(Reason::SlotMismatch);
    }
    Ok(slot)
}

fn positional_spread(parsed: &ParsedFile, slot: Node<'_>) -> bool {
    match parsed.language {
        Language::Python => slot.kind() == "dictionary_splat" || python::positional_spread(slot),
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            slot.kind() == "spread_element"
        }
        Language::Cpp => slot.kind() == "parameter_pack_expansion",
        Language::Go => slot.kind() == "variadic_argument",
        Language::Lua => slot.kind() == "vararg_expression",
        Language::Terraform => slot.kind() == "ellipsis" || hcl::splat(slot),
        Language::Bash => !bash::value(parsed, slot),
        _ => false,
    }
}

fn keyword<'tree>(
    parsed: &ParsedFile,
    shape: &OperationShape<'tree>,
    operand: Node<'_>,
    claimed: &str,
) -> Result<Node<'tree>, Reason> {
    let slots = shape.arguments.ok_or(Reason::RoleNotBound)?.slots();
    if slots.iter().any(|node| node.kind() == "dictionary_splat") {
        return Err(Reason::OperandSpread);
    }
    let mut matches = slots.into_iter().filter(|&slot| {
        slot.kind() == "keyword_argument"
            && field(slot, "name", "identifier")
                .is_some_and(|name| parsed.node_text(&name) == claimed)
    });
    let slot = matches.next().ok_or(Reason::SlotMismatch)?;
    if matches.next().is_some() {
        return Err(Reason::SlotMismatch);
    }
    let value = slot
        .child_by_field_name("value")
        .ok_or(Reason::RoleNotBound)?;
    if same_node(operand, slot) {
        return Err(Reason::OperandNotDirectChild);
    }
    Ok(value)
}

fn assignment<'tree>(
    language: Language,
    shape: &OperationShape<'tree>,
) -> Result<(Node<'tree>, Node<'tree>), Reason> {
    let op = shape.operation;
    if language == Language::Lua {
        return lua::assignment(op);
    }
    if language == Language::Terraform {
        return hcl::assignment(op);
    }
    let lhs = shape.head;
    let rhs = assignment_rhs(language, op).ok_or(Reason::RoleNotBound)?;
    if language == Language::Go {
        if lhs.kind() != "expression_list" || rhs.kind() != "expression_list" {
            return Err(Reason::RoleNotBound);
        }
        return match (named(lhs).as_slice(), named(rhs).as_slice()) {
            ([lhs], [rhs])
                if op.kind() != "short_var_declaration" || lhs.kind() == "identifier" =>
            {
                Ok((*lhs, *rhs))
            }
            ([_], [_]) => Err(Reason::RoleNotBound),
            _ => Err(Reason::ParallelAssignment),
        };
    }
    if matches!(
        lhs.kind(),
        "pattern_list"
            | "tuple_pattern"
            | "list_pattern"
            | "tuple"
            | "list"
            | "array_pattern"
            | "object_pattern"
            | "tuple_expression"
    ) {
        return Err(Reason::ParallelAssignment);
    }
    Ok((lhs, rhs))
}

fn assignment_rhs(language: Language, op: Node<'_>) -> Option<Node<'_>> {
    use Language::*;
    match language {
        Bash => op.child_by_field_name("value"),
        Terraform => named(op).get(1).copied(),
        Lua => named(op)
            .into_iter()
            .find(|child| child.kind() == "expression_list"),
        _ => op.child_by_field_name("right"),
    }
}

fn assignment_target(language: Language, lhs: Node<'_>, target: Node<'_>) -> bool {
    use Language::*;
    if lhs.kind() == "identifier" && !matches!(language, Bash) {
        return same_node(lhs, target);
    }
    let terminal = match language {
        Python if python::member(lhs) => lhs.child_by_field_name("attribute"),
        JavaScript | TypeScript | Tsx if js::assignment_member(lhs) => {
            lhs.child_by_field_name("property")
        }
        Go if go::member(lhs) => lhs.child_by_field_name("field"),
        Java if java::member(lhs) => lhs.child_by_field_name("field"),
        C | Cpp if c_family::member(lhs) => lhs.child_by_field_name("field"),
        Rust if rust::member(lhs) => lhs.child_by_field_name("field"),
        Lua if lua::member(lhs) => lhs.child_by_field_name("field"),
        Bash if lhs.kind() == "variable_name" => Some(lhs),
        _ => None,
    };
    terminal.is_some_and(|name| same_node(name, target) || same_node(lhs, target))
}

fn check_value(parsed: &ParsedFile, operand: Node<'_>) -> Result<(), Reason> {
    use Language::*;
    let admitted = match parsed.language {
        Python => python::value(parsed, operand),
        JavaScript | TypeScript | Tsx => js::value(operand),
        Go => go::value(operand),
        Java => java::value(operand),
        C | Cpp => c_family::value(parsed.language, operand),
        Rust => rust::value(operand),
        Lua => lua::value(operand),
        Bash => bash::value(parsed, operand),
        Terraform => hcl::value(operand),
    };
    if admitted {
        Ok(())
    } else if positional_spread(parsed, operand) {
        Err(Reason::OperandSpread)
    } else {
        Err(Reason::OperandKindUnlisted)
    }
}

#[cfg(test)]
pub(in super::super) fn candidates<'tree, 'p>(
    parsed: &'p ParsedFile,
    shape: &OperationShape<'tree>,
) -> Vec<(Node<'tree>, Role<'p>)> {
    let op = shape.operation;
    if matches!(shape.form, Form::Assignment | Form::HclAttribute) {
        return assignment(parsed.language, shape)
            .ok()
            .map(|(_, value)| value)
            .or_else(|| assignment_rhs(parsed.language, op))
            .map(|value| (value, Role::AssignmentValue))
            .into_iter()
            .collect();
    }
    if shape.form == Form::JsxAttribute {
        return js::attribute_value(parsed, op, shape.head)
            .ok()
            .or_else(|| named(op).get(1).copied())
            .map(|value| (value, Role::AttributeValue))
            .into_iter()
            .collect();
    }

    let mut candidates = Vec::new();
    let mut ordinal = 0;
    for slot in shape
        .arguments
        .into_iter()
        .flat_map(|arguments| arguments.slots())
    {
        candidates.push((slot, Role::Argument(ordinal)));
        if slot.kind() == "keyword_argument" {
            if let (Some(name), Some(value)) = (
                slot.child_by_field_name("name"),
                slot.child_by_field_name("value"),
            ) {
                candidates.push((value, Role::KeywordArgument(parsed.node_text(&name))));
            }
        } else {
            ordinal += 1;
        }
    }
    if let Ok(Some(path)) = callee::call_path(parsed, shape) {
        if let Some(receiver) = path.receiver {
            candidates.push((receiver, Role::ReceiverValue));
        }
    }
    candidates
}
