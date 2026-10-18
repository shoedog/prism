use super::*;
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn nodes(root: Node<'_>) -> Vec<Node<'_>> {
    let mut stack = vec![root];
    let mut out = Vec::new();
    while let Some(node) = stack.pop() {
        out.push(node);
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    out.sort_by_key(|n| (n.start_byte(), n.end_byte(), n.id()));
    out
}

const EDGES: &str = r#"
let x = {v:0};
function owner(x, q = x.v) {
  x.v = 1;
  { consume(x.v); let x = {v:2}; consume(x.v); }
  { const x = {v:3}; consume(x.v); }
  { var hoisted = x; } consume(hoisted.v);
  try { consume(x.v); } catch (x) { consume(x.v); }
  consume((function x(y = x.v) { return x.v; })(x));
  consume((function other(x) { return x.v; })(x));
  class x { [x.v](x): typeof x { return x.v; } }
  function nested(x = x.v) { var x; return x.v; }
  function sibling() { return x.v; }
  consume(x.v);
}
function same(x = x.v) { var x; consume(x.v); }
consume(function(x = x.v) { var x; consume(x.v); });
consume((x) => { x.v = 2; return x.v; });
function receivers() {
  this.v = 1;
  consume(() => this.v);
  consume(function() { return this.v; });
  class C { [this.v]() { return this.v; } field = this.v; static { consume(this.v); } }
}
"#;

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "assertion `left == right` failed")]
fn scope_boundaries_must_match_language() {
    let p = ParsedFile::parse("a.js", "function f(x) { return x; }", Language::JavaScript).unwrap();
    p.js_ts_scope_binds(p.all_functions()[0], &[], "x", 25);
}

#[test]
fn registration_requires_root_identity_even_with_same_address() {
    let a = ParsedFile::parse("a.js", "function f(x) {}", Language::JavaScript).unwrap();
    let b = ParsedFile::parse("b.js", "function g(y) {}", Language::JavaScript).unwrap();
    let session = Session::new(&a);
    assert!(!boolean(cached(
        &a,
        "x",
        || Key::At(0),
        || Value::Bool(false)
    )));
    // Simulate address reuse independently of tree identity, without unsafe code.
    ACTIVE
        .with(|slot| slot.borrow_mut().last_mut().unwrap().file = &b as *const ParsedFile as usize);
    assert!(boolean(cached(
        &b,
        "x",
        || Key::At(0),
        || Value::Bool(true)
    )));
    drop(session);
    assert!(ACTIVE.with(|slot| slot.borrow().is_empty()));
}

fn differential(p: &ParsedFile, exhaustive: bool) -> usize {
    let _session = Session::new(p);
    let mut comparisons = 0;
    let mut owners = p.all_functions();
    if owners.is_empty() {
        owners.push(p.tree.root_node());
    }
    for owner in owners {
        let all = nodes(owner);
        let refs: Vec<_> = all
            .iter()
            .copied()
            .filter(|n| p.language.is_identifier_node(n.kind()) || n.kind() == "this")
            .collect();
        let stride = if exhaustive {
            1
        } else {
            (refs.len() / 12).max(1)
        };
        for node in refs.iter().step_by(stride) {
            let name = p.node_text(node);
            let mut defs = vec![owner.start_byte(), node.start_byte()];
            if let Some(last) = refs.iter().rev().find(|n| p.node_text(n) == name) {
                defs.push(last.start_byte());
            }
            if exhaustive {
                defs.extend(
                    refs.iter()
                        .filter(|n| p.node_text(n) == name)
                        .map(|n| n.start_byte()),
                );
            }
            defs.sort();
            defs.dedup();
            for raw in [name, "absent", "this"] {
                for byte in &defs {
                    let line = p
                        .tree
                        .root_node()
                        .descendant_for_byte_range(*byte, byte + 1)
                        .map(|n| n.start_position().row + 1)
                        .unwrap_or(0);
                    let expected = direct(|| {
                        (
                            crate::profile::gates::fence::<false>(p, node, &owner, raw, *byte),
                            crate::profile::gates::shadow::<false>(p, node, &owner, raw, line),
                            crate::profile::gates::def_scope(p, &owner, raw, *byte),
                        )
                    });
                    let actual = (
                        crate::profile::gates::fence::<false>(p, node, &owner, raw, *byte),
                        crate::profile::gates::shadow::<false>(p, node, &owner, raw, line),
                        crate::profile::gates::def_scope(p, &owner, raw, *byte),
                    );
                    assert_eq!(
                        actual,
                        expected,
                        "{} {:?} owner={} ref={} name={raw} def={byte}",
                        p.path,
                        p.language,
                        owner.start_byte(),
                        node.start_byte()
                    );
                    comparisons += 1;
                }
            }
        }
    }
    comparisons
}

