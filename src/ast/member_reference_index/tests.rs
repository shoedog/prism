use super::*;

// Mirrors the supplied member-fixtures.py byte for byte.
fn member_fixture(n: usize, distinct: bool) -> String {
    let mut source = "export function giant(obj) {\n".to_string();
    if distinct {
        for i in 0..n {
            source += &format!("  obj.p{i} = {i};\n  consume(obj.p{i});\n");
        }
    } else {
        source += "  obj.p0 = 0;\n";
        source += &"  consume(obj.p0);\n".repeat(n);
    }
    source += "}\nfunction consume(value) { return value; }\n";
    source
}

#[test]
fn member_index_cost_is_per_file_not_per_search() {
    for n in [256, 512, 1024] {
        let source = member_fixture(n, false);
        let parsed = ParsedFile::parse("fixture.ts", &source, Language::TypeScript).unwrap();
        let owner = parsed.all_functions()[0];
        let search = MemberReferenceSearch::new(&parsed);
        let path = AccessPath::from_expr("obj.p0");
        let byte = source.find("obj.p0").unwrap();
        NORMALIZATIONS.with(|c| c.set(0));
        for _ in 0..8 {
            let refs =
                search.find_path_reference_spans_fenced_in(&owner, &path, 2, byte, None, false);
            assert_eq!(refs.len(), n);
        }
        let indexed = NORMALIZATIONS.with(|c| c.replace(0));
        for _ in 0..8 {
            let refs =
                parsed.legacy_path_reference_spans_fenced_in(&owner, &path, 2, byte, None, false);
            assert_eq!(refs.len(), n);
        }
        let legacy = NORMALIZATIONS.with(|c| c.get());
        println!("member-statements-{n}: legacy={legacy}, indexed={indexed}, searches=8");
        assert_eq!(legacy, 8 * (n + 1));
        assert_eq!(
            indexed,
            n + 1,
            "normalise each member exactly once per file"
        );
    }
}

fn paths(parsed: &ParsedFile, node: Node<'_>, out: &mut BTreeSet<AccessPath>) {
    if ParsedFile::is_field_access_node(node.kind()) {
        out.insert(
            parsed
                .bounded_member_path(node)
                .map(|m| m.path)
                .unwrap_or_else(|| AccessPath::from_expr(parsed.node_text(&node))),
        );
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        paths(parsed, child, out);
    }
}

fn differential(parsed: &ParsedFile, exhaustive: bool) -> usize {
    let mut all_paths = BTreeSet::new();
    paths(parsed, parsed.tree.root_node(), &mut all_paths);
    all_paths.insert(AccessPath::from_expr("absent.member"));
    let search = MemberReferenceSearch::new(parsed);
    let mut comparisons = 0;
    for owner in parsed.all_functions() {
        for path in &all_paths {
            let def_byte = parsed.source.find(&path.base).unwrap_or(owner.start_byte());
            for def_line in [0, owner.start_position().row + 1]
                .into_iter()
                .skip(usize::from(!exhaustive))
            {
                assert_eq!(
                    search.scoped(&owner, path, def_line),
                    parsed.legacy_path_references_scoped(&owner, path, def_line),
                    "{}: {path:?}",
                    parsed.path
                );
                for reads in [false, true]
                    .into_iter()
                    .take(if exhaustive { 2 } else { 1 })
                {
                    let within = owner
                        .child_by_field_name("body")
                        .map(|n| (n.start_byte(), n.end_byte()));
                    for scope in [
                        None,
                        within,
                        Some((owner.start_byte(), owner.start_byte() + 1)),
                    ]
                    .into_iter()
                    .take(if exhaustive { 3 } else { 1 })
                    {
                        assert_eq!(search.find_path_reference_spans_fenced_in(&owner, path, def_line, def_byte, scope, reads),
                            parsed.legacy_path_reference_spans_fenced_in(&owner, path, def_line, def_byte, scope, reads),
                            "{}: owner={} path={path:?} cutoff={def_line} scope={scope:?} reads={reads}", parsed.path, owner.start_byte());
                        comparisons += 1;
                    }
                }
            }
        }
    }
    comparisons
}

