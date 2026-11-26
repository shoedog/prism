use super::*;

#[test]
fn competing_metadata_writes_are_unknown_and_import_cycles_preserve_source_writes() {
    for (views, local, expected) in [
        ("f.methods = ['PATCH']\n", "", Some("PATCH")),
        ("", "f.methods = ['POST']\n", Some("POST")),
        ("f.methods = ['PATCH']\n", "f.methods = ['POST']\n", None),
        ("import app\nf.methods = ['PATCH']\n", "", Some("PATCH")),
    ] {
        let app = format!("{APP}from views import f\n{local}app.add_url_rule('/x', view_func=f)\n");
        let views = format!("def f(): pass\n{views}");
        let files = parsed(&[("app.py", &app), ("views.py", &views)]);
        let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
        let methods = &rule(&facts).effective_methods;
        assert!(
            match methods {
                MethodSet::Known { .. } =>
                    expected.is_some_and(|verb| { *methods == MethodSet::known([verb, "OPTIONS"]) }),
                MethodSet::Unknown { .. } => expected.is_none(),
                _ => false,
            },
            "{local} {views}: {methods:?}"
        );
    }
}

#[test]
fn third_file_and_dynamic_view_metadata_are_indexed_by_definition() {
    let app = format!(
        "{APP}import setup\nfrom views import f, g\napp.add_url_rule('/control', view_func=g)\napp.add_url_rule('/x', view_func=f)\n"
    );
    for (setup, writes) in [
        ("from views import f\n", ""),
        (
            "from views import f as target\ntarget.methods = ['POST']\n",
            "",
        ),
        ("from views import f\nsetattr(f, 'methods', ['POST'])\n", ""),
        ("", "setattr(f, 'methods', ['POST'])\n"),
        ("", "setattr(f, '__name__', 'changed')\n"),
    ] {
        let app = format!("{app}{writes}");
        let files = parsed(&[
            ("app.py", &app),
            ("views.py", "def f(): pass\ndef g(): pass\n"),
            ("setup.py", setup),
        ]);
        let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(control), FlaskEvent::Rule(changed)] = facts.as_slice() else {
            panic!("expected two rules");
        };
        let default = MethodSet::known(["GET", "HEAD", "OPTIONS"]);
        assert_eq!(control.effective_methods, default);
        assert_eq!(
            control.effective_endpoint,
            EffectiveEndpoint::Known("g".into())
        );
        if writes.contains("__name__") {
            assert_eq!(changed.effective_methods, default);
            assert_eq!(changed.effective_endpoint, EffectiveEndpoint::Unknown);
        } else if setup.contains("methods") || writes.contains("methods") {
            assert!(
                matches!(changed.effective_methods, MethodSet::Unknown { .. }),
                "{setup} {writes}"
            );
        } else {
            assert_eq!(changed.effective_methods, default);
        }
    }
}

#[test]
fn escaped_decorator_owner_cannot_prove_handler_identity() {
    let mut results = Vec::new();
    for setup in [
        "configure(other)",
        "configure(app)",
        "alias = app\nconfigure(router=alias)",
        "holder.router = app",
        "holder[0] = app",
        "configure({'router': app})",
    ] {
        check(&format!("other = Flask(__name__)\n{setup}\n@app.get('/one')\ndef f(): pass\npublic = Flask(__name__)\npublic.add_url_rule('/two', view_func=f)\n"), |events| {
            let [FlaskEvent::Rule(_), FlaskEvent::Rule(second)] = events else {
                panic!("expected two rules for {setup}");
            };
            assert!(proved(&second.route.receiver));
            results.push((setup, matches!(second.route.callbacks[0].status, CallbackStatus::Resolved { .. })));
        });
    }
    assert_eq!(
        results
            .iter()
            .map(|(_, resolved)| *resolved)
            .collect::<Vec<_>>(),
        [true, false, false, false, false, false],
        "{results:?}"
    );
}

#[test]
fn escaped_converter_values_withhold_parameter_tokens_only() {
    let mut results = Vec::new();
    for setup in [
        "configure(other.url_map.converters)",
        "configure(app.config)",
        "configure(app.url_map)",
        "configure(app.url_map.converters)",
        "r = app.url_map.converters\nconfigure(registry=(r))",
        "holder.registry = app.url_map.converters",
        "holder[0] = app.url_map",
        "app.url_map.configure()",
        "setattr(app.url_map, 'converters', registry)",
    ] {
        check(&format!("other = Flask(__name__)\n{setup}\n@app.get('/files/<string:rest>')\ndef f(): pass\n@app.get('/literal')\ndef g(): pass\n"), |events| {
            let [FlaskEvent::Rule(param), FlaskEvent::Rule(literal)] = events else {
                panic!("expected two rules for {setup}");
            };
            assert!(proved(&param.route.receiver) && proved(&literal.route.receiver));
            assert_eq!(literal.path_tokens, Some(vec![PathToken::Literal("/literal".into())]));
            results.push((setup, param.path_tokens.is_some()));
        });
    }
    assert_eq!(
        results
            .iter()
            .map(|(_, tokens)| *tokens)
            .collect::<Vec<_>>(),
        [true, true, false, false, false, false, false, false, false],
        "{results:?}"
    );
}

