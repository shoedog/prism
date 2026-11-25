use super::*;

pub(super) fn view_metadata<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    context: &SetupContext<'a>,
    origin: Node<'a>,
    argument: Option<Node<'a>>,
    def: Option<Node<'a>>,
    callback: Option<&Callback>,
) -> (EffectiveEndpoint, FlaskViewMetadata) {
    let mut metadata = FlaskViewMetadata::default();
    if let Some(Callback {
        status: CallbackStatus::Resolved { identity, .. },
        ..
    }) = callback
    {
        let name = identity.name.rsplit('.').next().unwrap_or(&identity.name);
        let member = def.map(|node| PythonMember { file, node }).or_else(|| {
            match bindings.value_at(&file.path, argument?, context) {
                PythonValue::Definition(member) => Some(member),
                _ => None,
            }
        });
        if let Some(member) = member {
            return function_metadata(
                bindings,
                member,
                PythonUse {
                    file,
                    node: origin,
                    context,
                },
                name,
            );
        }
    }
    let native = argument.and_then(|call| {
        let (class, "as_view") = call_method(file, call)? else {
            return None;
        };
        let PythonValue::Definition(class) = bindings.value_at(&file.path, class, context) else {
            return None;
        };
        native_view(bindings, class, file, call, context, &mut metadata)
    });
    if let Some(name) = native {
        return (EffectiveEndpoint::Known(name), metadata);
    }
    if callback.is_some() {
        let text = argument.map_or("unproved view metadata", |node| file.node_text(&node));
        metadata.methods = Some(MethodSet::unknown(text));
        metadata.required = Some(MethodSet::unknown(text));
    }
    let endpoint = if callback.is_some() {
        EffectiveEndpoint::Unknown
    } else {
        EffectiveEndpoint::Absent
    };
    (endpoint, metadata)
}

fn function_metadata<'a>(
    bindings: &PythonBindings<'a>,
    member: PythonMember<'a>,
    use_site: PythonUse<'a, '_>,
    name: &str,
) -> (EffectiveEndpoint, FlaskViewMetadata) {
    let PythonUse {
        file,
        node: origin,
        context,
    } = use_site;
    let mut endpoint = EffectiveEndpoint::Known(name.into());
    let mut metadata = FlaskViewMetadata::default();
    for write in bindings.view_metadata_writes(member, file, origin, context) {
        let (source, field, value, write) = (write.file, write.field, write.value, write.node);
        let text = source.node_text(&value.unwrap_or(write));
        let methods = || {
            value.map_or_else(
                || MethodSet::unknown(text),
                |node| literal_methods(source, node),
            )
        };
        match field {
            "methods" => {
                metadata.methods = if value.is_some_and(|node| ungroup(node).kind() == "none") {
                    None
                } else {
                    match methods() {
                        MethodSet::Known { known } if known.is_empty() => None,
                        other => Some(other),
                    }
                };
            }
            "required_methods" => metadata.required = Some(methods()),
            "provide_automatic_options" => {
                metadata.automatic =
                    value.map_or_else(|| Some(Err(text.into())), |node| flag(source, node));
            }
            "__name__" => {
                endpoint = value
                    .and_then(|node| literal_string(source, node))
                    .map_or(EffectiveEndpoint::Unknown, EffectiveEndpoint::Known);
            }
            _ => unreachable!("prefiltered view metadata field"),
        }
    }
    (endpoint, metadata)
}

fn native_view<'a>(
    bindings: &PythonBindings<'a>,
    class: PythonMember<'a>,
    file: &ParsedFile,
    call: Node<'a>,
    context: &SetupContext<'a>,
    metadata: &mut FlaskViewMetadata,
) -> Option<String> {
    if class.node.kind() != "class_definition"
        || class.node.parent()?.kind() == "decorated_definition"
    {
        return None;
    }
    let bases = children(class.node.child_by_field_name("superclasses")?);
    let [base] = bases.as_slice() else {
        return None;
    };
    let PythonValue::Constructor(kind @ (PythonKind::View | PythonKind::MethodView)) =
        bindings.value_at(&class.file.path, *base, context)
    else {
        return None;
    };
    let body = children(class.node.child_by_field_name("body")?);
    let mut handlers = Vec::new();
    for node in body {
        if node.kind() == "function_definition" {
            let name = class.file.node_text(&node.child_by_field_name("name")?);
            if name == "as_view" {
                return None;
            }
            if kind == PythonKind::MethodView
                && matches!(
                    name,
                    "get" | "head" | "post" | "options" | "delete" | "put" | "trace" | "patch"
                )
            {
                handlers.push(name.to_uppercase());
            }
        } else if node.kind() != "pass_statement" {
            return None;
        }
    }
    if !handlers.is_empty() {
        metadata.methods = Some(MethodSet::known(handlers));
    }
    literal_string(file, arguments(call).get(file, "name", 0).written()?)
}
