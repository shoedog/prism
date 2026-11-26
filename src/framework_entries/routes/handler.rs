//! Callback values at Python registration sites.
use super::python::{attribute, children};
use super::receivers::{PythonBindings, PythonMember, PythonValue, SetupContext};
use super::{CallableId, Callback, CallbackStatus, Reason};
use crate::ast::ParsedFile;
use tree_sitter::Node;

pub(crate) struct PythonUse<'a, 'context> {
    pub file: &'a ParsedFile,
    pub node: Node<'a>,
    pub context: &'context SetupContext<'a>,
}

pub(crate) fn python_callback<'a>(
    bindings: &PythonBindings<'a>,
    use_site: PythonUse<'a, '_>,
    position: usize,
    transparent: impl FnMut(PythonUse<'a, '_>) -> bool,
) -> Callback {
    let PythonUse {
        file,
        node,
        context,
    } = use_site;
    let text = file.node_text(&node).to_owned();
    let reason = match bindings.capture_value(&file.path, node, context) {
        PythonValue::Definition(PythonMember { file, node })
            if node.kind() == "function_definition" =>
        {
            if has_opaque_decorator(PythonMember { file, node }, None, context, transparent) {
                Reason::InnerDecorator
            } else {
                return callback(
                    position,
                    CallbackStatus::Resolved {
                        identity: identity(file, node),
                        argument_text: text,
                    },
                );
            }
        }
        PythonValue::Unknown(Reason::ImportedUnproven) => Reason::ImportedUnproven,
        PythonValue::Opaque(PythonMember { file, node }) => match node.kind() {
            "lambda" => Reason::AnonymousUnindexed,
            "attribute" => Reason::MemberHandler,
            "call"
                if node
                    .child_by_field_name("function")
                    .and_then(attribute)
                    .is_some_and(|(_, name)| file.node_text(&name) == "as_view") =>
            {
                Reason::ClassView
            }
            "call" => Reason::WrapperCall,
            _ => Reason::CallbackUnproven,
        },
        _ => Reason::CallbackUnproven,
    };
    callback(position, CallbackStatus::Unresolved { text, reason })
}

pub(crate) fn has_opaque_decorator<'a>(
    member: PythonMember<'a>,
    above: Option<Node<'a>>,
    context: &SetupContext<'a>,
    mut transparent: impl FnMut(PythonUse<'a, '_>) -> bool,
) -> bool {
    member
        .node
        .parent()
        .filter(|parent| parent.kind() == "decorated_definition")
        .is_some_and(|parent| {
            children(parent).into_iter().any(|node| {
                node.kind() == "decorator"
                    && above.is_none_or(|above| node.start_byte() > above.start_byte())
                    && !transparent(PythonUse {
                        file: member.file,
                        node,
                        context,
                    })
            })
        })
}

pub(crate) fn python_decorated_callback(
    file: &ParsedFile,
    def: Node<'_>,
    registration: Node<'_>,
) -> Callback {
    callback(
        0,
        CallbackStatus::Resolved {
            identity: identity(file, def),
            argument_text: file
                .node_text(&def.child_by_field_name("name").unwrap_or(registration))
                .to_owned(),
        },
    )
}

fn callback(position: usize, status: CallbackStatus) -> Callback {
    Callback {
        position,
        status,
        delegate: None,
        role: None,
    }
}

fn identity(file: &ParsedFile, def: Node<'_>) -> CallableId {
    let mut names = Vec::new();
    let mut ancestor = Some(def);
    while let Some(node) = ancestor {
        if matches!(node.kind(), "function_definition" | "class_definition") {
            if node.kind() == "function_definition" && node != def {
                names.push("<locals>");
            }
            names
                .push(file.node_text(&node.child_by_field_name("name").expect("named definition")));
        }
        ancestor = node.parent();
    }
    names.reverse();
    let start = def
        .parent()
        .filter(|parent| parent.kind() == "decorated_definition")
        .and_then(|parent| children(parent).into_iter().next())
        .unwrap_or(def);
    CallableId {
        file: file.path.clone(),
        name: names.join("."),
        start_line: start.start_position().row + 1,
        end_line: def.end_position().row + 1,
        start_byte: start.start_byte(),
        end_byte: def.end_byte(),
    }
}
