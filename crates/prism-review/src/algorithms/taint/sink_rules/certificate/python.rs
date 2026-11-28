use super::*;
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

pub(super) fn erased_edge(parent: Node<'_>, child: Node<'_>) -> bool {
    parent.kind() == "type"
        || (parent.kind() == "type_alias_statement"
            && parent.child_by_field_name("right") == Some(child))
}

pub(super) fn callee<'tree>(parsed: &ParsedFile, head: Node<'tree>) -> Result<Path<'tree>, Reason> {
    match head.kind() {
        "identifier" => Ok(Path::plain(head)),
        "attribute" => field(head, "attribute", "identifier")
            .map(|name| Path::member(head, name, head.child_by_field_name("object")))
            .ok_or(Reason::CalleeKindUnlisted),
        "subscript" => {
            let keys = fields(head, "subscript");
            match keys.as_slice() {
                [key] if !has_token(head, ",") && plain_string(parsed, *key) => {
                    Ok(Path::literal_key(*key, None))
                }
                _ => Err(Reason::SubscriptKeyNotPlainLiteral),
            }
        }
        _ => Err(Reason::CalleeKindUnlisted),
    }
}

fn plain_string(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.kind() != "string" {
        return false;
    }
    let children = named(node);
    let Some(start) = children
        .first()
        .filter(|node| node.kind() == "string_start")
    else {
        return false;
    };
    let prefix = parsed.node_text(start).split(['\'', '"']).next();
    matches!(prefix, Some("" | "r" | "R" | "u" | "U"))
        && children.iter().all(|child| {
            matches!(
                child.kind(),
                "string_start" | "string_content" | "string_end"
            )
        })
}

pub(super) fn member(node: Node<'_>) -> bool {
    rooted_member(node, "attribute", |attribute| {
        field(attribute, "attribute", "identifier")?;
        attribute.child_by_field_name("object")
    })
}

pub(super) fn value(parsed: &ParsedFile, node: Node<'_>) -> bool {
    match node.kind() {
        "integer" | "float" | "true" | "false" | "none" => true,
        "string" => plain_string(parsed, node),
        _ => member(node),
    }
}

pub(super) fn check_argument_layout(parsed: &ParsedFile, slots: &[Node<'_>]) -> Result<(), Reason> {
    let mut keywords = HashSet::new();
    let mut unpacked_keywords = false;
    for &slot in slots {
        match slot.kind() {
            "keyword_argument" => {
                let name = field(slot, "name", "identifier").ok_or(Reason::SlotMismatch)?;
                let normalized: String = parsed.node_text(&name).nfkc().collect();
                if !keywords.insert(normalized) {
                    return Err(Reason::SlotMismatch);
                }
            }
            "dictionary_splat" => unpacked_keywords = true,
            "list_splat" if !unpacked_keywords => {}
            "list_splat" => return Err(Reason::SlotMismatch),
            _ if positional_spread(slot) => return Err(Reason::OperandSpread),
            _ if unpacked_keywords || !keywords.is_empty() => return Err(Reason::SlotMismatch),
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn positional_spread(mut node: Node<'_>) -> bool {
    while matches!(
        node.kind(),
        "parenthesized_expression" | "parenthesized_list_splat"
    ) {
        let Some(inner) = unwrapped(node, "(", ")") else {
            return false;
        };
        node = inner;
    }
    node.kind() == "list_splat"
}
