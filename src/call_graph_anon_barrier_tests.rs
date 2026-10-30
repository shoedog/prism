//! B0/B1 fail-first natural-source regressions, with emission-independent injection.
use super::*;
use crate::resolution::{ResolutionConfidence::Exact, ResolutionKind, ResolvedCallee};

fn metas<'a>(p: &'a ParsedFile, n: &tree_sitter::Node<'a>) -> Vec<crate::ast::CallSiteMeta<'a>> {
    let range = p.node_line_range(n);
    p.function_calls_with_qualifier_and_spans_on_lines(
        n,
        &(range.0..=range.1).collect(),
        &BTreeSet::new(),
    )
    .0
}
fn fixture(source: &str, lib: &str) -> (CallGraph, CallSite) {
    fixture_deps(source, lib, &[])
}
fn fixture_deps(source: &str, lib: &str, deps: &[(&str, &str)]) -> (CallGraph, CallSite) {
    let mut fs = BTreeMap::from([
        (
            "a.ts".into(),
            ParsedFile::parse("a.ts", source, Language::TypeScript).unwrap(),
        ),
        (
            "lib.ts".into(),
            ParsedFile::parse("lib.ts", lib, Language::TypeScript).unwrap(),
        ),
    ]);
    for (path, source) in deps {
        fs.insert(
            (*path).into(),
            ParsedFile::parse(path, source, Language::TypeScript).unwrap(),
        );
    }
    let p = &fs["a.ts"];
    let marker = source.find("/*T*/").unwrap() + 5;
    let node = candidate_nodes(p, true)
        .into_iter()
        .find(|n| {
            p.language.function_name(n).is_none()
                && n.start_byte() <= marker
                && marker < n.end_byte()
                && metas(p, n).iter().any(|m| m.start_byte == marker)
        })
        .unwrap_or_else(|| panic!("missing anonymous site: {source}"));
    let meta = metas(p, &node)
        .into_iter()
        .find(|m| m.start_byte == marker)
        .unwrap();
    let mut graph = CallGraph::build(&fs);
    let mut site = inject(&mut graph, p, &node, &meta.callee_name);
    site.qualifier = meta.qualifier.clone();
    let imports = p.extract_imports();
    let class = LegacyClassifier.classify(ReceiverCtx {
        receiver_expr: meta.receiver_node,
        qualifier: meta.qualifier.as_deref(),
        fn_node: node,
        call_line: meta.line,
        call_start_byte: meta.start_byte,
        parsed: p,
        recv_var: None,
        file_imports: Some(&imports),
        proven_imported_receiver_types: None,
    });
    site.receiver_materialized = class.materialized;
    site.receiver_lexically_bound =
        p.js_ts_receiver_lexically_bound_at_call(&node, meta.receiver_node);
    site.receiver_type = class.recovered.as_ref().map(|r| r.static_type.clone());
    site.receiver_recovery = class.recovered.as_ref().map(|r| r.recovery);
    (graph, site)
}
fn check(source: &str, lib: &str, want: bool) {
    let (graph, site) = fixture(source, lib);
    let out = graph.resolve_call_site_full(&site);
    assert_eq!(
        out.resolved.iter().any(|r| r.confidence == Exact),
        want,
        "{source}: {out:?}"
    );
}
const LIB: &str = "export function f(){} export function hook(){return {f};}";

#[test]
fn b0_each_file_trigger_and_clean_control() {
    for suffix in [
        "const \\u0078=1;",
        "with(obj){}",
        "eval('x')",
        "(0,eval)('x')",
        "globalThis[\"eval\"]('x')",
        "namespace N { import x = M.q; }",
        "export import x = M.q;",
        "function broken( {",
    ] {
        check(
            &format!("import {{f}} from './lib'; sink(()=>{{/*T*/f();}}); {suffix}"),
            LIB,
            false,
        );
    }
    for suffix in [
        "const \\u0078=1;",
        "with(obj){}",
        "eval('x')",
        "namespace N {import x=M.q;}",
        "function broken( {",
    ] {
        check(
            &format!("sink(()=>{{function f(){{}} /*T*/f();}}); {suffix}"),
            LIB,
            false,
        );
    }
    check(
        "import {f} from './lib'; sink(()=>{/*T*/f();}); const x=1;",
        LIB,
        true,
    );
    // Identifier-like text in literals/comments does not trigger the token fence.
    check(
        "import {f} from './lib'; sink(()=>{/*T*/f();}); const x='eval with \\u0078'; // eval\n",
        LIB,
        true,
    );
}

