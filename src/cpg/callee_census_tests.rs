//! Operation-only replay of the Step-5b/5c admission prefixes, before graph lookup.
use super::*;
use crate::languages::Language;

use super::callee_cases as cases;

#[test]
fn language_reachability_census() {
    for (ext, language, source) in cases::languages() {
        let value = census(&cases::parsed(ext, language, source));
        eprintln!("{language:?}: {value}");
    }
}

pub(super) fn synthetic(m: usize) -> BTreeMap<String, ParsedFile> {
    let mut sources = BTreeMap::from([("helpers.ts".to_string(),
        "export function helper(x: number) { if (x) { return x; } return 0; }\nexport function empty(x: number) { const y = x; }\n".to_string())]);
    for file in 0..4 {
        let mut source = "import { helper, empty } from './helpers';\n".to_string();
        for i in 0..m {
            source.push_str(&format!("function caller{file}_{i}(x: number) {{ const a = helper(x); const b = helper(x); const c = empty(x); return a; }}\n"));
        }
        sources.insert(format!("caller{file}.ts"), source);
    }
    sources
        .into_iter()
        .map(|(path, source)| {
            let parsed = ParsedFile::parse(&path, &source, Language::TypeScript).unwrap();
            (path, parsed)
        })
        .collect()
}

// Exactly the recursive collect_return_infos visitation contract (including
// pruned nested-callable roots and return roots, excluding their children).
pub(super) fn visits(p: &ParsedFile, node: tree_sitter::Node<'_>, root: usize) -> usize {
    if matches!(node.kind(), "return_statement" | "return_expression")
        || (node.id() != root
            && p.language
                .callable_boundary_node_types()
                .contains(&node.kind()))
    {
        return 1;
    }
    let mut cursor = node.walk();
    1 + node
        .children(&mut cursor)
        .map(|n| visits(p, n, root))
        .sum::<usize>()
}

fn census(files: &BTreeMap<String, ParsedFile>) -> serde_json::Value {
    eprintln!("census: building call graph");
    let cg = CallGraph::build(files);
    eprintln!("census: replaying admission prefixes");
    let mut params: BTreeMap<FunctionId, usize> = BTreeMap::new();
    let mut returns: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
    let mut all5b = 0;
    let mut all5c = 0;
    for (caller, sites) in &cg.calls {
        let p = &files[&caller.file];
        let mut seen = BTreeSet::new();
        for site in sites {
            for r in cg.resolve_call_site(site) {
                if r.kind == ResolutionKind::R6MultiOwnerCandidate {
                    continue;
                }
                if p.call_argument_texts_and_spans_at(
                    site.start_byte,
                    site.effective_source_callee_name(),
                )
                .is_empty()
                {
                    continue;
                }
                let Some(cp) = files.get(&r.target.file) else {
                    continue;
                };
                if seen.insert(r.target.clone()) {
                    *params.entry(r.target.clone()).or_default() += 1;
                    if matches!(
                        cp.language,
                        Language::JavaScript | Language::TypeScript | Language::Tsx
                    ) || cp.functions().iter().any(|f| {
                        f.name.as_deref() == Some(r.target.name.as_str())
                            && f.start_line == r.target.start_line
                    }) {
                        all5b += 1;
                    }
                    if std::env::var_os("CALLEE_RETURN_ONLY").is_none() {
                        let _ = compute_param_def_nodes(
                            cp,
                            r.target,
                            &BTreeMap::new(),
                            &DiGraph::new(),
                        );
                    }
                }
            }
            let Some(a) = p.assignment_for_exact_call(site.start_byte, site.end_byte) else {
                continue;
            };
            let outcome = cg.resolve_call_site_full(site);
            let Some(r) = singleton_exact(&outcome) else {
                continue;
            };
            if a.lvalues.is_empty() || a.lvalues.iter().any(|lhs| !lhs.path.fields.is_empty()) {
                continue;
            }
            let Some(cp) = files.get(&r.target.file) else {
                continue;
            };
            all5c += 1;
            let Some(node) = cp.all_functions().into_iter().find(|n| {
                cp.node_line_range(n).0 == r.target.start_line
                    && cp
                        .language
                        .function_name(n)
                        .is_some_and(|name| cp.node_text(&name) == r.target.name)
            }) else {
                continue;
            };
            let _inventory = cp.return_value_nodes(&node);
            let entry = returns.entry(r.target.clone()).or_default();
            entry.0 += 1;
            entry.1 += visits(cp, node, node.id());
        }
    }
    serde_json::json!({
        "parameter_discoveries": params.values().sum::<usize>(), "parameter_distinct": params.len(),
        "all_functions_5b": all5b, "all_functions_5c": all5c,
        "return_builds": returns.values().map(|n| n.0).sum::<usize>(), "return_distinct": returns.len(),
        "return_nodes_visited": returns.values().map(|n| n.1).sum::<usize>(),
        "return_distinct_nodes": returns.values().map(|n| n.1 / n.0).sum::<usize>(),
        "parameters_per_callee": params.iter().map(|(id, n)| (format!("{}:{}:{}", id.file, id.start_line, id.name), n)).collect::<BTreeMap<_,_>>(),
        "returns_per_callee": returns.iter().map(|(id, n)| (format!("{}:{}:{}", id.file, id.start_line, id.name), n)).collect::<BTreeMap<_,_>>()
    })
}

