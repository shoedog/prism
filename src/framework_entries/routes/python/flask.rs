//! Flask declarations and local setup facts. Native activation is a consumer of these facts.
use super::{arguments, attribute, children, literal_methods, literal_string, site, PythonArgs};
use crate::ast::ParsedFile;
use crate::framework_entries::routes::{
    handler::{python_callback, python_decorated_callback, PythonUse},
    receivers::{
        python_scope, PythonBindings, PythonKind, PythonMember, PythonValue, SetupContext,
    },
    Callback, CallbackStatus, Declaration, DeclaredPath, Diagnostic, Framework, MethodSet,
    MountPrefix, PathToken, Protocol, Reason, Receiver, Route, Site,
};
use tree_sitter::Node;

#[derive(Clone, Default)]
pub(crate) struct FlaskViewMetadata {
    pub methods: Option<MethodSet>,
    pub required: Option<MethodSet>,
    pub automatic: Option<Result<bool, String>>,
    pub app_automatic: Option<Result<bool, String>>,
}

pub(crate) fn flask_methods(
    file: &ParsedFile,
    args: &PythonArgs<'_>,
    metadata: &FlaskViewMetadata,
    shortcut: Option<&str>,
) -> (Option<MethodSet>, Option<MethodSet>, MethodSet) {
    let keyword = |key| args.keyword(file, key);
    let written = shortcut
        .map(|verb| MethodSet::known([verb.to_uppercase()]))
        .or_else(|| keyword("methods").map(|node| literal_methods(file, node)));
    let default = (written.is_none() && metadata.methods.is_none() && !args.unpacked())
        .then(|| MethodSet::known(["GET"]));
    if args.unpacked() {
        return (written, default, MethodSet::unknown("unpacked arguments"));
    }
    let mut effective = written
        .clone()
        .or_else(|| metadata.methods.clone())
        .or_else(|| default.clone())
        .unwrap_or_else(|| MethodSet::unknown("unpacked arguments"));
    if let MethodSet::Known { known } = &mut effective {
        if let Some(required) = &metadata.required {
            match required {
                MethodSet::Known { known: required } => known.extend(required.iter().cloned()),
                other => return (written, default, other.clone()),
            }
        }
        let policy = keyword("provide_automatic_options")
            .and_then(|node| flag(file, node))
            .or_else(|| metadata.automatic.clone());
        if !known.contains("OPTIONS") && policy.is_none() {
            match &metadata.app_automatic {
                Some(Ok(false)) => {}
                Some(Err(text)) => return (written, default, MethodSet::unknown(text)),
                _ => {
                    known.insert("OPTIONS".into());
                }
            }
        }
        if let Some(Err(text)) = policy {
            if !known.contains("OPTIONS") {
                return (written, default, MethodSet::unknown(text));
            }
        }
        if known.contains("GET") {
            known.insert("HEAD".into());
        }
    }
    (written, default, effective)
}

fn flag(file: &ParsedFile, node: Node<'_>) -> Option<Result<bool, String>> {
    match node.kind() {
        "true" => Some(Ok(true)),
        "false" => Some(Ok(false)),
        "none" => None,
        _ => Some(Err(file.node_text(&node).into())),
    }
}

pub(crate) struct FlaskRule<'a> {
    pub route: Route,
    pub origin: Node<'a>,
    pub effective_methods: MethodSet,
    pub effective_endpoint: Option<String>,
    pub view_metadata: FlaskViewMetadata,
    pub shortcut: Option<String>,
    pub options: PythonArgs<'a>,
    pub record_once: bool,
}

pub(crate) struct FlaskMount<'a> {
    pub site: Site,
    pub origin: Node<'a>,
    pub parent: Receiver,
    pub child: Receiver,
    pub prefix: MountPrefix,
    pub options: PythonArgs<'a>,
}

