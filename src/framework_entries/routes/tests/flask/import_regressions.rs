use super::*;
use crate::framework_entries::routes::receivers::refusals::ImportRefusalReason::*;

const APP: &str = "from flask import Flask\napp = Flask(__name__)\n";

#[test]
fn each_attribute_hop_needs_evidence_in_the_capture_scope() {
    for (imports, proved) in [
        ("import pkg\n", false),
        ("import pkg\nfrom pkg.branch import routes\n", false),
        (
            "import pkg\ndef other():\n    import pkg.branch.routes as nested\n",
            false,
        ),
        ("import pkg . branch . routes\n", true),
        (
            "import pkg\nfrom pkg import branch\nfrom pkg.branch import routes\n",
            true,
        ),
    ] {
        let app = format!("{APP}{imports}app.add_url_rule('/x', view_func=pkg.branch.routes.f)\nimport pkg.branch.routes as later\n");
        let files = parsed(&[
            ("app.py", &app),
            ("pkg/branch/routes.py", "def f(): pass\n"),
        ]);
        let bindings = index_python(&files);
        let events = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(rule)] = events.as_slice() else {
            panic!("one rule");
        };
        assert_eq!(
            matches!(
                rule.route.callbacks[0].status,
                CallbackStatus::Resolved { .. }
            ),
            proved,
            "{imports}",
        );
        assert!(bindings.import_refusals().is_empty());
    }
}

#[test]
fn chooses_the_first_predicate_across_hops_and_the_terminal() {
    let app = format!(
        "{APP}import pkg.routes as selected\napp.add_url_rule('/x', view_func=selected.f)\n"
    );
    let files = parsed(&[
        ("app.py", &app),
        ("pkg/__init__.py", "from . import routes\nroutes = None\n"),
        (
            "pkg/routes.py",
            "def f(): pass\ntry: pass\nexcept* Exception as __getattr__: pass\n",
        ),
    ]);
    let bindings = index_python(&files);
    flask_facts(&bindings, "app.py", &SetupContext::default());
    let refusals = bindings.import_refusals();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].reason, GetattrModule);
    assert_eq!(
        &app[refusals[0].start_byte..refusals[0].end_byte],
        "selected.f"
    );
}