#[test]
fn differential_binding_regions_and_owners() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = ParsedFile::parse("edge.ts", EDGES, lang).unwrap();
        assert!(differential(&p, true) > 500);
    }
}

#[test]
fn differential_generated_bindings() {
    for declaration in ["var", "let", "const"] {
        for default in ["", " = x.v"] {
            for binder in [
                "",
                "let x = {v: 2};",
                "var x = {v: 2};",
                "function x() {}",
                "class x {}",
            ] {
                let source = format!("function outer(x{default}) {{\n x.v = 1;\n {{ {binder} consume(x.v); }}\n {declaration} local = x;\n consume((x => x.v)(x));\n consume(function(x{default}) {{ var x; return x.v; }});\n return x.v;\n }}");
                let p = ParsedFile::parse("generated.ts", &source, Language::TypeScript).unwrap();
                assert!(differential(&p, true) > 100);
            }
        }
    }
}

#[test]
fn differential_all_repository_fixtures() {
    // Match the existing member-index control: deep C parsing needs 64 MiB.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(all_repository_fixtures)
        .unwrap()
        .join()
        .unwrap();
}

fn all_repository_fixtures() {
    fn visit(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == "frontend-portal" {
                continue;
            }
            if path.is_dir() {
                visit(&path, files);
            } else {
                files.push(path);
            }
        }
    }
    let mut files = Vec::new();
    visit(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        &mut files,
    );
    files.sort();
    let mut parsed = 0;
    let mut comparisons = 0;
    for path in files {
        let Some(lang) = Language::from_path(path.to_str().unwrap()) else {
            continue;
        };
        let source = std::fs::read_to_string(&path).unwrap();
        let p = ParsedFile::parse(path.to_str().unwrap(), &source, lang).unwrap();
        comparisons += differential(&p, false);
        parsed += 1;
    }
    println!("binding memo: {parsed} parsed fixtures, {comparisons} direct/memo verdict triples");
    assert!(parsed > 50);
    assert!(comparisons > 1000);
}

#[test]
fn differential_every_non_js_gate_language() {
    let cases = [
        (
            Language::Python,
            "def f(x):\n y = x\n if y:\n  x = y\n return x\n",
        ),
        (
            Language::Go,
            "package p\nfunc f(x int) int { y := x; { x := y; use(x) }; return x }",
        ),
        (
            Language::Java,
            "class C { int f(int x) { int y = x; { int x = y; use(x); } return x; } }",
        ),
        (
            Language::C,
            "int f(int x) { int y = x; { int x = y; use(x); } return x; }",
        ),
        (
            Language::Cpp,
            "int f(int x) { int y = x; { int x = y; use(x); } return x; }",
        ),
        (
            Language::Rust,
            "fn f(x: i32) -> i32 { let y = x; { let x = y; use_x(x); } x }",
        ),
        (
            Language::Lua,
            "function f(x) local y = x; do local x = y; use(x) end; return x end",
        ),
        (Language::Terraform, "locals { x = 1\n y = local.x\n }"),
        (Language::Bash, "f() { local x=1; echo $x; }"),
    ];
    let mut seen = BTreeSet::new();
    for (lang, source) in cases {
        let p = ParsedFile::parse("non-js", source, lang).unwrap();
        assert!(
            differential(&p, true) > 0,
            "{lang:?} selected no identifiers"
        );
        seen.insert(lang);
    }
    assert_eq!(seen.len(), Language::all().len() - 3);
}

