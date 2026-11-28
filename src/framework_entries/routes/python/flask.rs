//! Flask declarations and local setup facts. Native activation is a consumer of these facts.
use super::{
    arguments, attribute, children, literal_methods, literal_string, site, ungroup, PythonArg,
    PythonArgs,
};
use crate::ast::ParsedFile;
use crate::framework_entries::routes::{
    handler::{has_opaque_decorator, python_callback, python_decorated_callback, PythonUse},
    receivers::{
        python_scope, PythonBindings, PythonKind, PythonMember, PythonValue, SetupContext,
    },
    Callback, CallbackStatus, Declaration, DeclaredPath, Diagnostic, Framework, MethodSet,
    MountPrefix, PathToken, Protocol, Reason, Receiver, Route, Site,
};
use tree_sitter::Node;

mod activation;
mod metadata;
pub(in crate::framework_entries::routes) mod registration;
use activation::{first_iterable_call, setup_invocations};
use metadata::view_metadata;

const ADD_URL_RULE: &[&str] = &["rule", "endpoint", "view_func", "provide_automatic_options"];
const SETUP_STATE_ADD_URL_RULE: &[&str] = &["rule", "endpoint", "view_func"];

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
    let written = shortcut
        .map(|verb| MethodSet::known([verb.to_uppercase()]))
        .or_else(|| match args.keyword(file, "methods").none_as_absent() {
            PythonArg::Written(node) => Some(literal_methods(file, node)),
            PythonArg::Absent => None,
            PythonArg::Unknown(node) => Some(MethodSet::unknown(file.node_text(&node))),
        });
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
        let policy = args
            .get(file, "provide_automatic_options", 3)
            .written()
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
    match ungroup(node).kind() {
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
    pub effective_endpoint: EffectiveEndpoint,
    pub build_only: Option<bool>,
    pub path_tokens: Option<Vec<PathToken>>,
    pub view_metadata: FlaskViewMetadata,
    pub shortcut: Option<String>,
    pub options: PythonArgs<'a>,
    pub record_once: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EffectiveEndpoint {
    Known(String),
    Unknown,
    Absent,
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
        endpoint: EffectiveEndpoint,
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
    let active = |scope: Node<'_>| {
        context.factory.map_or(scope.kind() == "module", |factory| {
            !context.frames.is_empty() && factory.file.path == path && factory.node == scope
        })
    };
    let invocations = if context.factory.is_none() {
        setup_invocations(bindings, file)
    } else {
        Default::default()
    };
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
                    .is_some_and(|p| p.kind() == "expression_statement")
                    || first_iterable_call(node) =>
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
        let invocation = (!decorator && method == "add_url_rule")
            .then(|| invocations.get(&scope.id()))
            .flatten();
        let context = invocation.unwrap_or(context);
        let value = |node| bindings.capture_value(path, node, context);
        let options = arguments(call);
        if !accepted_call(file, &options, method, decorator) {
            continue;
        }
        let deferred = if method == "add_url_rule" {
            record_once_owner(bindings, file, scope, object)
                .filter(|owner| active(python_scope(*owner)))
        } else {
            None
        };
        if !active(scope) && invocation.is_none() && deferred.is_none() {
            if context.factory.is_none() {
                // Source captures can be diagnosed without activating a lexical scope.
                value(object);
                capture_registration_argument(file, &options, method, value);
            }
            continue;
        }
        let owner = deferred.map(value).filter(|value| {
            matches!(value, PythonValue::Router(router) if router.kind == PythonKind::Blueprint)
        });
        let record_once = owner.is_some();
        if record_once && !options.accepts(file, SETUP_STATE_ADD_URL_RULE, true) {
            continue;
        }
        let owner = owner.unwrap_or_else(|| value(object));
        let default_converters =
            matches!(&owner, PythonValue::Router(router) if bindings.default_converters(router));
        let mut parent = receiver(owner);
        if !decorator && method == "register_blueprint" {
            let argument = options.get(file, "blueprint", 0).written();
            let child = match argument.map(value) {
                Some(PythonValue::Router(router)) if router.kind == PythonKind::Blueprint => {
                    receiver(PythonValue::Router(router))
                }
                _ => Receiver::Unproven {
                    reason: Reason::MountChildUnproven,
                },
            };
            let escaped = matches!(child, Receiver::Unproven { .. })
                && argument.is_some_and(|child| bindings.escaped_mount_child(file, child, context));
            if matches!(parent, Receiver::Unproven { .. })
                && !escaped
                && !bindings.router_hint(file, object, context)
            {
                continue;
            }
            let written_prefix = prefix(file, &options);
            let prefix = if escaped && written_prefix == MountPrefix::Absent {
                MountPrefix::Unevaluated {
                    text: "unproved blueprint mount".into(),
                }
            } else {
                written_prefix
            };
            facts.push(FlaskEvent::Mount(FlaskMount {
                site: site(file, origin),
                origin,
                parent,
                child,
                prefix,
                options,
            }));
        } else {
            let retained_source = match parent {
                Receiver::Unproven {
                    reason: Reason::ReceiverUnproven,
                } if method != "endpoint" => bindings.escaped_mount_child(file, object, context),
                Receiver::Unproven {
                    reason: Reason::ImportedUnproven,
                } if method != "endpoint"
                    && bindings.terminal_router_reexport(file, object, context) =>
                {
                    parent = Receiver::Unproven {
                        reason: Reason::TerminalReexport,
                    };
                    true
                }
                _ => false,
            };
            if matches!(parent, Receiver::Unproven { .. })
                && !retained_source
                && !bindings.router_hint(file, object, context)
            {
                capture_registration_argument(file, &options, method, value);
                continue;
            }
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
                    RegistrationReceiver {
                        parent,
                        record_once,
                        default_converters,
                        retained_source,
                    },
                    method,
                    options,
                    def,
                ));
            }
        }
    }
    facts
}

