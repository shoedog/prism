use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum Form {
    Call,
    Constructor,
    BashCommand,
    HclCall,
    Assignment,
    HclAttribute,
    JsxAttribute,
}

#[derive(Clone, Copy)]
pub(super) enum Arguments<'tree> {
    Container(Node<'tree>),
    BashFields(Node<'tree>),
}

impl<'tree> Arguments<'tree> {
    pub(super) fn slots(self) -> Vec<Node<'tree>> {
        match self {
            Self::Container(container) => named(container),
            Self::BashFields(command) => fields(command, "argument"),
        }
    }
}

pub(in super::super) struct OperationShape<'tree> {
    pub(super) operation: Node<'tree>,
    pub(super) form: Form,
    pub(super) head: Node<'tree>,
    pub(super) arguments: Option<Arguments<'tree>>,
}

#[derive(Clone, Copy)]
enum Layout {
    Call(&'static str, &'static str),
    Assignment,
    JsxAttribute,
    BashCommand,
    BashAssignment,
    HclCall,
    HclAttribute,
    LuaAssignment,
}

fn layout(language: Language, kind: &str) -> Option<(Form, Layout)> {
    use Form::*;
    use Language::*;
    Some(match (language, kind) {
        (Python, "call") => (Call, Layout::Call("function", "argument_list")),
        (JavaScript | TypeScript | Tsx | Rust, "call_expression") => {
            (Call, Layout::Call("function", "arguments"))
        }
        (JavaScript | TypeScript | Tsx, "new_expression") => {
            (Constructor, Layout::Call("constructor", "arguments"))
        }
        (Go | C | Cpp, "call_expression") => (Call, Layout::Call("function", "argument_list")),
        (Cpp, "new_expression") | (Java, "object_creation_expression") => {
            (Constructor, Layout::Call("type", "argument_list"))
        }
        (Java, "method_invocation") => (Call, Layout::Call("name", "argument_list")),
        (Lua, "function_call") => (Call, Layout::Call("name", "arguments")),
        (Bash, "command") => (BashCommand, Layout::BashCommand),
        (Terraform, "function_call") => (HclCall, Layout::HclCall),
        (Python, "assignment")
        | (JavaScript | TypeScript | Tsx | Java | C | Cpp | Rust, "assignment_expression")
        | (Go, "assignment_statement" | "short_var_declaration") => {
            (Assignment, Layout::Assignment)
        }
        (Lua, "assignment_statement") => (Assignment, Layout::LuaAssignment),
        (Bash, "variable_assignment") => (Assignment, Layout::BashAssignment),
        (Terraform, "attribute") => (HclAttribute, Layout::HclAttribute),
        (JavaScript | Tsx, "jsx_attribute") => (JsxAttribute, Layout::JsxAttribute),
        _ => return None,
    })
}

fn head_for(operation: Node<'_>, layout: Layout) -> Option<Node<'_>> {
    match layout {
        Layout::Call(head, _) => operation.child_by_field_name(head),
        Layout::Assignment => operation.child_by_field_name("left"),
        Layout::LuaAssignment => named(operation).first().copied(),
        Layout::BashAssignment | Layout::BashCommand => operation.child_by_field_name("name"),
        Layout::JsxAttribute | Layout::HclCall | Layout::HclAttribute => {
            named(operation).first().copied()
        }
    }
}

/// The census first finds a listed operation kind and declared name region.
/// Its modifiers, context and head still go through the certificate.
#[cfg(test)]
pub(in super::super) fn operation_head(
    language: Language,
    operation: Node<'_>,
) -> Option<Node<'_>> {
    let (_, layout) = layout(language, operation.kind())?;
    head_for(operation, layout)
}

pub(in super::super) fn certify_operation<'tree>(
    parsed: &ParsedFile,
    operation: Node<'tree>,
) -> Result<OperationShape<'tree>, Reason> {
    let root = parsed.tree.root_node();
    if !within(operation, root)
        || operation.is_missing()
        || operation.has_error()
        || operation.is_error()
    {
        return Err(Reason::ParseRecovery);
    }
    if non_runtime(parsed, operation) {
        return Err(Reason::OpContextNonRuntime);
    }
    if parsed.language.is_js_family() && js::attribute_spread(operation) {
        return Err(Reason::AttributeSpread);
    }
    let (form, layout) = layout(parsed.language, operation.kind()).ok_or(Reason::OpKindUnlisted)?;
    let head = head_for(operation, layout).ok_or(Reason::OpSpecialForm)?;
    let arguments = match layout {
        Layout::Call(_, container) => {
            check_call(parsed, operation, form, head)?;
            let args = field(operation, "arguments", container)
                .filter(|&node| delimited_arguments(node))
                .ok_or(Reason::OpArgumentsUnlisted)?;
            Some(Arguments::Container(args))
        }
        Layout::BashCommand => Some(Arguments::BashFields(operation)),
        Layout::HclCall => Some(Arguments::Container(hcl::arguments(operation)?)),
        Layout::JsxAttribute => {
            if js::attribute_tag(operation).is_none() {
                return Err(Reason::OpContextNonRuntime);
            }
            None
        }
        Layout::Assignment => {
            if operation.child_by_field_name("right").is_none()
                || (matches!(
                    parsed.language,
                    Language::C | Language::Cpp | Language::Go | Language::Java
                ) && operation.kind() != "short_var_declaration"
                    && field(operation, "operator", "=").is_none())
                || (parsed.language.is_js_family() && has_token(operation, "using"))
            {
                return Err(Reason::OpSpecialForm);
            }
            None
        }
        Layout::LuaAssignment | Layout::BashAssignment | Layout::HclAttribute => None,
    };
    Ok(OperationShape {
        operation,
        form,
        head,
        arguments,
    })
}

fn delimited_arguments(node: Node<'_>) -> bool {
    let children = tokens(node);
    children.first().is_some_and(|node| node.kind() == "(")
        && children.last().is_some_and(|node| node.kind() == ")")
}

fn check_call(parsed: &ParsedFile, op: Node<'_>, form: Form, head: Node<'_>) -> Result<(), Reason> {
    use Language::*;
    match parsed.language {
        JavaScript | TypeScript | Tsx => js::check_call(op, head),
        Go => go::check_call(parsed, op, head),
        Java => java::check_call(op, form),
        C | Cpp => c_family::check_call(parsed, op, form, head),
        Python | Rust | Lua | Bash | Terraform => Ok(()),
    }
}

fn non_runtime(parsed: &ParsedFile, operation: Node<'_>) -> bool {
    match parsed.language {
        Language::TypeScript | Language::Tsx if js::declaration_file(&parsed.path) => return true,
        Language::Go if go::size_query(parsed, operation) => return true,
        _ => {}
    }
    let children = ancestors(operation);
    for (child, parent) in children.zip(ancestors(operation).skip(1)) {
        let excluded = match parsed.language {
            Language::TypeScript | Language::Tsx => js::erased_edge(parent, child),
            Language::Python => python::erased_edge(parent, child),
            Language::Go => go::non_runtime_edge(parsed, parent, child),
            Language::Java => {
                matches!(
                    parent.kind(),
                    "annotation" | "marker_annotation" | "annotation_argument_list"
                ) || (parent.kind() == "annotation_type_element_declaration"
                    && parent.child_by_field_name("value") == Some(child))
            }
            Language::Rust => rust::non_runtime_edge(parent, child),
            Language::C | Language::Cpp => c_family::non_runtime_edge(parsed, parent, child),
            _ => false,
        };
        if excluded {
            return true;
        }
    }
    false
}
