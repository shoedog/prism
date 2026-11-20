//! REAL: round-bound hints and parsed temporary repositories, except where explicitly marked.
use super::*;
use crate::algorithms::taint::{
    boundary::collect::{collect_round, site_has_loaded_callable},
    source_scope::admission::slice_scoped,
    TaintConfig,
};
use prism::api::{
    build_context, load_review_inputs, ContextReadOutcome, ReviewContextLoader, ReviewContextRound,
    ReviewOptions,
};

struct Fixture {
    _dir: tempfile::TempDir,
    inputs: prism::api::ReviewInputs,
    options: ReviewOptions,
}
impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        Self::named(files, "main.js", 2)
    }
    fn named(files: &[(&str, &str)], original: &str, line: usize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (name, source) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, source).unwrap();
        }
        let options = ReviewOptions::new(dir.path());
        let diff = serde_json::json!({"files": [{"file_path":original, "modify_type":"Modified", "diff_lines":[line]}]});
        let inputs = load_review_inputs(&options, &diff.to_string()).unwrap();
        assert!(
            inputs.files.contains_key(original),
            "inadmissible precondition: original missing"
        );
        Self {
            _dir: dir,
            inputs,
            options,
        }
    }
    fn union(&self, extras: &[&str]) -> BTreeMap<String, ParsedFile> {
        let loader = ReviewContextLoader::new(&self.inputs).unwrap();
        let mut files = self.inputs.files.clone();
        for path in extras {
            match loader.read(path, 1 << 20).outcome {
                ContextReadOutcome::Loaded(file) => {
                    files.insert(file.path, file.parsed);
                }
                _ => panic!("inadmissible precondition: reader refused {path}"),
            }
        }
        files
    }
    fn collection(&self) -> BoundaryCollection {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let loader = ReviewContextLoader::new(&self.inputs).unwrap();
        let round = ReviewContextRound::new(&loader, &self.inputs.files);
        let ctx = round.build_context(None, &built.ctx.types);
        let originals = self.inputs.files.keys().cloned().collect();
        let outcome =
            slice_scoped(&ctx, &self.inputs.diff, &TaintConfig::default(), &originals).unwrap();
        let mut hints = round.hints().unwrap();
        let c = collect_round(&mut hints, &ctx, &outcome.loading);
        assert_eq!(
            c.records.len(),
            1,
            "inadmissible precondition: original eligible call"
        );
        c
    }
}
const MAIN: &str = "import { h } from './a';\nexport function main(x) { h(x); }\n";
const BARREL: &str = "export { g as h } from './b';\n";
const BODY: &str = "export function g(x) {}\n";
fn fixture() -> Fixture {
    Fixture::new(&[("main.js", MAIN), ("a.js", BARREL), ("b.js", BODY)])
}
fn first(f: &Fixture, c: &BoundaryCollection) -> FrontierState {
    let built = build_context(&f.inputs, &f.options).unwrap();
    plan_round(
        1,
        2,
        c,
        &built.ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |_, _| None,
    )
}
#[test]
fn barrel_continuation_reaches_the_body_at_depth_two_and_renames() {
    let f = fixture();
    let c = f.collection();
    let state = first(&f, &c);
    assert_eq!(state.load_order, ["a.js"]);
    let files = f.union(&["a.js"]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let ctx = round.build_context(None, &baseline.ctx.types);
    let mut hints = round.hints().unwrap();
    let resolved = site_has_loaded_callable(&ctx, &c.records[0].site);
    let next = plan_round(
        2,
        2,
        &collection(vec![]),
        &ctx,
        &state,
        &BTreeSet::new(),
        &mut |barrel, member| {
            hints
                .follow_barrel(barrel, member)
                .map(|h| (h.file, h.member))
        },
    );
    if resolved {
        assert!(next
            .dispositions
            .iter()
            .any(|(_, _, d)| *d == Disposition::Resolved));
    } else {
        assert_eq!(next.load_order, ["b.js"]);
        assert_eq!(next.open[0].member.as_deref(), Some("g"));
    }
}
#[test]
fn depth_one_stops_at_the_barrel() {
    let f = fixture();
    let c = f.collection();
    let state = first(&f, &c);
    let files = f.union(&["a.js"]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let ctx = round.build_context(None, &baseline.ctx.types);
    let mut hints = round.hints().unwrap();
    assert!(
        !site_has_loaded_callable(&ctx, &c.records[0].site),
        "inadmissible precondition: barrel already resolved"
    );
    let next = plan_round(
        2,
        1,
        &collection(vec![]),
        &ctx,
        &state,
        &BTreeSet::new(),
        &mut |barrel, member| {
            hints
                .follow_barrel(barrel, member)
                .map(|h| (h.file, h.member))
        },
    );
    assert!(next.load_order.is_empty());
    assert!(next
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::DepthBound));
}

#[test]
fn two_hop_chain_stops_before_its_body() {
    let f = Fixture::new(&[
        ("main.js", MAIN),
        ("a.js", "export { q as h } from './b';\n"),
        ("b.js", "export { g as q } from './c';\n"),
        ("c.js", BODY),
    ]);
    let c = f.collection();
    let s1 = first(&f, &c);
    let files = f.union(&["a.js", "b.js"]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let ctx = round.build_context(None, &baseline.ctx.types);
    let mut hints = round.hints().unwrap();
    assert!(
        !site_has_loaded_callable(&ctx, &c.records[0].site),
        "inadmissible precondition: call already resolved"
    );
    let s2 = plan_round(
        2,
        2,
        &collection(vec![]),
        &ctx,
        &s1,
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert_eq!(s2.open[0].target, "b.js");
    let s3 = plan_round(
        3,
        2,
        &collection(vec![]),
        &ctx,
        &s2,
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert!(s3.open.is_empty());
    assert!(s3
        .dispositions
        .iter()
        .any(|(_, path, d)| path == "c.js" && *d == Disposition::DepthBound));
}
#[test]
fn real_barrel_cycle_terminates_without_reread() {
    let f = Fixture::new(&[
        ("main.js", MAIN),
        ("a.js", "export { h } from './b';\n"),
        ("b.js", "export { h } from './a';\n"),
    ]);
    let c = f.collection();
    let s1 = first(&f, &c);
    let files = f.union(&["a.js", "b.js"]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let ctx = round.build_context(None, &baseline.ctx.types);
    let mut hints = round.hints().unwrap();
    assert!(
        !site_has_loaded_callable(&ctx, &c.records[0].site),
        "inadmissible precondition: cycle resolved"
    );
    let s2 = plan_round(
        2,
        3,
        &collection(vec![]),
        &ctx,
        &s1,
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert_eq!(s2.open[0].target, "b.js");
    let s3 = plan_round(
        3,
        3,
        &collection(vec![]),
        &ctx,
        &s2,
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert!(s3.open.is_empty());
    assert!(s3.load_order.is_empty());
    assert!(s3
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::BarrelCycle));
}
#[test]
fn real_loaded_target_resolution_and_unresolved() {
    for (body, expected) in [
        ("export const x = 1;\n", Disposition::LoadedUnresolved),
        ("export function h(x) {}\n", Disposition::Resolved),
    ] {
        let f = Fixture::new(&[("main.js", MAIN), ("a.js", body)]);
        let c = f.collection();
        let state = first(&f, &c);
        let files = f.union(&["a.js"]);
        let loader = ReviewContextLoader::new(&f.inputs).unwrap();
        let round = ReviewContextRound::new(&loader, &files);
        let baseline = build_context(&f.inputs, &f.options).unwrap();
        let ctx = round.build_context(None, &baseline.ctx.types);
        let mut hints = round.hints().unwrap();
        assert_eq!(
            site_has_loaded_callable(&ctx, &c.records[0].site),
            expected == Disposition::Resolved,
            "inadmissible precondition: resolution state"
        );
        let next = plan_round(
            2,
            2,
            &collection(vec![]),
            &ctx,
            &state,
            &BTreeSet::new(),
            &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
        );
        assert!(next.open.is_empty());
        assert_eq!(next.dispositions.len(), 1);
        assert_eq!(next.dispositions[0].2, expected);
    }
}

// REAL when a producer trace reaches a context argument; otherwise CONTROLLED by one
// injected DataFlow edge after all three language candidates have been tried.
#[test]
fn round_collector_sees_a_context_caller() {
    use prism::cpg::{CpgEdge, FlowConfidence};
    if real_context_caller_recall_candidates() {
        return;
    }
    let f = Fixture::new(&[
        ("main.js", MAIN),
        (
            "a.js",
            "import { g } from './b';\nexport function h(x) { g(x); }\n",
        ),
        ("b.js", BODY),
    ]);
    let files = f.union(&["a.js"]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let mut ctx = round.build_context(None, &baseline.ctx.types);
    let originals = f.inputs.files.keys().cloned().collect();
    let mut outcome =
        slice_scoped(&ctx, &f.inputs.diff, &TaintConfig::default(), &originals).unwrap();
    let mut hints = round.hints().unwrap();
    let call = ctx
        .cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.file == "a.js" && c.callee_name == "g")
        .expect("inadmissible precondition: context call");
    let call_line = call.line;
    let spans = hints.argument_spans(call);
    let reaches_argument = |trace: &crate::algorithms::taint::TaintTrace| {
        trace.roots.iter().any(|root| {
            std::iter::once(&root.location)
                .chain(root.reached.iter().map(|r| &r.location))
                .any(|loc| {
                    loc.file == "a.js"
                        && spans.iter().any(|s| {
                            s.start <= loc.start_byte
                                && loc.end_byte <= s.end
                                && loc.start_byte < loc.end_byte
                        })
                })
        })
    };
    if !reaches_argument(&outcome.loading) {
        let source = outcome
            .loading
            .roots
            .first()
            .expect("inadmissible precondition: eligible root")
            .location
            .clone();
        let from = ctx
            .cpg
            .nodes_at(&source.file, source.line)
            .into_iter()
            .find(|&n| ctx.cpg.to_var_location(n).as_ref() == Some(&source))
            .expect("inadmissible precondition: root graph node");
        let to = ctx
            .cpg
            .nodes_at("a.js", call_line)
            .into_iter()
            .find(|&n| {
                ctx.cpg.to_var_location(n).is_some_and(|loc| {
                    spans.iter().any(|s| {
                        s.start <= loc.start_byte
                            && loc.end_byte <= s.end
                            && loc.start_byte < loc.end_byte
                    })
                })
            })
            .expect("inadmissible precondition: argument graph node");
        ctx.cpg
            .graph
            .add_edge(from, to, CpgEdge::DataFlow(FlowConfidence::Exact));
        outcome = slice_scoped(&ctx, &f.inputs.diff, &TaintConfig::default(), &originals).unwrap();
    }
    assert!(
        reaches_argument(&outcome.loading),
        "inadmissible precondition: loading trace reaches context argument"
    );
    let c = collect_round(&mut hints, &ctx, &outcome.loading);
    assert!(c.records.iter().any(|r| r.site.caller.file == "a.js"));
    let state = plan_round(
        2,
        2,
        &c,
        &ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert!(state.load_order.contains(&"b.js".into()));
}

// REAL attempts: each uses only the producer's graph and the eligible loading trace.
fn real_crossfile_candidate(
    files: &[(&str, &str)],
    original: &str,
    line: usize,
    context: &str,
    callee: &str,
    target: &str,
) -> bool {
    let f = Fixture::named(files, original, line);
    let files = f.union(&[context]);
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = ReviewContextRound::new(&loader, &files);
    let baseline = build_context(&f.inputs, &f.options).unwrap();
    let ctx = round.build_context(None, &baseline.ctx.types);
    let originals = f.inputs.files.keys().cloned().collect();
    let outcome = slice_scoped(&ctx, &f.inputs.diff, &TaintConfig::default(), &originals).unwrap();
    let mut hints = round.hints().unwrap();
    let call = ctx
        .cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.file == context && c.callee_name == callee)
        .expect("inadmissible precondition: real context call");
    let spans = hints.argument_spans(call);
    let reached = outcome.loading.roots.iter().any(|root| {
        std::iter::once(&root.location)
            .chain(root.reached.iter().map(|r| &r.location))
            .any(|loc| {
                loc.file == context
                    && spans.iter().any(|s| {
                        s.start <= loc.start_byte
                            && loc.end_byte <= s.end
                            && loc.start_byte < loc.end_byte
                    })
            })
    });
    if !reached {
        return false;
    }
    let collection = collect_round(&mut hints, &ctx, &outcome.loading);
    assert!(collection
        .records
        .iter()
        .any(|r| r.site.caller.file == context));
    let state = plan_round(
        2,
        2,
        &collection,
        &ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    assert!(state.load_order.contains(&target.to_string()));
    true
}
fn real_context_caller_recall_candidates() -> bool {
    let js = [
        ("main.js", MAIN),
        (
            "a.js",
            "import { g } from './b';\nexport function h(x) { g(x); }\n",
        ),
        ("b.js", BODY),
    ];
    let py = [
        ("main.py", "from a import h\ndef main(x):\n    h(x)\n"),
        ("a.py", "from b import g\ndef h(x):\n    g(x)\n"),
        ("b.py", "def g(x):\n    pass\n"),
    ];
    let go = [
        ("go.mod", "module example.com/app\n"),
        (
            "main.go",
            "package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.H(x) }\n",
        ),
        (
            "svc/a.go",
            "package svc\nimport \"example.com/app/leaf\"\nfunc H(x string) { leaf.G(x) }\n",
        ),
        ("leaf/b.go", "package leaf\nfunc G(x string) {}\n"),
    ];
    let observed = [
        real_crossfile_candidate(&js, "main.js", 2, "a.js", "g", "b.js"),
        real_crossfile_candidate(&py, "main.py", 3, "a.py", "g", "b.py"),
        real_crossfile_candidate(&go, "main.go", 3, "svc/a.go", "G", "leaf/b.go"),
    ];
    eprintln!("P14 real loading-reach candidates JS/Python/Go: {observed:?}");
    observed.into_iter().any(|value| value)
}