#[test]
fn member_index_differential_all_fixtures() {
    fn visit(dir: &std::path::Path, files: &mut usize, comparisons: &mut usize) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name == "frontend-portal"
                    || name.starts_with("f-")
                    || name.starts_with("F-private")
                    || name.contains("SecBench")
                {
                    continue;
                }
                visit(&path, files, comparisons);
            } else if entry.file_type().unwrap().is_file() {
                let Some(language) = Language::from_path(&path.to_string_lossy()) else {
                    continue;
                };
                let Ok(source) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let parsed = ParsedFile::parse(&path.to_string_lossy(), &source, language).unwrap();
                *comparisons += differential(&parsed, false);
                *files += 1;
            }
        }
    }
    // The existing deep C parse fixture needs the same generous stack as
    // cpg_build_survives_deep_c_initializer_without_stack_overflow.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let (mut files, mut comparisons) = (0, 0);
            visit(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
                &mut files,
                &mut comparisons,
            );
            for n in [256, 512, 1024] {
                for distinct in [false, true] {
                    let source = member_fixture(n, distinct);
                    let parsed =
                        ParsedFile::parse("generated.ts", &source, Language::TypeScript).unwrap();
                    comparisons += differential(&parsed, false);
                    files += 1;
                }
            }
            println!("all fixtures plus generated: files={files}, span comparisons={comparisons}");
            assert!(files > 100 && comparisons > 1000);
        })
        .unwrap()
        .join()
        .unwrap();
}

const EDGES: &str = r#"
function owner(obj, seam = obj.x) {
  obj.x = 1;
  const alias = obj;
  alias.x = 2;
  use(obj.x, obj.x, alias.x);
  { let obj = {}; use(obj.x); }
  try {} catch (obj) { use(obj.x); }
  function nested(obj) { use(obj.x); }
  const named = function obj() { use(obj.x); };
  const captured = () => obj?.x;
  use(obj.x.y, obj["x"], obj[key], obj?.[key], this.x);
  obj.x = obj.x;
  use((obj as T).x, (obj satisfies T).x);
  obj.x[key].y.z = 1;
  with (other) { use(obj.x); }
  return function() { return this.x + obj.x; };
}
function sibling(obj) { obj.x = 3; return obj.x; }
const anon = (obj = obj.x) => { obj.x = 1; return obj.x; };
class Example { [obj.x](obj = obj.x) { return obj.x + this.x; } }
function multiline(obj) {
 obj.a.b.c.d.e.f = 1;
 use(obj.a.b.c.d.e.f
  .g);
}
"#;

#[test]
fn member_index_differential_edges_and_non_js() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("edges.ts", EDGES, language).unwrap();
        assert!(differential(&parsed, true) > 100);
    }
    for (language, source) in [
        (Language::Go, "package p\nfunc f(obj T) {\n obj.x = 1\n use(obj.x, obj.x)\n}\nfunc g(obj T) {\n use(obj.x)\n}\n"),
        (Language::Rust, "fn f(obj: T) {\n obj.x = 1;\n use_it(obj.x, obj.x.y);\n}\nfn g(obj: T) {\n use_it(obj.x);\n}\n"),
        (Language::Python, "def f(obj):\n obj.x = 1\n use(obj.x, obj.x.y)\ndef g(obj):\n use(obj.x)\n"),
        (Language::C, "void f(T *obj) {\n obj->x = 1;\n use(obj->x);\n}\n"),
        (Language::Cpp, "void f(T *obj) {\n obj->x = 1;\n use(obj->x);\n}\n"),
        (Language::Java, "class C { void f(T obj) {\n obj.x = 1;\n use(obj.x);\n} }"),
        (Language::Lua, "function f(obj)\n obj.x = 1\n use(obj.x)\nend\n"),
    ] {
        let parsed = ParsedFile::parse("nonjs", source, language).unwrap();
        let mut found = BTreeSet::new();
        paths(&parsed, parsed.tree.root_node(), &mut found);
        assert!(!found.is_empty(), "{language:?} must reach member walk");
        assert!(differential(&parsed, true) > 0, "{language:?}");
    }
}

