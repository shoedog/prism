//! The `Attribution` shape itself: provenance fields, the loaded/same-file
//! split, why a specifier is unresolvable, and the repo-listing hand-off.
use super::import_targets::{Attribution, Basis, ImportTargets, Why};
use super::import_targets_tests::{
    attribute, attribute_loaded, attribute_nth, attribute_with, in_repo, write_repo,
};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn js_provenance_names_the_specifier_and_the_exported_member() {
    let dir = write_repo(&[
        (
            "app.js",
            "import d from './d';\nimport { f as g } from './named';\nimport * as ns from './lib';\nconst lib = require('./cjs');\n\
             export function h(x) { d(x); g(x); ns.h(x); lib.k(x); d.p(x); }\n",
        ),
        ("d.js", ""),
        ("named.js", ""),
        ("lib.js", ""),
        ("cjs.js", ""),
    ]);
    let cases = [
        ("d", "./d", Some("default")),
        ("g", "./named", Some("f")),
        ("h", "./lib", Some("h")),
        ("k", "./cjs", Some("k")),
        // A property of the default export is not an export named `p`.
        ("p", "./d", None),
    ];
    for (callee, spec, member) in cases {
        let found = attribute(&dir, "app.js", callee);
        let hit = in_repo(&found);
        assert_eq!(hit.spec, spec, "{callee}");
        assert_eq!(hit.member.as_deref(), member, "{callee}");
        assert_eq!(hit.basis, Basis::Bound);
        assert!(hit.declaring.is_empty());
    }
}

#[test]
fn alias_fallbacks_are_candidates_and_a_single_target_is_bound() {
    let config = r#"{"compilerOptions":{"baseUrl":".","paths":{"@a/*":["lib/*","fallback/*"],"@b/*":["lib/*"]}}}"#;
    let dir = write_repo(&[
        ("tsconfig.json", config),
        (
            "app.ts",
            "import { a } from '@a/cmd';\nimport { b } from '@b/cmd';\nexport function h(x: string) { a(x); b(x); }\n",
        ),
        ("lib/cmd.ts", ""),
        ("fallback/cmd.ts", ""),
    ]);
    assert_eq!(
        in_repo(&attribute(&dir, "app.ts", "a")).basis,
        Basis::Candidates
    );
    assert_eq!(in_repo(&attribute(&dir, "app.ts", "b")).basis, Basis::Bound);
}

#[test]
fn caller_and_loaded_targets_are_split_into_same_file_and_loaded() {
    let dir = write_repo(&[
        (
            "app.js",
            "import { f } from './lib';\nimport { me } from './app';\nexport function h(x) { f(x); me(x); }\n",
        ),
        ("lib.js", "export function f(x) {}\n"),
    ]);
    assert_eq!(
        attribute_loaded(&dir, "app.js", "f", &["lib.js"]),
        Attribution::Loaded(vec!["lib.js".to_string()])
    );
    assert_eq!(attribute(&dir, "app.js", "me"), Attribution::SameFile);
    // A self-import where the caller is not in the loaded set: the caller is
    // still not a boundary.
    assert_eq!(
        attribute_with(&dir, "app.js", "me", 0, &[], false),
        Attribution::SameFile
    );
}

#[test]
fn unresolvable_carries_why() {
    let why_of = |found: Attribution| match found {
        Attribution::Unresolvable { why, .. } => why,
        other => panic!("expected Unresolvable, got {other:?}"),
    };
    let dir = write_repo(&[
        (
            "app.js",
            "import { a } from './none';\nimport { b } from './x';\nexport function h(v) { a(v); b(v); }\n",
        ),
        ("x.js", ""),
    ]);
    assert_eq!(why_of(attribute(&dir, "app.js", "a")), Why::NoFile);

    let config = r#"{"compilerOptions":{"baseUrl":".","paths":{"@m/*":["m1/*","m2/*","m3/*","m4/*","m5/*"]}}}"#;
    let many = write_repo(&[
        ("tsconfig.json", config),
        (
            "app.ts",
            "import { f } from '@m/c';\nexport function h(x: string) { f(x); }\n",
        ),
        ("m1/c.ts", ""),
        ("m2/c.ts", ""),
        ("m3/c.ts", ""),
        ("m4/c.ts", ""),
        ("m5/c.ts", ""),
    ]);
    assert_eq!(why_of(attribute(&many, "app.ts", "f")), Why::TooMany);

    let go_mod = "module example.com/app\n\nrequire original.example/a v1.0.0\n\nreplace original.example/a v1.0.0 => ./fork\n";
    let go = write_repo(&[
        ("go.mod", go_mod),
        ("fork/go.mod", "module fork.example/a\n"),
        ("fork/p/a.go", "package p\n\nfunc Run() {}\n"),
        (
            "main.go",
            "package main\n\nimport \"original.example/a/p\"\nimport \"example.com/app/gen\"\n\nfunc h() {\n\tp.Run()\n\tgen.Make()\n}\n",
        ),
    ]);
    assert_eq!(
        why_of(attribute(&go, "main.go", "Run")),
        Why::ReplaceUnproven
    );
    assert_eq!(why_of(attribute(&go, "main.go", "Make")), Why::NoFile);
}

