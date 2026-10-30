//! Language-family witnesses through parsed files and the production resolver.
use crate::resolution::*;
use crate::{ast::ParsedFile, languages::Language};

fn graph(files: &[(&str, &str)]) -> CallGraph {
    CallGraph::build(
        &files
            .iter()
            .map(|(path, source)| {
                let language = Language::from_path(path).unwrap_or(Language::Python);
                (
                    path.to_string(),
                    ParsedFile::parse(path, source, language).unwrap(),
                )
            })
            .collect(),
    )
}

fn outcome<'a>(cg: &'a CallGraph, file: &str, name: &str) -> ResolutionOutcome<'a> {
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.file == file && s.callee_name == name)
        .unwrap_or_else(|| panic!("missing call {file}: {name}"));
    cg.resolve_call_site_full(site)
}

fn assert_exact(cg: &CallGraph, caller: &str, name: &str, target: &str) {
    let result = outcome(cg, caller, name);
    assert_eq!(result.resolved.len(), 1, "{caller}: {result:?}");
    assert_eq!(result.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(result.resolved[0].target.file, target);
}

macro_rules! rejects_foreign {
    ($test:ident, $caller:literal, $source:literal, $callee:literal, $definition:literal) => {
        #[test]
        fn $test() {
            let cg = graph(&[($caller, $source), ($callee, $definition)]);
            let result = outcome(&cg, $caller, "len");
            assert!(result.resolved.is_empty(), "foreign candidate: {result:?}");
            assert_eq!(result.drop, Some(DropReason::UnknownName));
        }
    };
}

rejects_foreign!(
    python_rust,
    "caller.py",
    "def run():\n    return len([])\n",
    "lib.rs",
    "fn len() {}\n"
);
rejects_foreign!(
    go_rust,
    "caller.go",
    "package p\nfunc run() { len() }\n",
    "lib.rs",
    "fn len() {}\n"
);
rejects_foreign!(
    javascript_python,
    "caller.js",
    "function run() { len(); }",
    "lib.py",
    "def len():\n    pass\n"
);
rejects_foreign!(
    bash_rust,
    "caller.sh",
    "run() {\n len\n}\n",
    "lib.rs",
    "fn len() {}\n"
);
rejects_foreign!(
    java_rust,
    "Caller.java",
    "class Caller { void run() { len(); } }",
    "lib.rs",
    "fn len() {}\n"
);
rejects_foreign!(
    unknown_callee,
    "caller.py",
    "def run():\n    return len([])\n",
    "lib.unknown",
    "def len():\n    pass\n"
);
rejects_foreign!(
    unknown_caller,
    "caller.unknown",
    "def run():\n    return len([])\n",
    "lib.py",
    "def len():\n    pass\n"
);

#[test]
fn same_family_cross_file_controls() {
    for (caller, source, target, definition) in [
        (
            "caller.cpp",
            "void run() { len(); }",
            "lib.c",
            "void len() {}",
        ),
        (
            "caller.cpp",
            "void run() { len(); }",
            "lib.h",
            "void len() {}",
        ),
        (
            "caller.ts",
            "function run() { len(); }",
            "lib.js",
            "function len() {}",
        ),
        (
            "caller.tsx",
            "function run() { len(); }",
            "lib.ts",
            "function len() {}",
        ),
        (
            "caller.py",
            "def run():\n    len()\n",
            "lib.py",
            "def len():\n    pass\n",
        ),
        ("caller.rs", "fn run() { len(); }", "lib.rs", "fn len() {}"),
    ] {
        assert_exact(
            &graph(&[(caller, source), (target, definition)]),
            caller,
            "len",
            target,
        );
    }
}

#[test]
fn mixed_pool_promotes_python_target() {
    let cg = graph(&[
        ("caller.py", "def run():\n    len()\n"),
        ("lib.py", "def len():\n    pass\n"),
        ("lib.rs", "fn len() {}"),
    ]);
    assert_exact(&cg, "caller.py", "len", "lib.py");
}

#[test]
fn same_file_wins_even_with_foreign_collision() {
    for file in ["caller.py", "caller.unknown"] {
        let cg = graph(&[
            (file, "def len():\n    pass\ndef run():\n    len()\n"),
            ("lib.rs", "fn len() {}"),
        ]);
        assert_exact(&cg, file, "len", file);
    }
}

#[test]
fn c_field_pool_refuses_foreign_and_static_but_keeps_cpp_candidate() {
    let caller = (
        "caller.c",
        "struct S { void (*len)(void); };\nvoid run(struct S *s) { s->len(); }",
    );
    for (file, definition, expected) in [
        ("lib.rs", "fn len() {}", false),
        ("lib.cpp", "static void len() {}", false),
        ("lib.cpp", "void len() {}", true),
    ] {
        let cg = graph(&[caller, (file, definition)]);
        let result = outcome(&cg, "caller.c", "len");
        assert_eq!(
            result.resolved.len(),
            usize::from(expected),
            "{file}: {result:?}"
        );
        if expected {
            assert_eq!(
                result.resolved[0].confidence,
                ResolutionConfidence::NameOnly
            );
            assert_eq!(result.resolved[0].target.file, file);
        }
    }
}

#[test]
fn c_static_local_wins_and_cross_file_static_is_inaccessible() {
    let cg = graph(&[
        ("caller.c", "static void len() {}\nvoid run() { len(); }"),
        ("lib.cpp", "void len() {}"),
    ]);
    assert_exact(&cg, "caller.c", "len", "caller.c");
    let cg = graph(&[
        ("caller.cpp", "void run() { len(); }"),
        ("lib.c", "static void len() {}"),
    ]);
    assert!(outcome(&cg, "caller.cpp", "len").resolved.is_empty());
}

#[test]
fn go_bare_value_pool_rejects_foreign_same_directory() {
    let cg = graph(&[
        ("caller.go", "package p\nfunc run() {}"),
        ("lib.rs", "fn len() {}"),
    ]);
    let mut ambiguous = 0;
    assert_eq!(
        resolve_go_bare_value_ref(
            &cg.functions,
            &cg.method_owners,
            &cg.go_file_profiles,
            &mut ambiguous,
            "caller.go",
            "len"
        ),
        None
    );
    assert_eq!(
        ambiguous, 0,
        "foreign functions cannot make a Go value ambiguous"
    );
}

#[test]
fn imported_module_pool_rejects_foreign_stem() {
    let cg = graph(&[
        ("caller.py", "import lib\ndef run():\n    lib.len()\n"),
        ("lib.rs", "fn len() {}"),
    ]);
    assert!(outcome(&cg, "caller.py", "len").resolved.is_empty());
}

#[test]
fn qualified_stem_pool_rejects_foreign_language() {
    let cg = graph(&[
        ("caller.cpp", "void run() { lib::len(); }"),
        ("lib.py", "def len():\n    pass\n"),
    ]);
    assert!(outcome(&cg, "caller.cpp", "lib::len").resolved.is_empty());
}
