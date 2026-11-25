use super::callee_legacy::with_legacy;
use super::*;
use crate::languages::Language;
use crate::resolution::go::satisfiers::CpgCallResolver;

fn parsed_files(files: &[(&str, &str)], language: Language) -> BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|&(path, source)| {
            let parsed = ParsedFile::parse(path, source, language).unwrap();
            assert_eq!(parsed.parse_error_count, 0, "{path}: {source}");
            (path.to_owned(), parsed)
        })
        .collect()
}

struct FixedPool<'a> {
    cg: &'a CallGraph,
    pool: Vec<ResolvedCallee<'a>>,
}

impl CpgCallResolver for FixedPool<'_> {
    fn call_graph(&self) -> &CallGraph {
        self.cg
    }

    fn resolve_call_site(&self, _: &CallSite) -> Vec<ResolvedCallee<'_>> {
        self.pool.clone()
    }
}

fn call_site<'a>(cg: &'a CallGraph, caller: &str, name: &str) -> &'a CallSite {
    let mut sites = cg
        .calls
        .values()
        .flatten()
        .filter(|site| site.caller.name == caller && site.callee_name == name);
    let site = sites
        .next()
        .unwrap_or_else(|| panic!("{caller}: missing {name} site"));
    assert!(sites.next().is_none(), "{caller}: duplicate {name} site");
    site
}

fn run_site(
    cpg: &CodePropertyGraph,
    files: &BTreeMap<String, ParsedFile>,
    site: &CallSite,
    pool: Vec<ResolvedCallee<'_>>,
    read_arguments: impl Fn(
        &ParsedFile,
        &CallSite,
    ) -> (Vec<spread_args::Argument>, spread_args::Offsets),
) -> Vec<PendingArgumentFlow> {
    CodePropertyGraph::step5b_edges_for_caller_with_exact(
        &site.caller,
        &BTreeSet::from([site.clone()]),
        &FixedPool {
            cg: &cpg.call_graph,
            pool,
        },
        &cpg.var_index,
        &CodePropertyGraph::exact_var_index_from_graph(&cpg.graph),
        &cpg.graph,
        &ParamCache::new(files),
        read_arguments,
    )
}

