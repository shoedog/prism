//! Method-rung language-family witnesses (#585) through parsed files and the production resolver.
//!
//! Every case states two things. `unfiltered` is the candidate pool's rung, kind and confidence
//! exactly as the rung chose them before admission (`resolve_call_site_candidates`, which the
//! family filter does not touch). `filtered` is the public result: the unfiltered targets minus
//! those outside the caller's language family, with the unfiltered kind and confidence kept.
use crate::call_graph::CallSite;
use crate::resolution::*;
use crate::{ast::ParsedFile, languages::Language};

const E: ResolutionConfidence = ResolutionConfidence::Exact;
const N: ResolutionConfidence = ResolutionConfidence::NameOnly;

type Edge = (&'static str, ResolutionConfidence, ResolutionKind);

fn graph(files: &[(&str, &str)]) -> CallGraph {
    CallGraph::build(
        &files
            .iter()
            .map(|(path, source)| {
                let language = Language::from_path(path).expect("fixture extension");
                (
                    path.to_string(),
                    ParsedFile::parse(path, source, language).unwrap(),
                )
            })
            .collect(),
    )
}

fn site<'a>(cg: &'a CallGraph, file: &str, name: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|s| s.caller.file == file && s.callee_name == name)
        .unwrap_or_else(|| panic!("missing call {file}: {name}"))
}