#[test]
fn go_qualified_call_names_the_declaring_files() {
    let dir = write_repo(&[
        ("go.mod", "module example.com/app\n"),
        (
            "main.go",
            "package main\n\nimport \"example.com/app/util\"\n\nfunc h(x string) {\n\tutil.Run(x)\n}\n",
        ),
        ("util/a.go", "package util\n\nfunc Run(x string) {}\n"),
        ("util/b.go", "package util\n\nfunc Other() {}\n"),
    ]);
    let found = attribute(&dir, "main.go", "Run");
    let hit = in_repo(&found);
    assert_eq!(hit.files, ["util/a.go", "util/b.go"]);
    assert_eq!(hit.declaring, ["util/a.go"]);
    assert_eq!(hit.member.as_deref(), Some("Run"));
    assert_eq!(hit.spec, "example.com/app/util");
    assert_eq!(hit.basis, Basis::Bound);
}

#[test]
fn python_provenance_and_candidate_basis() {
    let dir = write_repo(&[
        ("pkg/__init__.py", "import os as m\n"),
        ("pkg/m.py", "def f(x):\n    pass\n"),
        ("pkg/n.py", "def g(x):\n    pass\n"),
        (
            "app.py",
            "from pkg import m\nfrom pkg.n import g as gg\n\ndef h(x):\n    m.f(x)\n    gg(x)\n",
        ),
    ]);
    let found = attribute(&dir, "app.py", "f");
    let both = in_repo(&found);
    assert_eq!(both.basis, Basis::Candidates);
    assert_eq!(both.member.as_deref(), Some("f"));
    let found = attribute(&dir, "app.py", "gg");
    let one = in_repo(&found);
    assert_eq!(
        (one.spec.as_str(), one.member.as_deref()),
        ("pkg.n", Some("g"))
    );
    assert_eq!(one.basis, Basis::Bound);
}

#[test]
fn go_build_tag_siblings_are_candidates() {
    let dir = write_repo(&[
        ("go.mod", "module example.com/app\n"),
        (
            "main.go",
            "package main\n\nfunc h(x string) {\n\thelper(x)\n\tother(x)\n}\n",
        ),
        ("h_linux.go", "package main\n\nfunc helper(x string) {}\n"),
        ("h_windows.go", "package main\n\nfunc helper(x string) {}\n"),
        ("o.go", "package main\n\nfunc other(x string) {}\n"),
    ]);
    assert_eq!(
        in_repo(&attribute(&dir, "main.go", "helper")).basis,
        Basis::Candidates
    );
    let found = attribute_nth(&dir, "main.go", "other", 0);
    assert_eq!(in_repo(&found).basis, Basis::Bound);
}

#[test]
fn the_listing_comes_from_the_single_inputs_walk() {
    let dir = write_repo(&[
        ("a.js", "export function f(){}"),
        ("node_modules/x/i.js", ""),
    ]);
    let (_, listing) =
        crate::repo_loader::scope_graph_build_inputs_with_listing(dir.path(), &BTreeMap::new());
    let listing = listing.expect("walk succeeded");
    assert!(listing.contains("a.js"));
    assert!(!listing.contains("node_modules/x/i.js"));
}

#[cfg(unix)]
#[test]
fn an_unreadable_directory_is_a_reported_reason_not_a_silent_none() {
    use std::os::unix::fs::PermissionsExt;
    let dir = write_repo(&[("a.js", ""), ("locked/inner.js", "")]);
    let locked = dir.path().join("locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (inputs, listing) =
        crate::repo_loader::scope_graph_build_inputs_with_listing(dir.path(), &BTreeMap::new());
    let readable = std::fs::read_dir(&locked).is_ok(); // running as root ignores the mode
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return;
    }
    let reason = listing.clone().expect_err("walk must fail");
    assert_eq!(reason.dir, "locked");
    assert!(
        reason.to_string().starts_with("cannot list locked: "),
        "{reason}"
    );
    // The Rust coverage decision keeps its old answer on a failed walk.
    assert!(!inputs.complete);
    let refused = ImportTargets::new(&inputs, &listing, BTreeSet::new());
    assert_eq!(refused.err().map(|e| e.dir), Some("locked".to_string()));
}
