//! Slice L, the rule by construction. One predicate, "the alias route produced a proven hit for
//! this site" (`CallGraph::js_ts_alias_route_hit`: exactly one Exact target through the terminal's
//! own route), is the only thing that may separate a followed-alias site from an unfollowed one.
//! Wherever it is false, a site bound to one of the three `Alias*` variants must behave exactly
//! as a site bound `MayCall("alias")` does on main, for the resolver and for the textual
//! function-pointer path ("Level 1").
//!
//! The test runs every fixture of the alias matrix (all files, every call-site record, not only
//! the marked line) and checks, without a product switch:
//!   1. resolver: for each `Source` site with an `Alias*` binding and a false predicate, the
//!      resolved record set equals the one obtained after rewriting the binding to
//!      `MayCall("alias")` on a clone of the site and resolving through the public entry point;
//!   2. textual path: the indirect records the graph holds equal the audit seam's records with the
//!      stand-down on (`CallGraph::audit_indirect_records(files, true)`), and the records with the
//!      stand-down forced off (main's behavior) differ from them only by records whose source
//!      site has a true predicate.
use prism::call_graph::{CallGraph, CallSite, CallSiteOrigin, FunctionId, JsLocalBinding};
use prism::repo_loader::load_repo;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(serde::Deserialize)]
struct Fixture {
    id: String,
    files: BTreeMap<String, String>,
}

type SiteKey = (FunctionId, usize, usize);
type RecordKey = (FunctionId, usize, usize, String);

#[derive(Default)]
struct Tally {
    sites: usize,
    alias_sites: usize,
    predicate_true: usize,
    compared: usize,
    indirect_with: usize,
    indirect_suppressed: usize,
    violations: Vec<String>,
}

fn fixtures() -> Vec<Fixture> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/js_alias_matrix/fixtures.jsonl");
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// Reviewer inputs (rounds 2 and 3) the matrix does not carry yet, run through the same checks.
fn review_inputs() -> Vec<Fixture> {
    let fx = |id: &str, files: &[(&str, &str)]| Fixture {
        id: id.to_string(),
        files: files
            .iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect(),
    };
    let comp = "function Comp() { return null; }\n";
    let lib = "export function target() { return null; }\n";
    let cb_lib = "export function target() { return 1; }\nexport function cbImpl() { return 2; }\n";
    vec![
        fx(
            "r3-jsx-intrinsic-same-function",
            &[(
                "app.jsx",
                &format!("{comp}export function sameFn() {{ const h = Comp; return <h />; }}\n"),
            )],
        ),
        fx(
            "r3-jsx-intrinsic-nested",
            &[(
                "app.jsx",
                &format!(
                    "{comp}export function outer() {{ const k = Comp; return function inner() {{ \
                     return <k />; }}; }}\n"
                ),
            )],
        ),
        fx(
            "r3-jsx-upper-control",
            &[(
                "app.jsx",
                &format!("{comp}export function upper() {{ const H = Comp; return <H />; }}\n"),
            )],
        ),
        fx(
            "r3-jsx-intrinsic-ns-destructure",
            &[
                ("lib.jsx", lib),
                (
                    "app.jsx",
                    "import * as N from './lib';\nexport function run() {\n  const { target: h } = N;\n  \
                     return <h />;\n}\n",
                ),
            ],
        ),
        fx(
            "r3-jsx-intrinsic-ns-member",
            &[
                ("lib.jsx", lib),
                (
                    "app.jsx",
                    "import * as N from './lib';\nexport function run() {\n  const h = N.target;\n  \
                     return <h />;\n}\n",
                ),
            ],
        ),
        fx(
            "r3-same-line-neighbour",
            &[
                ("lib.js", cb_lib),
                (
                    "app.js",
                    "import { target, cbImpl } from './lib';\nconst obj = {};\nobj.field = cbImpl;\n\
                     export function sameLine() { const h = target; return h(obj.field()); }\n\
                     export function direct() { return target(obj.field()); }\n",
                ),
            ],
        ),
        fx(
            "r3-callback-neighbour",
            &[
                ("lib.js", "export function target() { return 1; }\n"),
                (
                    "other.js",
                    "export function otherTarget() { return 2; }\n\
                     export function init(o) { o.callback = otherTarget; }\n",
                ),
                (
                    "app.js",
                    "import { target } from './lib';\nexport function run(obj) {\n  const h = target;\n  \
                     h(); obj.callback();\n}\n",
                ),
            ],
        ),
    ]
}

fn is_alias(binding: &JsLocalBinding) -> bool {
    matches!(
        binding,
        JsLocalBinding::AliasCallable(_)
            | JsLocalBinding::AliasImport { .. }
            | JsLocalBinding::AliasNamespaceMember { .. }
    )
}

/// The resolved record set of a site: targets with grade and kind, and the drop reason.
fn outcome_of(cg: &CallGraph, site: &CallSite) -> String {
    let out = cg.resolve_call_site_full(site);
    let targets: Vec<String> = out
        .resolved
        .iter()
        .map(|r| format!("{:?}|{:?}|{}", r.target, r.confidence, r.kind.as_str()))
        .collect();
    format!("{targets:?} drop={:?}", out.drop)
}

fn record_key(caller: &FunctionId, site: &CallSite) -> RecordKey {
    (
        caller.clone(),
        site.start_byte,
        site.end_byte,
        site.callee_name.clone(),
    )
}