pub(crate) enum FlaskEvent<'a> {
    Rule(Box<FlaskRule<'a>>),
    Mount(FlaskMount<'a>),
    ViewWrite {
        site: Site,
        origin: Node<'a>,
        receiver: Receiver,
        endpoint: Option<String>,
        callback: Callback,
    },
    Config {
        site: Site,
        origin: Node<'a>,
        receiver: Receiver,
        key: Option<String>,
        value: Node<'a>,
    },
    Diagnostic(Diagnostic),
}

fn receiver(value: PythonValue<'_>) -> Receiver {
    match value {
        PythonValue::Router(router) => Receiver::Router {
            identity: router.identity,
            proof: None,
        },
        PythonValue::Unknown(reason) => Receiver::Unproven { reason },
        _ => Receiver::Unproven {
            reason: Reason::ReceiverUnproven,
        },
    }
}

fn call_method<'a, 'source>(
    file: &'source ParsedFile,
    call: Node<'a>,
) -> Option<(Node<'a>, &'source str)> {
    let (object, name) = attribute(call.child_by_field_name("function")?)?;
    Some((object, file.node_text(&name)))
}

fn route_method(name: &str) -> bool {
    matches!(name, "route" | "get" | "post" | "put" | "patch" | "delete")
}

pub(crate) fn flask_facts<'a>(
    bindings: &PythonBindings<'a>,
    path: &str,
    context: &SetupContext<'a>,
) -> Vec<FlaskEvent<'a>> {
    let Some(&file) = bindings.files.get(path) else {
        return vec![];
    };
    let active = |scope| {
        context
            .factory
            .is_none_or(|factory| factory.file.path == path && factory.node == scope)
    };
    let value = |node| bindings.value_at(path, node, context);
    let mut facts = Vec::new();
    // Visit decorator groups in application order, and all other sites in source order.
    let sites = bindings
        .nodes(path)
        .iter()
        .copied()
        .flat_map(|node| match node.kind() {
            "decorated_definition" => children(node)
                .into_iter()
                .rev()
                .filter(|n| n.kind() == "decorator")
                .collect(),
            "assignment" => vec![node],
            "call"
                if node
                    .parent()
                    .is_some_and(|p| p.kind() == "expression_statement") =>
            {
                vec![node]
            }
            _ => vec![],
        });
    for origin in sites {
        let scope = python_scope(origin);
        if origin.kind() == "assignment" {
            if active(scope) {
                facts.extend(mapping_write(bindings, file, origin, context));
            }
            continue;
        }
        let decorator = origin.kind() == "decorator";
        let Some(call) = (if decorator {
            origin.named_child(0)
        } else {
            Some(origin)
        }) else {
            continue;
        };
        let Some((object, method)) = call_method(file, call) else {
            if active(scope) && !decorator && saved_decorator(bindings, file, call, context) {
                facts.push(FlaskEvent::Diagnostic(Diagnostic {
                    site: site(file, origin),
                    reason: Reason::UnsupportedRegistration,
                }));
            }
            continue;
        };
        let deferred = if method == "add_url_rule" {
            record_once_owner(bindings, file, scope, object)
                .filter(|owner| active(python_scope(*owner)))
        } else {
            None
        };
        if !active(scope) && deferred.is_none() {
            continue;
        }
        let owner = deferred.map(value).filter(|value| {
            matches!(value, PythonValue::Router(router) if router.kind == PythonKind::Blueprint)
        });
        let record_once = owner.is_some();
        let parent = receiver(owner.unwrap_or_else(|| value(object)));
        if matches!(parent, Receiver::Unproven { .. })
            && !bindings.router_hint(file, object, context)
        {
            continue;
        }
        let options = arguments(call);
        if !decorator && method == "register_blueprint" {
            let child = match options.get(file, "blueprint", 0).map(value) {
                Some(PythonValue::Router(router)) if router.kind == PythonKind::Blueprint => {
                    receiver(PythonValue::Router(router))
                }
                _ => Receiver::Unproven {
                    reason: Reason::MountChildUnproven,
                },
            };
            let prefix = prefix(file, &options);
            facts.push(FlaskEvent::Mount(FlaskMount {
                site: site(file, origin),
                origin,
                parent,
                child,
                prefix,
                options,
            }));
        } else {
            let def = origin
                .parent()
                .and_then(|p| p.child_by_field_name("definition"))
                .filter(|def| def.kind() == "function_definition");
            if (decorator && def.is_some() && (route_method(method) || method == "endpoint"))
                || (!decorator && method == "add_url_rule")
            {
                facts.extend(declaration(
                    bindings,
                    PythonUse {
                        file,
                        node: origin,
                        context,
                    },
                    parent,
                    method,
                    options,
                    def,
                    record_once,
                ));
            }
        }
    }
    facts
}

