//! Go attribution tests on tempdir repos.
use super::import_targets_tests::{
    assert_external, assert_files, assert_loaded, assert_same_file, assert_unknown,
    assert_unresolvable, attribute, attribute_loaded, attribute_nth, write_repo,
};

const GO_MOD: &str = "module example.com/app\n\ngo 1.21\n";

fn main_go(import: &str, call: &str) -> String {
    format!("package main\n\nimport {import}\n\nfunc h(x string) {{\n\t{call}\n}}\n")
}

#[test]
fn qualified_call_names_every_non_test_file_of_the_package() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go("\"example.com/app/util\"", "util.Run(x)"),
        ),
        ("util/b.go", "package util\n\nfunc Other() {}\n"),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        ("util/a_test.go", "package util\n\nfunc TestRun() {}\n"),
    ]);
    assert_files(
        attribute(&dir, "main.go", "Run"),
        &["util/a.go", "util/b.go"],
    );
}

#[test]
fn testdata_directories_are_not_package_files_or_targets() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go("\"example.com/app/util\"", "util.Run(x)"),
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        (
            "util/testdata/fixture.go",
            "package fixture\n\nfunc Run(x string) {}\n",
        ),
        (
            "util/sub/testdata/deep.go",
            "package deep\n\nfunc Run(x string) {}\n",
        ),
    ]);
    assert_files(attribute(&dir, "main.go", "Run"), &["util/a.go"]);
    // A testdata package is no import target either: under the repo's own
    // module path with no listed directory, it is unresolvable.
    let imports_testdata = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go(
                "fixture \"example.com/app/util/testdata\"",
                "fixture.Run(x)",
            ),
        ),
        (
            "util/testdata/fixture.go",
            "package fixture\n\nfunc Run(x string) {}\n",
        ),
    ]);
    assert_unresolvable(
        attribute(&imports_testdata, "main.go", "Run"),
        "example.com/app/util/testdata",
    );
}

#[test]
fn aliased_import_call() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go("u \"example.com/app/util\"", "u.Run(x)"),
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
    ]);
    assert_files(attribute(&dir, "main.go", "Run"), &["util/a.go"]);
}

#[test]
fn std_and_third_party_imports_are_external() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go(
                "(\n\t\"os/exec\"\n\t\"github.com/other/lib\"\n)",
                "exec.Command(x)\n\tlib.Do(x)",
            ),
        ),
    ]);
    assert_external(attribute(&dir, "main.go", "Command"));
    assert_external(attribute(&dir, "main.go", "Do"));
}

#[test]
fn vendored_package_is_not_a_target() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        ("main.go", &main_go("\"github.com/other/v\"", "v.Run(x)")),
        (
            "vendor/github.com/other/v/a.go",
            "package v\n\nfunc Run(x string) {}\n",
        ),
    ]);
    assert_external(attribute(&dir, "main.go", "Run"));
}

#[test]
fn own_module_path_with_no_directory_is_unresolvable() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go(
                "(\n\t\"example.com/app/gen/pb\"\n\t\"example.com/app/util\"\n)",
                "pb.Make(x)\n\tutil.Run(x)",
            ),
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
    ]);
    assert_unresolvable(attribute(&dir, "main.go", "Make"), "example.com/app/gen/pb");
    assert_files(attribute(&dir, "main.go", "Run"), &["util/a.go"]);
}

#[test]
fn unproven_versioned_local_replace_is_unresolvable_not_external() {
    let go_mod = "module example.com/app\n\ngo 1.21\n\nrequire original.example/a v1.0.0\n\nreplace original.example/a v1.0.0 => ./fork\n";
    let dir = write_repo(&[
        ("go.mod", go_mod),
        ("fork/go.mod", "module fork.example/a\n\ngo 1.21\n"),
        ("fork/p/a.go", "package p\n\nfunc Run(x string) {}\n"),
        (
            "main.go",
            &main_go(
                "(\n\talias \"original.example/a/p\"\n\t\"github.com/other/lib\"\n)",
                "alias.Run(x)\n\tlib.Do(x)",
            ),
        ),
    ]);
    assert_unresolvable(attribute(&dir, "main.go", "Run"), "original.example/a/p");
    assert_external(attribute(&dir, "main.go", "Do"));
}

#[test]
fn unversioned_local_replace_is_proven_and_attributed() {
    let go_mod = "module example.com/app\n\ngo 1.21\n\nrequire original.example/a v0.0.0\n\nreplace original.example/a => ./fork\n";
    let dir = write_repo(&[
        ("go.mod", go_mod),
        ("fork/go.mod", "module fork.example/a\n\ngo 1.21\n"),
        ("fork/p/a.go", "package p\n\nfunc Run(x string) {}\n"),
        ("main.go", &main_go("\"original.example/a/p\"", "p.Run(x)")),
    ]);
    assert_files(attribute(&dir, "main.go", "Run"), &["fork/p/a.go"]);
}