fn capture_registration_argument<'a>(
    file: &ParsedFile,
    options: &PythonArgs<'a>,
    method: &str,
    capture: impl FnOnce(Node<'a>) -> PythonValue<'a>,
) {
    let argument = match method {
        "register_blueprint" => options.get(file, "blueprint", 0),
        "add_url_rule" => options.get(file, "view_func", 2),
        _ => return,
    };
    if let Some(node) = argument.written() {
        capture(node);
    }
}

fn accepted_call(
    file: &ParsedFile,
    options: &PythonArgs<'_>,
    method: &str,
    decorator: bool,
) -> bool {
    match (decorator, method) {
        (false, "register_blueprint") => options.accepts(file, &["blueprint"], true),
        (false, "add_url_rule") => options.accepts(file, ADD_URL_RULE, true),
        (true, "endpoint") => options.accepts(file, &["endpoint"], false),
        (true, method) if route_method(method) => {
            options.accepts(file, &["rule"], true)
                && !matches!(options.keyword(file, "view_func"), PythonArg::Written(_))
                && (method == "route"
                    || !matches!(options.keyword(file, "methods"), PythonArg::Written(_)))
        }
        _ => false,
    }
}

fn transparent_decorator<'a>(bindings: &PythonBindings<'a>, use_site: PythonUse<'a, '_>) -> bool {
    let PythonUse {
        file,
        node,
        context,
    } = use_site;
    node.named_child(0).is_some_and(|call| {
        call_method(file, call).is_some_and(|(object, method)| {
            let args = arguments(call);
            let required = if method == "endpoint" {
                "endpoint"
            } else {
                "rule"
            };
            accepted_call(file, &args, method, true)
                && !args.unpacked()
                && args.get(file, required, 0).written().is_some()
                && matches!(
                    bindings.capture_value(&file.path, object, context),
                    PythonValue::Router(_)
                )
        })
    })
}

fn prefix(file: &ParsedFile, options: &PythonArgs<'_>) -> MountPrefix {
    match options.keyword(file, "url_prefix").none_as_absent() {
        PythonArg::Written(node) => literal_string(file, node).map_or_else(
            || MountPrefix::Unevaluated {
                text: file.node_text(&node).into(),
            },
            |value| MountPrefix::Literal { value },
        ),
        PythonArg::Unknown(node) => MountPrefix::Unevaluated {
            text: file.node_text(&node).into(),
        },
        PythonArg::Absent => MountPrefix::Absent,
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
    let field = file.node_text(&field);
    if !matches!(field, "config" | "view_functions") {
        return None;
    }
    let value = node.child_by_field_name("right")?;
    let receiver = receiver(bindings.value_at(&file.path, owner, context));
    if !matches!(receiver, Receiver::Router { .. }) {
        return None;
    }
    let key = left
        .child_by_field_name("subscript")
        .and_then(|key| literal_string(file, key));
    let site = site(file, node);
    match field {
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
            endpoint: key.map_or(EffectiveEndpoint::Unknown, EffectiveEndpoint::Known),
            callback: python_callback(
                bindings,
                PythonUse {
                    file,
                    node: value,
                    context,
                },
                0,
                |decorator| transparent_decorator(bindings, decorator),
            ),
        }),
        _ => None,
    }
}

struct RegistrationReceiver {
    parent: Receiver,
    record_once: bool,
    default_converters: bool,
    retained_source: bool,
}