#[test]
fn route_api_name_belt_tracks_static_file_reach() {
    let mut results = Vec::new();
    for (import, patch) in [
        ("", "def patch(x): x.get = replacement\n"),
        (
            "import patch\n",
            "def patch(x): x.unrelated = replacement\n",
        ),
        ("import patch\n", "def patch(x): x.get = replacement\n"),
        (
            "from patch import patch\n",
            "def patch(x): setattr(x, 'route', replacement)\n",
        ),
        (
            "import patch\n",
            "def patch(x): x.converters['string'] = PathConverter\n",
        ),
        (
            "import patch\n",
            "def patch(x): setattr(x, 'url_map', replacement)\n",
        ),
        (
            "import patch\n",
            "def patch(x): x.converters.update(replacement)\n",
        ),
        (
            "import patch\n",
            "def patch(x): x.view_functions['f'] = replacement\n",
        ),
    ] {
        let app = format!("{APP}{import}@app.get('/<string:x>')\ndef f(): pass\n");
        let files = parsed(&[("app.py", &app), ("patch.py", patch)]);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
        let fact = rule(&facts);
        results.push((proved(&fact.route.receiver), fact.path_tokens.is_some()));
    }
    assert_eq!(
        results,
        [
            (true, true),
            (true, true),
            (false, false),
            (false, false),
            (true, false),
            (true, false),
            (true, false),
            (false, false),
        ]
    );
    check(
        "setattr(app, 'get', replacement)\n@app.get('/x')\ndef f(): pass\n",
        |events| {
            assert!(!proved(&rule(events).route.receiver));
        },
    );
}

#[test]
fn class_global_writes_refuse_module_binding() {
    for (body, expected) in [
        ("    global app\n", true),
        ("    app = replacement\n", true),
        (
            "    app = replacement\ndef declare():\n    global app\n",
            true,
        ),
        ("    global app\n    app = replacement\n", false),
    ] {
        check(
            &format!("class Reset:\n{body}@app.get('/x')\ndef f(): pass\n"),
            |events| {
                assert_eq!(proved(&rule(events).route.receiver), expected, "{body}");
            },
        );
    }
}

#[test]
fn exception_alias_and_multi_target_delete_controls() {
    for (write, expected) in [
        ("try: pass\nexcept Exception as ignored: pass", true),
        ("try: pass\nexcept Exception as app: pass", false),
        ("del first, second", true),
        ("del first, app", false),
    ] {
        check(
            &format!("{write}\n@app.get('/x')\ndef f(): pass\n"),
            |events| {
                assert_eq!(proved(&rule(events).route.receiver), expected, "{write}");
            },
        );
    }
}

#[test]
fn setup_attributes_preserve_route_proof_only_for_allowed_fields() {
    let mut results = Vec::new();
    for field in [
        "secret_key",
        "debug",
        "testing",
        "wsgi_app",
        "config",
        "json",
        "json_provider_class",
        "json_encoder",
        "json_decoder",
        "session_interface",
        "permanent_session_lifetime",
        "jinja_env",
        "template_folder",
        "url_rule_class",
        "get",
        "jsonify",
    ] {
        check(
            &format!("app.{field} = setup_value\n@app.get('/x')\ndef f(): pass\n"),
            |events| {
                results.push((field, proved(&rule(events).route.receiver)));
            },
        );
    }
    assert_eq!(
        results
            .iter()
            .map(|(_, proved)| *proved)
            .collect::<Vec<_>>(),
        [
            true, true, true, true, true, true, true, true, true, true, true, true, false, false,
            false, false
        ],
        "{results:?}"
    );
    check("app.wsgi_app = ProxyFix(app.wsgi_app)\nsetattr(app, 'debug', True)\n@app.get('/x')\ndef f(): pass\n", |events| {
        assert!(proved(&rule(events).route.receiver));
    });
}

#[test]
fn config_replacement_withholds_only_implicit_options_policy() {
    check("app.config = config\n@app.get('/implicit')\ndef f(): pass\n@app.get('/explicit', provide_automatic_options=False)\ndef g(): pass\n", |events| {
        let [FlaskEvent::Rule(implicit), FlaskEvent::Rule(explicit)] = events else {
            panic!("expected two rules");
        };
        assert!(proved(&implicit.route.receiver) && proved(&explicit.route.receiver));
        assert!(matches!(implicit.effective_methods, MethodSet::Unknown { .. }));
        assert_eq!(explicit.effective_methods, MethodSet::known(["GET", "HEAD"]));
    });
}

#[test]
fn only_a_proved_native_mount_keeps_the_blueprint_argument() {
    for (patch, expected) in [("", true), ("app.register_blueprint = custom\n", false)] {
        check(&format!("bp = Blueprint('bp', __name__)\n{patch}app.register_blueprint(bp)\n@bp.get('/x')\ndef f(): pass\n"), |events| {
            let [FlaskEvent::Mount(_), FlaskEvent::Rule(rule)] = events else {
                panic!("expected mount and rule candidates");
            };
            assert_eq!(proved(&rule.route.receiver), expected, "{patch}");
        });
    }
}
