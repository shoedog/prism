use super::*;
use crate::framework_entries::routes::{
    python::flask::{FlaskMount, FlaskRule},
    receivers::refusals::ImportRefusalReason,
};

const OWNER: &str = "from flask import Blueprint\nbp = Blueprint('bp', __name__)\n";
const IMPORT: &str = "from pkg.owner import bp";
const VIEWS: &str = "@bp.route('/')\ndef view(): pass\n";
const MOUNTS: &str = "from flask import Blueprint\nfrom pkg.owner import bp\nouter = Blueprint('outer', __name__)\nouter.register_blueprint(bp)\n";
const PARAMETER_MOUNT: &str =
    "from pkg.owner import bp\ndef init_app(app):\n    app.register_blueprint(bp)\n";

fn fixture(append: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let view = format!("{IMPORT}\n{VIEWS}");
    let mut files = parsed(&[
        ("pkg/__init__.py", ""),
        ("pkg/owner.py", OWNER),
        ("pkg/views.py", &view),
    ]);
    files.extend(parsed(append));
    files
}

#[track_caller]
fn rule<'a>(events: &'a [FlaskEvent<'_>]) -> &'a FlaskRule<'a> {
    let [FlaskEvent::Rule(rule)] = events else {
        panic!("expected one Rule event, got {} events", events.len());
    };
    rule
}

#[track_caller]
fn mount<'a>(events: &'a [FlaskEvent<'_>]) -> &'a FlaskMount<'a> {
    let [FlaskEvent::Mount(mount)] = events else {
        panic!("expected one Mount event, got {} events", events.len());
    };
    mount
}

fn assert_clear_rule(rule: &FlaskRule<'_>) {
    assert!(
        matches!(&rule.route.receiver, Receiver::Router { identity, .. }
        if identity.file == "pkg/owner.py")
    );
    assert!(
        matches!(&rule.route.callbacks[0].status, CallbackStatus::Resolved { identity, .. }
        if identity.name == "view")
    );
    assert_eq!(
        rule.effective_endpoint,
        EffectiveEndpoint::Known("view".into())
    );
    assert_eq!(
        rule.effective_methods,
        MethodSet::known(["GET", "HEAD", "OPTIONS"])
    );
}

fn assert_metadata_withheld(rule: &FlaskRule<'_>) {
    assert_eq!(rule.effective_endpoint, EffectiveEndpoint::Unknown);
    assert!(matches!(rule.effective_methods, MethodSet::Unknown { .. }));
}

fn unrelated_write(body: &str) -> String {
    format!(
        "from flask import Flask\n{IMPORT}\ndef make(name):\n    return Flask(name)\ndef replacement(): pass\ndef test_only():\n    unrelated = make(__name__)\n    {body}\n"
    )
}

fn assert_unrelated_facts_survive(body: &str) {
    let files = fixture(&[
        ("test_app.py", &unrelated_write(body)),
        ("pkg/mounts.py", MOUNTS),
    ]);
    let bindings = index_python(&files);
    let context = SetupContext::default();
    assert_clear_rule(rule(&flask_facts(&bindings, "pkg/views.py", &context)));
    let mounts = flask_facts(&bindings, "pkg/mounts.py", &context);
    let mount = mount(&mounts);
    assert!(matches!(&mount.parent, Receiver::Router { identity, .. }
        if identity.file == "pkg/mounts.py"));
    assert!(matches!(&mount.child, Receiver::Router { identity, .. }
        if identity.file == "pkg/owner.py"));
    assert!(flask_facts(&bindings, "test_app.py", &context).is_empty());
}

#[test]
fn unrelated_registry_write_keeps_the_imported_rule_and_mount_roles() {
    assert_unrelated_facts_survive("unrelated.view_functions['view'] = replacement");
}

#[test]
fn pass_in_place_of_the_registry_write() {
    assert_unrelated_facts_survive("pass");
}