fn prefix(file: &ParsedFile, options: &PythonArgs<'_>) -> MountPrefix {
    match options
        .keyword(file, "url_prefix")
        .filter(|node| node.kind() != "none")
    {
        Some(node) => literal_string(file, node).map_or_else(
            || MountPrefix::Unevaluated {
                text: file.node_text(&node).into(),
            },
            |value| MountPrefix::Literal { value },
        ),
        None => MountPrefix::Absent,
    }
}

fn saved_decorator<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    call: Node<'a>,
    context: &SetupContext<'a>,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let PythonValue::Opaque(saved) = bindings.value_at(&file.path, function, context) else {
        return false;
    };
    call_method(saved.file, saved.node).is_some_and(|(receiver, method)| {
        route_method(method)
            && matches!(
                bindings.value_at(&saved.file.path, receiver, context),
                PythonValue::Router(_)
            )
    })
}

fn record_once_owner<'a>(
    bindings: &PythonBindings<'a>,
    file: &ParsedFile,
    scope: Node<'a>,
    object: Node<'a>,
) -> Option<Node<'a>> {
    let wrapper = scope
        .parent()
        .filter(|p| p.kind() == "decorated_definition")?;
    let parameters = children(scope.child_by_field_name("parameters")?);
    let [parameter] = parameters.as_slice() else {
        return None;
    };
    let name = file.node_text(&object);
    if parameter.kind() != "identifier"
        || file.node_text(parameter) != name
        || bindings
            .reaching(file, scope, name, object.start_byte())
            .is_some()
    {
        return None;
    }
    let decorators: Vec<_> = children(wrapper)
        .into_iter()
        .filter(|n| n.kind() == "decorator")
        .collect();
    let [decorator] = decorators.as_slice() else {
        return None;
    };
    let (owner, name) = attribute(decorator.named_child(0)?)?;
    (file.node_text(&name) == "record_once").then_some(owner)
}

fn mapping_write<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    node: Node<'a>,
    context: &SetupContext<'a>,
) -> Option<FlaskEvent<'a>> {
    let left = node.child_by_field_name("left")?;
    let (owner, field) = attribute(left.child_by_field_name("value")?)?;
    let value = node.child_by_field_name("right")?;
    let receiver = receiver(bindings.value_at(&file.path, owner, context));
    if !matches!(receiver, Receiver::Router { .. }) {
        return None;
    }
    let key = left
        .child_by_field_name("subscript")
        .and_then(|key| literal_string(file, key));
    let site = site(file, node);
    match file.node_text(&field) {
        "config" => Some(FlaskEvent::Config {
            site,
            origin: node,
            receiver,
            key,
            value,
        }),
        "view_functions" => Some(FlaskEvent::ViewWrite {
            site,
            origin: node,
            receiver,
            endpoint: key,
            callback: python_callback(
                bindings,
                PythonUse {
                    file,
                    node: value,
                    context,
                },
                0,
            ),
        }),
        _ => None,
    }
}

