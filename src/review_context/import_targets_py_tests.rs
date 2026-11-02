//! Python attribution tests on tempdir repos.
use super::import_targets_tests::{
    assert_external, assert_files, assert_same_file, assert_unknown, assert_unresolvable,
    attribute, attribute_nth, write_repo,
};

const CALL: &str = "\n\ndef h(x):\n    ";

fn py(imports: &str, call: &str) -> String {
    format!("{imports}{CALL}{call}\n")
}

// ---- relative imports ----

#[test]
fn relative_from_import() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/app.py", &py("from .m import f", "f(x)")),
        ("pkg/m.py", "def f(x):\n    pass\n"),
    ]);
    assert_files(attribute(&dir, "pkg/app.py", "f"), &["pkg/m.py"]);
}

#[test]
fn relative_two_dots() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/sub/__init__.py", ""),
        ("pkg/sub/app.py", &py("from ..m import f", "f(x)")),
        ("pkg/m.py", "def f(x):\n    pass\n"),
    ]);
    assert_files(attribute(&dir, "pkg/sub/app.py", "f"), &["pkg/m.py"]);
}

#[test]
fn relative_import_of_missing_module_is_unresolvable() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/app.py", &py("from .gone import f", "f(x)")),
    ]);
    assert_unresolvable(attribute(&dir, "pkg/app.py", "f"), ".gone");
}

#[test]
fn package_wins_over_module_of_the_same_name() {
    let dir = write_repo(&[
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("pkg/m/__init__.py", "def f(x):\n    pass\n"),
        ("app.py", &py("from pkg.m import f", "f(x)")),
    ]);
    assert_files(attribute(&dir, "app.py", "f"), &["pkg/m/__init__.py"]);
}

// ---- absolute imports: the import root ----

#[test]
fn absolute_from_import_at_repo_root() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", &py("from pkg.m import f", "f(x)")),
    ]);
    assert_files(attribute(&dir, "app.py", "f"), &["pkg/m.py"]);
}

#[test]
fn script_directory_prefers_nearest_ancestor_then_falls_back() {
    let dir = write_repo(&[
        ("lib/m.py", "def f(x):\n    pass\n"),
        ("src/lib/m.py", "def f(x):\n    pass\n"),
        ("src/app/x.py", &py("from lib.m import f", "f(x)")),
        ("other/x.py", &py("from lib.m import f", "f(x)")),
    ]);
    assert_files(attribute(&dir, "src/app/x.py", "f"), &["src/lib/m.py"]);
    assert_files(attribute(&dir, "other/x.py", "f"), &["lib/m.py"]);
}

#[test]
fn package_caller_does_not_search_its_own_directory() {
    // `pkg/sub/a.py` is in a package: the import root is the repo root, so an
    // absolute `helpers` is the root module, not the sibling `pkg/helpers.py`.
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/sub/__init__.py", ""),
        ("pkg/helpers.py", "def f(x):\n    pass\n"),
        ("helpers.py", "def f(x):\n    pass\n"),
        ("pkg/sub/a.py", &py("from helpers import f", "f(x)")),
        ("pkg/sub/b.py", &py("from pkg.helpers import f", "f(x)")),
    ]);
    assert_files(attribute(&dir, "pkg/sub/a.py", "f"), &["helpers.py"]);
    assert_files(attribute(&dir, "pkg/sub/b.py", "f"), &["pkg/helpers.py"]);
}