#[test]
fn write_to_the_blueprint_withholds_only_the_matching_views_metadata() {
    let views = format!("{IMPORT}\n{VIEWS}\n@bp.route('/other')\ndef other(): pass\n");
    let write = format!("{IMPORT}\ndef replacement(): pass\ndef patch():\n    bp.view_functions['view'] = replacement\n");
    let files = fixture(&[("pkg/views.py", &views), ("patch.py", &write)]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "pkg/views.py", &SetupContext::default());
    let [FlaskEvent::Rule(affected), FlaskEvent::Rule(other)] = events.as_slice() else {
        panic!("expected both declaration sites");
    };
    assert!(matches!(affected.route.receiver, Receiver::Router { .. }));
    assert!(matches!(
        affected.route.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert_metadata_withheld(affected);
    assert_eq!(
        other.effective_endpoint,
        EffectiveEndpoint::Known("other".into())
    );
    assert_eq!(
        other.effective_methods,
        MethodSet::known(["GET", "HEAD", "OPTIONS"])
    );
}

#[test]
fn registry_write_to_one_factory_result_keeps_the_other_results_metadata() {
    let files = parsed(&[(
        "app.py",
        "from flask import Flask\ndef make():\n    app = Flask(__name__)\n    @app.route('/')\n    def view(): pass\n    return app\nfirst = make()\nsecond = make()\nfirst.view_functions['view'] = replacement\n",
    )]);
    let file = &files["app.py"];
    let bindings = index_python(&files);
    let roots = bindings.roots(Framework::Flask, None);
    let first = RootSelector::parse("app:first")
        .unwrap()
        .resolve(&roots)
        .unwrap();
    let second = RootSelector::parse("app:second")
        .unwrap()
        .resolve(&roots)
        .unwrap();
    assert_eq!(first.construction_byte, second.construction_byte);
    assert_ne!(first.invocations, second.invocations);
    let factory = Some(PythonMember {
        file,
        node: file.tree.root_node().named_child(1).unwrap(),
    });
    let first_facts = flask_facts(
        &bindings,
        "app.py",
        &SetupContext {
            factory,
            frames: first.invocations.clone(),
        },
    );
    let second_facts = flask_facts(
        &bindings,
        "app.py",
        &SetupContext {
            factory,
            frames: second.invocations.clone(),
        },
    );
    assert_metadata_withheld(rule(&first_facts));
    let second = rule(&second_facts);
    assert_eq!(
        second.effective_endpoint,
        EffectiveEndpoint::Known("view".into())
    );
    assert_eq!(
        second.effective_methods,
        MethodSet::known(["GET", "HEAD", "OPTIONS"])
    );
}

#[test]
fn registry_write_keeps_both_allocations_from_a_refused_import() {
    let owner = format!(
        "{OWNER}{VIEWS}\nbp = Blueprint('second', __name__)\n@bp.route('/later')\ndef later(): pass\nother = Blueprint('other', __name__)\n@other.route('/other')\ndef other_view(): pass\n",
    );
    let write = format!("{IMPORT}\nbp.view_functions[key] = replacement\n");
    let files = fixture(&[("pkg/owner.py", &owner), ("patch.py", &write)]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "pkg/owner.py", &SetupContext::default());
    let [FlaskEvent::Rule(first), FlaskEvent::Rule(second), FlaskEvent::Rule(other)] =
        events.as_slice()
    else {
        panic!("expected three declaration sites");
    };
    for candidate in [first, second] {
        assert!(matches!(candidate.route.receiver, Receiver::Router { .. }));
        assert_metadata_withheld(candidate);
    }
    assert_eq!(
        other.effective_endpoint,
        EffectiveEndpoint::Known("other_view".into())
    );
    let refusals = bindings.import_refusals();
    let captures: Vec<_> = refusals
        .iter()
        .map(|refusal| (refusal.file.as_str(), refusal.reason))
        .collect();
    assert_eq!(
        captures,
        [
            ("patch.py", ImportRefusalReason::MultiWrite),
            ("pkg/views.py", ImportRefusalReason::MultiWrite),
        ]
    );
}

#[test]
fn opaque_write_to_the_same_blueprint_already_withholds_its_view_metadata() {
    let owner = format!("{OWNER}{VIEWS}\ndef replacement(): pass\ndef patch(target):\n    target.view_functions['view'] = replacement\npatch(bp)\n");
    let files = fixture(&[("pkg/owner.py", &owner)]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "pkg/owner.py", &SetupContext::default());
    assert_metadata_withheld(rule(&events));
}

fn factory_source(setup: &str) -> String {
    format!(
        "from flask import Flask\n{IMPORT}\ndef make():\n    app = Flask(__name__)\n{setup}    app.register_blueprint(bp)\n    return app\n"
    )
}

fn factory_mount<R>(setup: &str, inspect: impl FnOnce(&FlaskMount<'_>) -> R) -> R {
    let files = fixture(&[("app.py", &factory_source(setup))]);
    let file = &files["app.py"];
    let context = SetupContext {
        factory: Some(PythonMember {
            file,
            node: file.tree.root_node().named_child(2).unwrap(),
        }),
        frames: vec![Invocation::Entry {
            entry: "app:make()".into(),
        }],
    };
    let bindings = index_python(&files);
    inspect(mount(&flask_facts(&bindings, "app.py", &context)))
}

#[test]
fn clean_factory_mount_and_imported_rule() {
    factory_mount("", |mount| {
        assert!(matches!(mount.parent, Receiver::Router { .. }));
        assert!(matches!(mount.child, Receiver::Router { .. }));
        assert_eq!(mount.prefix, MountPrefix::Absent);
    });
    let files = fixture(&[("app.py", &factory_source(""))]);
    assert_clear_rule(rule(&flask_facts(
        &index_python(&files),
        "pkg/views.py",
        &SetupContext::default(),
    )));
}

#[test]
fn escaping_factory_still_withholds_both_mount_roles() {
    factory_mount("    extension.init_app(app)\n", |mount| {
        assert_eq!(
            mount.parent,
            Receiver::Unproven {
                reason: Reason::ReceiverUnproven
            }
        );
        assert_eq!(
            mount.child,
            Receiver::Unproven {
                reason: Reason::MountChildUnproven
            }
        );
    });
}

fn assert_mount_does_not_erase_rule(source: &str) {
    let files = fixture(&[("app.py", source)]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "pkg/views.py", &SetupContext::default());
    let rule = rule(&events);
    assert_eq!(
        rule.route
            .declaration
            .as_ref()
            .unwrap()
            .path
            .literal
            .as_deref(),
        Some("/")
    );
    assert_eq!(
        rule.route.receiver,
        Receiver::Unproven {
            reason: Reason::ReceiverUnproven
        }
    );
    assert_metadata_withheld(rule);
    assert!(bindings.import_refusals().is_empty());
}

#[test]
fn opaque_factory_setup_keeps_the_imported_blueprint_declaration() {
    assert_mount_does_not_erase_rule(&factory_source("    extension.init_app(app)\n"));
}

#[test]
fn escaped_factory_withholds_the_effective_mount_prefix() {
    factory_mount("    extension.init_app(app)\n", |mount| {
        assert!(matches!(mount.prefix, MountPrefix::Unevaluated { .. }));
    });
}

#[test]
fn parameter_mount_keeps_the_imported_blueprint_declaration() {
    assert_mount_does_not_erase_rule(PARAMETER_MOUNT);
}

#[test]
fn direct_import_only_emits_a_proved_rule() {
    let files = fixture(&[]);
    let bindings = index_python(&files);
    assert_clear_rule(rule(&flask_facts(
        &bindings,
        "pkg/views.py",
        &SetupContext::default(),
    )));
    assert!(bindings.import_refusals().is_empty());
}

#[test]
fn terminal_reexport_keeps_the_rule_with_an_uncounted_receiver_refusal() {
    assert_terminal_reexport("from pkg.owner import bp\n");
}

#[test]
fn terminal_assignment_reexport_keeps_the_rule_with_an_uncounted_receiver_refusal() {
    assert_terminal_reexport("from pkg.owner import bp as _bp\nbp = _bp\n");
}

fn assert_terminal_reexport(initializer: &str) {
    let views = format!("from pkg import bp\n{VIEWS}");
    let files = fixture(&[("pkg/__init__.py", initializer), ("pkg/views.py", &views)]);
    let bindings = index_python(&files);
    let events = flask_facts(&bindings, "pkg/views.py", &SetupContext::default());
    let rule = rule(&events);
    assert_eq!(
        rule.route
            .declaration
            .as_ref()
            .unwrap()
            .path
            .literal
            .as_deref(),
        Some("/")
    );
    let receiver = serde_json::to_value(&rule.route.receiver).unwrap();
    assert_eq!(
        receiver,
        serde_json::json!({"status": "unproven", "reason": "terminal_reexport"})
    );
    assert_metadata_withheld(rule);
    assert!(bindings.import_refusals().is_empty());
}