#[test]
fn b0_evaluation_environments_and_table_controls() {
    for source in [
        "import {f} from './lib'; namespace N {sink(()=>{/*T*/f();});}",
        "import {f} from './lib'; module N {sink(()=>{/*T*/f();});}",
        "import {f} from './lib'; enum E {x=sink(()=>{/*T*/f();})}",
        "import {f} from './lib'; class C {static {sink(()=>{/*T*/f();});}}",
    ] {
        check(source, LIB, false);
    }
    for source in [
        "import {f} from './lib'; function outer(x=sink(()=>{/*T*/f();})){}",
        "import {f} from './lib'; @dec(()=>{/*T*/f();}) class C{}",
        "import {f} from './lib'; class C {[sink(()=>{/*T*/f();})](){}}",
        "import {f} from './lib'; class C {field=sink(()=>{/*T*/f();});}",
    ] {
        check(source, LIB, true);
    }
}

#[test]
fn b1_local_alias_returned_and_import_certificates() {
    for source in [
        "sink(()=>{function f(){} /*T*/f();});",
        "import {f} from './lib'; sink(()=>{const a=f; /*T*/a();});",
        "sink(()=>{function f(){} const a=f; /*T*/a();});",
        "import {hook} from './lib'; sink(()=>{const {f}=hook(); /*T*/f();});",
        "import {f} from './lib'; sink(()=>{/*T*/f();});",
        "import * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
        "import * as ns from './lib'; sink(()=>{const a=ns.f; /*T*/a();});",
    ] {
        check(source, LIB, true);
    }
    for source in [
        "sink((f)=>{/*T*/f();});",
        "import {f} from './lib'; sink((f)=>{const a=f; /*T*/a();});",
        "import {hook} from './lib'; sink((hook)=>{const {f}=hook(); /*T*/f();});",
        "import type {f} from './lib'; sink(()=>{/*T*/f();});",
        "import type * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
        "import f=require('./lib'); sink(()=>{/*T*/f();});",
        "import {f} from './lib'; namespace N {import f=M.q; sink(()=>{/*T*/f();});}",
        "import {f} from './lib'; namespace N {export import f=M.q; sink(()=>{/*T*/f();});}",
    ] {
        check(source, LIB, false);
    }
}

#[test]
fn namespace_export_proof_never_admits_stem_fallback() {
    for lib in [
        "function f(){} export {};",
        "function f(){} function g(){} export {g as f};",
    ] {
        check(
            "import * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
            lib,
            false,
        );
    }
    check(
        "import * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
        LIB,
        true,
    );
}

#[test]
fn dependency_environments_are_checked_in_initializers_and_hooks() {
    check(
        "import {hook} from './lib'; sink(()=>{const {f}=hook(); /*T*/f();});",
        "function f(){} export function hook(){eval('f=other');return {f};}",
        false,
    );
    check(
        "import {hook} from './lib'; sink(()=>{const {f}=hook(); /*T*/f();});",
        "function f(){} namespace N {export function hook(){return {f};}} export {hook};",
        false,
    );
    check(
        "import {f} from './lib'; namespace N {export const a=f;} sink(()=>{/*T*/a();});",
        LIB,
        false,
    );
}

#[test]
fn census_w1_through_w6_and_disclosed_receiver_recall() {
    for source in [
        "import * as ns from './lib'; sink((\\u006es)=>{/*T*/ns.f();});",
        "class C{static f(){}} class K{static {sink(()=>{/*T*/C.f();});}}",
        "import {f} from './lib'; namespace N{import f=M.q;sink(()=>{/*T*/f();});}",
        "import type * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
        "class C{f(){}} sink(()=>{/*T*/C.f();});",
        "function f(){} function outer(){eval('var f=other');sink(()=>{/*T*/f();});}",
        "class C{static f(){}} sink(()=>{/*T*/C.f();});",
        "class C{f(){}} function outer(x:C){sink(()=>{/*T*/x.f();});}",
        "class C{f(){}} sink(()=>{const x=new C(); /*T*/x.f();});",
    ] {
        check(source, LIB, false);
    }
}