fn declaration<'a>(
    bindings: &PythonBindings<'a>,
    use_site: PythonUse<'a, '_>,
    parent: Receiver,
    method: &str,
    options: PythonArgs<'a>,
    def: Option<Node<'a>>,
    record_once: bool,
) -> Option<FlaskEvent<'a>> {
    let PythonUse {
        file,
        node: origin,
        context,
    } = use_site;
    let build_only = options
        .keyword(file, "build_only")
        .is_some_and(|n| n.kind() == "true");
    let endpoint_arg = match method {
        "add_url_rule" => options.get(file, "endpoint", 1),
        "endpoint" => options.get(file, "endpoint", 0),
        _ => options.keyword(file, "endpoint"),
    }
    .filter(|arg| arg.kind() != "none");
    let endpoint = endpoint_arg.and_then(|node| literal_string(file, node));
    let argument = (method == "add_url_rule")
        .then(|| options.get(file, "view_func", 2))
        .flatten()
        .filter(|node| node.kind() != "none");
    let inner = |def: Node<'a>| {
        children(def.parent().unwrap())
            .into_iter()
            .filter(|child| child.kind() == "decorator" && child.start_byte() > origin.start_byte())
            .any(|decorator| {
                decorator
                    .named_child(0)
                    .and_then(|call| call_method(file, call))
                    .is_none_or(|(object, method)| {
                        (!route_method(method) && method != "endpoint")
                            || !matches!(
                                bindings.value_at(&file.path, object, context),
                                PythonValue::Router(_)
                            )
                    })
            })
    };
    let mut callbacks: Vec<_> = def
        .map(|def| python_decorated_callback(file, def, origin))
        .or_else(|| {
            argument.map(|node| {
                python_callback(
                    bindings,
                    PythonUse {
                        file,
                        node,
                        context,
                    },
                    0,
                )
            })
        })
        .into_iter()
        .collect();
    for callback in &mut callbacks {
        let refusal = match parent {
            Receiver::Unproven { reason } => Some(reason),
            _ if def.is_some_and(inner) => Some(Reason::InnerDecorator),
            _ => None,
        };
        if let Some(reason) = refusal {
            let text = match &callback.status {
                CallbackStatus::Resolved { argument_text, .. }
                | CallbackStatus::Anonymous { argument_text, .. } => argument_text.clone(),
                CallbackStatus::Unresolved { text, .. } => text.clone(),
            };
            callback.status = CallbackStatus::Unresolved { text, reason };
        }
    }
    let site = site(file, origin);
    if method == "endpoint" {
        return Some(FlaskEvent::ViewWrite {
            site,
            origin,
            receiver: parent,
            endpoint,
            callback: callbacks.pop()?,
        });
    }
    let path = options.get(file, "rule", 0)?;
    let loop_generated = std::iter::successors(origin.parent(), |node| node.parent())
        .take_while(|node| *node != python_scope(origin))
        .any(|node| matches!(node.kind(), "for_statement" | "while_statement"));
    if loop_generated {
        return Some(FlaskEvent::Diagnostic(Diagnostic {
            site,
            reason: Reason::RuntimeGeneratedRegistration,
        }));
    }
    let (native_name, metadata) = view_metadata(
        bindings,
        file,
        context,
        origin,
        argument,
        def,
        callbacks.first(),
    );
    let shortcut = (route_method(method) && method != "route").then_some(method);
    let (methods, default_methods, effective_methods) =
        flask_methods(file, &options, &metadata, shortcut);
    let effective_endpoint = if endpoint_arg.is_some() {
        endpoint.clone()
    } else {
        native_name
    };
    let candidate = matches!(parent, Receiver::Unproven { .. });
    let reason = match parent {
        Receiver::Unproven { reason } => Some(reason),
        _ if build_only => Some(Reason::BuildOnly),
        _ => None,
    };
    let route = Route {
        framework: Framework::Flask,
        protocol: Protocol::Http,
        site,
        receiver: parent,
        element_site: None,
        install_site: None,
        element_ordinal: None,
        origin_plugin: None,
        declaration: Some(Declaration {
            methods,
            path: DeclaredPath {
                raw: file.node_text(&path).into(),
                literal: literal_string(file, path),
            },
        }),
        default_methods,
        endpoint,
        candidate: candidate.then_some(true),
        reason,
        superseded_keys: vec![],
        conflicts: vec![],
        paths: vec![],
        callbacks,
    };
    Some(FlaskEvent::Rule(Box::new(FlaskRule {
        route,
        origin,
        effective_methods,
        effective_endpoint,
        view_metadata: metadata,
        shortcut: shortcut.map(str::to_owned),
        options,
        record_once,
    })))
}