#[test]
fn stdlib_name_is_external_unless_the_import_root_holds_it() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/sub/__init__.py", ""),
        ("pkg/sub/json.py", "def dumps(x):\n    pass\n"),
        ("pkg/logging.py", "def info(x):\n    pass\n"),
        ("pkg/sub/a.py", &py("import json", "json.dumps(x)")),
        ("pkg/sub/b.py", &py("import logging", "logging.info(x)")),
        // A script directory is its own import root: its json.py shadows the stdlib.
        ("scripts/json.py", "def dumps(x):\n    pass\n"),
        ("scripts/a.py", &py("import json", "json.dumps(x)")),
        // So is the directory above a top-level package.
        ("top/__init__.py", ""),
        ("top/c.py", &py("import json", "json.dumps(x)")),
    ]);
    assert_external(attribute(&dir, "pkg/sub/a.py", "dumps"));
    assert_external(attribute(&dir, "pkg/sub/b.py", "info"));
    assert_files(
        attribute(&dir, "scripts/a.py", "dumps"),
        &["scripts/json.py"],
    );
    // No root-level json.py yet: still the stdlib.
    assert_external(attribute(&dir, "top/c.py", "dumps"));
    let with_root = write_repo(&[
        ("json.py", "def dumps(x):\n    pass\n"),
        ("top/__init__.py", ""),
        ("top/c.py", &py("import json", "json.dumps(x)")),
        // A script directory above which json.py sits: its sys.path is its own
        // directory, so the standard library wins.
        ("tools/d.py", &py("import json", "json.dumps(x)")),
    ]);
    assert_files(attribute(&with_root, "top/c.py", "dumps"), &["json.py"]);
    assert_external(attribute(&with_root, "tools/d.py", "dumps"));
}

#[test]
fn stdlib_and_third_party_are_external() {
    let dir = write_repo(&[(
        "app.py",
        &py(
            "import os\nfrom subprocess import run",
            "os.system(x)\n    run(x)",
        ),
    )]);
    assert_external(attribute(&dir, "app.py", "system"));
    assert_external(attribute(&dir, "app.py", "run"));
}

#[test]
fn missing_module_under_an_in_repo_package_is_unresolvable() {
    let dir = write_repo(&[
        ("myapp/__init__.py", ""),
        ("myapp/services/__init__.py", ""),
        (
            "app.py",
            &py(
                "from myapp.services.gone import f\nfrom elsewhere.mod import g",
                "f(x)\n    g(x)",
            ),
        ),
    ]);
    assert_unresolvable(attribute(&dir, "app.py", "f"), "myapp.services.gone");
    // A top-level name the repo does not hold is a third-party package.
    assert_external(attribute(&dir, "app.py", "g"));
}

// ---- aliases and dotted imports ----

#[test]
fn import_as_alias_member_call() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", &py("import pkg.m as m", "m.f(x)")),
    ]);
    assert_files(attribute(&dir, "app.py", "f"), &["pkg/m.py"]);
}

#[test]
fn dotted_import_qualifier_names_the_submodule() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", &py("import pkg.m", "pkg.m.f(x)")),
    ]);
    assert_files(attribute(&dir, "app.py", "f"), &["pkg/m.py"]);
}

#[test]
fn several_dotted_imports_of_one_package_both_stay_usable() {
    for imports in ["import pkg.a\nimport pkg.b", "import pkg.b\nimport pkg.a"] {
        let dir = write_repo(&[
            ("pkg/__init__.py", ""),
            ("pkg/a.py", "def f(x):\n    pass\n"),
            ("pkg/b.py", "def g(x):\n    pass\n"),
            (
                "app.py",
                &py(imports, "pkg.a.f(x)\n    pkg.b.g(x)\n    pkg.f(x)"),
            ),
        ]);
        assert_files(attribute(&dir, "app.py", "f"), &["pkg/a.py"]);
        assert_files(attribute_nth(&dir, "app.py", "f", 1), &["pkg/__init__.py"]);
        assert_files(attribute(&dir, "app.py", "g"), &["pkg/b.py"]);
    }
}

#[test]
fn a_later_alias_rebinds_the_package_name() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/a.py", "def f(x):\n    pass\n"),
        (
            "app.py",
            &py("import pkg.a\nimport other.mod as pkg", "pkg.a.f(x)"),
        ),
    ]);
    assert_external(attribute(&dir, "app.py", "f"));
}

#[test]
fn package_attribute_against_submodule_is_a_candidate_pair() {
    // `pkg/__init__.py` may set `m` itself (it is not read): both are named,
    // the initializer first, as Python tries the attribute first.
    let dir = write_repo(&[
        ("pkg/__init__.py", "import os as m\n"),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", &py("from pkg import m", "m.f(x)")),
        ("pkg/user.py", &py("from . import m", "m.f(x)")),
        ("app2.py", &py("import pkg", "pkg.m.f(x)")),
    ]);
    for caller in ["app.py", "pkg/user.py", "app2.py"] {
        assert_files(
            attribute(&dir, caller, "f"),
            &["pkg/__init__.py", "pkg/m.py"],
        );
    }
}