// ---- unqualified calls: the declaring sibling, by a lexical scan ----

#[test]
fn unqualified_call_finds_the_declaring_sibling() {
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
    assert_files(attribute(&dir, "main.go", "helper"), &["helper.go"]);
}

#[test]
fn comments_and_strings_do_not_declare_functions() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nfunc h(x string) {\n\tblock(x)\n\tline(x)\n\tquoted(x)\n\traw(x)\n}\n",
        ),
        (
            "other.go",
            "package main\n\n/*\nfunc block(x string) {}\n*/\n\n// func line(x string) {}\nvar s = \"func quoted(x string) {}\"\nvar doc = `\nfunc raw(x string) {}\n`\n",
        ),
    ]);
    for callee in ["block", "line", "quoted", "raw"] {
        assert_unknown(attribute(&dir, "main.go", callee));
    }
}

#[test]
fn legal_whitespace_and_generics_still_declare() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        ("main.go", "package main\n\nfunc h(x string) {\n\thelper(x)\n\tgeneric(x)\n\tmethod(x)\n}\n"),
        (
            "other.go",
            "package main\n\nfunc  helper (x string) {}\n\nfunc\tgeneric[T any](v T) {}\n\nfunc (r R) method(x string) {}\n",
        ),
    ]);
    assert_files(attribute(&dir, "main.go", "helper"), &["other.go"]);
    assert_files(attribute(&dir, "main.go", "generic"), &["other.go"]);
    // A method is not a package-level function.
    assert_unknown(attribute(&dir, "main.go", "method"));
}

#[test]
fn build_tag_variants_are_all_named() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nfunc h(x string) {\n\thelper(x)\n}\n",
        ),
        (
            "helper_linux.go",
            "package main\n\nfunc helper(x string) {}\n",
        ),
        (
            "helper_windows.go",
            "package main\n\nfunc helper(x string) {}\n",
        ),
    ]);
    assert_files(
        attribute(&dir, "main.go", "helper"),
        &["helper_linux.go", "helper_windows.go"],
    );
}

#[test]
fn same_file_function_is_same_file_and_undeclared_is_unknown() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nfunc helper(x string) {}\n\nfunc h(x string) {\n\thelper(x)\n\tmystery(x)\n}\n",
        ),
    ]);
    assert_same_file(attribute(&dir, "main.go", "helper"));
    assert_unknown(attribute(&dir, "main.go", "mystery"));
}

// ---- local bindings ----

#[test]
fn local_bindings_shadow_the_qualifier_and_the_sibling() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            "package main\n\nimport \"example.com/app/util\"\n\n\
             func a(util T, x string) {\n\tutil.Run(x)\n}\n\n\
             func b(helper func(string), x string) {\n\thelper(x)\n}\n\n\
             func c(x string) {\n\thelper := pick()\n\thelper(x)\n}\n\n\
             func d(x string) {\n\tutil.Run(x)\n\thelper(x)\n}\n",
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        ("helper.go", "package main\n\nfunc helper(x string) {}\n"),
    ]);
    assert_unknown(attribute_nth(&dir, "main.go", "Run", 0));
    assert_unknown(attribute_nth(&dir, "main.go", "helper", 0));
    assert_unknown(attribute_nth(&dir, "main.go", "helper", 1));
    assert_files(attribute_nth(&dir, "main.go", "Run", 1), &["util/a.go"]);
    assert_files(attribute_nth(&dir, "main.go", "helper", 2), &["helper.go"]);
}

// ---- the declaring file decides what is a boundary ----

#[test]
fn a_loaded_declarer_is_not_a_boundary_but_an_unloaded_one_is() {
    let dir = write_repo(&[
        ("go.mod", GO_MOD),
        (
            "main.go",
            &main_go("\"example.com/app/util\"", "util.Run(x)"),
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        ("util/b.go", "package util\n\nfunc Other() {}\n"),
    ]);
    // `Run` is declared in a.go, which is loaded: the call resolves there.
    assert_loaded(attribute_loaded(&dir, "main.go", "Run", &["util/a.go"]));
    // Only b.go loaded: the declarer a.go is outside.
    assert_files(
        attribute_loaded(&dir, "main.go", "Run", &["util/b.go"]),
        &["util/a.go"],
    );
}