#[test]
fn named_owner_outcomes_are_unchanged() {
    for source in [
        "class C{f(){}} sink(()=>{/*T*/C.f();});",
        "function f(){} eval('f=other'); sink(()=>{/*T*/f();});",
        "import * as ns from './lib'; sink((\\u006es)=>{/*T*/ns.f();});",
    ] {
        let (graph, mut site) = fixture(source, LIB);
        site.caller.name = "named".into();
        assert_eq!(
            graph.resolve_call_site_full(&site),
            graph.resolve_call_site_candidates(&site)
        );
    }
}

#[test]
fn boundary_dominates_all_route_kinds_and_rejects_wrong_site_or_target() {
    let (graph, site) = fixture("import {f} from './lib'; sink(()=>{/*T*/f();});", LIB);
    let target = &graph.functions["f"][0];
    let body = include_str!("resolution.rs")
        .split("pub enum ResolutionKind {")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    let kinds: Vec<ResolutionKind> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('/') && l.ends_with(','))
        .map(|l| serde_json::from_value(serde_json::json!(l.trim_end_matches(','))).unwrap())
        .collect();
    assert!(kinds.len() >= 28);
    for kind in kinds {
        let candidate = crate::resolution::ResolutionOutcome::hit(vec![ResolvedCallee {
            target,
            confidence: Exact,
            kind,
        }]);
        let out = graph.admit_anonymous_exact(&site, candidate);
        assert!(
            out.resolved.iter().all(|r| r.confidence != Exact),
            "{kind:?}"
        );
    }
    let proven = graph.resolve_call_site_candidates(&site);
    assert_eq!(
        graph.admit_anonymous_exact(&site, proven.clone()).resolved[0].confidence,
        Exact
    );
    let mut wrong_site = site.clone();
    wrong_site.start_byte += 1;
    assert_ne!(
        graph
            .admit_anonymous_exact(&wrong_site, proven.clone())
            .resolved[0]
            .confidence,
        Exact
    );
    let mut wrong_target = proven;
    wrong_target.resolved[0].target = &graph.functions["hook"][0];
    assert_ne!(
        graph.admit_anonymous_exact(&site, wrong_target).resolved[0].confidence,
        Exact
    );
}

#[test]
fn import_and_namespace_export_dependencies_require_clean_source_spans() {
    for source in [
        "import {f} from './lib'; sink(()=>{/*T*/f();});",
        "import * as ns from './lib'; sink(()=>{/*T*/ns.f();});",
        "import {f} from './lib'; sink(()=>{const a=f; /*T*/a();});",
    ] {
        check(source, "export function f(){} eval('f=other');", false);
    }
    let (mut graph, site) = fixture("import {f} from './lib'; sink(()=>{/*T*/f();});", LIB);
    graph
        .js_ts_resolved_exports
        .get_mut("lib.ts")
        .unwrap()
        .get_mut("f")
        .unwrap()
        .span = None;
    assert!(graph
        .resolve_call_site_full(&site)
        .resolved
        .iter()
        .all(|r| r.confidence != Exact));
    let (mut graph, site) = fixture("import {f} from './lib'; sink(()=>{/*T*/f();});", LIB);
    graph.import_bindings.get_mut("a.ts").unwrap()[0].member = Some("hook".into());
    assert!(graph
        .resolve_call_site_full(&site)
        .resolved
        .iter()
        .all(|r| r.confidence != Exact));
}
#[test]
fn uncertified_multiple_candidates_drop_and_cache_load_preserves_proof() {
    let (graph, site) = fixture("import {f} from './lib'; sink(()=>{/*T*/f();});", LIB);
    let candidate = || ResolvedCallee {
        target: &graph.functions["f"][0],
        confidence: Exact,
        kind: ResolutionKind::FreeMulti,
    };
    let out = graph.admit_anonymous_exact(
        &site,
        crate::resolution::ResolutionOutcome::hit(vec![candidate(), candidate()]),
    );
    assert!(out.resolved.is_empty());
    assert!(out.drop.is_some());
    let bytes = bincode::serialize(&graph).unwrap();
    let loaded: CallGraph = bincode::deserialize(&bytes).unwrap();
    let resolved = loaded.resolve_call_site_full(&site);
    assert_eq!(resolved.resolved.len(), 1);
    assert_eq!(resolved.resolved[0].confidence, Exact);
    let mut missing = loaded.clone();
    missing.js_anonymous = Default::default();
    assert!(missing
        .resolve_call_site_full(&site)
        .resolved
        .iter()
        .all(|r| r.confidence != Exact));
}