fn bindings(
    cpg: &CodePropertyGraph,
    caller: &str,
    parameter: &str,
) -> Vec<(String, FlowConfidence)> {
    let mut found = Vec::new();
    for edge in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(label) = cpg.graph[edge] else {
            continue;
        };
        let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
        let from = cpg.to_var_location(from);
        let to = cpg.to_var_location(to);
        if let (Some(from), Some(to)) = (from, to) {
            if from.function == caller
                && from.function != to.function
                && from.kind == VarAccessKind::Use
                && to.kind == VarAccessKind::Def
                && to.path.base == parameter
            {
                found.push((from.path.to_string(), label));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn build_both(files: &BTreeMap<String, ParsedFile>) -> [CodePropertyGraph; 2] {
    [
        CodePropertyGraph::build(files),
        with_legacy(|| CodePropertyGraph::build(files)),
    ]
}

#[test]
fn rest_exact_requires_the_complete_singleton_pool() {
    for (caller, definition, kind) in [
        (
            "function main(v) { project(v); }",
            "function project(...items) { use(items); }",
            ResolutionKind::LocalDef,
        ),
        (
            "import * as api from './lib';\nfunction main(v) { api.project(v); }",
            "export const project = (...items) => { use(items); };",
            ResolutionKind::ImportQualified,
        ),
    ] {
        let local = format!("{definition}\n{caller}\nfunction other(...decoy) {{ use(decoy); }}");
        let foreign = format!("{caller}\nfunction other(...decoy) {{ use(decoy); }}");
        let mut inputs = if kind == ResolutionKind::LocalDef {
            vec![("main.js", local.as_str())]
        } else {
            vec![("main.js", foreign.as_str()), ("lib.js", definition)]
        };
        let target_file = if kind == ResolutionKind::LocalDef {
            "main.js"
        } else {
            "lib.js"
        };
        inputs.push((
            "foreign.js",
            "function foreign(...foreign_rest) { use(foreign_rest); }",
        ));
        let files = parsed_files(&inputs, Language::JavaScript);
        let cpg = CodePropertyGraph::build(&files);
        let site = call_site(&cpg.call_graph, "main", "project");
        let actual = cpg.call_graph.resolve_call_site(site);
        assert!(object_literal_args::exact_singleton(&actual), "{actual:?}");
        let one = actual[0].clone();
        assert_eq!(one.kind, kind);
        assert_eq!(one.target.file, target_file);
        assert_eq!(
            bindings(&cpg, "main", "items"),
            [("v".into(), FlowConfidence::Exact)]
        );
        let other = &cpg.call_graph.functions["other"][0];
        let another = ResolvedCallee {
            target: other,
            ..one.clone()
        };
        let local_name_only = ResolvedCallee {
            confidence: ResolutionConfidence::NameOnly,
            ..another.clone()
        };
        let foreign_name_only = ResolvedCallee {
            target: &cpg.call_graph.functions["foreign"][0],
            ..local_name_only.clone()
        };

        for pool in [vec![one.clone(), another], vec![one.clone(), one.clone()]] {
            assert!(run_site(&cpg, &files, site, pool, spread_args::for_call).is_empty());
        }
        let edges = run_site(
            &cpg,
            &files,
            site,
            vec![one, local_name_only, foreign_name_only],
            spread_args::for_call,
        );
        assert_eq!(edges.len(), 1);
        let (_, to, ref edge) = edges[0].edge;
        assert_eq!(cpg.to_var_location(to).unwrap().path.base, "foreign_rest");
        assert_eq!(
            edge,
            &CpgEdge::DataFlow(ResolutionConfidence::NameOnly.into())
        );
    }
}

#[test]
fn rest_namespace_spelling_fallback_does_not_witness_the_export() {
    let caller = "import * as api from './lib';\nfunction main(v) { api.readFile(v); }";
    for (definition, want) in [
        ("function readFile(...items) { use(items); }\nfunction live() {}\nexport { live as readFile };", false),
        ("function readFile(...items) { use(items); }\nexport {};", false),
        ("function readFile(...items) { use(items); }\nexport { readFile };", true),
    ] {
        let files = parsed_files(&[("main.js", caller), ("lib.js", definition)], Language::JavaScript);
        for cpg in build_both(&files) {
            let site = call_site(&cpg.call_graph, "main", "readFile");
            let actual = cpg.call_graph.resolve_call_site(site);
            assert!(object_literal_args::exact_singleton(&actual), "{actual:?}");
            assert_eq!(actual[0].kind, ResolutionKind::ImportQualified);
            assert_eq!(actual[0].target.file, "lib.js");
            assert_eq!(actual[0].target.start_line, 1);
            let got = bindings(&cpg, "main", "items");
            assert_eq!(got, if want { vec![("v".into(), FlowConfidence::Exact)] } else { vec![] }, "{definition}");
        }
    }
}

#[test]
fn rest_name_only_requires_a_foreign_candidate() {
    let owner = "export class Box { pack(first, ...items) { use(first, items); } }";
    let caller = "function main(receiver, prefix, value, dense) { receiver.pack(prefix, value, ...[dense]); }";
    let same_file = format!("{owner}\n{caller}");
    let label = FlowConfidence::from(ResolutionConfidence::NameOnly);
    for (inputs, target_file, expected) in [
        (vec![("main.js", same_file.as_str())], "main.js", vec![]),
        (
            vec![("main.js", caller), ("box.js", owner)],
            "box.js",
            vec![("dense".into(), label), ("value".into(), label)],
        ),
    ] {
        let files = parsed_files(&inputs, Language::JavaScript);
        for cpg in build_both(&files) {
            let site = call_site(&cpg.call_graph, "main", "pack");
            let resolved = cpg.call_graph.resolve_call_site(site);
            assert_eq!(resolved.len(), 1, "{resolved:?}");
            assert_eq!(resolved[0].target.file, target_file);
            assert_eq!(resolved[0].kind, ResolutionKind::R6SingleOwner);
            assert_eq!(resolved[0].confidence, ResolutionConfidence::NameOnly);
            assert_eq!(bindings(&cpg, "main", "first"), [("prefix".into(), label)]);
            assert_eq!(bindings(&cpg, "main", "items"), expected, "{target_file}");
        }
    }
}

#[test]
fn rest_opaque_iteration_requires_occurrence_specific_evidence() {
    let producers = [
        ("const xs = [];", false, false),
        ("const xs = [1];", true, true),
        ("const xs = '';", false, false),
        ("const xs = 'x';", true, true),
        ("const xs = ``;", false, false),
        ("const xs = `x`;", true, true),
        ("const empty = []; const xs = empty;", false, false),
        ("const one = [1]; const xs = one;", true, true),
        (
            "const xs = [1]; const alias = xs; alias.length = 0;",
            false,
            true,
        ),
        (
            "const xs = [1]; const alias = xs; alias[Symbol.iterator] = function*() {};",
            false,
            true,
        ),
        ("const xs = [1]; escape(xs);", false, true),
        (
            "const xs = [1]; Array.prototype[Symbol.iterator] = function*() {};",
            false,
            true,
        ),
        ("const xs = producer;", false, true),
        (
            "function* generate() {} const xs = generate();",
            false,
            true,
        ),
        (
            "function* generate() { yield 1; } const xs = generate();",
            false,
            true,
        ),
        ("const xs = new Set();", false, true),
        ("const xs = new Set([1]);", false, true),
        ("const xs = [][Symbol.iterator]();", false, true),
        ("const xs = [1][Symbol.iterator](); xs.next();", false, true),
        ("const xs = [1][Symbol.iterator]();", false, true),
        ("const xs = [1].slice(1);", false, true),
        ("const xs = [1].slice(0);", false, true),
    ];
    for (setup, exact, name_only) in producers {
        for (decl, callee, confidence, witness) in [
            (
                "function pack(prefix, ...items) { use(prefix, items); }",
                "pack",
                ResolutionConfidence::Exact,
                exact,
            ),
            (
                "class Box { pack(prefix, ...items) { use(prefix, items); } }",
                "receiver.pack",
                ResolutionConfidence::NameOnly,
                name_only,
            ),
        ] {
            let caller = format!(
                "function main(receiver, producer, prefix, scalar) {{\n{setup}\n{callee}(prefix, ...xs, scalar);\n}}"
            );
            let local = format!("{decl}\n{caller}");
            let inputs = if confidence == ResolutionConfidence::Exact {
                vec![("main.js", local.as_str())]
            } else {
                vec![("main.js", caller.as_str()), ("box.js", decl)]
            };
            let files = parsed_files(&inputs, Language::JavaScript);
            for cpg in build_both(&files) {
                let site = call_site(&cpg.call_graph, "main", "pack");
                let actual = cpg.call_graph.resolve_call_site(site);
                assert_eq!(actual.len(), 1, "{actual:?}");
                assert_eq!(actual[0].confidence, confidence);
                let label = confidence.into();
                let mut expected = vec![("scalar".into(), label)];
                if witness {
                    expected.push(("xs".into(), label));
                }
                assert_eq!(bindings(&cpg, "main", "items"), expected, "{caller}");
            }
        }
    }
}

#[test]
fn rest_typescript_empty_and_nonempty_wrapped_operands() {
    for (setup, want) in [
        ("const xs = [] as const;", false),
        ("const xs = [1] as const;", true),
        ("let xs: [] = [];", false),
        ("let xs: [number] = [1];", true),
        ("let xs: readonly [] = [];", false),
        ("let xs: readonly [number] = [1];", true),
        ("let xs = '' as string;", false),
        ("let xs = `x` as string;", true),
        ("const xs = producer as [number];", false),
    ] {
        let source = format!(
            "function pack(...items: unknown[]) {{ use(items); }}\nfunction main(producer: unknown) {{\n{setup}\npack(...((xs as any)!));\n}}"
        );
        for (path, language) in [
            ("main.ts", Language::TypeScript),
            ("main.tsx", Language::Tsx),
        ] {
            let files = parsed_files(&[(path, &source)], language);
            for cpg in build_both(&files) {
                let site = call_site(&cpg.call_graph, "main", "pack");
                let pool = cpg.call_graph.resolve_call_site(site);
                assert!(object_literal_args::exact_singleton(&pool), "{pool:?}");
                let got = bindings(&cpg, "main", "items");
                assert_eq!(
                    got,
                    if want {
                        vec![("xs".into(), FlowConfidence::Exact)]
                    } else {
                        vec![]
                    },
                    "{source}"
                );
            }
        }
    }
}

#[path = "rest_constructor_tests.rs"]
mod constructors;

#[test]
fn rest_lower_bound_counts_only_guaranteed_arguments() {
    let files = parsed_files(
        &[(
            "main.js",
            "
function pack(a, b, ...tail) {
  use(a, b, tail);
}
function main(xs, early, late) {
  pack(...xs, 0, early);
  pack(...xs, 0, 0, late);
}",
        )],
        Language::JavaScript,
    );
    for cpg in build_both(&files) {
        assert_eq!(
            bindings(&cpg, "main", "tail"),
            [("late".into(), FlowConfidence::Exact)]
        );
        assert!(bindings(&cpg, "main", "a").is_empty());
        assert!(bindings(&cpg, "main", "b").is_empty());
    }
}

#[test]
fn rest_review_parameter_decorator() {
    for (member, decorator, expected) in [
        ("m", "@drain ", false),
        ("constructor", "@drain ", false),
        ("m", "", true),
    ] {
        let source = format!(
            "function take(...r: unknown[]) {{ return r; }}\n\
             function main() {{ const xs = [1];\n\
               function drain(...args: unknown[]) {{ xs.length = 0; }}\n\
               class C {{ {member}({decorator}arg: unknown) {{}} }}\n\
               return take(...xs);\n\
             }}"
        );
        assert_review_rest(&source, "m.ts", Language::TypeScript, "xs", expected);
    }
}

#[test]
fn rest_review_parameter_default() {
    for (initializer, expected) in [
        ("(String.prototype[Symbol.iterator] = function*(){})", false),
        ("0", true),
    ] {
        let source = format!(
            "function take(...r) {{ return r; }}\n\
             function main(arg = {initializer}) {{ const xs = 'x'; return take(...xs); }}"
        );
        assert_review_rest(&source, "m.js", Language::JavaScript, "xs", expected);
    }
}

#[test]
fn rest_review_parameter_default_in_enclosing_callable() {
    for (initializer, expected) in [("replaceIterator()", false), ("0", true)] {
        let source = format!(
            "function take(...r: unknown[]) {{ return r; }}\n\
             function outer(arg: number = {initializer}) {{\n\
               function main() {{ const xs = 'x'; return take(...xs); }}\n\
               return main;\n\
             }}"
        );
        assert_review_rest(&source, "m.ts", Language::TypeScript, "xs", expected);
    }
}

#[test]
fn rest_review_parameter_pattern_computed_key() {
    for (key, expected) in [
        (
            "(Array.prototype[Symbol.iterator] = function* () {}, \"k\")",
            false,
        ),
        ("\"k\"", true),
    ] {
        let source = format!(
            "function take(...r: unknown[]) {{ return r; }}\n\
             export function main({{ [{key}]: a }}: any) {{ const xs = [1]; return take(...xs); }}"
        );
        assert_review_rest(&source, "m.ts", Language::TypeScript, "xs", expected);
    }
}

#[test]
fn rest_review_enclosing_callable_body() {
    for (outer_body, expected) in [
        (
            "Array.prototype[Symbol.iterator] = function* () {};\n\
             function main() { const xs = [1]; return take(...xs); }\n\
             return main();",
            false,
        ),
        (
            "function main() { const xs = [1]; return take(...xs); }\n\
             Array.prototype[Symbol.iterator] = function* () {};\n\
             return main();",
            false,
        ),
        (
            "function main() { const xs = [1]; return take(...xs); }\n\
             const unused = 0;\n\
             return main;",
            true,
        ),
    ] {
        let source = format!(
            "function take(...r) {{ return r; }}\n\
             export function outer() {{\n{outer_body}\n}}"
        );
        assert_review_rest(&source, "m.js", Language::JavaScript, "xs", expected);
    }
}

#[test]
fn rest_review_escaped_captured_write() {
    assert_review_rest_body(r"function reset(){\u0072=[]} reset();", false);
    assert_review_rest_body(r"\u0072=[];", false);
    assert_review_rest_body("function reset(){r=[]} reset();", false);
    assert_review_receiver_rest(r"function reset(){\u0072=[]} reset();", false);
    assert_review_receiver_rest(r"\u0072=[];", false);
}

#[test]
fn rest_review_direct_eval() {
    assert_review_rest_body("eval('r=[]');", false);
}

#[test]
fn rest_review_nested_eval() {
    assert_review_rest_body("function reset(){eval('r=[]')} reset();", false);
}

#[test]
fn rest_review_nested_shadow() {
    assert_review_rest_body("function reset(r){r=[]} reset();", true);
}

#[test]
fn rest_review_escaped_nested_shadow() {
    assert_review_receiver_rest(r"function reset(\u0072){\u0072=[]} reset();", true);
    assert_review_receiver_rest(r"function reset(\u0072){r=[]; use(r)} reset();", true);
    assert_review_receiver_rest(r"function reset(r){\u0072=[]} reset();", true);
}

fn assert_review_receiver_rest(body: &str, expected: bool) {
    // A same-file bare call can already be refused by the escaped-identifier
    // resolution guard. A call on `this` independently identifies the method.
    let source = format!(
        "class C {{\n take(...r) {{\n {body}\n return r;\n }}\n main(v) {{ return this.take(v); }}\n}}"
    );
    let files = parsed_files(&[("m.js", &source)], Language::JavaScript);
    let expected = if expected {
        vec![("v".into(), FlowConfidence::Exact)]
    } else {
        vec![]
    };
    for cpg in build_both(&files) {
        let site = call_site(&cpg.call_graph, "main", "take");
        let pool = cpg.call_graph.resolve_call_site(site);
        assert!(
            object_literal_args::exact_singleton(&pool),
            "{body}: {pool:?}"
        );
        assert_eq!(pool[0].kind, ResolutionKind::SelfReceiver);
        assert_eq!(pool[0].target.name, "take");
        assert_eq!(bindings(&cpg, "main", "r"), expected, "{body}");
    }
}

fn assert_review_rest_body(body: &str, expected: bool) {
    let source = format!(
        "function take(...r){{\n {body}\n return r;\n}}\nfunction main(v){{ return take(v); }}"
    );
    assert_review_rest(&source, "m.js", Language::JavaScript, "v", expected);
}

fn assert_review_rest(
    source: &str,
    path: &str,
    language: Language,
    argument: &str,
    expected: bool,
) {
    let files = parsed_files(&[(path, source)], language);
    let expected = if expected {
        vec![(argument.to_string(), FlowConfidence::Exact)]
    } else {
        vec![]
    };
    for cpg in build_both(&files) {
        assert_eq!(bindings(&cpg, "main", "r"), expected, "{source}");
    }
}
