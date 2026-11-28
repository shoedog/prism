//! Round-1 fold of PR #928 (lane 12; reports `rev-p2-sol-1.md` S1-S6 and the Opus findings
//! F1-F5), one RED row per finding beside a control that keeps the proof. Every finding is
//! IN-CONTRACT (a proof relied on a name that is not one static binding at the read) and
//! CONSTRUCTED (no census incidence).
//!
//! Row names: `<finding>_<shape>`; `<finding>_control_<shape>` keeps the proof (or the withheld
//! baseline). A fixture that does not parse cleanly is a failed probe.
use super::*;
use crate::framework_entries::routes::python::flask::FlaskRule;
use crate::framework_entries::routes::receivers::Clause;

pub(super) const APP: &str = "from flask import Flask\napp = Flask(__name__)\n";

pub(super) fn clean(files: &BTreeMap<String, ParsedFile>) {
    for (path, file) in files {
        assert!(
            !file.tree.root_node().has_error(),
            "fixture {path} does not parse cleanly (a failed probe, not a row)"
        );
    }
}

/// Per rule event of `app.py`: whether its receiver is proved, and the callback statuses.
pub(super) fn rules_of(sources: &[(&str, &str)]) -> Vec<(bool, Vec<CallbackStatus>)> {
    let files = parsed(sources);
    clean(&files);
    let bindings = index_python(&files);
    flask_facts(&bindings, "app.py", &SetupContext::default())
        .iter()
        .filter_map(|event| match event {
            FlaskEvent::Rule(rule) => Some((
                matches!(rule.route.receiver, Receiver::Router { .. }),
                rule.route
                    .callbacks
                    .iter()
                    .map(|callback| callback.status.clone())
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

/// The (file, name) of every resolved callback of every rule.
pub(super) fn resolved(sources: &[(&str, &str)]) -> Vec<(String, String)> {
    rules_of(sources)
        .into_iter()
        .flat_map(|(_, callbacks)| callbacks)
        .filter_map(|status| match status {
            CallbackStatus::Resolved { identity, .. } => Some((identity.file, identity.name)),
            _ => None,
        })
        .collect()
}

pub(super) fn pair(file: &str, name: &str) -> (String, String) {
    (file.to_owned(), name.to_owned())
}

pub(super) fn proved_rules(sources: &[(&str, &str)]) -> usize {
    rules_of(sources)
        .iter()
        .filter(|(proved, callbacks)| {
            *proved
                && callbacks
                    .iter()
                    .all(|status| matches!(status, CallbackStatus::Resolved { .. }))
        })
        .count()
}

// S1: a setup-local import displaced by a LATER local def: the reaching occurrence is the
// def, so the callback is the local `handler`, never the imported `views.old`.
#[test]
fn s1_setup_local_import_displaced_by_a_later_def() {
    let app = format!(
        "{APP}def install():\n    from views import old as handler\n    def handler(): return 'local'\n    app.add_url_rule('/x', view_func=handler)\ninstall()\n"
    );
    let got = resolved(&[("app.py", &app), ("views.py", "def old(): pass\n")]);
    assert!(!got.contains(&pair("views.py", "old")), "{got:?}");
}
#[test]
fn s1_control_setup_local_import_alone_keeps_the_proof() {
    let app = format!(
        "{APP}def install():\n    from views import old as handler\n    app.add_url_rule('/x', view_func=handler)\ninstall()\n"
    );
    let got = resolved(&[("app.py", &app), ("views.py", "def old(): pass\n")]);
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn s1_control_def_before_the_import_is_displaced_by_the_import() {
    let app = format!(
        "{APP}def install():\n    def handler(): return 'local'\n    from views import old as handler\n    app.add_url_rule('/x', view_func=handler)\ninstall()\n"
    );
    let got = resolved(&[("app.py", &app), ("views.py", "def old(): pass\n")]);
    assert_eq!(got, [pair("views.py", "old")]);
}

// S2: an in-repo star copies the child module that package import syntax installed.
fn star_child(init: &str, capture: &str) -> Vec<(String, String)> {
    let app = format!("{APP}{capture}app.add_url_rule('/x', view_func=child.handle)\n");
    resolved(&[
        ("app.py", &app),
        ("pkg/__init__.py", init),
        ("pkg/child.py", "def handle(): pass\nmarker = 1\n"),
        ("old.py", "def handle(): pass\n"),
    ])
}
const STAR_CAPTURE: &str = "import old as child\nfrom pkg import *\n";
#[test]
fn s2_star_misses_the_child_installed_by_a_package_import() {
    for init in [
        "from .child import marker\n",
        "from . import child\n",
        "import pkg.child\n",
    ] {
        let got = star_child(init, STAR_CAPTURE);
        assert!(!got.contains(&pair("old.py", "handle")), "{init}: {got:?}");
    }
}
#[test]
fn s2_control_all_excluding_the_child_clears() {
    let got = star_child(
        "from .child import marker\n__all__ = ['marker']\n",
        STAR_CAPTURE,
    );
    assert_eq!(got, [pair("old.py", "handle")]);
}
#[test]
fn s2_control_no_initializer_import_of_the_child() {
    let got = star_child("marker = 1\n", STAR_CAPTURE);
    assert_eq!(got, [pair("old.py", "handle")]);
}

// S3: a store on a LATER local binding is not a store on the earlier Flask allocation.
pub(super) fn roots(source: &str) -> usize {
    let files = parsed(&[("app.py", source)]);
    clean(&files);
    index_python(&files)
        .roots(crate::framework_entries::routes::Framework::Flask, None)
        .len()
}
#[test]
fn s3_store_on_a_later_local_binding_leaves_the_earlier_allocation_a_root() {
    let source = "from flask import Flask\nfrom types import SimpleNamespace\ndef make():\n    app = Flask(__name__)\n    app.add_url_rule('/x', endpoint='x')\n    saved = app\n    app = SimpleNamespace()\n    app.label = 1\n    return saved\nexported = make()\n";
    assert_eq!(roots(source), 1);
}
#[test]
fn s3_control_store_before_the_rebind_mutates_the_allocation() {
    let source = "from flask import Flask\nfrom types import SimpleNamespace\ndef make():\n    app = Flask(__name__)\n    app.label = 1\n    saved = app\n    app = SimpleNamespace()\n    return saved\nexported = make()\n";
    assert_eq!(roots(source), 0);
}
#[test]
fn s3_control_no_store_at_all() {
    let source = "from flask import Flask\nfrom types import SimpleNamespace\ndef make():\n    app = Flask(__name__)\n    app.add_url_rule('/x', endpoint='x')\n    saved = app\n    app = SimpleNamespace()\n    return saved\nexported = make()\n";
    assert_eq!(roots(source), 1);
}

// S4: `record_once` state formal behind a nested `nonlocal` redirect.
fn record_once(swap: &str) -> usize {
    let source = format!(
        "from flask import Blueprint, Flask\napp = Flask(__name__)\nbp = Blueprint('b', __name__)\nclass Sink:\n    def add_url_rule(self, *a, **k): pass\ndef f(): pass\n@bp.record_once\ndef install(state):\n{swap}    state.add_url_rule('/ghost', view_func=f)\napp.register_blueprint(bp)\n"
    );
    let files = parsed(&[("app.py", &source)]);
    clean(&files);
    let bindings = index_python(&files);
    flask_facts(&bindings, "app.py", &SetupContext::default())
        .iter()
        .filter(|event| match event {
            FlaskEvent::Rule(rule) => {
                let rule: &FlaskRule<'_> = rule;
                rule.record_once && matches!(rule.route.receiver, Receiver::Router { .. })
            }
            _ => false,
        })
        .count()
}
#[test]
fn s4_record_once_state_redirected_by_a_nested_nonlocal_write() {
    let swap = "    def swap():\n        nonlocal state\n        state = Sink()\n    swap()\n";
    assert_eq!(record_once(swap), 0);
}
#[test]
fn s4_control_nested_write_to_its_own_local_state() {
    let swap = "    def swap():\n        state = Sink()\n    swap()\n";
    assert_eq!(record_once(swap), 1);
}
#[test]
fn s4_control_no_nested_write() {
    assert_eq!(record_once(""), 1);
}

// S5: a class-private (mangled) formal shadows the module function of the mangled name.
#[test]
fn s5_mangled_formal_does_not_fall_through_to_a_module_function() {
    let app = format!(
        "{APP}def _C__self(): pass\nclass C:\n    def install(__self):\n        app.add_url_rule('/x', endpoint='x', view_func=__self)\nC().install()\n"
    );
    let got = resolved(&[("app.py", &app)]);
    assert!(!got.contains(&pair("app.py", "_C__self")), "{got:?}");
}
#[test]
fn s5_control_ordinary_formal_and_unmangled_module_function() {
    let app = format!(
        "{APP}def view(): pass\nclass C:\n    def install(self):\n        app.add_url_rule('/x', endpoint='x', view_func=view)\nC().install()\n"
    );
    assert_eq!(resolved(&[("app.py", &app)]), [pair("app.py", "view")]);
}

// S6 / F3: a module-level `global K` is no redirect; the module scope still binds K.
pub(super) fn config_methods(global: &str) -> MethodSet {
    let source = format!(
        "from flask import Flask\n{global}app = Flask(__name__)\napp.config = {{'PROVIDE_AUTOMATIC_OPTIONS': False}}\n@app.route('/x')\ndef f(): pass\n"
    );
    let files = parsed(&[("app.py", &source)]);
    clean(&files);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let [FlaskEvent::Rule(rule)] = facts.as_slice() else {
        panic!("expected one rule");
    };
    rule.effective_methods.clone()
}
#[test]
fn s6_module_level_global_keeps_the_config_replacement_barrier() {
    let control = config_methods("");
    assert!(matches!(control, MethodSet::Unknown { .. }), "{control:?}");
    assert_eq!(config_methods("global app\n"), control);
}

// F1 (Opus): function and class scope vs loop back edges. PRICED, no new machinery: a
// registration whose read sits in a for/while body is a RuntimeGeneratedRegistration
// diagnostic and never a rule event (python/flask.rs `loop_generated`, in a setup function
// and in a factory alike); a registration in a class body belongs to no active scope; the
// mutation index reads every binding occurrence with no position filter. So no proof relies
// on an in-loop positional read, and the pins below are GREEN on arrival. The one wrong proof
// the loop shape reached is a registration AFTER the loop whose name a loop body rebinds: the
// earlier import won over the later reaching write (the S1 mechanism), pinned RED here.
fn loop_rows(body: &str) -> Vec<(String, String)> {
    let app = format!("{APP}def install():\n{body}install()\n");
    resolved(&[("app.py", &app), ("views.py", "def old(): pass\n")])
}
pub(super) fn diagnostics_of(body: &str) -> (usize, usize) {
    let app = format!("{APP}def install():\n{body}install()\n");
    let files = parsed(&[("app.py", &app), ("views.py", "def old(): pass\n")]);
    clean(&files);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let rules = facts
        .iter()
        .filter(|event| matches!(event, FlaskEvent::Rule(_)))
        .count();
    let generated = facts
        .iter()
        .filter(|event| {
            matches!(event, FlaskEvent::Diagnostic(diagnostic)
                if diagnostic.reason == Reason::RuntimeGeneratedRegistration)
        })
        .count();
    (rules, generated)
}
#[test]
fn f1_setup_local_import_rebound_in_a_loop_before_a_registration_after_it() {
    let got = loop_rows("    from views import old as handler\n    for _ in range(2):\n        note(handler)\n        handler = swap()\n    app.add_url_rule('/x', endpoint='x', view_func=handler)\n");
    assert!(!got.contains(&pair("views.py", "old")), "{got:?}");
}
#[test]
fn f1_control_loop_reads_without_rebinding_keeps_the_import() {
    let got = loop_rows("    from views import old as handler\n    for _ in range(2):\n        note(handler)\n    app.add_url_rule('/x', endpoint='x', view_func=handler)\n");
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn f1_pin_registration_in_a_loop_body_is_a_diagnostic_never_a_rule() {
    for body in [
        "    from views import old as handler\n    for _ in range(2):\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n        handler = swap()\n",
        "    def handler(): pass\n    while more():\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n        handler = swap()\n",
        "    def handler(): pass\n    for handler in handlers():\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n",
    ] {
        assert_eq!(diagnostics_of(body), (0, 1), "{body}");
    }
}
#[test]
fn f1_control_registration_outside_a_loop_is_a_rule() {
    let body = "    def handler(): pass\n    for _ in range(2):\n        note(handler)\n    app.add_url_rule('/x', endpoint='x', view_func=handler)\n";
    assert_eq!(diagnostics_of(body), (1, 0));
}
// Class scope: a registration in a class body belongs to no active declaration scope, so a
// class-body loop rebinding produces no rule event at all (nothing to withhold, pinned).
#[test]
fn f1_pin_class_body_loop_rebinding_registers_nothing() {
    let app = format!(
        "{APP}class C:\n    handler = 1\n    for _ in range(2):\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n        handler = swap()\n"
    );
    assert!(rules_of(&[("app.py", &app)]).is_empty());
}

// F2: a nested external star of an in-repo star is not seen by the capture side.
#[test]
fn f2_transitive_external_star_does_not_refuse_the_capturing_module() {
    let app = format!("from lib import *\n{APP}@app.route('/x')\ndef f(): pass\n");
    let sources = [
        ("app.py", app.as_str()),
        ("lib.py", "from numpy import *\nclass Victim: pass\n"),
    ];
    assert_eq!(proved_rules(&sources), 1);
}
#[test]
fn f2_control_in_repo_star_chain_that_binds_the_key_withholds() {
    let app = format!("from lib import *\n{APP}@app.route('/x')\ndef f(): pass\n");
    let sources = [
        ("app.py", app.as_str()),
        ("lib.py", "from numpy import *\nfrom deeper import *\n"),
        ("deeper.py", "app = 1\n"),
    ];
    assert_eq!(proved_rules(&sources), 0);
}

// F5: `__annotate__` (Python 3.14) is a module-system name; chained assignment.
#[test]
fn f5_annotate_is_a_module_system_name() {
    let source = "def __annotate__(format): return {}\n";
    let files = parsed(&[("app.py", source)]);
    let bindings = index_python(&files);
    let file = &files["app.py"];
    assert_eq!(
        bindings.certify_in(file, file.tree.root_node(), "__annotate__"),
        Err(Clause::Count)
    );
}
#[test]
fn f5_chained_assignment_router_is_conservatively_unproven() {
    let source =
        "from flask import Flask\napp = bp = Flask(__name__)\n@app.route('/x')\ndef f(): pass\n";
    assert_eq!(proved_rules(&[("app.py", source)]), 0);
}

// One direct `Result<(), Clause>` assertion per clause, so a missed binding occurrence cannot
// pass as "no event".
pub(super) fn module_verdict(sources: &[(&str, &str)], key: &str) -> Result<(), Clause> {
    let files = parsed(sources);
    let bindings = index_python(&files);
    let file = &files["app.py"];
    bindings.certify_in(file, file.tree.root_node(), key)
}
#[test]
fn certify_clauses_one_assertion_each() {
    let one = |source: &str| module_verdict(&[("app.py", source)], "k");
    assert_eq!(one("k = 1\n"), Ok(()));
    assert_eq!(one("def k(): pass\n"), Ok(()));
    assert_eq!(one("from m import k\n"), Ok(()));
    assert_eq!(one("class k: pass\n"), Ok(()));
    // (e) parse error
    assert_eq!(one("k = 1\nx = (\n"), Err(Clause::ParseError));
    // (a) zero and two-plus bindings
    assert_eq!(one("x = 1\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\nk = 2\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\nfor k in x: pass\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\ndel k\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\nwith a as k: pass\n"), Err(Clause::Count));
    assert_eq!(
        one("k = 1\ntry: pass\nexcept E as k: pass\n"),
        Err(Clause::Count)
    );
    assert_eq!(one("k = 1\n(k := 2)\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\nk += 1\n"), Err(Clause::Count));
    assert_eq!(one("k = 1\ntype k = int\n"), Err(Clause::Count));
    assert_eq!(
        one("k = 1\nmatch x:\n    case [k]: pass\n"),
        Err(Clause::Count)
    );
    // (d) module-system names count as one more binding
    for name in ["__doc__", "__name__", "__file__", "__annotate__"] {
        let source = format!("{name} = 1\n");
        let verdict = module_verdict(&[("app.py", &source)], name);
        assert_eq!(verdict, Err(Clause::Count), "{name}");
    }
    // redirects
    assert_eq!(
        one("k = 1\ndef f():\n    global k\n    k = 2\n"),
        Err(Clause::Redirect)
    );
    // (c) a star the repository says can bind the key
    let star =
        |lib: &str| module_verdict(&[("app.py", "from lib import *\n"), ("lib.py", lib)], "k");
    assert_eq!(star("k = 1\n"), Err(Clause::Star));
    assert_eq!(star("x = 1\n"), Err(Clause::Count));
    let with_star = module_verdict(
        &[
            ("app.py", "k = 1\nfrom lib import *\n"),
            ("lib.py", "k = 2\n"),
        ],
        "k",
    );
    assert_eq!(with_star, Err(Clause::Star));
    // (b) kind of the single binding
    assert_eq!(one("if c:\n    k = 1\n"), Err(Clause::Kind));
    assert_eq!(one("k, j = 1, 2\n"), Err(Clause::Kind));
    assert_eq!(one("for k in x: pass\n"), Err(Clause::Kind));
    assert_eq!(one("(k := 1)\n"), Err(Clause::Kind));
    assert_eq!(one("k: int\n"), Err(Clause::Count));
    assert_eq!(
        one("try:\n    import k\nexcept E:\n    pass\n"),
        Err(Clause::Kind)
    );
}
#[test]
fn certify_function_scope_redirect_and_unrelated_scopes() {
    let source =
        "def f():\n    k = 1\n    def g():\n        nonlocal k\n        k = 2\ndef h():\n    k = 3\n";
    let files = parsed(&[("app.py", source)]);
    let bindings = index_python(&files);
    let file = &files["app.py"];
    let functions: Vec<_> = bindings
        .nodes("app.py")
        .iter()
        .copied()
        .filter(|node| node.kind() == "function_definition")
        .collect();
    let [f, _g, h] = functions.as_slice() else {
        panic!("expected three functions")
    };
    assert_eq!(bindings.certify_in(file, *f, "k"), Err(Clause::Redirect));
    assert_eq!(bindings.certify_in(file, *h, "k"), Ok(()));
}

// Class-setup method selection (`C().install()`) is a separate proof rule of the activation,
// not the certificate: the class body holds only defs, so the LAST `install` def is the one
// the instance reaches. Pinned.
#[test]
fn class_setup_selects_the_last_method_def() {
    let app = format!(
        "{APP}class C:\n    def install(self):\n        app.add_url_rule('/first', endpoint='first')\n    def install(self):\n        app.add_url_rule('/second', endpoint='second')\nC().install()\n"
    );
    let files = parsed(&[("app.py", &app)]);
    let bindings = index_python(&files);
    let facts = flask_facts(&bindings, "app.py", &SetupContext::default());
    let endpoints: Vec<_> = facts
        .iter()
        .filter_map(|event| match event {
            FlaskEvent::Rule(rule) => rule.route.endpoint.clone(),
            _ => None,
        })
        .collect();
    assert_eq!(endpoints, ["second"]);
}
