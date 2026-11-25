use super::Occurrence;
use prism::ast::ParsedFile;
use prism::languages::Language;
use tree_sitter::Node;

pub(super) fn call_operand<'a>(
    parsed: &ParsedFile,
    call: Node<'a>,
    reached: Node<'a>,
) -> Option<Occurrence<'a>> {
    let callee = verified_callee(parsed, call)?;
    let (slot, operand) = if call.kind() == "command" {
        let mut cursor = call.walk();
        find_operand(
            call.children_by_field_name("argument", &mut cursor),
            reached,
        )?
    } else {
        let args = parsed.language.call_arguments(&call)?;
        let mut cursor = args.walk();
        find_operand(
            args.named_children(&mut cursor)
                .filter(|arg| arg.kind() != "ellipsis"),
            reached,
        )?
    };
    Some(Occurrence {
        call,
        callee,
        operand,
        slot,
    })
}

fn find_operand<'a>(
    args: impl Iterator<Item = Node<'a>>,
    reached: Node<'a>,
) -> Option<(usize, Node<'a>)> {
    args.filter(|arg| !arg.is_extra())
        .enumerate()
        .find(|(_, arg)| {
            arg.start_byte() <= reached.start_byte() && reached.end_byte() <= arg.end_byte()
        })
}

fn verified_callee<'a>(parsed: &ParsedFile, call: Node<'a>) -> Option<Node<'a>> {
    let callee = match (parsed.language, call.kind()) {
        (Language::Bash, "command") => {
            let word = call.child_by_field_name("name")?.named_child(0)?;
            return (word.kind() == "word"
                && parsed
                    .node_text(&word)
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_./+-".contains(&b)))
            .then_some(word);
        }
        (Language::Java, "object_creation_expression") | (Language::Cpp, "new_expression") => {
            type_leaf(call.child_by_field_name("type")?)?
        }
        (language, "new_expression") if language.is_js_family() => {
            let constructor = call.child_by_field_name("constructor")?;
            if constructor.kind() == "member_expression" {
                constructor.child_by_field_name("property")?
            } else {
                constructor
            }
        }
        (Language::Rust, _) => {
            let mut callee = parsed.call_function_name(&call)?;
            if callee.kind() == "generic_function" {
                callee = callee.child_by_field_name("function")?;
            }
            if callee.kind() == "scoped_identifier" {
                callee = callee.child_by_field_name("name")?;
            }
            callee
        }
        _ => parsed.call_function_name(&call)?,
    };
    (callee.named_child_count() == 0
        && !callee.is_missing()
        && matches!(
            callee.kind(),
            "identifier" | "property_identifier" | "field_identifier" | "type_identifier"
        ))
    .then_some(callee)
}

fn type_leaf(mut ty: Node<'_>) -> Option<Node<'_>> {
    loop {
        ty = match ty.kind() {
            "qualified_identifier" | "template_type" => ty.child_by_field_name("name")?,
            "scoped_type_identifier" => ty.named_child(ty.named_child_count().checked_sub(1)?)?,
            "generic_type" => ty.named_child(0)?,
            _ => return Some(ty),
        };
    }
}
