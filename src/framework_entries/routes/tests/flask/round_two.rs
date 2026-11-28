//! Closing fold of PR #928 (lane 12; `rev-p2-sol-2.md` findings 1-6 and the Opus findings
//! N1, N3, N4), one RED row per finding beside a control that keeps the proof. Every finding is
//! IN-CONTRACT and CONSTRUCTED. Two classes fail closed at the certificate (A: a function-scope
//! import key bound more than once; B: a star whose package export set is not determined).
//!
//! Row names: `<item>_<shape>`; `<item>_control_<shape>` keeps the proof (or the baseline). A
//! fixture that does not parse cleanly is a failed probe.
use super::round_one::{clean, config_methods, pair, proved_rules, resolved, roots, rules_of, APP};
use super::*;
use crate::framework_entries::routes::receivers::Clause;

// A: a function-scope key bound more than once with an import among the bindings fails the
// Count clause; the certificate withholds the proof, no first/last selection.
fn install(body: &str) -> Vec<(String, String)> {
    let app = format!("{APP}def install():\n{body}install()\n");
    resolved(&[
        ("app.py", &app),
        ("views.py", "def old(): pass\ndef new(): pass\n"),
    ])
}
const REG: &str = "    app.add_url_rule('/x', endpoint='x', view_func=handler)\n";
#[test]
fn a_two_aliases_in_one_import_statement() {
    let got = install(&format!(
        "    from views import old as handler, new as handler\n{REG}"
    ));
    assert!(got.is_empty(), "{got:?}");
}
#[test]
fn a_two_import_statements() {
    let got = install(&format!(
        "    from views import old as handler\n    from views import new as handler\n{REG}"
    ));
    assert!(got.is_empty(), "{got:?}");
}
#[test]
fn a_import_then_a_later_assignment() {
    let got = install(&format!(
        "    from views import old as handler\n    handler = swap()\n{REG}"
    ));
    assert!(got.is_empty(), "{got:?}");
}
#[test]
fn a_import_then_a_later_def() {
    let got = install(&format!(
        "    from views import old as handler\n    def handler(): pass\n{REG}"
    ));
    assert!(!got.contains(&pair("views.py", "old")), "{got:?}");
}
#[test]
fn a_control_one_binding_per_key() {
    let got = install(&format!(
        "    from views import old as handler, new as other\n{REG}"
    ));
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn a_control_def_before_the_import_keeps_the_import() {
    let got = install(&format!(
        "    def handler(): pass\n    from views import old as handler\n{REG}"
    ));
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn a_control_same_spelling_in_another_scope() {
    let got = install(&format!(
        "    from views import old as handler\n    def other():\n        handler = swap()\n{REG}"
    ));
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn a_certificate_verdict_of_a_function_scope_import() {
    let source = "def f():\n    from m import a as k, b as k\ndef g():\n    from m import a as k\n";
    let files = parsed(&[("app.py", source)]);
    let bindings = index_python(&files);
    let file = &files["app.py"];
    let functions: Vec<_> = bindings
        .nodes("app.py")
        .iter()
        .copied()
        .filter(|node| node.kind() == "function_definition")
        .collect();
    let [f, g] = functions.as_slice() else {
        panic!("expected two functions")
    };
    assert_eq!(bindings.certify_in(file, *f, "k"), Err(Clause::Count));
    assert_eq!(bindings.certify_in(file, *g, "k"), Ok(()));
}

// B: a star contributes a name only when the exporting module's export set is determined;
// a package initializer whose imports (transitively) install children leaves it open.
fn star_pkg(init: &str, bootstrap: &str, extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let app = format!(
        "{APP}import old as child\nfrom pkg import *\napp.add_url_rule('/x', endpoint='x', view_func=child.handle)\n"
    );
    let mut sources = vec![
        ("app.py", app.as_str()),
        ("pkg/__init__.py", init),
        ("pkg/bootstrap.py", bootstrap),
        ("pkg/child.py", "def handle(): pass\nmarker = 1\n"),
        ("old.py", "def handle(): pass\n"),
    ];
    sources.extend_from_slice(extra);
    resolved(&sources)
}
const BOOT_INIT: &str = "from .bootstrap import marker\n";
#[test]
fn b_star_misses_a_child_installed_by_a_transitive_import() {
    let got = star_pkg(BOOT_INIT, "import pkg.child\nmarker = 1\n", &[]);
    assert!(!got.contains(&pair("old.py", "handle")), "{got:?}");
}
#[test]
fn b_star_misses_a_child_installed_two_modules_down() {
    let got = star_pkg(
        BOOT_INIT,
        "from . import mid\nmarker = 1\n",
        &[("pkg/mid.py", "from pkg.child import marker as m\n")],
    );
    assert!(!got.contains(&pair("old.py", "handle")), "{got:?}");
}
#[test]
fn b_star_beyond_the_depth_cap_is_undetermined() {
    let mut files: Vec<(String, String)> = Vec::new();
    for index in 0..30 {
        let next = if index == 29 {
            "import pkg.child\n".to_owned()
        } else {
            format!("from . import m{}\n", index + 1)
        };
        files.push((format!("pkg/m{index}.py"), next));
    }
    let extra: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let got = star_pkg("from . import m0\n", "marker = 1\n", &extra);
    assert!(!got.contains(&pair("old.py", "handle")), "{got:?}");
}
#[test]
fn b_control_all_excluding_the_child_determines_the_export_set() {
    let init = "from .bootstrap import marker\n__all__ = ['marker']\n";
    let got = star_pkg(init, "import pkg.child\nmarker = 1\n", &[]);
    assert_eq!(got, [pair("old.py", "handle")]);
}
#[test]
fn b_control_initializer_chain_that_never_installs_the_child() {
    let got = star_pkg(BOOT_INIT, "marker = 1\n", &[]);
    assert_eq!(got, [pair("old.py", "handle")]);
}
#[test]
fn b_control_initializer_import_cycle_without_the_child() {
    let got = star_pkg(BOOT_INIT, "import pkg\nmarker = 1\n", &[]);
    assert_eq!(got, [pair("old.py", "handle")]);
}
#[test]
fn b_control_plain_module_exports_its_own_bindings_only() {
    let app = format!(
        "{APP}import old as child\nfrom lib import *\napp.add_url_rule('/x', endpoint='x', view_func=child.handle)\n"
    );
    let got = resolved(&[
        ("app.py", &app),
        ("lib.py", "from other import marker\nx = 1\n"),
        ("other.py", "import child\nmarker = 1\n"),
        ("old.py", "def handle(): pass\n"),
    ]);
    assert_eq!(got, [pair("old.py", "handle")]);
}

// 1: an except-as or for target is bound on entry to the clause body, so a read inside the
// body sees it, not the earlier import (the compound statement's END is not its activation).
#[test]
fn i1_except_target_is_active_inside_its_clause_body() {
    let got = install(
        "    from views import old as handler\n    try:\n        raise E()\n    except E as handler:\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n",
    );
    assert!(!got.contains(&pair("views.py", "old")), "{got:?}");
}
#[test]
fn i1_control_unrelated_except_target_keeps_the_import() {
    let got = install(
        "    from views import old as handler\n    try:\n        raise E()\n    except E as err:\n        app.add_url_rule('/x', endpoint='x', view_func=handler)\n",
    );
    assert_eq!(got, [pair("views.py", "old")]);
}
fn factory_mounts(body: &str) -> Vec<bool> {
    let source = format!(
        "from flask import Blueprint, Flask\nactual = Blueprint('actual', __name__)\ndef make():\n    app = Flask(__name__)\n{body}    return app\nexported = make()\n"
    );
    let files = parsed(&[("app.py", &source)]);
    clean(&files);
    let file = &files["app.py"];
    let root = file.tree.root_node();
    let factory = root
        .named_children(&mut root.walk())
        .find(|node| node.kind() == "function_definition")
        .unwrap();
    let start = source.rfind("make()").unwrap();
    let context = SetupContext {
        factory: Some(PythonMember {
            file,
            node: factory,
        }),
        frames: vec![Invocation::Call {
            file: "app.py".into(),
            start_byte: start,
            end_byte: start + "make()".len(),
        }],
    };
    flask_facts(&index_python(&files), "app.py", &context)
        .iter()
        .filter_map(|event| match event {
            FlaskEvent::Mount(mount) => Some(matches!(mount.child, Receiver::Router { .. })),
            _ => None,
        })
        .collect()
}
#[test]
fn i1_for_target_is_active_inside_its_loop_body() {
    let proved = factory_mounts(
        "    bp = Blueprint('old', __name__)\n    for bp in [actual]:\n        app.register_blueprint(bp)\n",
    );
    assert!(!proved.contains(&true), "{proved:?}");
}
#[test]
fn i1_control_mount_after_a_plain_binding() {
    let proved =
        factory_mounts("    bp = Blueprint('old', __name__)\n    app.register_blueprint(bp)\n");
    assert_eq!(proved, [true]);
}

// 2: a chained assignment is peeled on the right when enumerating mutation sources.
fn chain_methods(rebind: &str) -> MethodSet {
    let source = format!(
        "from flask import Flask\ndef make():\n    app = Flask(__name__)\n    saved = app\n{rebind}    app.config = {{**app.config, 'PROVIDE_AUTOMATIC_OPTIONS': False}}\n    return saved\nexported = make()\n@exported.route('/x')\ndef f(): pass\n"
    );
    let files = parsed(&[("app.py", &source)]);
    clean(&files);
    let facts = flask_facts(&index_python(&files), "app.py", &SetupContext::default());
    let rules: Vec<_> = facts
        .iter()
        .filter_map(|event| match event {
            FlaskEvent::Rule(rule) => Some(rule.effective_methods.clone()),
            _ => None,
        })
        .collect();
    let [methods] = rules.as_slice() else {
        panic!("expected one rule: {}", rules.len())
    };
    methods.clone()
}
#[test]
fn i2_chained_rebind_keeps_the_config_barrier_of_the_allocation() {
    let direct = chain_methods("");
    assert!(matches!(direct, MethodSet::Unknown { .. }), "{direct:?}");
    assert_eq!(chain_methods("    app = spare = app\n"), direct);
}
#[test]
fn i2_control_config_replacement_on_a_fresh_namespace_is_no_barrier() {
    let methods = chain_methods("    app = SimpleNamespace()\n");
    assert!(!matches!(methods, MethodSet::Unknown { .. }), "{methods:?}");
}

// 3: an unqualified `__class__` read in a method is the implicit class cell.
fn class_cell(method_body: &str) -> Vec<(String, String)> {
    let app = format!(
        "{APP}def __class__(): pass\nclass C:\n    def install(self):\n{method_body}        app.add_url_rule('/x', endpoint='x', view_func=__class__)\nC().install()\n"
    );
    resolved(&[("app.py", &app)])
}
#[test]
fn i3_class_cell_is_not_the_module_function() {
    let got = class_cell("");
    assert!(!got.contains(&pair("app.py", "__class__")), "{got:?}");
}
#[test]
fn i3_control_global_declaration_reads_the_module_name() {
    let got = class_cell("        global __class__\n");
    assert_eq!(got, [pair("app.py", "__class__")]);
}
#[test]
fn i3_control_plain_function_reads_the_module_name() {
    let app = format!(
        "{APP}def __class__(): pass\ndef install():\n    app.add_url_rule('/x', endpoint='x', view_func=__class__)\ninstall()\n"
    );
    assert_eq!(resolved(&[("app.py", &app)]), [pair("app.py", "__class__")]);
}

// 4: a bare annotation is a lexical declaration, not a value write.
#[test]
fn i4_bare_annotation_leaves_the_import_reaching() {
    let got = install(&format!(
        "    from views import old as handler\n    handler: object\n{REG}"
    ));
    assert_eq!(got, [pair("views.py", "old")]);
}
#[test]
fn i4_control_annotation_with_a_value_displaces_the_import() {
    let got = install(&format!(
        "    from views import old as handler\n    handler: object = swap()\n{REG}"
    ));
    assert!(got.is_empty(), "{got:?}");
}
#[test]
fn i4_control_annotation_alone_still_owns_the_name_locally() {
    let app = format!(
        "{APP}def handler(): pass\ndef install():\n    handler: object\n    app.add_url_rule('/x', endpoint='x', view_func=handler)\ninstall()\n"
    );
    assert!(!resolved(&[("app.py", &app)]).contains(&pair("app.py", "handler")));
}

// 5 (Opus N1): a class-private (mangled) `record_once` state formal rebound in the body. A class
// body is never an active scope, so the event stream never reaches this shape; the owner
// function is asked directly.
fn record_once_owner_of(rebind: &str) -> bool {
    let source = format!(
        "from flask import Blueprint\nbp = Blueprint('b', __name__)\nclass C:\n    @bp.record_once\n    def install(__state):\n{rebind}        __state.add_url_rule('/ghost', view_func=f)\n"
    );
    let files = parsed(&[("app.py", &source)]);
    clean(&files);
    let file = &files["app.py"];
    let bindings = index_python(&files);
    let nodes = bindings.nodes("app.py");
    let scope = nodes
        .iter()
        .copied()
        .find(|node| node.kind() == "function_definition")
        .unwrap();
    let call = source.find("__state.add_url_rule").unwrap();
    let object = file
        .tree
        .root_node()
        .descendant_for_byte_range(call, call + "__state".len())
        .unwrap();
    crate::framework_entries::routes::python::flask::record_once_owner(
        &bindings, file, scope, object,
    )
    .is_some()
}
#[test]
fn n1_mangled_state_formal_rebound_in_the_body() {
    assert!(!record_once_owner_of("        __state = object()\n"));
}
#[test]
fn n1_control_mangled_state_formal_not_rebound() {
    assert!(record_once_owner_of(""));
}

// 6 (Opus N3): S3 cutoff guards. A rebind that is not a definite direct write must not drop the
// earlier allocation from the mutation owners.
fn cut(rebind: &str) -> usize {
    roots(&format!(
        "from flask import Flask\nfrom types import SimpleNamespace\ndef make():\n    app = Flask(__name__)\n    app.add_url_rule('/x', endpoint='x')\n    saved = app\n{rebind}    app.label = 1\n    return saved\nexported = make()\n"
    ))
}
#[test]
fn n3_if_guarded_rebind_is_no_cutoff() {
    assert_eq!(cut("    if cond():\n        app = SimpleNamespace()\n"), 0);
}
#[test]
fn n3_tuple_rebind_is_no_cutoff() {
    assert_eq!(cut("    app, other = SimpleNamespace(), 1\n"), 0);
}
#[test]
fn n3_nested_nonlocal_rebind_called_after_is_no_cutoff() {
    let rebind =
        "    def swap():\n        nonlocal app\n        app = SimpleNamespace()\n    swap()\n";
    assert_eq!(cut(rebind), 0);
}
#[test]
fn n3_control_direct_rebind_is_the_cutoff() {
    assert_eq!(cut("    app = SimpleNamespace()\n"), 1);
}

// 7 (Opus N4): a module-level `global K` redirects nothing; the module still binds K.
#[test]
fn n4_module_level_global_is_no_redirect() {
    let one = |source: &str| {
        let files = parsed(&[("app.py", source)]);
        let bindings = index_python(&files);
        let file = &files["app.py"];
        bindings.certify_in(file, file.tree.root_node(), "k")
    };
    assert_eq!(one("global k\nk = 1\n"), Ok(()));
    assert_eq!(one("global k\nk = 1\nk = 2\n"), Err(Clause::Count));
    let app = "from flask import Flask\nglobal app\napp = Flask(__name__)\n@app.route('/x')\ndef f(): pass\n";
    assert_eq!(proved_rules(&[("app.py", app)]), 1);
    assert!(matches!(
        config_methods("global app\n"),
        MethodSet::Unknown { .. }
    ));
    let _ = rules_of;
}

// Known limit (not folded): `unwritten_alias` compares the alias's SPELLING with its mangled
// key, so for a class-private alias (`__a`) the holder barrier (an alias handed on, here stored
// in a list) never sees the escape. A holder barrier is outside the contract (DESIGN 5.3,
// limits); the row pins what the product does TODAY (the mangled alias stays proved).
#[test]
fn known_limit_mangled_alias_escape_is_not_seen_by_the_holder_barrier() {
    let plain = format!(
        "{APP}def install():\n    a = app\n    held = [a]\n    a.add_url_rule('/x', endpoint='x')\ninstall()\n"
    );
    assert_eq!(proved_rules(&[("app.py", &plain)]), 0);
    let mangled = format!(
        "{APP}class C:\n    def install(self):\n        __a = app\n        held = [__a]\n        __a.add_url_rule('/x', endpoint='x')\nC().install()\n"
    );
    assert_eq!(proved_rules(&[("app.py", &mangled)]), 1);
}
