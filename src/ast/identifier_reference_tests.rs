use super::*;
use crate::access_path::AccessPath;
use crate::ast::member_reference_index::MemberReferenceSearch;
use crate::languages::Language;

fn reset() {
    QUERIES.with(|n| n.set(0));
    CAPTURES.with(|n| n.set(0));
    EXAMINED.with(|n| n.set(0));
}
fn counts() -> (usize, usize, usize) {
    (
        QUERIES.with(|n| n.get()),
        CAPTURES.with(|n| n.get()),
        EXAMINED.with(|n| n.get()),
    )
}

#[test]
fn identifier_index_cost_is_per_file_not_per_search() {
    let mut source = "function giant() {\n".to_string();
    for i in 0..32 {
        source += &format!("const x{i} = {i};\nconsume(x{i});\n");
    }
    source += "}\n";
    let parsed = ParsedFile::parse("fixture.ts", &source, Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    let search = MemberReferenceSearch::new(&parsed);
    reset();
    for i in 0..32 {
        let name = format!("x{i}");
        let result = search.find_path_reference_spans_fenced_in(
            &owner,
            &AccessPath::simple(&name),
            2 * i + 2,
            source.find(&name).unwrap(),
            None,
            false,
        );
        assert_eq!(result.len(), 2);
    }
    assert_eq!(
        counts().0,
        1,
        "one identifier query per file, independent of definitions"
    );
}

fn fixture(family: &str, n: usize) -> String {
    let mut s = "export function giant() {\n".to_string();
    match family {
        "locals" => {
            for i in 0..n {
                s += &format!("  const x{i} = {i};\n  consume(x{i});\n");
            }
        }
        "statements" => {
            s += "  const x = 0;\n";
            s += &"  consume(x);\n".repeat(n);
        }
        "nested" => {
            for i in 0..n {
                s += &format!("  function child{i}() {{ const x{i} = {i}; return x{i}; }}\n");
            }
        }
        "locals-nocall" => {
            for i in 0..n {
                s += &format!("  const x{i} = {i};\n  x{i};\n");
            }
        }
        _ => unreachable!(),
    }
    s += "}\n";
    if family != "locals-nocall" {
        s += "function consume(value) { return value; }\n";
    }
    s
}

#[test]
fn identifier_index_scaling_operation_counts() {
    for family in ["locals", "statements", "nested", "locals-nocall"] {
        for n in [512, 1024, 2048] {
            let source = fixture(family, n);
            let parsed = ParsedFile::parse("fixture.ts", &source, Language::TypeScript).unwrap();
            let owner = parsed.all_functions()[0];
            let queries: Vec<_> = (0..if family == "statements" { 8 } else { n })
                .map(|i| {
                    if family == "statements" {
                        "x".to_string()
                    } else {
                        format!("x{i}")
                    }
                })
                .collect();
            let search = MemberReferenceSearch::new(&parsed);
            reset();
            let actual: Vec<Vec<_>> = queries
                .iter()
                .map(|name| {
                    search
                        .simple
                        .index()
                        .unwrap()
                        .matching(name, owner.byte_range())
                        .collect()
                })
                .collect();
            let indexed = counts();
            reset();
            let query = get_query(parsed.language, QueryKind::Identifiers).unwrap();
            let expected: Vec<Vec<_>> = queries
                .iter()
                .map(|name| {
                    record_query();
                    raw(&parsed, query, owner.byte_range())
                        .into_iter()
                        .filter(|node| {
                            record_capture();
                            parsed.node_text(node) == name
                        })
                        .collect()
                })
                .collect();
            let legacy = counts();
            assert_eq!(actual, expected);
            assert!(actual.iter().all(|refs| !refs.is_empty()));
            assert_eq!(indexed.0, 1);
            assert_eq!(legacy.0, queries.len());
            // Every repeated legacy range covers the same owner; the index
            // covers the file once (including consume's three identifiers).
            assert_eq!(
                legacy.1,
                queries.len() * (indexed.1 - if family == "locals-nocall" { 0 } else { 3 })
            );
            println!("{family}-{n}: searches={} legacy_queries={} indexed_queries={} legacy_captures={} indexed_captures={} name_candidates={}", queries.len(), legacy.0, indexed.0, legacy.1, indexed.1, indexed.2);
        }
    }
}

fn raw<'a>(parsed: &'a ParsedFile, query: &Query, range: Range<usize>) -> Vec<Node<'a>> {
    let ident = query.capture_index_for_name("ident").unwrap();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range);
    let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
    let mut out = Vec::new();
    while let Some(m) = matches.next() {
        for c in m.captures {
            if c.index == ident {
                out.push(c.node);
            }
        }
    }
    out
}

