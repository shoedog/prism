use super::*;

#[test]
fn opaque_view_argument_withholds_metadata() {
    let app = format!("{APP}import setup\nfrom views import f\nsetup.patch(f)\napp.add_url_rule('/x', view_func=f)\n");
    for (body, opaque) in [
        ("pass", false),
        ("return None", false),
        ("view.methods = ['POST']", true),
    ] {
        let setup = format!("def patch(view):\n    {body}\n");
        let files = parsed(&[
            ("app.py", &app),
            ("views.py", "def f(): pass\n"),
            ("setup.py", &setup),
        ]);
        let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
        let fact = rule(&facts);
        assert!(proved(&fact.route.receiver));
        assert!(matches!(
            fact.route.callbacks[0].status,
            CallbackStatus::Resolved { .. }
        ));
        if opaque {
            assert!(
                matches!(fact.effective_methods, MethodSet::Unknown { .. }),
                "{body}: {:?}",
                fact.effective_methods
            );
            assert_eq!(fact.effective_endpoint, EffectiveEndpoint::Unknown);
            assert!(matches!(
                fact.view_metadata.methods,
                Some(MethodSet::Unknown { .. })
            ));
            assert!(matches!(
                fact.view_metadata.required,
                Some(MethodSet::Unknown { .. })
            ));
            assert!(matches!(fact.view_metadata.automatic, Some(Err(_))));
        } else {
            assert_eq!(
                fact.effective_methods,
                MethodSet::known(["GET", "HEAD", "OPTIONS"])
            );
            assert_eq!(
                fact.effective_endpoint,
                EffectiveEndpoint::Known("f".into())
            );
            assert!(fact.view_metadata.methods.is_none());
            assert!(fact.view_metadata.required.is_none());
            assert!(fact.view_metadata.automatic.is_none());
        }
    }
}

#[test]
fn only_proved_add_url_rule_preserves_view_metadata() {
    for (replacement, native) in [("", true), ("other.add_url_rule = patch\n", false)] {
        let app = format!("{APP}from setup import patch\nfrom views import f, g\nother = Flask(__name__)\n{replacement}other.add_url_rule('/other', None, f)\napp.add_url_rule('/control', view_func=g)\napp.add_url_rule('/x', endpoint='explicit', view_func=(f))\n");
        let files = parsed(&[
            ("app.py", &app),
            ("views.py", "def f(): pass\ndef g(): pass\n"),
            (
                "setup.py",
                "def patch(rule, endpoint, view):\n    view.methods = ['POST']\n",
            ),
        ]);
        let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(other), FlaskEvent::Rule(control), FlaskEvent::Rule(fact)] =
            facts.as_slice()
        else {
            panic!("expected three rules");
        };
        let default = MethodSet::known(["GET", "HEAD", "OPTIONS"]);
        assert_eq!(proved(&other.route.receiver), native);
        assert!(proved(&fact.route.receiver));
        assert_eq!(control.effective_methods, default);
        assert_eq!(
            fact.effective_endpoint,
            EffectiveEndpoint::Known("explicit".into())
        );
        if native {
            assert_eq!(fact.effective_methods, default);
        } else {
            assert!(matches!(fact.effective_methods, MethodSet::Unknown { .. }));
        }
    }
}

#[test]
fn factory_registration_does_not_supply_order_for_another_function() {
    let app = format!("{APP}def f(): pass\nfrom mutating import patch\ndef configure():\n    patch(f)\nconfigure()\nfrom noop import patch\ndef make():\n    app.add_url_rule('/x', view_func=f)\nmake()\n");
    let files = parsed(&[
        ("app.py", &app),
        (
            "mutating.py",
            "def patch(view):\n    view.methods = ['POST']\n",
        ),
        ("noop.py", "def patch(view): pass\n"),
    ]);
    let file = &files["app.py"];
    let factory = file.tree.root_node().named_child(7).unwrap();
    let start_byte = app.rfind("make()").unwrap();
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
    let fact = rule(&facts);
    assert!(proved(&fact.route.receiver));
    assert_eq!(fact.effective_endpoint, EffectiveEndpoint::Unknown);
    assert!(matches!(fact.effective_methods, MethodSet::Unknown { .. }));
}
