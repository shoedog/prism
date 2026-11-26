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
fn rest_template_substitution_controls_argument_edge() {
    for (value, expected) in [("`${a}`", false), ("`a`", true)] {
        let source = format!("function take(...r) {{ use(r); }}\nfunction main(a) {{ const xs = {value}; return take(...xs); }}");
        let files = parsed_files(&[("m.js", &source)], Language::JavaScript);
        for cpg in build_both(&files) {
            let site = call_site(&cpg.call_graph, "main", "take");
            assert!(object_literal_args::exact_singleton(
                &cpg.call_graph.resolve_call_site(site)
            ));
            let want = if expected {
                vec![("xs".into(), FlowConfidence::Exact)]
            } else {
                vec![]
            };
            assert_eq!(bindings(&cpg, "main", "r"), want, "{source}");
        }
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

fn rest_edges_into(cpg: &CodePropertyGraph, edges: &[PendingEdge], parameter: &str) -> Vec<String> {
    let mut found: Vec<String> = edges
        .iter()
        .filter_map(|&(from, to, _)| {
            let to = cpg.to_var_location(to)?;
            (to.kind == VarAccessKind::Def && to.path.base == parameter)
                .then(|| cpg.to_var_location(from).unwrap().path.to_string())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn rest_def_node_requires_one_verifiable_js_implementation() {
    let js = parsed_files(
        &[("m.js", "function pack(...items) {\n  use(items);\n}\n")],
        Language::JavaScript,
    );
    let cpg = CodePropertyGraph::build(&js);
    let pack = cpg.call_graph.functions["pack"][0].clone();
    let compute = |files: &BTreeMap<String, ParsedFile>,
                   callee: &FunctionId,
                   index: &VarIndex,
                   graph: &DiGraph<CpgNode, CpgEdge>| {
        rest_params::compute_rest_def_node(&files["m.js"], callee, index, graph)
    };
    // Control: one JS implementation, a recorded Def at the identifier's bytes.
    let (offset, def) = compute(&js, &pack, &cpg.var_index, &cpg.graph).unwrap();
    assert_eq!(offset, 0);

    // A callee that names no function of the file.
    let renamed = FunctionId {
        name: "elsewhere".into(),
        ..pack.clone()
    };
    let overlay = VarIndex::from_map(BTreeMap::from([(
        (
            pack.file.clone(),
            renamed.name.clone(),
            pack.start_line,
            pack.start_line,
            AccessPath::simple("items"),
            VarAccess::Def,
        ),
        def,
    )]));
    assert_eq!(compute(&js, &renamed, &overlay, &cpg.graph), None);

    // No recorded Def for the formal.
    assert_eq!(
        compute(&js, &pack, &VarIndex::new(), &cpg.graph),
        None,
        "an index without the entry Def finds nothing"
    );

    // A Def whose bytes are not the identifier's is not the entry Def.
    let mut moved = cpg.graph.clone();
    let CpgNode::Variable { start_byte, .. } = &mut moved[def] else {
        panic!("the entry Def is a variable");
    };
    *start_byte += 1;
    assert_eq!(compute(&js, &pack, &cpg.var_index, &moved), None);

    // Two functions of one name on the same lines are not one implementation.
    let twin_files = parsed_files(
        &[(
            "m.js",
            "function pack(...a) { use(a); } function pack(...b) { use(b); }\n",
        )],
        Language::JavaScript,
    );
    let twin = CodePropertyGraph::build(&twin_files);
    let functions = &twin.call_graph.functions["pack"];
    assert_eq!(functions.len(), 2);
    assert_eq!(
        compute(&twin_files, &functions[0], &twin.var_index, &twin.graph),
        None
    );
    let split_files = parsed_files(
        &[(
            "m.js",
            "function pack(...a) { use(a); }\nfunction pack(...b) { use(b); }\n",
        )],
        Language::JavaScript,
    );
    let split = CodePropertyGraph::build(&split_files);
    assert!(compute(
        &split_files,
        &split.call_graph.functions["pack"][0],
        &split.var_index,
        &split.graph
    )
    .is_some());

    // Only the JS family has runtime rest slots.
    let mut non_js = js.clone();
    non_js.get_mut("m.js").unwrap().language = Language::Python;
    assert_eq!(compute(&non_js, &pack, &cpg.var_index, &cpg.graph), None);
}

#[test]
fn rest_namespace_export_match_requires_a_namespace_import() {
    let files = parsed_files(
        &[
            (
                "main.js",
                "import * as api from './lib';\nfunction main(v) { api.project(v); }",
            ),
            (
                "lib.js",
                "export const project = (...items) => { use(items); };",
            ),
        ],
        Language::JavaScript,
    );
    let cpg = CodePropertyGraph::build(&files);
    let site = call_site(&cpg.call_graph, "main", "project");
    let pool = cpg.call_graph.resolve_call_site(site);
    assert!(object_literal_args::exact_singleton(&pool));
    assert_eq!(pool[0].kind, ResolutionKind::ImportQualified);
    let mut non_namespace = site.clone();
    non_namespace.local_binding = crate::call_graph::JsLocalBinding::AliasNamespaceMember {
        module_path: "./lib".into(),
        member: "project".into(),
    };
    for (site, expected) in [(site, true), (&non_namespace, false)] {
        assert_eq!(
            cpg.call_graph
                .js_ts_namespace_export_matches(site, pool[0].target),
            expected
        );
        let edges = run_site(&cpg, &files, site, pool.clone(), spread_args::for_call);
        let names: Vec<_> = edges
            .iter()
            .map(|e| cpg.to_var_location(e.edge.0).unwrap().path.to_string())
            .collect();
        assert_eq!(names, if expected { vec!["v"] } else { vec![] });
    }
}

#[test]
fn rest_admission_refuses_name_only_in_the_same_file() {
    // A NameOnly candidate in the call's own file is no evidence of the implementation.
    let decl = "class Box { pack(prefix, ...items) { use(prefix, items); } }";
    let caller = "function main(receiver, prefix, scalar) {\nreceiver.pack(prefix, scalar);\n}";
    let local = format!("{decl}\n{caller}");
    for (inputs, same_file, expected) in [
        (
            vec![("main.js", local.as_str())],
            true,
            Vec::<(String, FlowConfidence)>::new(),
        ),
        (
            vec![("main.js", caller), ("box.js", decl)],
            false,
            vec![("scalar".to_string(), ResolutionConfidence::NameOnly.into())],
        ),
    ] {
        let files = parsed_files(&inputs, Language::JavaScript);
        // `build` runs on the build pool, where the legacy switch is not set, so the
        // frozen twin is assembled directly.
        let legacy = with_legacy(|| {
            CodePropertyGraph::assemble_graph(
                CallGraph::build(&files),
                DataFlowGraph::build(&files),
                &files,
                None,
            )
        });
        for cpg in [CodePropertyGraph::build(&files), legacy] {
            let site = call_site(&cpg.call_graph, "main", "pack");
            let pool = cpg.call_graph.resolve_call_site(site);
            assert_eq!(pool.len(), 1, "{pool:?}");
            assert_eq!(pool[0].confidence, ResolutionConfidence::NameOnly);
            assert_eq!(pool[0].target.file == "main.js", same_file);
            assert_eq!(bindings(&cpg, "main", "items"), expected, "{inputs:?}");
        }
        // The serial twin applies the same admission and file lookup.
        let cpg = CodePropertyGraph::build(&files);
        let edges = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let want: Vec<String> = expected.iter().map(|(from, _)| from.clone()).collect();
        assert_eq!(rest_edges_into(&cpg, &edges, "items"), want, "{inputs:?}");
    }
}

#[test]
fn rest_serial_twin_skips_a_callee_outside_its_file_map() {
    let inputs = [
        (
            "main.js",
            "import { project } from './lib';\nfunction main(v) { project(v); }",
        ),
        (
            "lib.js",
            "export function project(...items) { use(items); }",
        ),
    ];
    let files = parsed_files(&inputs, Language::JavaScript);
    let cpg = CodePropertyGraph::build(&files);
    let site = call_site(&cpg.call_graph, "main", "project");
    let pool = cpg.call_graph.resolve_call_site(site);
    assert!(object_literal_args::exact_singleton(&pool), "{pool:?}");
    assert_eq!(pool[0].target.file, "lib.js");
    let twin = |files: &BTreeMap<String, ParsedFile>| {
        let edges = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            files,
        );
        rest_edges_into(&cpg, &edges, "items")
    };
    assert_eq!(twin(&files), ["v"]);
    let caller_only = parsed_files(&inputs[..1], Language::JavaScript);
    assert!(twin(&caller_only).is_empty());
    assert!(twin(&BTreeMap::new()).is_empty());
}

#[test]
fn rest_write_through_typescript_assertion_is_a_write() {
    for (target, expected) in [
        ("(r as unknown[]) = [];", false),
        ("(r satisfies unknown[]) = [];", false),
        ("r! = [];", false),
        ("(<unknown[]>r) = [];", false),
        ("use(r as unknown[]);", true),
    ] {
        let source = format!(
            "function take(...r: unknown[]) {{\n {target}\n return r;\n}}\n\
             function main(v: unknown) {{ return take(v); }}"
        );
        assert_review_rest(&source, "m.ts", Language::TypeScript, "v", expected);
    }
}

#[test]
fn rest_cross_file_free_single_exact_is_not_admitted() {
    // A bare call that only a sibling file's global definition can satisfy is
    // static-binding Exact but not import-proven, so it binds no rest element.
    for (caller, lib, kind, expected) in [
        (
            "function main(v) { project(v); }",
            "function project(...items) { use(items); }",
            ResolutionKind::FreeSingle,
            vec![],
        ),
        (
            "import { project } from './lib';\nfunction main(v) { project(v); }",
            "export function project(...items) { use(items); }",
            ResolutionKind::ImportMember,
            vec![("v".to_string(), FlowConfidence::Exact)],
        ),
    ] {
        let files = parsed_files(
            &[("main.js", caller), ("lib.js", lib)],
            Language::JavaScript,
        );
        let legacy = with_legacy(|| {
            CodePropertyGraph::assemble_graph(
                CallGraph::build(&files),
                DataFlowGraph::build(&files),
                &files,
                None,
            )
        });
        for cpg in [CodePropertyGraph::build(&files), legacy] {
            let site = call_site(&cpg.call_graph, "main", "project");
            let pool = cpg.call_graph.resolve_call_site(site);
            assert_eq!(pool.len(), 1, "{pool:?}");
            assert_eq!(pool[0].confidence, ResolutionConfidence::Exact);
            assert_eq!(pool[0].kind, kind);
            assert_eq!(bindings(&cpg, "main", "items"), expected, "{caller}");
        }
        let cpg = CodePropertyGraph::build(&files);
        let edges = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let want: Vec<String> = expected.iter().map(|(from, _)| from.clone()).collect();
        assert_eq!(rest_edges_into(&cpg, &edges, "items"), want, "{caller}");
    }
}
