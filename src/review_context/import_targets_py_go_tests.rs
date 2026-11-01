//! Python and Go attribution tests on tempdir repos.
use super::import_targets::Attribution;
use super::import_targets_tests::{attribute, in_repo, write_repo};

fn unresolvable(spec: &str) -> Attribution {
    Attribution::Unresolvable { spec: spec.into() }
}

#[test]
fn py_relative_from_import() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/app.py", "from .m import f\n\ndef h(x):\n    f(x)\n"),
        ("pkg/m.py", "def f(x):\n    pass\n"),
    ]);
    assert_eq!(attribute(&dir, "pkg/app.py", "f"), in_repo(&["pkg/m.py"]));
}

#[test]
fn py_relative_two_dots() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/sub/__init__.py", ""),
        (
            "pkg/sub/app.py",
            "from ..m import f\n\ndef h(x):\n    f(x)\n",
        ),
        ("pkg/m.py", "def f(x):\n    pass\n"),
    ]);
    assert_eq!(
        attribute(&dir, "pkg/sub/app.py", "f"),
        in_repo(&["pkg/m.py"])
    );
}

#[test]
fn py_relative_import_of_missing_module_is_unresolvable() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/app.py", "from .gone import f\n\ndef h(x):\n    f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "pkg/app.py", "f"), unresolvable(".gone"));
}

#[test]
fn py_absolute_from_import_at_repo_root() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", "from pkg.m import f\n\ndef h(x):\n    f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "app.py", "f"), in_repo(&["pkg/m.py"]));
}

#[test]
fn py_absolute_import_prefers_nearest_ancestor() {
    let dir = write_repo(&[
        ("lib/m.py", "def f(x):\n    pass\n"),
        ("src/lib/m.py", "def f(x):\n    pass\n"),
        (
            "src/app/x.py",
            "from lib.m import f\n\ndef h(v):\n    f(v)\n",
        ),
    ]);
    assert_eq!(
        attribute(&dir, "src/app/x.py", "f"),
        in_repo(&["src/lib/m.py"])
    );
}

#[test]
fn py_absolute_import_falls_back_to_a_farther_ancestor() {
    let dir = write_repo(&[
        ("lib/m.py", "def f(x):\n    pass\n"),
        (
            "src/app/x.py",
            "from lib.m import f\n\ndef h(v):\n    f(v)\n",
        ),
    ]);
    assert_eq!(attribute(&dir, "src/app/x.py", "f"), in_repo(&["lib/m.py"]));
}

#[test]
fn py_import_as_alias_member_call() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", "import pkg.m as m\n\ndef h(x):\n    m.f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "app.py", "f"), in_repo(&["pkg/m.py"]));
}

#[test]
fn py_dotted_import_qualifier_names_the_submodule() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", "import pkg.m\n\ndef h(x):\n    pkg.m.f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "app.py", "f"), in_repo(&["pkg/m.py"]));
}