#[test]
fn reexport_barrel_dependencies_and_positive_control() {
    for tail in [
        "",
        " eval('x');",
        " const \\u0078=1;",
        " namespace N {import x=M.q;}",
        " function broken( {",
    ] {
        let lib = format!("export {{f}} from './impl'; {tail}");
        let (graph, site) = fixture_deps(
            "import {f} from './lib'; sink(()=>{/*T*/f();});",
            &lib,
            &[("impl.ts", "export function f(){}")],
        );
        assert_eq!(
            graph
                .resolve_call_site_full(&site)
                .resolved
                .iter()
                .any(|r| r.confidence == Exact),
            tail.is_empty(),
            "{tail}"
        );
    }
}

#[test]
fn evaluation_table_refuses_unlisted_type_positions() {
    let source = "function outer(): T {}";
    let p = ParsedFile::parse("a.ts", source, Language::TypeScript).unwrap();
    let start = source.find('T').unwrap();
    let node = p
        .tree
        .root_node()
        .descendant_for_byte_range(start, start + 1)
        .unwrap();
    assert!(!p.anonymous_evaluation_supported(node));
}

#[test]
fn extensionless_javascript_anonymous_owners_cannot_bypass_admission() {
    let source = "class C { f(){} } sink(()=>{/*T*/C.f();});";
    let (_, mut site) = fixture(source, LIB);
    site.caller.file = "virtual".into();
    let files = BTreeMap::from([(
        "virtual".into(),
        ParsedFile::parse("virtual", source, Language::JavaScript).unwrap(),
    )]);
    let graph = CallGraph::build(&files);
    let candidate = graph.resolve_call_site_candidates(&site);
    assert!(
        candidate.resolved.iter().any(|r| r.confidence == Exact),
        "probe requires a real candidate: {candidate:?}"
    );
    let out = graph.resolve_call_site_full(&site);
    assert!(
        out.resolved.iter().all(|r| r.confidence != Exact),
        "{out:?}"
    );
}

#[test]
fn static_eval_member_spellings_are_all_vetoed() {
    let mut failures = Vec::new();
    for reference in [
        "eval",
        "globalThis.eval",
        "globalThis['eval']",
        "globalThis['\\u0065val']",
        "globalThis['\\145val']",
        "globalThis[`eval`]",
        "globalThis[`\\u0065val`]",
        "globalThis[(`eval`)]",
        "globalThis[(0,'eval')]",
        "globalThis?.[`eval`]",
        "globalThis[`${key}`]",
    ] {
        let source = format!("function f(){{}} {reference}('f=other'); sink(()=>{{/*T*/f();}});");
        let js = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
        assert!(
            !js.tree.root_node().has_error(),
            "invalid JS probe: {reference}"
        );
        let (graph, site) = fixture(&source, LIB);
        let out = graph.resolve_call_site_full(&site);
        if out.resolved.iter().any(|r| r.confidence == Exact) {
            failures.push(format!("{reference}: {out:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} missed eval reference spellings:\n{}",
        failures.len(),
        failures.join("\n")
    );
    check(
        "function f(){} globalThis[`ordinary`](); sink(()=>{/*T*/f();});",
        LIB,
        true,
    );
    check(
        "function f(){} globalThis['ordinary'](); sink(()=>{/*T*/f();});",
        LIB,
        true,
    );
}
