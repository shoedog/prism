use super::*;

const APP: &str = "from flask import Flask, Blueprint\napp = Flask(__name__)\n";

fn check(source: &str, inspect: impl FnOnce(&[FlaskEvent<'_>])) {
    let source = format!("{APP}{source}");
    let files = parsed(&[("app.py", &source)]);
    inspect(&flask_facts(
        &index_python(&files),
        "app.py",
        &SetupContext::default(),
    ));
}

fn rule<'events, 'tree>(
    events: &'events [FlaskEvent<'tree>],
) -> &'events super::super::super::python::flask::FlaskRule<'tree> {
    let [FlaskEvent::Rule(rule)] = events else {
        panic!("expected one rule, got {} events", events.len())
    };
    rule
}

fn proved(receiver: &Receiver) -> bool {
    matches!(receiver, Receiver::Router { .. })
}

#[test]
fn match_capture_invalidates_receiver() {
    for (pattern, expected) in [
        ("_", true),
        ("Sink.get", true),
        ("app", false),
        ("[app]", false),
        ("Sink(value=app)", false),
        ("_ as app", false),
        ("{'key': app}", false),
        ("{'key': _, **app}", false),
    ] {
        check(&format!("match value:\n    case {pattern}:\n        @app.get('/x')\n        def f(): pass\n"), |events| {
            assert_eq!(proved(&rule(events).route.receiver), expected, "{pattern}");
        });
    }
}

#[test]
fn attribute_writes_invalidate_router_and_module_aliases() {
    for (write, expected) in [
        ("other.get = decorate", true),
        ("alias.get = decorate", false),
        ("app.get = decorate", false),
        ("del alias.get", false),
        ("(alias.get, something) = values", false),
        (
            "if condition:\n    alias = other\nalias.get = decorate",
            false,
        ),
        ("def modify():\n    app.get = decorate\nmodify()", false),
    ] {
        check(
            &format!(
                "other = Flask(__name__)\nalias = app\n{write}\n@app.get('/x')\ndef f(): pass\n"
            ),
            |events| {
                assert_eq!(proved(&rule(events).route.receiver), expected, "{write}");
            },
        );
    }
    for (write, expected) in [
        ("", true),
        ("routes.bp = routes.other\n", false),
        ("alias.bp = routes.other\n", false),
    ] {
        let source = format!(
            "{APP}import routes\nalias = routes\n{write}app.register_blueprint(routes.bp)\n"
        );
        let files = parsed(&[("app.py", &source), ("routes.py", "from flask import Blueprint\nbp = Blueprint('bp', __name__)\nother = Blueprint('other', __name__)\n")]);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Mount(mount)] = facts.as_slice() else {
            panic!("expected mount")
        };
        assert_eq!(proved(&mount.child), expected, "{write}");
        assert!(proved(&mount.parent));
    }

    for (write, expected) in [("", true), ("made.get = replace", false)] {
        let source = format!("from flask import Flask\ndef create():\n    inner = Flask(__name__)\n    return inner\nmade = create()\n{write}\n");
        let files = parsed(&[("app.py", &source)]);
        let selector = RootSelector::parse("app:made").unwrap();
        let roots = index_python(&files).roots(Framework::Flask, None);
        assert_eq!(selector.resolve(&roots).is_ok(), expected, "{write}");
    }
}