#[test]
fn follows_eligible_initializer_module_alias() {
    let app = format!("{APP}import pkg\napp.add_url_rule('/x', view_func=pkg.routes.f)\n");
    let files = parsed(&[
        ("app.py", &app),
        ("pkg/__init__.py", "from . import real as routes\n"),
        ("pkg/real.py", "def f(): pass\n"),
    ]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = events.as_slice() else {
        panic!("one rule");
    };
    assert!(matches!(&rule.route.callbacks[0].status,
        CallbackStatus::Resolved { identity, .. } if identity.file == "pkg/real.py"
    ));
    assert!(bindings.import_refusals().is_empty());
}

#[test]
fn preserves_possible_view_through_unestablished_hop_and_reexport() {
    let views = format!("{APP}def f(): pass\nimport helper\napp.add_url_rule('/x', view_func=f)\n");
    let files = parsed(&[
        ("views.py", &views),
        (
            "helper.py",
            "import pkg\npkg.routes.f.__name__ = 'other'\npkg.routes.f.methods = ['POST']\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/routes.py", "from views import f\n"),
    ]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "views.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = events.as_slice() else {
        panic!("one rule");
    };
    assert!(matches!(
        rule.route.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
    assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Unknown);
    assert!(bindings.import_refusals().is_empty());
}

#[test]
fn no_effect_refusals_record_each_call_to_the_same_view() {
    let app = format!("{APP}import setup\ndef f(): pass\nsetup.patch(f)\nsetup.patch(f)\napp.add_url_rule('/x', view_func=f)\n");
    let files = parsed(&[
        ("app.py", &app),
        ("setup.py", "def patch(f): pass\ndef patch(f): pass\n"),
    ]);
    let bindings = index_python(&files);
    flask_facts(&bindings, "app.py", &SetupContext::default());
    let refusals = bindings.import_refusals();
    assert_eq!(refusals.len(), 2);
    assert!(refusals.iter().all(|refusal| refusal.reason == MultiWrite));
    assert_eq!(
        refusals
            .iter()
            .map(|refusal| refusal.start_byte)
            .collect::<Vec<_>>(),
        app.match_indices("setup.patch(f)")
            .map(|(offset, _)| offset)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn package_alias_cannot_move_a_hop_read_past_a_later_import() {
    for (imports, proved) in [
        (
            "import pkg\nselected = pkg.routes\nimport pkg.routes\n",
            false,
        ),
        ("import pkg.routes\nselected = pkg.routes\n", true),
    ] {
        let app = format!("{APP}{imports}app.add_url_rule('/x', view_func=selected.f)\n");
        let files = parsed(&[("app.py", &app), ("pkg/routes.py", "def f(): pass\n")]);
        let bindings = index_python(&files);
        let events = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(rule)] = events.as_slice() else {
            panic!("one rule");
        };
        assert_eq!(
            matches!(
                rule.route.callbacks[0].status,
                CallbackStatus::Resolved { .. }
            ),
            proved,
            "{imports}",
        );
        // `pkg` bound by `import pkg` and `import pkg.routes` is two binding occurrences.
        let refusals = bindings.import_refusals();
        if proved {
            assert!(refusals.is_empty());
        } else {
            assert!(!refusals.is_empty());
            assert!(refusals.iter().all(|refusal| refusal.reason == MultiWrite));
        }
    }
}

#[test]
fn unrelated_module_store_does_not_poison_a_qualified_no_effect_callee() {
    let app = format!("{APP}import setup\nfrom views import f\nselected = setup\nselected.unused = None\nselected.patch(f)\napp.add_url_rule('/x', view_func=f)\n");
    let files = parsed(&[
        ("app.py", &app),
        ("setup.py", "def patch(view): return (None)\n"),
        ("views.py", "def f(): pass\n"),
    ]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = events.as_slice() else {
        panic!("one rule");
    };
    assert_eq!(
        rule.effective_methods,
        MethodSet::known(["GET", "HEAD", "OPTIONS"])
    );
    assert!(bindings.import_refusals().is_empty());
}

#[test]
fn multiple_bindings_in_one_import_statement_are_competing_sites() {
    let app = format!("{APP}from views import f\napp.add_url_rule('/x', view_func=f)\n");
    let files = parsed(&[
        ("app.py", &app),
        ("views.py", "from impl import first as f, second as ｆ\n"),
        ("impl.py", "def first(): pass\ndef second(): pass\n"),
    ]);
    let bindings = index_python(&files);
    flask_facts(&bindings, "app.py", &SetupContext::default());
    let refusals = bindings.import_refusals();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].reason, MultiWrite);
}

#[test]
fn qualified_import_preserves_router_object_attribute_escape() {
    for (escape, tokens_known) in [("", true), ("sink(setup.app.url_map)\n", false)] {
        let app = format!("import setup\n{escape}@setup.app.get('/x/<int:id>')\ndef f(id): pass\n");
        let files = parsed(&[("app.py", &app), ("setup.py", APP)]);
        let bindings = index_python(&files);
        let events = flask_facts(&bindings, "app.py", &SetupContext::default());
        let [FlaskEvent::Rule(rule)] = events.as_slice() else {
            panic!("one rule");
        };
        assert_eq!(rule.path_tokens.is_some(), tokens_known);
    }
}

#[test]
fn refused_star_export_preserves_possible_view_metadata_owner() {
    let views = format!("{APP}def f(): pass\nimport helper\napp.add_url_rule('/x', view_func=f)\n");
    let files = parsed(&[
        ("views.py", &views),
        ("reexport.py", "from views import *\n"),
        (
            "helper.py",
            "from reexport import f\nf.methods = ['POST']\n",
        ),
    ]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "views.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = events.as_slice() else {
        panic!("one rule");
    };
    assert!(matches!(
        rule.route.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
    let refusals = bindings.import_refusals();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].reason, StarImport);
    assert_eq!(refusals[0].file, "helper.py");
}