fn edges(outcome: &ResolutionOutcome<'_>) -> Vec<(String, ResolutionConfidence, ResolutionKind)> {
    let mut out: Vec<_> = outcome
        .resolved
        .iter()
        .map(|r| (r.target.file.clone(), r.confidence, r.kind))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn owned(expected: &[Edge]) -> Vec<(String, ResolutionConfidence, ResolutionKind)> {
    let mut out: Vec<_> = expected
        .iter()
        .map(|(file, conf, kind)| (file.to_string(), *conf, *kind))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn case(
    files: &[(&str, &str)],
    caller: &str,
    callee: &str,
    unfiltered: &[Edge],
    filtered: &[Edge],
) {
    let cg = graph(files);
    let site = site(&cg, caller, callee);
    let before = cg.resolve_call_site_candidates(site);
    assert_eq!(
        edges(&before),
        owned(unfiltered),
        "unfiltered rung choice (fixture calibration): {before:?}"
    );
    let after = cg.resolve_call_site_full(site);
    assert_eq!(
        edges(&after),
        owned(filtered),
        "family-filtered result: {after:?}"
    );
    // The CPG build resolves through the memoized twin; it must return the same result.
    let memo = crate::resolution::GoSatisfierMemo::new(&cg).resolve_call_site_full(site);
    assert_eq!(
        edges(&memo),
        owned(filtered),
        "memo (CPG build path) family-filtered result: {memo:?}"
    );
    assert_eq!(
        memo.drop, after.drop,
        "memo drop reason: {memo:?} vs {after:?}"
    );
    if filtered.is_empty() {
        assert_eq!(after.drop, Some(DropReason::UnknownName), "{after:?}");
    } else {
        assert_eq!(after.drop, None, "{after:?}");
    }
}

const RS_CONFIG: &str = "pub struct Config;\nimpl Config {\n    pub fn load() {}\n}\n";
const RS_CONFIG_M: &str = "pub struct Config;\nimpl Config {\n    pub fn load(&self) {}\n}\n";
const PY_CONFIG: &str = "class Config:\n    def load(self):\n        pass\n";

// ---- R3b `qualifier_owner` (owner_lookup) ----

#[test]
fn r3b_issue_repro_python_does_not_bind_rust_impl() {
    case(
        &[
            ("py/app.py", "def run():\n    Config.load()\n"),
            ("rs/src/lib.rs", RS_CONFIG),
        ],
        "py/app.py",
        "load",
        &[("rs/src/lib.rs", E, ResolutionKind::QualifierOwner)],
        &[],
    );
}

#[test]
fn r3b_mixed_pool_keeps_python_class_as_nameonly_never_promoted() {
    case(
        &[
            ("py/app.py", "def run():\n    Config.load()\n"),
            ("py/models.py", PY_CONFIG),
            ("rs/src/lib.rs", RS_CONFIG),
        ],
        "py/app.py",
        "load",
        &[
            ("py/models.py", N, ResolutionKind::QualifierOwner),
            ("rs/src/lib.rs", N, ResolutionKind::QualifierOwner),
        ],
        &[("py/models.py", N, ResolutionKind::QualifierOwner)],
    );
}

#[test]
fn r3b_python_class_alone_stays_exact() {
    case(
        &[
            ("py/app.py", "def run():\n    Config.load()\n"),
            ("py/models.py", PY_CONFIG),
        ],
        "py/app.py",
        "load",
        &[("py/models.py", E, ResolutionKind::QualifierOwner)],
        &[("py/models.py", E, ResolutionKind::QualifierOwner)],
    );
}

#[test]
fn r3b_foreign_only_pool_does_not_fall_through_to_a_later_rung() {
    // Main stops at R3b with the Rust target. Without the filter a naive "skip foreign pools"
    // rule would fall through to R6, which would then offer the unrelated `Other.load`.
    case(
        &[
            ("py/app.py", "def run():\n    Config.load()\n"),
            (
                "py/other.py",
                "class Other:\n    def load(self):\n        pass\n",
            ),
            ("rs/src/lib.rs", RS_CONFIG),
        ],
        "py/app.py",
        "load",
        &[("rs/src/lib.rs", E, ResolutionKind::QualifierOwner)],
        &[],
    );
}

#[test]
fn r3b_javascript_caller_still_binds_typescript_owner() {
    case(
        &[
            ("app.js", "function run() { Config.load(); }\n"),
            ("lib.ts", "class Config {\n  static load() {}\n}\n"),
        ],
        "app.js",
        "load",
        &[("lib.ts", E, ResolutionKind::QualifierOwner)],
        &[("lib.ts", E, ResolutionKind::QualifierOwner)],
    );
}

// ---- R1 qualified `T::m` (owner_lookup_in_modules) ----

#[test]
fn r1_cpp_qualified_call_does_not_bind_rust_impl() {
    case(
        &[
            ("app.cpp", "void run() { Config::load(); }\n"),
            ("rs/src/lib.rs", RS_CONFIG),
        ],
        "app.cpp",
        "Config::load",
        &[("rs/src/lib.rs", E, ResolutionKind::QualifiedOwner)],
        &[],
    );
}

#[test]
fn r1_mixed_pool_keeps_cpp_class_as_nameonly() {
    case(
        &[
            ("app.cpp", "void run() { Config::load(); }\n"),
            (
                "lib.cpp",
                "class Config {\npublic:\n  static void load() {}\n};\n",
            ),
            ("rs/src/lib.rs", RS_CONFIG),
        ],
        "app.cpp",
        "Config::load",
        &[
            ("lib.cpp", N, ResolutionKind::QualifiedOwner),
            ("rs/src/lib.rs", N, ResolutionKind::QualifiedOwner),
        ],
        &[("lib.cpp", N, ResolutionKind::QualifiedOwner)],
    );
}

#[test]
fn r1_rust_caller_does_not_bind_python_class() {
    case(
        &[
            ("rs/src/app.rs", "fn run() {\n    Config::load();\n}\n"),
            (
                "py/models.py",
                "class Config:\n    def load():\n        pass\n",
            ),
        ],
        "rs/src/app.rs",
        "Config::load",
        &[("py/models.py", E, ResolutionKind::QualifiedOwner)],
        &[],
    );
}

// ---- R4b implicit-this ----

#[test]
fn r4b_java_implicit_this_does_not_bind_rust_impl() {
    case(
        &[
            (
                "App.java",
                "class App {\n    void run() {\n        helper();\n    }\n}\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct App;\nimpl App {\n    pub fn helper() {}\n}\n",
            ),
        ],
        "App.java",
        "helper",
        &[("rs/src/lib.rs", E, ResolutionKind::ImplicitThis)],
        &[],
    );
}

#[test]
fn r4b_mixed_pool_keeps_java_class_as_nameonly() {
    case(
        &[
            (
                "App.java",
                "class App {\n    void run() {\n        helper();\n    }\n}\n",
            ),
            ("Other.java", "class App {\n    void helper() {}\n}\n"),
            (
                "rs/src/lib.rs",
                "pub struct App;\nimpl App {\n    pub fn helper() {}\n}\n",
            ),
        ],
        "App.java",
        "helper",
        &[
            ("Other.java", N, ResolutionKind::ImplicitThis),
            ("rs/src/lib.rs", N, ResolutionKind::ImplicitThis),
        ],
        &[("Other.java", N, ResolutionKind::ImplicitThis)],
    );
}

#[test]
fn r4b_cpp_implicit_this_does_not_bind_rust_impl() {
    case(
        &[
            (
                "app.cpp",
                "class App {\npublic:\n  void run() { helper(); }\n};\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct App;\nimpl App {\n    pub fn helper() {}\n}\n",
            ),
        ],
        "app.cpp",
        "helper",
        &[("rs/src/lib.rs", E, ResolutionKind::ImplicitThis)],
        &[],
    );
}

// ---- `self.` / `this.` / receiver variable ----

#[test]
fn self_receiver_go_receiver_var_does_not_bind_rust_impl() {
    case(
        &[
            (
                "a.go",
                "package p\ntype T struct{}\nfunc (t *T) A() {\n\tt.B()\n}\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct T;\nimpl T {\n    pub fn B(&self) {}\n}\n",
            ),
        ],
        "a.go",
        "B",
        &[("rs/src/lib.rs", E, ResolutionKind::SelfReceiver)],
        &[],
    );
}

#[test]
fn self_receiver_mixed_pool_keeps_go_method_as_nameonly() {
    case(
        &[
            (
                "a.go",
                "package p\ntype T struct{}\nfunc (t *T) A() {\n\tt.B()\n}\n",
            ),
            ("b.go", "package p\nfunc (t *T) B() {}\n"),
            (
                "rs/src/lib.rs",
                "pub struct T;\nimpl T {\n    pub fn B(&self) {}\n}\n",
            ),
        ],
        "a.go",
        "B",
        &[
            ("b.go", N, ResolutionKind::SelfReceiver),
            ("rs/src/lib.rs", N, ResolutionKind::SelfReceiver),
        ],
        &[("b.go", N, ResolutionKind::SelfReceiver)],
    );
}

#[test]
fn self_receiver_java_this_does_not_bind_rust_impl() {
    case(
        &[
            (
                "App.java",
                "class App {\n    void run() {\n        this.helper();\n    }\n}\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct App;\nimpl App {\n    pub fn helper(&self) {}\n}\n",
            ),
        ],
        "App.java",
        "helper",
        &[("rs/src/lib.rs", E, ResolutionKind::SelfReceiver)],
        &[],
    );
}

#[test]
fn control_self_receiver_python_self_refused_on_main() {
    // Control, not a witness: main already refuses this site, so the filter has nothing to remove.
    case(
        &[
            (
                "py/app.py",
                "class Config:\n    def run(self):\n        self.load()\n",
            ),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "py/app.py",
        "load",
        &[],
        &[],
    );
}

// ---- recovered receivers: typed_param, constructor_local ----

#[test]
fn typed_param_python_does_not_bind_rust_impl() {
    case(
        &[
            ("py/app.py", "def run(c: Config):\n    c.load()\n"),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "py/app.py",
        "load",
        &[("rs/src/lib.rs", E, ResolutionKind::TypedParam)],
        &[],
    );
}

#[test]
fn typed_param_mixed_pool_keeps_python_class_as_nameonly() {
    case(
        &[
            ("py/app.py", "def run(c: Config):\n    c.load()\n"),
            ("py/models.py", PY_CONFIG),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "py/app.py",
        "load",
        &[
            ("py/models.py", N, ResolutionKind::TypedParam),
            ("rs/src/lib.rs", N, ResolutionKind::TypedParam),
        ],
        &[("py/models.py", N, ResolutionKind::TypedParam)],
    );
}

#[test]
fn constructor_local_python_does_not_bind_rust_impl() {
    case(
        &[
            ("py/app.py", "def run():\n    c = Config()\n    c.load()\n"),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "py/app.py",
        "load",
        &[("rs/src/lib.rs", E, ResolutionKind::ConstructorLocal)],
        &[],
    );
}

#[test]
fn constructor_local_mixed_pool_keeps_python_class_as_nameonly() {
    case(
        &[
            ("py/app.py", "def run():\n    c = Config()\n    c.load()\n"),
            ("py/models.py", PY_CONFIG),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "py/app.py",
        "load",
        &[
            ("py/models.py", N, ResolutionKind::ConstructorLocal),
            ("rs/src/lib.rs", N, ResolutionKind::ConstructorLocal),
        ],
        &[("py/models.py", N, ResolutionKind::ConstructorLocal)],
    );
}

#[test]
fn rust_bare_receiver_does_not_bind_python_class() {
    case(
        &[
            ("rs/src/app.rs", "fn run(c: &Config) {\n    c.load();\n}\n"),
            ("py/models.py", PY_CONFIG),
        ],
        "rs/src/app.rs",
        "load",
        &[("py/models.py", E, ResolutionKind::QualifiedOwner)],
        &[],
    );
}

// ---- R6 residue ----

#[test]
fn r6_single_owner_python_does_not_bind_rust_impl() {
    case(
        &[
            ("py/app.py", "def run(x):\n    x.push(1)\n"),
            (
                "rs/src/lib.rs",
                "pub struct Stack;\nimpl Stack {\n    pub fn push(&mut self, v: i32) {}\n}\n",
            ),
        ],
        "py/app.py",
        "push",
        &[("rs/src/lib.rs", N, ResolutionKind::R6SingleOwner)],
        &[],
    );
}

#[test]
fn r6_single_owner_mixed_pool_keeps_python_class() {
    case(
        &[
            ("py/app.py", "def run(x):\n    x.push(1)\n"),
            (
                "py/stack.py",
                "class Stack:\n    def push(self, v):\n        pass\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct Stack;\nimpl Stack {\n    pub fn push(&mut self, v: i32) {}\n}\n",
            ),
        ],
        "py/app.py",
        "push",
        &[
            ("py/stack.py", N, ResolutionKind::R6SingleOwner),
            ("rs/src/lib.rs", N, ResolutionKind::R6SingleOwner),
        ],
        &[("py/stack.py", N, ResolutionKind::R6SingleOwner)],
    );
}

#[test]
fn r6_multi_owner_foreign_only_pool_refuses() {
    case(
        &[
            ("py/app.py", "def run(x):\n    x.push(1)\n"),
            (
                "rs/src/lib.rs",
                "pub struct A;\npub struct B;\nimpl A {\n    pub fn push(&mut self, v: i32) {}\n}\nimpl B {\n    pub fn push(&mut self, v: i32) {}\n}\n",
            ),
        ],
        "py/app.py",
        "push",
        &[
            ("rs/src/lib.rs", N, ResolutionKind::R6MultiOwnerCandidate),
            ("rs/src/lib.rs", N, ResolutionKind::R6MultiOwnerCandidate),
        ],
        &[],
    );
}

#[test]
fn r6_multi_owner_mixed_pool_stays_multi_owner_candidate_never_single() {
    // Two owners before the filter, one target after: still NameOnly R6MultiOwnerCandidate.
    case(
        &[
            ("py/app.py", "def run(x):\n    x.push(1)\n"),
            (
                "py/a.py",
                "class A:\n    def push(self, v):\n        pass\n",
            ),
            (
                "rs/src/lib.rs",
                "pub struct B;\nimpl B {\n    pub fn push(&mut self, v: i32) {}\n}\n",
            ),
        ],
        "py/app.py",
        "push",
        &[
            ("py/a.py", N, ResolutionKind::R6MultiOwnerCandidate),
            ("rs/src/lib.rs", N, ResolutionKind::R6MultiOwnerCandidate),
        ],
        &[("py/a.py", N, ResolutionKind::R6MultiOwnerCandidate)],
    );
}

#[test]
fn r6_c_caller_still_binds_cpp_owner() {
    case(
        &[
            ("app.c", "void run(struct S *p) {\n  p->push(1);\n}\n"),
            ("lib.cpp", "class S {\npublic:\n  void push(int v) {}\n};\n"),
        ],
        "app.c",
        "push",
        &[("lib.cpp", N, ResolutionKind::R6SingleOwner)],
        &[("lib.cpp", N, ResolutionKind::R6SingleOwner)],
    );
}

#[test]
fn r6_javascript_caller_still_binds_typescript_owner() {
    case(
        &[
            ("app.js", "function run(x) { x.push(1); }\n"),
            ("lib.ts", "class Stack {\n  push(v) {}\n}\n"),
        ],
        "app.js",
        "push",
        &[("lib.ts", N, ResolutionKind::R6SingleOwner)],
        &[("lib.ts", N, ResolutionKind::R6SingleOwner)],
    );
}

#[test]
fn typed_param_typescript_caller_falls_to_r6_and_drops_foreign_owner() {
    // No in-repo TS class: main reaches R6 and binds the Rust impl NameOnly.
    case(
        &[
            ("app.ts", "function run(c: Config) { c.load(); }\n"),
            ("rs/src/lib.rs", RS_CONFIG_M),
        ],
        "app.ts",
        "load",
        &[("rs/src/lib.rs", N, ResolutionKind::R6SingleOwner)],
        &[],
    );
}

// ---- Tail order: anonymous admission runs before the family filter ----

#[test]
fn anonymous_admission_runs_before_the_family_filter() {
    // An anonymous callable (`() => ...`) calls `Config.load()`. R3b pools the same-family JS
    // class and the Rust impl as a multi-target NameOnly pool. Admission cannot prove the site,
    // so it clears the multi-target pool (`LocalBindingUnproven`), which is main's result.
    // Filtering first would narrow the pool to the JS class alone, and admission would then keep
    // that single target as a NameOnly edge that main drops: an addition.
    let cg = graph(&[
        (
            "app.js",
            "function run(items) {\n  items.forEach(() => {\n    Config.load();\n  });\n}\n",
        ),
        ("models.js", "class Config {\n  static load() {}\n}\n"),
        ("rs/src/lib.rs", RS_CONFIG),
    ]);
    let site = site(&cg, "app.js", "load");
    let pool = cg.resolve_call_site_candidates(site);
    assert_eq!(
        edges(&pool),
        owned(&[
            ("models.js", N, ResolutionKind::QualifierOwner),
            ("rs/src/lib.rs", N, ResolutionKind::QualifierOwner),
        ]),
        "fixture calibration: mixed pool at an anonymous site: {pool:?}"
    );
    for (label, outcome) in [
        ("CallGraph", cg.resolve_call_site_full(site)),
        (
            "memo",
            crate::resolution::GoSatisfierMemo::new(&cg).resolve_call_site_full(site),
        ),
    ] {
        assert!(outcome.resolved.is_empty(), "{label}: {outcome:?}");
        assert_eq!(
            outcome.drop,
            Some(DropReason::LocalBindingUnproven),
            "{label}"
        );
    }
    // The swapped order is the addition the real order prevents.
    let swapped = cg.admit_anonymous_exact(
        site,
        cg.resolve_call_site_candidates(site)
            .retain_family(&site.caller.file),
    );
    assert_eq!(
        edges(&swapped),
        owned(&[("models.js", N, ResolutionKind::QualifierOwner)]),
        "fixture must distinguish the orders: {swapped:?}"
    );
}
