use super::super::python::{
    arguments,
    flask::{
        flask_facts, flask_methods, flask_path_tokens, EffectiveEndpoint, FlaskEvent,
        FlaskViewMetadata,
    },
    literal_methods, literal_string,
};
use super::super::receivers::{index_python, PythonMember, SetupContext};
use super::*;
use crate::{ast::ParsedFile, languages::Language};
use tree_sitter::Node;

mod import_regressions;
mod measure;
mod review;
mod source_fact_gaps;
mod static_import;

fn parsed(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|&(path, source)| {
            (
                path.into(),
                ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect()
}

#[test]
fn function_local_lookalike_receiver_is_unproven() {
    let source = include_str!("../../../../tests/routes_matrix/fl21/app.py");
    let files = parsed(&[("app.py", source)]);
    let file = &files["app.py"];
    let context = SetupContext {
        factory: Some(PythonMember {
            file,
            node: file.tree.root_node().named_child(3).unwrap(),
        }),
        frames: vec![Invocation::Call {
            file: "app.py".into(),
            start_byte: source.rfind("configure_preview()").unwrap(),
            end_byte: source.trim_end().len(),
        }],
    };
    let facts = flask_facts(&index_python(&files), "app.py", &context);
    let [FlaskEvent::Rule(fact)] = facts.as_slice() else {
        panic!("expected one source declaration")
    };
    let route = &fact.route;
    assert!(matches!(route.receiver, Receiver::Unproven { .. }));
    assert_eq!(route.candidate, Some(true));
    assert!(route.paths.is_empty());
    assert!(matches!(
        route.callbacks[0].status,
        CallbackStatus::Unresolved {
            reason: Reason::ReceiverUnproven,
            ..
        }
    ));
}

#[test]
fn endpoint_only_has_no_registered_callback() {
    let source = include_str!("../../../../tests/routes_matrix/es2-07/app.py");
    let files = parsed(&[("app.py", source)]);
    let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule), FlaskEvent::ViewWrite { endpoint, .. }] = facts.as_slice() else {
        panic!("expected only the URL rule as a declaration and one view-map write")
    };
    assert_eq!(rule.route.endpoint.as_deref(), Some("stock-index"));
    assert_eq!(endpoint, &EffectiveEndpoint::Known("stock-index".into()));
    assert!(rule.route.callbacks.is_empty());
}

fn expression(file: &ParsedFile) -> Node<'_> {
    file.tree
        .root_node()
        .named_child(0)
        .unwrap()
        .named_child(0)
        .unwrap()
}