fn check_fixture(f: &Fixture) -> Tally {
    let mut t = Tally::default();
    let dir = tempfile::tempdir().unwrap();
    for (rel, src) in &f.files {
        let p = dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, src).unwrap();
    }
    let repo = load_repo(dir.path()).unwrap();
    let cg =
        CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());

    let mut true_sites: BTreeSet<SiteKey> = BTreeSet::new();
    for sites in cg.calls.values() {
        for site in sites.iter().filter(|s| s.origin == CallSiteOrigin::Source) {
            t.sites += 1;
            if !is_alias(&site.local_binding) {
                continue;
            }
            t.alias_sites += 1;
            if cg.js_ts_alias_route_hit(site) {
                t.predicate_true += 1;
                true_sites.insert((site.caller.clone(), site.start_byte, site.end_byte));
                // The predicate is the resolver's own answer: the public outcome is exactly one
                // Exact target, equal to the route's, with no drop.
                let public = cg.resolve_call_site_full(site);
                let route = cg.audit_alias_route_targets(site);
                let single = match public.resolved.as_slice() {
                    [one]
                        if public.drop.is_none()
                            && one.confidence == prism::resolution::ResolutionConfidence::Exact =>
                    {
                        Some(one.target.clone())
                    }
                    _ => None,
                };
                if single.is_none() || route.as_deref() != single.as_ref().map(std::slice::from_ref)
                {
                    t.violations.push(format!(
                        "{} {}:{}: predicate true but the public resolver answered {:?} (drop {:?}), route {:?}",
                        f.id,
                        site.caller.file,
                        site.line,
                        public.resolved.iter().map(|r| &r.target).collect::<Vec<_>>(),
                        public.drop,
                        route
                    ));
                }
                continue;
            }
            let mut as_main = site.clone();
            as_main.local_binding = JsLocalBinding::MayCall("alias".to_string());
            t.compared += 1;
            let (got, want) = (outcome_of(&cg, site), outcome_of(&cg, &as_main));
            if got != want {
                t.violations.push(format!(
                    "{} {}:{}: resolver differs from the MayCall(\"alias\") twin\n  head: {got}\n  main: {want}",
                    f.id, site.caller.file, site.line
                ));
            }
        }
    }

    // Textual path.
    let held: BTreeSet<RecordKey> = cg
        .calls
        .iter()
        .flat_map(|(caller, sites)| {
            sites
                .iter()
                .filter(|s| s.origin == CallSiteOrigin::IndirectResolution)
                .map(move |s| record_key(caller, s))
        })
        .collect();
    let keys = |records: Vec<(FunctionId, CallSite)>| -> BTreeSet<RecordKey> {
        records.iter().map(|(c, s)| record_key(c, s)).collect()
    };
    let with = keys(cg.audit_indirect_records(&repo.files, true));
    let without = keys(cg.audit_indirect_records(&repo.files, false));
    t.indirect_with = with.len();
    if held != with {
        t.violations.push(format!(
            "{}: held indirect records differ from the stand-down-on audit\n  held: {held:?}\n  audit: {with:?}",
            f.id
        ));
    }
    for extra in without.difference(&with) {
        t.indirect_suppressed += 1;
        let source = (extra.0.clone(), extra.1, extra.2);
        if !true_sites.contains(&source) {
            t.violations.push(format!(
                "{}: textual record {extra:?} is suppressed at a site whose predicate is false",
                f.id
            ));
        }
    }
    for missing in with.difference(&without) {
        t.violations.push(format!(
            "{}: stand-down added a textual record {missing:?} main does not have",
            f.id
        ));
    }
    t
}

#[test]
fn the_alias_route_predicate_is_the_only_divergence_from_main_on_every_matrix_site() {
    let mut fixtures = fixtures();
    fixtures.extend(review_inputs());
    assert!(fixtures.len() > 1000, "matrix fixtures: {}", fixtures.len());
    let tallies: Vec<Tally> =
        prism::build_pool::install(|| fixtures.par_iter().map(check_fixture).collect());
    let mut total = Tally::default();
    for t in &tallies {
        total.sites += t.sites;
        total.alias_sites += t.alias_sites;
        total.predicate_true += t.predicate_true;
        total.compared += t.compared;
        total.indirect_with += t.indirect_with;
        total.indirect_suppressed += t.indirect_suppressed;
    }
    eprintln!(
        "rule test: {} fixtures, {} Source sites, {} alias-bound, {} predicate-true, \
         {} predicate-false alias sites compared with their MayCall(\"alias\") twin, \
         {} indirect records held, {} suppressed by the stand-down",
        fixtures.len(),
        total.sites,
        total.alias_sites,
        total.predicate_true,
        total.compared,
        total.indirect_with,
        total.indirect_suppressed
    );
    let violations: Vec<&String> = tallies.iter().flat_map(|t| &t.violations).collect();
    assert!(
        violations.is_empty(),
        "{} violations, first 20:\n{}",
        violations.len(),
        violations
            .iter()
            .take(20)
            .map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Guard against a vacuous pass: both sides of the predicate and the suppression occur.
    assert!(
        total.predicate_true > 100,
        "predicate-true sites: {}",
        total.predicate_true
    );
    assert!(total.compared > 20, "compared sites: {}", total.compared);
    assert!(
        total.indirect_suppressed >= 4,
        "suppressed: {}",
        total.indirect_suppressed
    );
}