#[test]
#[ignore = "operation census; checker path explicitly supplied by operator"]
fn callee_operation_census() {
    let scratch = std::env::var("CALLEE_SCRATCH").unwrap();
    for m in [1, 8, 32] {
        let value = census(&synthetic(m));
        std::fs::write(
            format!("{scratch}/census-xlike-{m}.json"),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
        eprintln!("xlike-{m}: {value}");
    }
    let path = format!("{scratch}/fixtures-c1/checker/checker.ts");
    let source = std::fs::read_to_string(&path).unwrap();
    let files = BTreeMap::from([(
        "checker.ts".into(),
        ParsedFile::parse("checker.ts", &source, Language::TypeScript).unwrap(),
    )]);
    let value = census(&files);
    std::fs::write(
        format!("{scratch}/census-checker.json"),
        serde_json::to_string_pretty(&value).unwrap(),
    )
    .unwrap();
    eprintln!(
        "checker totals: {}",
        serde_json::json!({"parameters": value["parameter_discoveries"], "param_distinct": value["parameter_distinct"], "return_builds": value["return_builds"], "return_distinct": value["return_distinct"], "return_visits": value["return_nodes_visited"], "distinct_visits": value["return_distinct_nodes"], "all5b": value["all_functions_5b"], "all5c": value["all_functions_5c"]})
    );
}

#[test]
#[ignore = "replay captured main demands; requires explicit scratch and count observer"]
fn shared_parameter_demand_replay() {
    assert!(std::env::var_os("CALLEE_COUNT_REPLAY").is_some());
    let scratch = std::env::var("CALLEE_SCRATCH").unwrap();
    for name in ["xlike-1", "xlike-8", "xlike-32", "checker"] {
        let files = if name == "checker" {
            let source =
                std::fs::read_to_string(format!("{scratch}/fixtures-c1/checker/checker.ts"))
                    .unwrap();
            BTreeMap::from([(
                "checker.ts".into(),
                ParsedFile::parse("checker.ts", &source, Language::TypeScript).unwrap(),
            )])
        } else {
            synthetic(name.strip_prefix("xlike-").unwrap().parse().unwrap())
        };
        let before: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{scratch}/census-{name}.json")).unwrap(),
        )
        .unwrap();
        let mut owners: BTreeMap<String, BTreeSet<FunctionId>> = BTreeMap::new();
        for (file, parsed) in &files {
            for node in crate::anon_owner::owner_nodes(parsed) {
                if let Some(id) = crate::anon_owner::emitted_identity(parsed, &node, file) {
                    owners
                        .entry(format!("{}:{}:{}", id.file, id.start_line, id.name))
                        .or_default()
                        .insert(id);
                }
            }
        }
        super::callee_cost_tests::take_counts();
        let cache = ParamCache::new(&files);
        let index = BTreeMap::new();
        let graph = DiGraph::new();
        // Repeat exactly the main caller/callee demand population. Empty graph
        // inputs suffice for discovery counts; full graph identity is tested
        // separately, including supported byte-exact Def targets.
        for (key, repetitions) in before["parameters_per_callee"].as_object().unwrap() {
            let ids = &owners[key];
            assert_eq!(
                ids.len(),
                1,
                "serialized census key must identify one full owner"
            );
            let id = ids.first().unwrap();
            for _ in 0..repetitions.as_u64().unwrap() {
                let _ = cache.get(id, &files[&id.file], &index, &graph);
            }
        }
        let counts = super::callee_cost_tests::take_counts();
        let total: usize = counts.values().sum();
        assert_eq!(total as u64, before["parameter_distinct"].as_u64().unwrap());
        assert!(counts.values().all(|n| *n == 1));
        let result = serde_json::json!({"before": before["parameter_discoveries"], "after": total,
            "per_callee": counts.iter().map(|((_, id),n)| (format!("{}:{}:{}",id.file,id.start_line,id.name),n)).collect::<BTreeMap<_,_>>()});
        std::fs::write(
            format!("{scratch}/after-{name}.json"),
            serde_json::to_string_pretty(&result).unwrap(),
        )
        .unwrap();
        eprintln!(
            "{name}: {} -> {total} parameter computations",
            before["parameter_discoveries"]
        );
    }
}