#[test]
fn python_literal_expression_preserves_spelling() {
    for (source, expected) in [
        (r#""/n\u006ftes""#, Some("/notes")),
        (r#""/items/" + "latest""#, Some("/items/latest")),
        (r#"r"/a\n" "/b""#, Some("/a\\n/b")),
        (r#"f"/{section}""#, None),
        ("prefix", None),
    ] {
        let file = ParsedFile::parse("app.py", source, Language::Python).unwrap();
        let node = expression(&file);
        assert_eq!(file.node_text(&node), source);
        assert_eq!(literal_string(&file, node).as_deref(), expected, "{source}");
    }
}
#[test]
fn arguments_use_the_written_slot_and_preserve_literals() {
    let files = parsed(&[("app.py", "f('/first', endpoint='index', methods=('post', 'GET'))\nf(rule='/second', methods=['get', dynamic])\nf(*values)\n")]);
    let file = &files["app.py"];
    let root = file.tree.root_node();
    let call = |index| root.named_child(index).unwrap().named_child(0).unwrap();
    for (index, path) in ["/first", "/second"].into_iter().enumerate() {
        let args = arguments(call(index));
        assert_eq!(
            literal_string(file, args.get(file, "rule", 0).written().unwrap()).as_deref(),
            Some(path)
        );
        let methods = args.keyword(file, "methods").written().unwrap();
        assert_eq!(
            literal_methods(file, methods),
            if index == 0 {
                MethodSet::known(["POST", "GET"])
            } else {
                MethodSet::unknown("['get', dynamic]")
            }
        );
    }
    assert!(matches!(
        arguments(call(2)).get(file, "rule", 0),
        super::super::python::PythonArg::Unknown(_)
    ));
}

#[test]
fn imports_select_the_requested_member_and_refuse_forwarding() {
    let files = parsed(&[
        ("service/auth.py", "from flask import Blueprint as BP\nbp = BP('auth', __name__)\n"),
        ("service/__init__.py", "from .auth import bp\n"),
        ("service/app.py", "from . import auth\nfrom . import bp\nfrom flask import Flask\napp = Flask(__name__)\napp.register_blueprint(auth.bp, url_prefix='/auth')\napp.register_blueprint(bp)\nfrom other import *\napp.register_blueprint(auth.bp)\nfrom flask import Flask\napp = Flask(__name__)\napp.register_blueprint(auth.bp)\n"),
        ("test_routes.py", "from service.auth import bp\n@bp.get('/test')\ndef sample(): pass\n"),
    ]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "service/app.py", &SetupContext::default());
    let [FlaskEvent::Mount(first), FlaskEvent::Mount(forwarded), FlaskEvent::Mount(shadowed), FlaskEvent::Mount(starred)] =
        facts.as_slice()
    else {
        panic!("expected four mount sites")
    };
    assert!(
        matches!(&first.child, Receiver::Router { identity, .. } if identity.file == "service/auth.py")
    );
    assert_eq!(
        first.prefix,
        MountPrefix::Literal {
            value: "/auth".into()
        }
    );
    assert!(matches!(forwarded.child, Receiver::Unproven { .. }));
    assert!(matches!(shadowed.parent, Receiver::Unproven { .. }));
    assert!(matches!(starred.parent, Receiver::Router { .. }));
    assert!(matches!(starred.child, Receiver::Unproven { .. }));
    let test = flask_facts(&bindings, "test_routes.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = test.as_slice() else {
        panic!("expected an imported blueprint registration")
    };
    assert!(
        matches!(&rule.route.receiver, Receiver::Router { identity, .. } if identity.file == "service/auth.py")
    );
}

#[test]
fn factory_selector_seeds_entry_and_nested_frame() {
    let source = "from flask import Flask, Blueprint\ndef alloc():\n    app = Flask(__name__)\n    return app\ndef create():\n    return alloc()\ndef ambiguous():\n    if settings: return alloc()\n    return alloc()\nblueprint = Blueprint('x', __name__)\n";
    let files = parsed(&[("app.py", source)]);
    let bindings = index_python(&files);
    assert!(bindings.roots(Framework::Flask, None).is_empty());
    let selector = RootSelector::parse("app:create()").unwrap();
    let roots = bindings.roots(Framework::Flask, Some(&selector));
    let root = selector.resolve(&roots).unwrap();
    let call = source.find("return alloc()").unwrap() + "return ".len();
    assert_eq!(root.construction_byte, source.find("Flask(").unwrap());
    assert_eq!(
        root.invocations,
        [
            Invocation::Entry {
                entry: "app:create()".into()
            },
            Invocation::Call {
                file: "app.py".into(),
                start_byte: call,
                end_byte: call + "alloc()".len()
            }
        ]
    );
    for entry in ["app:ambiguous()", "app:blueprint"] {
        let selector = RootSelector::parse(entry).unwrap();
        assert!(selector
            .resolve(&bindings.roots(Framework::Flask, Some(&selector)))
            .is_err());
    }
}

#[test]
fn methods_use_metadata_and_keep_declared_operations_separate() {
    let files = parsed(&[("app.py", "f('/')\nf('/', methods=['GET'], provide_automatic_options=False)\nf('/', methods=['OPTIONS'], provide_automatic_options=False)\nf('/', methods=[])\n")]);
    let file = &files["app.py"];
    let methods = |i, metadata: &FlaskViewMetadata| {
        flask_methods(
            file,
            &arguments(
                file.tree
                    .root_node()
                    .named_child(i)
                    .unwrap()
                    .named_child(0)
                    .unwrap(),
            ),
            metadata,
            None,
        )
    };
    assert_eq!(
        methods(0, &FlaskViewMetadata::default()),
        (
            None,
            Some(MethodSet::known(["GET"])),
            MethodSet::known(["GET", "HEAD", "OPTIONS"])
        )
    );
    let view = FlaskViewMetadata {
        required: Some(MethodSet::known(["POST"])),
        ..Default::default()
    };
    assert_eq!(
        methods(1, &view),
        (
            Some(MethodSet::known(["GET"])),
            None,
            MethodSet::known(["GET", "HEAD", "POST"])
        )
    );
    assert_eq!(methods(2, &view).2, MethodSet::known(["OPTIONS", "POST"]));
    assert_eq!(
        methods(3, &view).0,
        Some(MethodSet::known(Vec::<String>::new()))
    );
    assert!(matches!(
        methods(
            0,
            &FlaskViewMetadata {
                methods: Some(MethodSet::unknown("dynamic")),
                ..Default::default()
            }
        )
        .2,
        MethodSet::Unknown { .. }
    ));
    assert_eq!(
        flask_path_tokens("/literal/:id/{draft}/<int:id>"),
        Some(vec![
            PathToken::Literal("/literal/:id/{draft}/".into()),
            PathToken::Parameter
        ])
    );
    assert_eq!(flask_path_tokens("/files/<path:rest>"), None);
    assert_eq!(flask_path_tokens("/files/<custom:id>"), None);
}

#[test]
fn decorators_and_callback_arguments_keep_their_own_identity() {
    let files = parsed(&[("app.py", "from flask import Flask\napp = Flask(__name__)\n@replace\n@app.get('/outer')\n@app.get('/stack')\ndef outer(): pass\n@app.get('/inner')\n@replace\ndef inner(): pass\ndef view(): pass\nalias = view\napp.add_url_rule('/alias', view_func=alias)\napp.add_url_rule('/lambda', view_func=lambda: None)\napp.add_url_rule('/member', view_func=obj.view)\napp.add_url_rule('/call', view_func=wrap(view))\n")]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let rules: Vec<_> = facts
        .iter()
        .filter_map(|fact| match fact {
            FlaskEvent::Rule(rule) => Some(rule),
            _ => None,
        })
        .collect();
    assert_eq!(rules.len(), 7);
    for rule in &rules[..2] {
        assert!(
            matches!(&rule.route.callbacks[0].status, CallbackStatus::Resolved { identity, .. } if identity.name == "outer" && identity.start_line == 3 && identity.end_line == 6)
        );
    }
    assert_eq!(
        rules[0]
            .route
            .declaration
            .as_ref()
            .unwrap()
            .path
            .literal
            .as_deref(),
        Some("/stack")
    );
    assert!(
        matches!(&rules[3].route.callbacks[0].status, CallbackStatus::Resolved { identity, argument_text } if identity.name == "view" && argument_text == "alias")
    );
    for (rule, expected) in rules.iter().zip([
        None,
        None,
        Some(Reason::InnerDecorator),
        None,
        Some(Reason::AnonymousUnindexed),
        Some(Reason::MemberHandler),
        Some(Reason::WrapperCall),
    ]) {
        if let Some(expected) = expected {
            assert!(
                matches!(&rule.route.callbacks[0].status, CallbackStatus::Unresolved { reason, .. } if *reason == expected)
            );
        }
    }
}

#[test]
fn local_facts_cover_view_metadata_deferred_rules_and_loss_diagnostics() {
    for (source, expected) in [
        (
            include_str!("../../../../tests/routes_matrix/cx05/app.py"),
            "class",
        ),
        (
            include_str!("../../../../tests/routes_matrix/es2-08/app.py"),
            "metadata",
        ),
        (
            include_str!("../../../../tests/routes_matrix/es4-04/app.py"),
            "record_once",
        ),
        (
            include_str!("../../../../tests/routes_matrix/es7-07/app.py"),
            "build_only",
        ),
        (
            include_str!("../../../../tests/routes_matrix/es1-20/app.py"),
            "generated",
        ),
        (
            include_str!("../../../../tests/routes_matrix/es3-07/app.py"),
            "saved",
        ),
    ] {
        let files = parsed(&[("app.py", source)]);
        let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
        match (expected, facts.as_slice()) {
            ("class", [FlaskEvent::Rule(rule)]) => {
                assert_eq!(rule.route.endpoint, None);
                assert_eq!(
                    rule.effective_endpoint,
                    EffectiveEndpoint::Known("preferences".into())
                );
                assert_eq!(
                    rule.effective_methods,
                    MethodSet::known(["GET", "HEAD", "OPTIONS", "PATCH"])
                );
                assert!(matches!(
                    rule.route.callbacks[0].status,
                    CallbackStatus::Unresolved {
                        reason: Reason::ClassView,
                        ..
                    }
                ));
            }
            ("metadata", [FlaskEvent::Rule(rule)]) => {
                assert_eq!(
                    rule.effective_methods,
                    MethodSet::known(["GET", "HEAD", "POST"])
                );
                assert_eq!(
                    rule.route.declaration.as_ref().unwrap().methods,
                    Some(MethodSet::known(["GET"]))
                );
            }
            (
                "record_once",
                [FlaskEvent::Rule(first), FlaskEvent::Rule(ordinary), FlaskEvent::Mount(_), FlaskEvent::Mount(_)],
            ) => {
                assert!(first.record_once && !ordinary.record_once);
                assert_eq!(first.route.receiver, ordinary.route.receiver);
            }
            ("build_only", [FlaskEvent::Rule(rule), FlaskEvent::Rule(ordinary)]) => {
                assert_eq!(rule.route.reason, Some(Reason::BuildOnly));
                assert!(rule.route.paths.is_empty());
                assert_eq!(ordinary.route.reason, None);
            }
            ("generated", [FlaskEvent::Diagnostic(diag)]) => {
                assert_eq!(diag.reason, Reason::RuntimeGeneratedRegistration)
            }
            ("saved", [FlaskEvent::Diagnostic(diag)]) => {
                assert_eq!(diag.reason, Reason::UnsupportedRegistration);
                assert_eq!(
                    &source[diag.site.start_byte..diag.site.end_byte],
                    "saved(inventory)"
                );
            }
            _ => panic!("unexpected facts for {expected}"),
        }
    }
}

#[test]
fn factory_sites_keep_invocation_identity_and_do_not_merge_instances() {
    let source = "from flask import Flask\ndef make():\n    app = Flask(__name__)\n    @app.get('/')\n    def index(): pass\n    return app\nfirst = make()\nsecond = make()\n";
    let files = parsed(&[("app.py", source)]);
    let bindings = index_python(&files);
    let file = &files["app.py"];
    let factory = file.tree.root_node().named_child(1).unwrap();
    let roots = bindings.roots(Framework::Flask, None);
    let first = RootSelector::parse("app:first").unwrap();
    let second = RootSelector::parse("app:second").unwrap();
    let (first, second) = (
        first.resolve(&roots).unwrap(),
        second.resolve(&roots).unwrap(),
    );
    assert_ne!(first.invocations, second.invocations);
    assert_eq!(first.construction_byte, second.construction_byte);
    assert_eq!(
        bindings
            .factory_return("app.py", factory, &first.invocations)
            .as_ref(),
        Some(first)
    );
    assert!(bindings.factory_return("app.py", factory, &[]).is_none());
    let source_facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    assert!(source_facts.is_empty());
    let context = SetupContext {
        factory: Some(PythonMember {
            file,
            node: factory,
        }),
        frames: second.invocations.clone(),
    };
    let selected = flask_facts(&bindings, "app.py", &context);
    let [FlaskEvent::Rule(selected)] = selected.as_slice() else {
        panic!("expected a selected factory fact");
    };
    assert_eq!(
        selected.route.site.start_byte,
        source.find("@app.get").unwrap()
    );
    assert!(
        matches!(&selected.route.receiver, Receiver::Router { identity, .. } if identity == second)
    );
}

#[test]
fn aliases_refuse_rebind_and_escape_but_preserve_router_lifetimes() {
    for (between, expected) in [
        ("", true),
        ("alias = object()\n", false),
        ("unknown(alias)\n", false),
    ] {
        let source = format!("from flask import Flask\napp = Flask(__name__)\nalias = app\n{between}app = Flask(__name__)\nalias.get('/')\n");
        let files = parsed(&[("app.py", &source)]);
        let bindings = index_python(&files);
        let file = &files["app.py"];
        let call = file
            .tree
            .root_node()
            .named_child(file.tree.root_node().named_child_count() - 1)
            .unwrap()
            .named_child(0)
            .unwrap();
        let (object, _) =
            super::super::python::attribute(call.child_by_field_name("function").unwrap()).unwrap();
        match bindings.value_at("app.py", object, &SetupContext::default()) {
            super::super::receivers::PythonValue::Router(router) if expected => {
                assert_eq!(
                    router.identity.construction_byte,
                    source.find("Flask(").unwrap()
                );
            }
            super::super::receivers::PythonValue::Unknown(_)
            | super::super::receivers::PythonValue::Opaque(_)
                if !expected => {}
            _ => panic!("unexpected evidence for {between:?}"),
        }
    }
}