fn declaration<'a>(
    bindings: &PythonBindings<'a>,
    use_site: PythonUse<'a, '_>,
    receiver: RegistrationReceiver,
    method: &str,
    options: PythonArgs<'a>,
    def: Option<Node<'a>>,
) -> Option<FlaskEvent<'a>> {
    let RegistrationReceiver {
        parent,
        record_once,
        default_converters,
        retained_source,
    } = receiver;
    let PythonUse {
        file,
        node: origin,
        context,
    } = use_site;
    let build_only = match options.keyword(file, "build_only") {
        PythonArg::Written(node) => flag(file, node).unwrap_or(Ok(false)).ok(),
        PythonArg::Absent => Some(false),
        PythonArg::Unknown(_) => None,
    };
    let endpoint_arg = match method {
        "add_url_rule" => options.get(file, "endpoint", 1),
        "endpoint" => options.get(file, "endpoint", 0),
        _ => options.keyword(file, "endpoint"),
    }
    .none_as_absent();
    let endpoint = endpoint_arg
        .written()
        .and_then(|node| literal_string(file, node));
    let argument = if method == "add_url_rule" {
        options.get(file, "view_func", 2).none_as_absent()
    } else {
        PythonArg::Absent
    };
    let mut callbacks: Vec<_> = def
        .map(|def| python_decorated_callback(file, def, origin))
        .or_else(|| match argument {
            PythonArg::Written(node) => Some(python_callback(
                bindings,
                PythonUse {
                    file,
                    node,
                    context,
                },
                0,
                |decorator| transparent_decorator(bindings, decorator),
            )),
            PythonArg::Unknown(node) => Some(Callback {
                position: 0,
                status: CallbackStatus::Unresolved {
                    text: file.node_text(&node).into(),
                    reason: Reason::SpreadCallback,
                },
                delegate: None,
                role: None,
            }),
            PythonArg::Absent => None,
        })
        .into_iter()
        .collect();
    for callback in &mut callbacks {
        let refusal = match parent {
            Receiver::Unproven { reason } => Some(reason),
            _ if def.is_some_and(|node| {
                has_opaque_decorator(
                    PythonMember { file, node },
                    Some(origin),
                    context,
                    |decorator| transparent_decorator(bindings, decorator),
                )
            }) =>
            {
                Some(Reason::InnerDecorator)
            }
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
            endpoint: endpoint.map_or(EffectiveEndpoint::Unknown, EffectiveEndpoint::Known),
            callback: callbacks.pop()?,
        });
    }
    let path = match options.get(file, "rule", 0) {
        PythonArg::Written(node) => DeclaredPath {
            raw: file.node_text(&node).into(),
            literal: literal_string(file, node),
        },
        PythonArg::Unknown(node) => DeclaredPath {
            raw: file.node_text(&node).into(),
            literal: None,
        },
        PythonArg::Absent => return None,
    };
    let path_tokens = path
        .literal
        .as_deref()
        .filter(|path| default_converters || !path.contains('<'))
        .and_then(flask_path_tokens);
    let loop_generated = std::iter::successors(origin.parent(), |node| node.parent())
        .take_while(|node| *node != python_scope(origin))
        .any(|node| matches!(node.kind(), "for_statement" | "while_statement"));
    if loop_generated {
        return Some(FlaskEvent::Diagnostic(Diagnostic {
            site,
            reason: Reason::RuntimeGeneratedRegistration,
        }));
    }
    let (native_name, mut metadata) = if retained_source {
        (
            EffectiveEndpoint::Unknown,
            FlaskViewMetadata::withheld("unproved registration receiver"),
        )
    } else {
        view_metadata(
            bindings,
            file,
            context,
            origin,
            argument.written(),
            def,
            callbacks.first(),
        )
    };
    let mut effective_endpoint = match endpoint_arg {
        PythonArg::Written(_) => endpoint
            .clone()
            .map_or(EffectiveEndpoint::Unknown, EffectiveEndpoint::Known),
        PythonArg::Unknown(_) => EffectiveEndpoint::Unknown,
        PythonArg::Absent => native_name,
    };
    if let Receiver::Router { identity, .. } = &parent {
        let key = match &effective_endpoint {
            EffectiveEndpoint::Known(key) => Some(key.as_str()),
            _ => None,
        };
        if bindings.view_map_written(identity, key) {
            metadata = FlaskViewMetadata::withheld("view_functions write");
            if matches!(endpoint_arg, PythonArg::Absent) {
                effective_endpoint = EffectiveEndpoint::Unknown;
            }
        }
        if bindings.config_replaced(identity) {
            metadata.app_automatic = Some(Err("unproved app.config".into()));
        }
    }
    let shortcut = (route_method(method) && method != "route").then_some(method);
    let (methods, default_methods, effective_methods) =
        flask_methods(file, &options, &metadata, shortcut);
    let candidate = matches!(parent, Receiver::Unproven { .. });
    let reason = match parent {
        Receiver::Unproven { reason } => Some(reason),
        _ if build_only == Some(true) => Some(Reason::BuildOnly),
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
        declaration: Some(Declaration { methods, path }),
        default_methods: default_methods.filter(|_| !candidate),
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
        build_only,
        path_tokens,
        view_metadata: metadata,
        shortcut: shortcut.map(str::to_owned),
        options,
        record_once,
    })))
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
        let end = rest.find('>')?;
        let (converter, name) = rest[..end]
            .rsplit_once(':')
            .unwrap_or(("default", &rest[..end]));
        if !identifier(name) {
            return None;
        }
        if !matches!(converter, "default" | "string" | "int" | "float" | "uuid") {
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
