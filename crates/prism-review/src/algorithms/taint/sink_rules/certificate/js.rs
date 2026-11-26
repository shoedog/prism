use super::*;

pub(super) fn check_call(op: Node<'_>, head: Node<'_>) -> Result<(), Reason> {
    if op.child_by_field_name("type_arguments").is_some() {
        return Err(Reason::OpTypeArguments);
    }
    if callee::unwrap_head(Language::Tsx, head)
        .is_some_and(|head| matches!(head.kind(), "import" | "super" | "this"))
    {
        return Err(Reason::OpSpecialForm);
    }
    Ok(())
}

pub(super) fn erased_edge(parent: Node<'_>, child: Node<'_>) -> bool {
    match parent.kind() {
        "type_query"
        | "type_annotation"
        | "omitting_type_annotation"
        | "adding_type_annotation"
        | "opting_type_annotation"
        | "asserts_annotation"
        | "type_predicate_annotation"
        | "type_arguments"
        | "type_parameters"
        | "implements_clause"
        | "index_type_query"
        | "lookup_type" => true,
        "type_alias_declaration" => parent.child_by_field_name("value") == Some(child),
        "as_expression" | "satisfies_expression" => named(parent).get(1) == Some(&child),
        _ => false,
    }
}

fn optional(node: Node<'_>) -> bool {
    node.child_by_field_name("optional_chain").is_some() || has_token(node, "?.")
}

pub(super) fn non_null_operand(node: Node<'_>) -> Option<Node<'_>> {
    match tokens(node).as_slice() {
        [expression, bang] if expression.is_named() && bang.kind() == "!" => Some(*expression),
        _ => None,
    }
}

// The listed optional accesses do not certify a call in the receiver's chain.
fn has_optional_receiver_call(mut node: Node<'_>) -> bool {
    let mut inside_call = false;
    loop {
        inside_call |= node.kind() == "call_expression";
        if inside_call && optional(node) {
            return true;
        }
        let base = match node.kind() {
            "member_expression" | "subscript_expression" => node.child_by_field_name("object"),
            "call_expression" => node.child_by_field_name("function"),
            "non_null_expression" => non_null_operand(node),
            _ => return false,
        };
        let Some(base) = base else {
            return false;
        };
        node = base;
    }
}

pub(super) fn callee(head: Node<'_>) -> Result<Path<'_>, Reason> {
    if has_optional_receiver_call(head) {
        return Err(Reason::CalleeKindUnlisted);
    }
    let receiver = head.child_by_field_name("object");
    match head.kind() {
        "identifier" => Ok(Path::plain(head)),
        "member_expression" => property(head)
            .map(|name| Path::member(head, name, receiver))
            .ok_or(Reason::CalleeKindUnlisted),
        "subscript_expression" => head
            .child_by_field_name("index")
            .filter(|&key| key.kind() == "string" || plain_template(key))
            .map(|key| Path::literal_key(key, receiver))
            .ok_or(Reason::SubscriptKeyNotPlainLiteral),
        _ => Err(Reason::CalleeKindUnlisted),
    }
}

fn plain_template(node: Node<'_>) -> bool {
    if node.kind() != "template_string" {
        return false;
    }
    let mut cursor = node.walk();
    let mut children = node.named_children(&mut cursor);
    !children.any(|child| child.kind() == "template_substitution")
}

fn property(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("property").filter(|child| {
        matches!(
            child.kind(),
            "property_identifier" | "private_property_identifier"
        )
    })
}

fn member(node: Node<'_>, allow_optional: bool) -> bool {
    rooted_member(node, "member_expression", |access| {
        property(access)?;
        if !allow_optional && optional(access) {
            return None;
        }
        access.child_by_field_name("object")
    })
}

pub(super) fn assignment_member(node: Node<'_>) -> bool {
    member(node, false)
}

pub(super) fn value(node: Node<'_>) -> bool {
    match node.kind() {
        "number" | "true" | "false" | "null" | "string" => true,
        "template_string" => plain_template(node),
        _ => member(node, true),
    }
}

pub(super) fn attribute_tag(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if matches!(
        parent.kind(),
        "jsx_opening_element" | "jsx_self_closing_element"
    ) && fields(parent, "attribute").contains(&node)
    {
        return Some(parent);
    }
    None
}

pub(super) fn attribute_spread(node: Node<'_>) -> bool {
    node.kind() == "jsx_expression"
        && matches!(named(node).as_slice(), [child] if child.kind() == "spread_element")
        && attribute_tag(node).is_some()
}

pub(super) fn attribute_value<'tree>(
    parsed: &ParsedFile,
    op: Node<'tree>,
    name: Node<'_>,
) -> Result<Node<'tree>, Reason> {
    if name.kind() != "property_identifier" {
        return Err(Reason::RoleNotBound);
    }
    let tag = attribute_tag(op).ok_or(Reason::RoleNotBound)?;
    let spelling = parsed.node_text(&name);
    for later in fields(tag, "attribute")
        .into_iter()
        .skip_while(|node| *node != op)
        .skip(1)
    {
        if attribute_spread(later)
            || (later.kind() == "jsx_attribute"
                && named(later).first().is_some_and(|name| {
                    name.kind() == "property_identifier" && parsed.node_text(name) == spelling
                }))
        {
            return Err(Reason::AttributeOverwritten);
        }
    }
    let children = named(op);
    let [_, value] = children.as_slice() else {
        return Err(Reason::RoleNotBound);
    };
    if spelling != "dangerouslySetInnerHTML" && value.kind() == "string" {
        return Ok(*value);
    }
    if value.kind() != "jsx_expression" {
        return Err(Reason::OperandKindUnlisted);
    }
    let inner = unwrapped(*value, "{", "}").ok_or(Reason::OperandKindUnlisted)?;
    if spelling != "dangerouslySetInnerHTML" {
        return Ok(inner);
    }
    if inner.kind() == "object" {
        if let [pair] = named(inner).as_slice() {
            if pair.kind() == "pair"
                && field(*pair, "key", "property_identifier")
                    .is_some_and(|key| parsed.node_text(&key) == "__html")
            {
                if let Some(value) = pair.child_by_field_name("value") {
                    return Ok(value);
                }
            }
        }
    }
    Err(Reason::OperandKindUnlisted)
}