fn language_cases() -> [(Language, &'static str); 12] {
    [
        (Language::JavaScript, "function f(longname) { longname = longname; const o = {longname}; return longname; }"),
        (Language::TypeScript, "function f(longname: T) { longname = longname; const o = {longname}; return longname; }"),
        (Language::Tsx, "function f(longname: T) { const o = {longname}; return <Tag>{longname}</Tag>; }"),
        (Language::Python, "def f(longname):\n longname = longname\n return longname\n"),
        (Language::Go, "package p\nfunc f(longname int) int { longname = longname; return longname }"),
        (Language::Rust, "fn f(longname: T) -> T { let x = longname; longname }"),
        (Language::C, "int f(int longname) { longname = longname; return longname; }"),
        (Language::Cpp, "int f(int longname) { longname = longname; return longname; }"),
        (Language::Java, "class C { int f(int longname) { longname = longname; return longname; } }"),
        (Language::Lua, "function f(longname)\n longname = longname\n return longname\nend"),
        (Language::Terraform, "locals { longname = longname }\noutput \"x\" { value = longname }"),
        (Language::Bash, "f() { longname=$longname; echo longname $longname; }"),
    ]
}

fn ranges_match(parsed: &ParsedFile, query: &Query) {
    let index = Index::build(parsed, query);
    let all = raw(parsed, query, 0..0);
    assert!(!all.is_empty());
    let names: BTreeSet<_> = all.iter().map(|n| parsed.node_text(n)).collect();
    let mut ranges = BTreeSet::from([
        (0, 0),
        (parsed.source.len(), parsed.source.len()),
        (0, parsed.source.len()),
    ]);
    for n in &all {
        let (s, e) = (n.start_byte(), n.end_byte());
        ranges.extend([
            (s, e),
            (s, s),
            (e, e),
            (s + 1, e),
            (s, e.saturating_sub(1)),
            (0, s),
            (e, 0),
        ]);
        if e > s + 1 {
            ranges.insert((s + 1, e - 1));
        }
    }
    for (s, e) in ranges {
        let expected = raw(parsed, query, s..e);
        for name in &names {
            let old: Vec<_> = expected
                .iter()
                .copied()
                .filter(|n| parsed.node_text(n) == *name)
                .collect();
            let new: Vec<_> = index.matching(name, s..e).collect();
            assert_eq!(new, old, "{:?} name={name} range={s}..{e}", parsed.language);
        }
        assert_eq!(index.matching("absent", s..e).count(), 0);
    }
}

#[test]
fn identifier_index_raw_ranges_order_duplicates_and_spelling() {
    for (lang, source) in language_cases() {
        let parsed = ParsedFile::parse("case", source, lang).unwrap();
        ranges_match(&parsed, get_query(lang, QueryKind::Identifiers).unwrap());
    }
    let parsed = ParsedFile::parse(
        "escaped.js",
        r"function f(x) { use(x, \u0078, x, x); }",
        Language::JavaScript,
    )
    .unwrap();
    ranges_match(
        &parsed,
        get_query(parsed.language, QueryKind::Identifiers).unwrap(),
    );
    let query = Query::new(
        &parsed.language.tree_sitter_language(),
        "(identifier) @ident @ident\n(identifier) @ident",
    )
    .unwrap();
    let nodes = raw(&parsed, &query, 0..0);
    assert!(
        nodes.windows(2).any(|w| w[0] == w[1]),
        "duplicate-capture witness"
    );
    ranges_match(&parsed, &query);
    // Recovery can produce zero-width nodes. Compare the query rather than
    // assuming missing identifiers are captured by a given grammar.
    for (lang, src) in [
        (Language::Rust, "fn f() { let = ; }"),
        (Language::JavaScript, "function f(x) { let x = ; x; }"),
    ] {
        let parsed = ParsedFile::parse("recovery", src, lang).unwrap();
        ranges_match(&parsed, get_query(lang, QueryKind::Identifiers).unwrap());
    }
}

const EDGES: &str = r#"
function owner(x, seam = x) {
 const raw = x;
 x = x; x++; use(x, x); ({x});
 { let x = 7; use(x); }
 try {} catch (x) { use(x); }
 function nested(x) { return x; }
 const named = function x() { return x; };
 class x { [x]() { return x; } }
 const captured = () => x;
 with (other) { use(x); }
 return function() { return x; };
}
function sibling(x) { return x; }
const anon = (x = x) => { x = x; return x; };
class Example { [x](x = x) { return x; } }
function plain(x) { x = x; use(x, x); return x; }
"#;

fn differential(parsed: &ParsedFile, exhaustive: bool) -> usize {
    let Some(query) = get_query(parsed.language, QueryKind::Identifiers) else {
        return 0;
    };
    let search = MemberReferenceSearch::new(parsed);
    let mut comparisons = 0;
    for owner in parsed.all_functions() {
        let captures = raw(parsed, query, owner.byte_range());
        let mut definitions = BTreeMap::new();
        for node in captures {
            definitions.entry(parsed.node_text(&node)).or_insert(node);
        }
        let mut roots = vec![None];
        if exhaustive {
            let mut pending = vec![owner];
            while let Some(n) = pending.pop() {
                if matches!(
                    n.kind(),
                    "statement_block"
                        | "block"
                        | "computed_property_name"
                        | "formal_parameters"
                        | "catch_clause"
                ) {
                    roots.push(Some((n.start_byte(), n.end_byte())));
                }
                let mut c = n.walk();
                pending.extend(n.children(&mut c));
            }
            roots.push(Some((owner.start_byte(), owner.start_byte() + 1)));
        }
        for (name, node) in definitions {
            let line = node.start_position().row + 1;
            let path = AccessPath::simple(name);
            assert_eq!(
                search.simple.scoped::<false>(&owner, name, line),
                oracle::legacy_scoped_refs::<false>(parsed, &owner, name, line)
            );
            for reads in [false, true] {
                for root in &roots {
                    let new = search.find_path_reference_spans_fenced_in(
                        &owner,
                        &path,
                        line,
                        node.start_byte(),
                        *root,
                        reads,
                    );
                    let old = parsed.legacy_path_reference_spans_fenced_in(
                        &owner,
                        &path,
                        line,
                        node.start_byte(),
                        *root,
                        reads,
                    );
                    assert_eq!(
                        new,
                        old,
                        "{} owner={} name={name} root={root:?} reads={reads}",
                        parsed.path,
                        owner.start_byte()
                    );
                    comparisons += 1;
                }
            }
        }
    }
    comparisons
}

#[test]
fn identifier_index_gates_edges_and_all_languages() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("edges", EDGES, lang).unwrap();
        assert!(parsed.all_functions().iter().any(|n| parsed
            .dfg_owner_name(n)
            .is_some_and(|s| s.starts_with("<anon@"))));
        assert!(differential(&parsed, true) > 100);
    }
    for (lang, src) in language_cases() {
        let parsed = ParsedFile::parse("case", src, lang).unwrap();
        // HCL blocks are DFG owners too. Exercise the root helper as well as
        // every language's owner/session route below.
        let root = parsed.tree.root_node();
        let search = IdentifierReferenceSearch::new(&parsed);
        assert_eq!(
            search.scoped::<false>(&root, "longname", 1),
            oracle::legacy_scoped_refs::<false>(&parsed, &root, "longname", 1)
        );
        assert!(
            !search.scoped::<false>(&root, "longname", 1).is_empty(),
            "{lang:?}"
        );
        assert!(
            parsed
                .all_functions()
                .iter()
                .any(|n| parsed.dfg_owner_name(n).is_some()),
            "{lang:?} must reach the DFG producer"
        );
        assert!(differential(&parsed, true) > 0, "{lang:?}");
    }
}

