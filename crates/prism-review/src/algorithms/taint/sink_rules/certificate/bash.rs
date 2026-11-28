use super::*;

pub(super) fn callee<'tree>(parsed: &ParsedFile, head: Node<'tree>) -> Result<Path<'tree>, Reason> {
    if head.kind() == "command_name" {
        if let [word] = named(head).as_slice() {
            let mut bytes = parsed.node_text(word).bytes();
            if word.kind() == "word"
                && bytes
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Ok(Path::plain(*word));
            }
        }
    }
    Err(Reason::CalleeKindUnlisted)
}

pub(super) fn value(parsed: &ParsedFile, node: Node<'_>) -> bool {
    match node.kind() {
        "word" => parsed
            .node_text(&node)
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_./:+-".contains(&b)),
        "raw_string" => true,
        "string" => {
            let children = tokens(node);
            match children.as_slice() {
                [open, content @ .., close] if open.kind() == "\"" && close.kind() == "\"" => {
                    content.iter().all(|node| node.kind() == "string_content")
                        || matches!(
                            content,
                            [expansion] if expansion.kind() == "simple_expansion"
                                && matches!(
                                    named(*expansion).as_slice(),
                                    [variable] if variable.kind() == "variable_name"
                                )
                        )
                }
                _ => false,
            }
        }
        _ => false,
    }
}