#[test]
fn native_signature_rejects_duplicate_and_excess_arguments() {
    check(
        "def f(): pass\napp.add_url_rule('/x', 'x', f, False)\n",
        |events| {
            assert_eq!(
                rule(events).effective_methods,
                MethodSet::known(["GET", "HEAD"])
            );
        },
    );
    check(
        "def f(): pass\n@app.route('/get', provide_automatic_options=False)\ndef get(): pass\n",
        |events| {
            assert_eq!(
                rule(events).effective_methods,
                MethodSet::known(["GET", "HEAD"])
            );
        },
    );
    for (target, policy, expected) in [
        ("bp", "False", true),
        ("state", "provide_automatic_options=False", true),
        ("state", "False", false),
    ] {
        let call = format!("{target}.add_url_rule('/x', 'x', f, {policy})");
        let setup = if target == "state" {
            format!("@bp.record_once\ndef defer(state):\n    {call}")
        } else {
            call
        };
        check(
            &format!("bp = Blueprint('bp', __name__)\ndef f(): pass\n{setup}\n"),
            |events| {
                if expected {
                    assert_eq!(
                        rule(events).effective_methods,
                        MethodSet::known(["GET", "HEAD"])
                    );
                } else {
                    assert!(events.is_empty());
                }
            },
        );
    }
    for source in [
        "app.add_url_rule('/pos', rule='/kw', view_func=f)",
        "app.add_url_rule('/x', 'x', f, False, provide_automatic_options=False)",
        "app.add_url_rule('/x', 'x', f, False, 42)",
        "app.add_url_rule('/x', endpoint='a', endpoint='b')",
        "@app.route('/pos', rule='/kw')\ndef g(): pass",
        "@app.route('/x', 'y')\ndef g(): pass",
        "@app.route('/x', view_func=f)\ndef g(): pass",
        "@app.get('/x', methods=['POST'])\ndef g(): pass",
    ] {
        check(&format!("def f(): pass\n{source}\n"), |events| {
            assert!(
                events
                    .iter()
                    .all(|event| !matches!(event, FlaskEvent::Rule(_))),
                "{source}"
            );
        });
    }
}

#[test]
fn method_none_grouping_and_empty_view_attribute() {
    for (attribute, keyword, expected) in [
        ("", "methods=None", &["GET", "HEAD", "OPTIONS"][..]),
        (
            "f.methods = ['PATCH']",
            "methods=(None)",
            &["PATCH", "OPTIONS"],
        ),
        ("f.methods = []", "", &["GET", "HEAD", "OPTIONS"]),
        ("f.methods = ()", "", &["GET", "HEAD", "OPTIONS"]),
        ("", "methods=(['POST'])", &["POST", "OPTIONS"]),
        ("f.methods = ['POST']", "methods=[]", &["OPTIONS"]),
    ] {
        check(
            &format!(
                "def f(): pass\n{attribute}\napp.add_url_rule('/x', view_func=f, {keyword})\n"
            ),
            |events| {
                assert_eq!(
                    rule(events).effective_methods,
                    MethodSet::known(expected.iter().copied()),
                    "{attribute} {keyword}"
                );
            },
        );
    }
}