#[test]
fn member_index_lazy_simple_and_reparse_lifetime() {
    let mut parsed = ParsedFile::parse(
        "same.ts",
        "function f(obj) {\n return obj.x;\n}",
        Language::TypeScript,
    )
    .unwrap();
    {
        let search = MemberReferenceSearch::new(&parsed);
        let owner = parsed.all_functions()[0];
        search.find_path_reference_spans_fenced_in(
            &owner,
            &AccessPath::simple("obj"),
            0,
            11,
            None,
            false,
        );
        assert!(search.index.get().is_none());
        assert_eq!(
            search.scoped(&owner, &AccessPath::from_expr("obj.x"), 0),
            BTreeSet::from([2])
        );
    }
    // Reuse the path and object slot, replacing the entire parsed tree.
    parsed = ParsedFile::parse(
        "same.ts",
        "function f(obj) {\n return obj.y;\n}",
        Language::TypeScript,
    )
    .unwrap();
    let search = MemberReferenceSearch::new(&parsed);
    let owner = parsed.all_functions()[0];
    assert!(search
        .scoped(&owner, &AccessPath::from_expr("obj.x"), 0)
        .is_empty());
    assert_eq!(
        search.scoped(&owner, &AccessPath::from_expr("obj.y"), 0),
        BTreeSet::from([2])
    );
}

#[test]
fn member_index_suppresses_matching_parent_before_gates() {
    let source = "function f(obj) {\n obj.x[key].y.z = 1;\n use(obj.x[key].y.z);\n}";
    let parsed = ParsedFile::parse("suppression.ts", source, Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    let path = AccessPath::from_expr("obj.x[key].y.z");
    let search = MemberReferenceSearch::new(&parsed);
    let matches: Vec<_> = search
        .matching(owner, &path, 0)
        .map(|n| parsed.node_text(&n))
        .collect();
    assert_eq!(matches, ["obj.x[key].y.z", "obj.x[key].y.z"]);
    assert_eq!(
        search.find_path_reference_spans_fenced_in(
            &owner,
            &path,
            0,
            source.find("obj").unwrap(),
            None,
            true
        ),
        parsed.legacy_path_reference_spans_fenced_in(
            &owner,
            &path,
            0,
            source.find("obj").unwrap(),
            None,
            true
        )
    );
}

#[test]
fn member_index_alias_twins_and_partial_rebuild_match_oracle() {
    use crate::data_flow::DataFlowGraph;
    let mut files = BTreeMap::from([
        (
            "edges.ts".to_string(),
            ParsedFile::parse("edges.ts", EDGES, Language::TypeScript).unwrap(),
        ),
        (
            "other.ts".to_string(),
            ParsedFile::parse(
                "other.ts",
                "function sibling(obj) {\n obj.x = 1;\n return obj.x;\n}",
                Language::TypeScript,
            )
            .unwrap(),
        ),
    ]);
    fn build(files: &BTreeMap<String, ParsedFile>, legacy: bool) -> Vec<u8> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .start_handler(move |_| USE_ORACLE.with(|flag| flag.set(legacy)))
            .build()
            .unwrap();
        pool.install(|| bincode::serialize(&DataFlowGraph::build(files)).unwrap())
    }
    let indexed = build(&files, false);
    let legacy = build(&files, true);
    assert_eq!(
        indexed, legacy,
        "complete ordered DFG including alias twins"
    );
    files.insert("edges.ts".into(), ParsedFile::parse("edges.ts", "function owner(obj) {\n obj.y = 1;\n const alias = obj;\n alias.y = 2;\n return obj.y;\n}", Language::TypeScript).unwrap());
    // build_subset uses the same file producer after a cache partial hit.
    let changed = BTreeSet::from(["edges.ts".to_string()]);
    let subset = DataFlowGraph::build_subset(&files, &changed);
    let fresh = DataFlowGraph::build(&BTreeMap::from([(
        "edges.ts".into(),
        ParsedFile::parse("edges.ts", &files["edges.ts"].source, Language::TypeScript).unwrap(),
    )]));
    assert_eq!(
        bincode::serialize(&subset).unwrap(),
        bincode::serialize(&fresh).unwrap()
    );
    assert_eq!(build(&files, false), build(&files, true));
}