#[test]
fn namespace_package_submodule_and_initializer_only_are_bound() {
    let dir = write_repo(&[
        ("ns/m.py", "def f(x):\n    pass\n"),
        ("app.py", &py("from ns import m", "m.f(x)")),
        ("init/__init__.py", "def f(x):\n    pass\n"),
        ("app2.py", &py("from init import m", "m.f(x)")),
    ]);
    assert_files(attribute(&dir, "app.py", "f"), &["ns/m.py"]);
    assert_files(attribute(&dir, "app2.py", "f"), &["init/__init__.py"]);
}

#[test]
fn package_init_is_the_target() {
    let dir = write_repo(&[
        ("pkg/__init__.py", "def f(x):\n    pass\n"),
        ("a.py", &py("from pkg import f", "f(x)")),
        ("b.py", &py("import pkg", "pkg.f(x)")),
    ]);
    assert_files(attribute(&dir, "a.py", "f"), &["pkg/__init__.py"]);
    assert_files(attribute(&dir, "b.py", "f"), &["pkg/__init__.py"]);
}

// ---- local names ----

#[test]
fn same_file_function_is_same_file_and_unbound_is_unknown() {
    let dir = write_repo(&[(
        "app.py",
        "def helper(x):\n    pass\n\ndef h(x, obj):\n    helper(x)\n    obj.go(x)\n",
    )]);
    assert_same_file(attribute(&dir, "app.py", "helper"));
    assert_unknown(attribute(&dir, "app.py", "go"));
}

#[test]
fn function_scope_bindings_shadow_imports() {
    let dir = write_repo(&[
        ("m.py", "def f(x):\n    pass\n"),
        (
            "app.py",
            "from m import f\nimport m\n\n\
             def a(f, x):\n    f(x)\n\n\
             def b(x):\n    f = pick()\n    f(x)\n\n\
             def c(m, x):\n    m.f(x)\n\n\
             def d(xs):\n    return [f(x) for f in xs]\n\n\
             def l(xs):\n    return sorted(xs, key=lambda f: f(1))\n\n\
             def e(x):\n    with open(x) as m:\n        m.f(x)\n\n\
             def ok(x):\n    f(x)\n    m.f(x)\n",
        ),
    ]);
    // Sites of `f` in source order: a, b, c (m.f), d, l's lambda, e (m.f), ok f(), ok m.f().
    for nth in 0..6 {
        assert_unknown(attribute_nth(&dir, "app.py", "f", nth));
    }
    assert_files(attribute_nth(&dir, "app.py", "f", 6), &["m.py"]);
    assert_files(attribute_nth(&dir, "app.py", "f", 7), &["m.py"]);
}

#[test]
fn module_level_rebinding_of_an_import_is_unknown() {
    let dir = write_repo(&[
        ("m.py", "def f(x):\n    pass\n"),
        ("a.py", &py("from m import f\nf = wrap(f)", "f(x)")),
        ("b.py", &py("from m import f\ndef f(y):\n    pass", "f(x)")),
        ("c.py", &py("from m import f", "f(x)")),
    ]);
    assert_unknown(attribute(&dir, "a.py", "f"));
    assert_unknown(attribute(&dir, "b.py", "f"));
    assert_files(attribute(&dir, "c.py", "f"), &["m.py"]);
}

#[test]
fn parameter_shadows_a_same_file_function() {
    let dir = write_repo(&[(
        "app.py",
        "def helper(x):\n    pass\n\ndef a(helper, x):\n    helper(x)\n\ndef b(x):\n    helper(x)\n",
    )]);
    assert_unknown(attribute_nth(&dir, "app.py", "helper", 0));
    assert_same_file(attribute_nth(&dir, "app.py", "helper", 1));
}