#[test]
fn epoch_isolation_and_session_drop() {
    let a = ParsedFile::parse(
        "same.ts",
        "function f(x) { return x; }",
        Language::TypeScript,
    )
    .unwrap();
    let b = ParsedFile::parse(
        "same.ts",
        "function f(y) { return x; }",
        Language::TypeScript,
    )
    .unwrap();
    let byte = a.source.rfind('x').unwrap();
    let session = Session::new(&a);
    let expected_a = direct(|| a.js_ts_binding_scope_at(byte, "x"));
    let expected_b = direct(|| b.js_ts_binding_scope_at(byte, "x"));
    assert_ne!(expected_a, expected_b);
    assert_eq!(a.js_ts_binding_scope_at(byte, "x"), expected_a);
    assert_eq!(
        b.js_ts_binding_scope_at(byte, "x"),
        expected_b,
        "foreign file without session cannot use active memo"
    );
    let other = Session::new(&b);
    assert_eq!(b.js_ts_binding_scope_at(byte, "x"), expected_b);
    assert_eq!(a.js_ts_binding_scope_at(byte, "x"), expected_a);
    drop(session); // Deliberately not stack order.
    assert_eq!(a.js_ts_binding_scope_at(byte, "x"), expected_a);
    drop(other);
    ACTIVE.with(|slot| assert!(slot.borrow().is_empty()));
}

#[test]
fn cache_keys_preserve_names_scopes_regions_and_bytes() {
    let source = "let x = 0; function a(x) { var body; { let inner; use(x); } } class C { [x](x): typeof x { return x; } } function b(y) { return x; } consume(function named(x = named) { return named; });";
    let p = ParsedFile::parse("keys.ts", source, Language::TypeScript).unwrap();
    let _session = Session::new(&p);
    let boundaries = p.language.callable_boundary_node_types();
    let all = nodes(p.tree.root_node());
    let scopes: Vec<_> = all
        .iter()
        .copied()
        .filter(|n| cacheable(*n, &boundaries))
        .collect();
    let bytes: Vec<_> = all.iter().map(|n| n.start_byte()).collect();
    let mut yes = 0;
    let mut no = 0;
    for name in ["x", "y", "body", "inner", "missing"] {
        for byte in &bytes {
            assert_eq!(
                p.js_ts_binding_scope_at(*byte, name),
                direct(|| p.js_ts_binding_scope_at(*byte, name)),
                "byte/name key {name} {byte}"
            );
            for scope in &scopes {
                assert_eq!(
                    p.scope_has_declaration(*scope, name),
                    direct(|| p.scope_has_declaration(*scope, name)),
                    "declaration scope/name key {name} {}",
                    scope.start_byte()
                );
                let expected = direct(|| p.js_ts_scope_binds(*scope, &boundaries, name, *byte));
                let actual = p.js_ts_scope_binds(*scope, &boundaries, name, *byte);
                assert_eq!(
                    actual,
                    expected,
                    "scope/region/name key: {name} {byte} {}",
                    scope.start_byte()
                );
                if actual {
                    yes += 1;
                } else {
                    no += 1;
                }
            }
        }
    }
    assert!(yes > 10 && no > 10);
}

fn build(files: &BTreeMap<String, ParsedFile>, threads: usize, bypass: bool) -> Vec<u8> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .start_handler(move |_| BYPASS.with(|b| b.set(bypass)))
        .build()
        .unwrap()
        .install(|| bincode::serialize(&DataFlowGraph::build(files)).unwrap())
}

#[test]
fn dfg_rayon_reparse_and_partial_rebuild() {
    let mut files = BTreeMap::from([
        (
            "edge.ts".into(),
            ParsedFile::parse("edge.ts", EDGES, Language::TypeScript).unwrap(),
        ),
        (
            "other.ts".into(),
            ParsedFile::parse(
                "other.ts",
                "function sibling(x) {\n x.v = 1;\n return x.v;\n }",
                Language::TypeScript,
            )
            .unwrap(),
        ),
    ]);
    let direct_before = build(&files, 1, true);
    assert_eq!(build(&files, 1, false), direct_before);
    assert_eq!(build(&files, 4, false), direct_before);
    files.insert(
        "edge.ts".into(),
        ParsedFile::parse(
            "edge.ts",
            "consume((x = x.v) => {\n var x;\n x.v = 1;\n return x.v;\n });",
            Language::TypeScript,
        )
        .unwrap(),
    );
    assert_eq!(build(&files, 4, false), build(&files, 1, true));
    let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["edge.ts".into()]));
    let fresh = BTreeMap::from([("edge.ts".into(), files["edge.ts"].clone())]);
    assert_eq!(bincode::serialize(&subset).unwrap(), build(&fresh, 1, true));
}