#[test]
fn identifier_index_differential_all_fixtures_and_generated() {
    fn visit(dir: &std::path::Path, files: &mut usize, comparisons: &mut usize) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == "frontend-portal"
                || name.starts_with("f-")
                || name.starts_with("F-private")
                || name.contains("SecBench")
            {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                visit(&path, files, comparisons);
            } else if let Some(lang) = Language::from_path(&path.to_string_lossy()) {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    let parsed = ParsedFile::parse(&path.to_string_lossy(), &source, lang).unwrap();
                    *comparisons += differential(&parsed, false);
                    *files += 1;
                }
            }
        }
    }
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let (mut files, mut comparisons) = (0, 0);
            visit(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
                &mut files,
                &mut comparisons,
            );
            for family in ["locals", "statements", "nested", "locals-nocall"] {
                for n in [16, 32, 64] {
                    let source = fixture(family, n);
                    let parsed =
                        ParsedFile::parse("generated.ts", &source, Language::TypeScript).unwrap();
                    comparisons += differential(&parsed, false);
                    files += 1;
                }
            }
            println!("identifier differential: files={files}, comparisons={comparisons}");
            assert!(files > 100 && comparisons > 1000);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn identifier_index_fallback_lifetime_and_gate_counts() {
    let mut parsed = ParsedFile::parse("same.ts", EDGES, Language::TypeScript).unwrap();
    {
        let search = IdentifierReferenceSearch::new(&parsed);
        assert!(search.index.get().is_none());
        let owner = parsed.all_functions()[0];
        let body = owner.child_by_field_name("body").unwrap();
        // Force the no-query condition without altering the query registry.
        assert!(search.index.set(None).is_ok());
        let mut expected = BTreeSet::new();
        parsed.collect_variable_refs_scoped_manual(owner, "x", 1, &mut expected);
        assert!(!expected.is_empty());
        assert_eq!(search.scoped::<false>(&owner, "x", 1), expected);
        let mut out = BTreeMap::new();
        search.collect_fenced::<true>(body, &owner, "x", 1, 0, true, &mut out);
        assert_eq!(
            out,
            expected
                .into_iter()
                .map(|line| (
                    line,
                    (parsed.line_start_byte(line), parsed.line_start_byte(line))
                ))
                .collect()
        );
    }
    for count in [false, true] {
        let owner = parsed.all_functions()[0];
        let search = IdentifierReferenceSearch::new(&parsed);
        fn run<const C: bool>(
            search: &IdentifierReferenceSearch<'_>,
            owner: &Node<'_>,
        ) -> BTreeMap<usize, (usize, usize)> {
            let mut out = BTreeMap::new();
            search.collect_fenced::<C>(*owner, owner, "x", 2, 16, true, &mut out);
            out
        }
        crate::profile::GATE_COUNTERS.with(|c| *c.borrow_mut() = Some(Default::default()));
        let actual = if count {
            run::<true>(&search, &owner)
        } else {
            run::<false>(&search, &owner)
        };
        let actual_counts = crate::profile::GATE_COUNTERS.with(|c| c.borrow_mut().take().unwrap());
        crate::profile::GATE_COUNTERS.with(|c| *c.borrow_mut() = Some(Default::default()));
        let mut old = BTreeMap::new();
        if count {
            parsed.legacy_collect_fenced_simple_refs::<true>(
                owner, &owner, "x", 2, 16, true, &mut old,
            );
        } else {
            parsed.legacy_collect_fenced_simple_refs::<false>(
                owner, &owner, "x", 2, 16, true, &mut old,
            );
        }
        let old_counts = crate::profile::GATE_COUNTERS.with(|c| c.borrow_mut().take().unwrap());
        assert_eq!(actual, old);
        assert_eq!(
            actual_counts.shadow_check_calls,
            old_counts.shadow_check_calls
        );
        assert_eq!(
            actual_counts.binding_fence_calls,
            old_counts.binding_fence_calls
        );
        assert_eq!(actual_counts.shadow_check_calls > 0, count);
        assert_eq!(actual_counts.binding_fence_calls > 0, count);
    }
    parsed = ParsedFile::parse(
        "same.ts",
        "function f(longname) { return longname; }",
        Language::TypeScript,
    )
    .unwrap();
    let search = IdentifierReferenceSearch::new(&parsed);
    let owner = parsed.all_functions()[0];
    assert!(search.scoped::<false>(&owner, "x", 1).is_empty());
    assert_eq!(
        search.scoped::<false>(&owner, "longname", 1),
        BTreeSet::from([1])
    );
}

#[test]
fn identifier_index_complete_dfg_and_partial_rebuild() {
    use crate::data_flow::DataFlowGraph;
    fn build(files: &BTreeMap<String, ParsedFile>, oracle: bool) -> Vec<u8> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .start_handler(move |_| super::super::USE_ORACLE.with(|f| f.set(oracle)))
            .build()
            .unwrap();
        pool.install(|| bincode::serialize(&DataFlowGraph::build(files)).unwrap())
    }
    let mut files = BTreeMap::from([
        (
            "edges.ts".into(),
            ParsedFile::parse("edges.ts", EDGES, Language::TypeScript).unwrap(),
        ),
        (
            "other.ts".into(),
            ParsedFile::parse(
                "other.ts",
                "const f = (x) => { const alias = x; x = alias; return x; };",
                Language::TypeScript,
            )
            .unwrap(),
        ),
    ]);
    assert_eq!(build(&files, false), build(&files, true));
    files.insert(
        "edges.ts".into(),
        ParsedFile::parse(
            "edges.ts",
            "function owner(y) {\n y = y;\n return y;\n}",
            Language::TypeScript,
        )
        .unwrap(),
    );
    let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["edges.ts".into()]));
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