#[test]
fn view_metadata_follows_proved_setup_order() {
    check("def f(): pass\ndef unused():\n    f.methods = ['POST']\napp.add_url_rule('/x', view_func=f)\n", |events| {
        assert!(matches!(rule(events).effective_methods, MethodSet::Unknown { .. }));
    });
    check("def f(): pass\nif condition:\n    f.methods = ['POST']\napp.add_url_rule('/x', view_func=f)\n", |events| {
        assert!(matches!(rule(events).effective_methods, MethodSet::Unknown { .. }));
    });
    for (guard, endpoint, expected) in [
        ("", "", EffectiveEndpoint::Known("renamed".into())),
        ("if condition: ", "", EffectiveEndpoint::Unknown),
        (
            "if condition: ",
            "endpoint='explicit', ",
            EffectiveEndpoint::Known("explicit".into()),
        ),
    ] {
        check(&format!("def f(): pass\n{guard}f.__name__ = 'renamed'\napp.add_url_rule('/x', {endpoint}view_func=f)\n"), |events| {
            assert_eq!(rule(events).effective_endpoint, expected);
        });
    }
    check(
        "def f(): pass\napp.add_url_rule('/x', view_func=f)\nf.methods = ['POST']\n",
        |events| {
            assert!(matches!(
                rule(events).effective_methods,
                MethodSet::Unknown { .. }
            ));
        },
    );
    check("def f(): pass\nbp = Blueprint('bp', __name__)\n@bp.record_once\ndef defer(state):\n    state.add_url_rule('/x', view_func=f)\napp.register_blueprint(bp)\nf.methods = ['POST']\n", |events| {
        let [FlaskEvent::Rule(rule), FlaskEvent::Mount(_)] = events else { panic!("expected deferred rule and mount"); };
        assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
    });
    let files = parsed(&[("app.py", "from flask import Flask\nfrom views import f\napp = Flask(__name__)\nf.methods = ['POST']\napp.add_url_rule('/x', view_func=f)\n"),
        ("views.py", "def f(): pass\nf.methods = ['PATCH']\n")]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    assert_eq!(
        rule(&facts).effective_methods,
        MethodSet::known(["POST", "OPTIONS"])
    );

    let source = format!("{APP}def f(): pass\ndef make():\n    app.add_url_rule('/x', view_func=f)\nf.methods = ['POST']\nmake()\n");
    let files = parsed(&[("app.py", &source)]);
    let file = &files["app.py"];
    let factory = file.tree.root_node().named_child(3).unwrap();
    let start_byte = source.rfind("make()").unwrap();
    let context = SetupContext {
        factory: Some(PythonMember {
            file,
            node: factory,
        }),
        frames: vec![Invocation::Call {
            file: "app.py".into(),
            start_byte,
            end_byte: start_byte + "make()".len(),
        }],
    };
    let facts = flask_facts(&index_python(&files), "app.py", &context);
    assert_eq!(
        rule(&facts).effective_methods,
        MethodSet::known(["POST", "OPTIONS"])
    );
}

#[test]
fn arguments_share_decorator_transparency() {
    for (extra, expected) in [("", true), ("@replace\n", false)] {
        check(
            &format!(
                "{extra}@app.get('/one')\ndef f(): pass\napp.add_url_rule('/two', view_func=f)\n"
            ),
            |events| {
                let [FlaskEvent::Rule(first), FlaskEvent::Rule(second)] = events else {
                    panic!("expected two rules")
                };
                assert!(matches!(
                    first.route.callbacks[0].status,
                    CallbackStatus::Resolved { .. }
                ));
                assert_eq!(
                    matches!(
                        second.route.callbacks[0].status,
                        CallbackStatus::Resolved { .. }
                    ),
                    expected
                );
            },
        );
    }
}

#[test]
fn keyword_unpacking_does_not_certify_absence() {
    check("bp = Blueprint('bp', __name__)\napp.register_blueprint(bp, **opts)\napp.register_blueprint(bp)\napp.add_url_rule('/x', **kw)\napp.add_url_rule('/y', endpoint='y')\n", |events| {
        let [FlaskEvent::Mount(hidden), FlaskEvent::Mount(clear), FlaskEvent::Rule(unknown), FlaskEvent::Rule(known)] = events else { panic!("expected two mounts and two rules") };
        assert_eq!(clear.prefix, MountPrefix::Absent);
        assert!(matches!(hidden.prefix, MountPrefix::Unevaluated { .. }));
        assert!(matches!(unknown.route.callbacks.as_slice(), [Callback { status: CallbackStatus::Unresolved { .. }, .. }]));
        assert!(matches!(unknown.effective_methods, MethodSet::Unknown { .. }));
        assert!(known.route.callbacks.is_empty());
        assert_eq!(unknown.build_only, None);
        assert_eq!(known.build_only, Some(false));
        assert_eq!(unknown.effective_endpoint, EffectiveEndpoint::Unknown);
    });
    let source = format!("{APP}app.register_blueprint(**mount)\napp.add_url_rule(**rule)\n");
    let files = parsed(&[("app.py", &source)]);
    let file = &files["app.py"];
    let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
    let [FlaskEvent::Mount(mount), FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected an unpacked mount and rule");
    };
    assert!(!proved(&mount.child));
    assert_eq!(rule.route.declaration.as_ref().unwrap().path.literal, None);
    for field in [
        "endpoint",
        "methods",
        "provide_automatic_options",
        "subdomain",
        "host",
        "build_only",
    ] {
        let super::super::super::python::PythonArg::Unknown(node) =
            rule.options.keyword(file, field)
        else {
            panic!("{field} was treated as known");
        };
        assert_eq!(file.node_text(&node), "**rule");
    }
}

#[test]
fn unselected_setup_cannot_prove_a_receiver() {
    check("def setup():\n    app.add_url_rule('/never', endpoint='never')\napp.add_url_rule('/now', endpoint='now')\n", |events| {
        let rule = rule(events);
        assert_eq!(rule.route.endpoint.as_deref(), Some("now"));
        assert!(proved(&rule.route.receiver));
    });
}

#[test]
fn effective_endpoint_distinguishes_unknown_from_absent() {
    check("app.add_url_rule('/x', view_func=lambda: None)\napp.add_url_rule('/y')\napp.add_url_rule('/z', endpoint='z')\napp.add_url_rule('/w', endpoint=dynamic)\n", |events| {
        let [FlaskEvent::Rule(unknown_view), FlaskEvent::Rule(absent), FlaskEvent::Rule(known), FlaskEvent::Rule(unknown_arg)] = events else { panic!("expected four rules") };
        assert_eq!(unknown_view.effective_endpoint, EffectiveEndpoint::Unknown);
        assert_eq!(unknown_arg.effective_endpoint, EffectiveEndpoint::Unknown);
        assert_eq!(absent.effective_endpoint, EffectiveEndpoint::Absent);
        assert_eq!(known.effective_endpoint, EffectiveEndpoint::Known("z".into()));
    });
}

#[test]
fn global_declaration_and_function_write_refuse_module_binding() {
    for (other, expected) in [
        ("def reset():\n    global app\n", true),
        ("def reset():\n    app = object()\n", true),
        ("def reset():\n    global app\n    app = object()\n", false),
    ] {
        check(
            &format!("{other}app.add_url_rule('/x', endpoint='x')\n"),
            |events| {
                assert_eq!(proved(&rule(events).route.receiver), expected, "{other}");
            },
        );
    }
}

#[test]
fn converter_arguments_require_a_default_converter() {
    for path in [
        "/<int(-2):v>",
        "/<any('a:b', 'c>d'):v>",
        "/<string(length=2):v>",
        "/<int:v",
    ] {
        assert_eq!(flask_path_tokens(path), None, "{path}");
    }
    assert_eq!(
        flask_path_tokens("/<int:a><uuid:b>"),
        Some(vec![
            PathToken::Literal("/".into()),
            PathToken::Parameter,
            PathToken::Parameter
        ])
    );
}

#[test]
fn physical_newlines_normalize_without_rewriting_escapes() {
    for (source, expected) in [
        ("r'''/first\r\nsecond\rthird'''", "/first\nsecond\nthird"),
        ("'''/first\\\rsecond'''", "/firstsecond"),
        (r#"r"/first\rsecond""#, "/first\\rsecond"),
        (r#""/first\rsecond""#, "/first\rsecond"),
    ] {
        let file = ParsedFile::parse("app.py", source, Language::Python).unwrap();
        let node = expression(&file);
        assert_eq!(file.node_text(&node), source);
        assert_eq!(literal_string(&file, node).as_deref(), Some(expected));
    }
}

#[test]
fn converter_registry_write_withholds_only_affected_parameter_tokens() {
    for (write, expected) in [
        ("", true),
        ("other.url_map.converters['string'] = PathConverter", true),
        ("app.config['string'] = PathConverter", true),
        ("app.url_map.converters['string'] = PathConverter", false),
        ("app.url_map.converters = replacement", false),
        (
            "registry = app.url_map.converters\nregistry.update(replacement)",
            false,
        ),
        (
            "registry = app.url_map.converters\ndel registry['string']",
            false,
        ),
        (
            "alias = app\nalias.url_map.converters['string'] = PathConverter",
            false,
        ),
    ] {
        check(&format!("other = Flask(__name__)\n{write}\n@app.get('/files/<string:rest>')\ndef f(): pass\n"), |events| {
            let rule = events.iter().find_map(|event| match event {
                FlaskEvent::Rule(rule) => Some(rule), _ => None,
            }).expect("rule");
            assert!(proved(&rule.route.receiver));
            assert_eq!(rule.path_tokens.is_some(), expected, "{write}");
        });
    }
}

#[test]
fn literal_bytes_unicode_names_and_dynamic_concat_refuse() {
    for (source, expected) in [
        (r#"b"/example""#, None),
        (r#""/\N{LATIN SMALL LETTER A}""#, None),
        (r#""/a" + name"#, None),
        (r#""/\141\x62""#, Some("/ab")),
        (r#""/a\q""#, Some("/a\\q")),
    ] {
        let file = ParsedFile::parse("app.py", source, Language::Python).unwrap();
        let node = expression(&file);
        assert_eq!(file.node_text(&node), source);
        assert_eq!(literal_string(&file, node).as_deref(), expected, "{source}");
    }
}

#[test]
fn module_ambiguity_and_relative_import_past_root_refuse() {
    let child = "from flask import Blueprint\nbp = Blueprint('x', __name__)\n";
    for (module, ambiguous, expected) in [
        ("a", false, true),
        ("a", true, false),
        ("..a", false, false),
    ] {
        let source = format!("{APP}from {module} import bp\napp.register_blueprint(bp)\n");
        let mut sources = vec![("pkg/app.py", source.as_str()), ("a.py", child)];
        if ambiguous {
            sources.push(("a/__init__.py", child));
        }
        let files = parsed(&sources);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "pkg/app.py", &SetupContext::default());
        let [FlaskEvent::Mount(mount)] = facts.as_slice() else {
            panic!("expected mount")
        };
        assert_eq!(proved(&mount.child), expected, "{module}, {ambiguous}");
    }
}

#[test]
fn scope_shadows_and_in_function_call_frames_respect_module_order() {
    let cases = [
        ("", "", "", true),
        ("app", "", "", false),
        ("", "    app = replacement\n", "", false),
        ("", "", "app = replacement\n", false),
    ];
    for (param, local_write, module_write, expected) in cases {
        let source = format!("{APP}def setup({param}):\n    app.add_url_rule('/x', endpoint='x')\n{local_write}def outer():\n    return setup()\n{module_write}");
        let files = parsed(&[("app.py", &source)]);
        let file = &files["app.py"];
        let inner_byte = source.rfind("setup()").unwrap();
        let context = SetupContext {
            factory: Some(PythonMember {
                file,
                node: file.tree.root_node().named_child(2).unwrap(),
            }),
            frames: vec![
                Invocation::Entry {
                    entry: "app:outer()".into(),
                },
                Invocation::Call {
                    file: "app.py".into(),
                    start_byte: inner_byte,
                    end_byte: inner_byte + "setup()".len(),
                },
            ],
        };
        let facts = flask_facts(&index_python(&files), "app.py", &context);
        assert_eq!(proved(&rule(&facts).route.receiver), expected, "{source}");
    }

    for (shadow, expected) in [("", true), ("    app = replacement\n", false)] {
        let source = format!("{APP}class C:\n{shadow}    app.get('/')\n");
        let files = parsed(&[("app.py", &source)]);
        let file = &files["app.py"];
        let bindings = index_python(&files);
        let object = bindings
            .nodes("app.py")
            .iter()
            .find_map(|node| {
                let (object, field) = super::super::super::python::attribute(*node)?;
                (file.node_text(&field) == "get").then_some(object)
            })
            .unwrap();
        let value = bindings.value_at("app.py", object, &SetupContext::default());
        assert_eq!(
            matches!(
                value,
                super::super::super::receivers::PythonValue::Router(_)
            ),
            expected
        );
    }
}

#[test]
fn factory_refuses_async_decorators_yield_and_parameters() {
    for (prefix, keyword, params, body, expected) in [
        ("", "def", "", "", true),
        ("", "async def", "", "", false),
        ("@wrap\n", "def", "", "", false),
        ("", "def", "", "    yield app\n", false),
        ("", "def", "config=None", "", false),
    ] {
        let source = format!("from flask import Flask\n{prefix}{keyword} make({params}):\n    app = Flask(__name__)\n{body}    return app\n");
        let files = parsed(&[("app.py", &source)]);
        let selector = RootSelector::parse("app:make()").unwrap();
        assert_eq!(
            selector
                .resolve(&index_python(&files).roots(Framework::Flask, Some(&selector)))
                .is_ok(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn record_once_requires_native_decorator_and_unwritten_state() {
    for (decorator, params, write, expected) in [
        ("@bp.record_once", "state", "", true),
        (
            "@bp.record_once",
            "state",
            "    state = replacement\n",
            false,
        ),
        ("@bp.record_once", "state, other", "", false),
        ("@wrap\n@bp.record_once", "state", "", false),
        ("@bp.record", "state", "", false),
    ] {
        check(&format!("bp = Blueprint('bp', __name__)\n{decorator}\ndef deferred({params}):\n{write}    state.add_url_rule('/deferred', endpoint='d')\n"), |events| {
            if expected {
                let rule = rule(events);
                assert!(rule.record_once && proved(&rule.route.receiver));
            } else {
                assert!(events.is_empty(), "{decorator}, {params}, {write}");
            }
        });
    }
}

#[test]
fn config_and_view_map_require_a_grounded_receiver() {
    check("def view(): pass\nlookalike = object()\nlookalike.config['k'] = False\nlookalike.view_functions['v'] = view\napp.config['k'] = False\napp.view_functions['v'] = view\n", |events| {
        let [FlaskEvent::Config { receiver: config, key, value, .. }, FlaskEvent::ViewWrite { receiver: map, endpoint, callback, .. }] = events else { panic!("expected only grounded mapping writes"); };
        assert!(proved(config) && proved(map));
        assert_eq!(key.as_deref(), Some("k"));
        assert_eq!(value.kind(), "false");
        assert_eq!(endpoint, &EffectiveEndpoint::Known("v".into()));
        assert!(matches!(callback.status, CallbackStatus::Resolved { .. }));
    });
}

#[test]
fn native_class_view_refuses_overrides_fields_and_decorated_classes() {
    for (decorator, contents, expected) in [
        ("", "    pass\n", Some(&["GET", "HEAD", "OPTIONS"][..])),
        (
            "",
            "    def patch(self): pass\n",
            Some(&["PATCH", "OPTIONS"][..]),
        ),
        ("", "    def as_view(self): pass\n", None),
        ("", "    methods = ['POST']\n", None),
        ("@replace\n", "    pass\n", None),
    ] {
        check(&format!("from flask.views import MethodView\n{decorator}class View(MethodView):\n{contents}app.add_url_rule('/v', view_func=View.as_view('view'))\n"), |events| {
            let rule = rule(events);
            if let Some(methods) = expected {
                assert_eq!(rule.effective_methods, MethodSet::known(methods.iter().copied()));
                assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Known("view".into()));
            } else {
                assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
                assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Unknown);
            }
        });
    }
}

#[test]
fn non_literal_automatic_options_remains_unknown() {
    check("def f(): pass\napp.add_url_rule('/x', view_func=f, provide_automatic_options=policy)\napp.add_url_rule('/y', view_func=f, provide_automatic_options=False)\n", |events| {
        let [FlaskEvent::Rule(unknown), FlaskEvent::Rule(disabled)] = events else { panic!("expected two rules"); };
        assert_eq!(unknown.effective_methods, MethodSet::unknown("policy"));
        assert_eq!(disabled.effective_methods, MethodSet::known(["GET", "HEAD"]));
    });
}