#[test]
fn py_package_init_is_the_target() {
    let dir = write_repo(&[
        ("pkg/__init__.py", "def f(x):\n    pass\n"),
        ("a.py", "from pkg import f\n\ndef h(x):\n    f(x)\n"),
        ("b.py", "import pkg\n\ndef h(x):\n    pkg.f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "a.py", "f"), in_repo(&["pkg/__init__.py"]));
    assert_eq!(attribute(&dir, "b.py", "f"), in_repo(&["pkg/__init__.py"]));
}

#[test]
fn py_from_package_import_submodule_call() {
    let dir = write_repo(&[
        ("pkg/__init__.py", ""),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("app.py", "from pkg import m\n\ndef h(x):\n    m.f(x)\n"),
    ]);
    assert_eq!(attribute(&dir, "app.py", "f"), in_repo(&["pkg/m.py"]));
}

#[test]
fn py_module_and_package_at_one_distance_return_both_sorted() {
    let dir = write_repo(&[
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("pkg/m/__init__.py", "def f(x):\n    pass\n"),
        ("app.py", "from pkg.m import f\n\ndef h(x):\n    f(x)\n"),
    ]);
    assert_eq!(
        attribute(&dir, "app.py", "f"),
        in_repo(&["pkg/m.py", "pkg/m/__init__.py"])
    );
}

#[test]
fn py_stdlib_and_third_party_are_external() {
    let dir = write_repo(&[(
        "app.py",
        "import os\nfrom subprocess import run\n\ndef h(x):\n    os.system(x)\n    run(x)\n",
    )]);
    assert_eq!(attribute(&dir, "app.py", "system"), Attribution::External);
    assert_eq!(attribute(&dir, "app.py", "run"), Attribution::External);
}

#[test]
fn py_same_file_function_is_local_and_unbound_is_unknown() {
    let dir = write_repo(&[(
        "app.py",
        "def helper(x):\n    pass\n\ndef h(x, obj):\n    helper(x)\n    obj.go(x)\n",
    )]);
    assert_eq!(attribute(&dir, "app.py", "helper"), Attribution::Local);
    assert_eq!(attribute(&dir, "app.py", "go"), Attribution::Unknown);
}

const GO_MOD: &str = "module example.com/app\n\ngo 1.21\n";

#[test]
fn go_qualified_call_names_every_non_test_file_of_the_package() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nimport \"example.com/app/util\"\n\nfunc h(x string) {\n\tutil.Run(x)\n}\n",
        ),
        ("util/b.go", "package util\n\nfunc Other() {}\n"),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        ("util/a_test.go", "package util\n\nfunc TestRun() {}\n"),
    ]);
    assert_eq!(
        attribute(&dir, "main.go", "Run"),
        in_repo(&["util/a.go", "util/b.go"])
    );
}

#[test]
fn go_aliased_import_call() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nimport u \"example.com/app/util\"\n\nfunc h(x string) {\n\tu.Run(x)\n}\n",
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "main.go", "Run"), in_repo(&["util/a.go"]));
}

#[test]
fn go_std_and_third_party_imports_are_external() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nimport (\n\t\"os/exec\"\n\t\"github.com/other/lib\"\n)\n\nfunc h(x string) {\n\texec.Command(x)\n\tlib.Do(x)\n}\n",
        ),
    ]);
    assert_eq!(attribute(&dir, "main.go", "Command"), Attribution::External);
    assert_eq!(attribute(&dir, "main.go", "Do"), Attribution::External);
}

#[test]
fn go_vendored_package_is_not_a_target() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nimport \"example.com/app/vendor/v\"\n\nfunc h(x string) {\n\tv.Run(x)\n}\n",
        ),
        ("vendor/v/a.go", "package v\n\nfunc Run(x string) {}\n"),
    ]);
    assert_eq!(attribute(&dir, "main.go", "Run"), Attribution::External);
}

#[test]
fn go_unqualified_call_finds_the_declaring_sibling() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nfunc h(x string) {\n\thelper(x)\n}\n",
        ),
        ("helper.go", "package main\n\nfunc helper(x string) {}\n"),
        ("other.go", "package main\n\nfunc unrelated() {}\n"),
        (
            "helper_test.go",
            "package main\n\nfunc helper(x string) {}\n",
        ),
    ]);
    assert_eq!(
        attribute(&dir, "main.go", "helper"),
        in_repo(&["helper.go"])
    );
}

#[test]
fn go_same_file_function_is_local_and_undeclared_is_unknown() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nfunc helper(x string) {}\n\nfunc h(x string) {\n\thelper(x)\n\tmystery(x)\n}\n",
        ),
    ]);
    assert_eq!(attribute(&dir, "main.go", "helper"), Attribution::Local);
    assert_eq!(attribute(&dir, "main.go", "mystery"), Attribution::Unknown);
}
