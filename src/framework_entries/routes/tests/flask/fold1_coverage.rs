//! Controls for conservative lexical setup, source import and metadata boundaries.
use super::*;
use crate::framework_entries::routes::receivers::refusals::ImportRefusalReason;

const APP: &str = "from flask import Flask\napp = Flask(__name__)\n";

#[test]
fn uninvoked_and_ambiguous_setup_bodies_do_not_add_registrations() {
    for setup in [
        "def install():\n    app.add_url_rule('/x')\n",
        "def install():\n    app.add_url_rule('/x')\ninstall()\ninstall()\n",
        "async def install():\n    app.add_url_rule('/x')\ninstall()\n",
        "def install():\n    app.add_url_rule('/x')\n    yield None\ninstall()\n",
        "def install(value):\n    app.add_url_rule('/x')\ninstall()\n",
        "class Setup:\n    def __init__(self): pass\n    def install(self):\n        app.add_url_rule('/x')\nSetup().install()\n",
        "class Setup(Base):\n    def install(self):\n        app.add_url_rule('/x')\nSetup().install()\n",
        "class Setup:\n    @decorate\n    def install(self):\n        app.add_url_rule('/x')\nSetup().install()\n",
    ] {
        let source = format!("{APP}{setup}");
        let files = parsed(&[("app.py", &source)]);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
        assert!(facts.is_empty(), "{setup}");
    }
}

#[test]
fn source_wildcard_with_dynamic_exports_or_a_cycle_withholds_terminal_identity() {
    let source = format!("{APP}from first import f\napp.add_url_rule('/x', view_func=f)\n");
    for second in [
        "from first import *\n",
        "__all__ = choose_exports()\n",
        "__all__ = ('f',)\n",
    ] {
        let files = parsed(&[
            ("app.py", &source),
            ("first.py", "def f(): pass\nfrom second import *\n"),
            ("second.py", second),
        ]);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
            panic!("expected a retained registration");
        };
        assert!(matches!(
            rule.route.callbacks[0].status,
            CallbackStatus::Unresolved {
                reason: Reason::ImportedUnproven,
                ..
            }
        ));
        let refusals = bindings.import_refusals();
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].reason, ImportRefusalReason::StarImport);
    }
}

#[test]
fn hidden_import_in_a_traced_escape_keeps_the_refusal_and_loses_metadata() {
    let source = format!(
        "{APP}from views import f as callback\nimport views\n\
         from helpers import patch\ncallback = object()\npatch(callback)\n\
         app.add_url_rule('/x', view_func=views.f)\n"
    );
    let files = parsed(&[
        ("app.py", &source),
        ("views.py", "def f(): pass\n"),
        ("helpers.py", "def patch(v): consume(v)\n"),
    ]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected a retained registration");
    };
    assert!(matches!(
        rule.route.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert_eq!(rule.route.default_methods, None);
    let refusals = bindings.import_refusals();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].reason, ImportRefusalReason::MultiWrite);
    let start = source.find("f as callback").unwrap();
    assert_eq!(
        (refusals[0].start_byte, refusals[0].end_byte),
        (start, start + 1)
    );
}

#[test]
fn native_class_metadata_requires_an_unmodified_single_base() {
    for (decl, known) in [
        (
            "class Handler(MethodView):\n    def get(self): pass\n    def patch(self): pass\n",
            true,
        ),
        (
            "class Handler(MethodView, Mixin):\n    def get(self): pass\n",
            false,
        ),
        (
            "@decorate\nclass Handler(MethodView):\n    def get(self): pass\n",
            false,
        ),
        (
            "class Handler(MethodView):\n    def as_view(self): pass\n    def get(self): pass\n",
            false,
        ),
        ("class Handler(MethodView):\n    get = custom\n", false),
        ("class Handler(Other):\n    def get(self): pass\n", false),
    ] {
        let source = format!(
            "{APP}from flask.views import MethodView\n{decl}\
             app.add_url_rule('/x', view_func=Handler.as_view(name='endpoint'))\n"
        );
        let files = parsed(&[("app.py", &source)]);
        let bindings = index_python(&files);
        let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
            panic!("expected one retained rule");
        };
        assert!(matches!(
            rule.route.callbacks[0].status,
            CallbackStatus::Unresolved {
                reason: Reason::ClassView,
                ..
            }
        ));
        if known {
            assert_eq!(
                rule.effective_methods,
                MethodSet::known(["GET", "HEAD", "OPTIONS", "PATCH"])
            );
            assert_eq!(
                rule.effective_endpoint,
                EffectiveEndpoint::Known("endpoint".into())
            );
        } else {
            assert!(
                matches!(rule.effective_methods, MethodSet::Unknown { .. }),
                "{decl}"
            );
            assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Unknown);
        }
    }
}