fn view_metadata<'a>(
    bindings: &PythonBindings<'a>,
    file: &'a ParsedFile,
    context: &SetupContext<'a>,
    origin: Node<'a>,
    argument: Option<Node<'a>>,
    def: Option<Node<'a>>,
    callback: Option<&Callback>,
) -> (Option<String>, FlaskViewMetadata) {
    let mut metadata = FlaskViewMetadata::default();
    let member = argument
        .and_then(|node| match bindings.value_at(&file.path, node, context) {
            PythonValue::Definition(member) => Some(member),
            _ => None,
        })
        .or_else(|| def.map(|node| PythonMember { file, node }));
    if let Some(Callback {
        status: CallbackStatus::Resolved { identity, .. },
        ..
    }) = callback
    {
        let name = identity.name.rsplit('.').next().unwrap_or(&identity.name);
        if let Some(member) = member {
            for &write in bindings.nodes(&member.file.path) {
                if write.start_byte() < member.node.end_byte()
                    || write.kind() != "assignment"
                    || (member.file.path == file.path && write.start_byte() >= origin.start_byte())
                {
                    continue;
                }
                let Some((object, field)) = write.child_by_field_name("left").and_then(attribute)
                else {
                    continue;
                };
                let PythonValue::Definition(target) =
                    bindings.value_at(&member.file.path, object, context)
                else {
                    continue;
                };
                if target.node != member.node {
                    continue;
                }
                let Some(value) = write.child_by_field_name("right") else {
                    continue;
                };
                let guarded = write.parent().and_then(|p| p.parent())
                    != python_scope(write)
                        .child_by_field_name("body")
                        .or(Some(python_scope(write)));
                let methods = || {
                    if guarded {
                        MethodSet::unknown(member.file.node_text(&value))
                    } else {
                        literal_methods(member.file, value)
                    }
                };
                match member.file.node_text(&field) {
                    "methods" => metadata.methods = Some(methods()),
                    "required_methods" => metadata.required = Some(methods()),
                    "provide_automatic_options" => {
                        metadata.automatic = if guarded {
                            Some(Err(member.file.node_text(&value).into()))
                        } else {
                            flag(member.file, value)
                        }
                    }
                    _ => {}
                }
            }
        }
        return (Some(name.into()), metadata);
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
    if native.is_some() {
        return (native, metadata);
    }
    if callback.is_some() {
        let text = argument.map_or("unproved view metadata", |node| file.node_text(&node));
        metadata.methods = Some(MethodSet::unknown(text));
        metadata.required = Some(MethodSet::unknown(text));
    }
    (None, metadata)
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
    literal_string(file, arguments(call).get(file, "name", 0)?)
}

pub(crate) fn flask_path_tokens(mut path: &str) -> Option<Vec<PathToken>> {
    let identifier = |name: &str| {
        let mut bytes = name.bytes();
        bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    };
    let mut tokens = Vec::new();
    while let Some((literal, rest)) = path.split_once('<') {
        let mut options = rest.match_indices('>');
        let parameter = options.find_map(|(end, _)| {
            let parameter = &rest[..end];
            let (converter, name) = parameter.rsplit_once(':').unwrap_or(("default", parameter));
            let converter = if let Some((converter, args)) = converter.split_once('(') {
                if !args.ends_with(')') || args.contains('\n') {
                    return None;
                }
                converter
            } else {
                converter
            };
            identifier(name).then_some((end, converter))
        })?;
        let (end, converter) = parameter;
        if !matches!(
            converter,
            "default" | "string" | "int" | "float" | "uuid" | "any"
        ) {
            return None;
        }
        if !literal.is_empty() {
            tokens.push(PathToken::Literal(literal.into()));
        }
        tokens.push(PathToken::Parameter);
        path = &rest[end + 1..];
    }
    if !path.is_empty() {
        tokens.push(PathToken::Literal(path.into()));
    }
    Some(tokens)
}
