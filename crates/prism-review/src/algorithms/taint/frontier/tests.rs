//! CONTROLLED: records and the hop closure are injected; contexts use parsed source.
use super::*;
use crate::algorithms::taint::boundary::{display, records::*};
use prism::{
    api::{ReviewHintBasis, ReviewUnresolvedReason},
    ast::ParsedFile,
    call_graph::FunctionId,
    cpg::CpgContext,
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "tests/real.rs"]
mod real;

fn files(paths: &[&str]) -> BTreeMap<String, ParsedFile> {
    paths
        .iter()
        .map(|p| {
            (
                (*p).into(),
                ParsedFile::parse(p, "export const x = 1;", Language::JavaScript).unwrap(),
            )
        })
        .collect()
}
fn record(byte: usize, targets: &[&str]) -> BoundaryRecord {
    BoundaryRecord {
        site: PhysicalSite {
            caller: FunctionId {
                file: "main.js".into(),
                name: "h".into(),
                start_line: 1,
                end_line: 2,
            },
            start_byte: byte,
            end_byte: byte + 1,
        },
        line: 1,
        callee: "f".into(),
        language: Language::JavaScript,
        hint: ReviewImportHint::InRepo {
            files: targets.iter().map(|s| (*s).into()).collect(),
            spec: "./x".into(),
            member: Some("f".into()),
            declaring: vec![],
            basis: ReviewHintBasis::Bound,
        },
        kind: BoundaryKind::InRepo,
    }
}
fn collection(records: Vec<BoundaryRecord>) -> BoundaryCollection {
    BoundaryCollection {
        availability: Availability::Available,
        records,
    }
}
fn plan(records: Vec<BoundaryRecord>, ctx: &CpgContext<'_>) -> FrontierState {
    plan_round(
        1,
        2,
        &collection(records),
        ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |_, _| None,
    )
}
#[test]
fn frontier_is_complete_beyond_ten() {
    let records: Vec<_> = (0..12).map(|i| record(i, &["x.js"])).collect();
    let shown = display::project(&collection(records.clone()));
    assert_eq!(
        shown.located().len(),
        10,
        "inadmissible precondition: display cap"
    );
    assert_eq!(
        shown.omitted_count(),
        2,
        "inadmissible precondition: omitted count"
    );
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    assert_eq!(plan(records, &ctx).open.len(), 12);
}
#[test]
fn same_line_calls_keep_byte_identity() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    let state = plan(vec![record(1, &["x.js"]), record(2, &["x.js"])], &ctx);
    assert_eq!(state.provenance.len(), 2);
    assert_eq!(state.load_order, ["x.js"]);
    assert_eq!(state.provenance[state.open[0].origin].site.start_byte, 1);
    assert_eq!(
        state,
        plan(vec![record(2, &["x.js"]), record(1, &["x.js"])], &ctx)
    );
    let mut carried = state.clone();
    carried.open.reverse();
    let sorted = plan_round(
        1,
        2,
        &collection(vec![]),
        &ctx,
        &carried,
        &BTreeSet::new(),
        &mut |_, _| None,
    );
    assert_eq!(sorted.provenance[sorted.open[0].origin].site.start_byte, 1);
}
#[test]
fn plan_is_order_independent() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    let a = record(1, &["a.js"]);
    let b = record(2, &["b.js"]);
    assert_eq!(plan(vec![a.clone(), b.clone()], &ctx).open.len(), 2);
    assert_eq!(
        plan(vec![a.clone(), b.clone()], &ctx),
        plan(vec![b, a.clone(), a], &ctx)
    );
}
#[test]
fn go_loads_declaring_files_only() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    for declarations in [vec![], vec!["a.go"], vec!["a.go", "b.go"]] {
        let mut r = record(1, &["a.go", "b.go", "c.go"]);
        r.language = Language::Go;
        if let ReviewImportHint::InRepo { declaring, .. } = &mut r.hint {
            *declaring = declarations.iter().map(|s| (*s).into()).collect();
        }
        let state = plan(vec![r], &ctx);
        assert_eq!(state.open.len(), declarations.len());
        if declarations.is_empty() {
            assert!(state
                .dispositions
                .iter()
                .any(|(_, _, d)| *d == Disposition::GoDeclarationUnknown));
        }
    }
}
#[test]
fn unresolvable_requests_nothing() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    let mut r = record(1, &["x.js"]);
    r.hint = ReviewImportHint::Unresolvable {
        spec: "./missing".into(),
        why: ReviewUnresolvedReason::NoFile,
    };
    r.kind = BoundaryKind::Unresolved;
    let state = plan(vec![r], &ctx);
    assert!(state.open.is_empty());
    assert!(state
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::Unresolvable));
}
#[test]
fn repeated_target_loads_once_with_every_caller() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    let state = plan(vec![record(1, &["x.js"]), record(2, &["x.js"])], &ctx);
    assert_eq!(state.open.len(), 2);
    assert_eq!(state.load_order.len(), 1);
}
#[test]
fn refused_target_is_not_requested_again() {
    let fs = files(&["main.js"]);
    let ctx = CpgContext::build(&fs, None);
    let first = plan(vec![record(1, &["x.js"])], &ctx);
    let second = plan_round(
        2,
        2,
        &collection(vec![]),
        &ctx,
        &first,
        &BTreeSet::from(["x.js".into()]),
        &mut |_, _| None,
    );
    assert!(second.open.is_empty());
    assert!(second.load_order.is_empty());
    assert!(second
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::TargetRefused));
}

// CONTROLLED: the parser creates the graph; the hop closure supplies barrel facts.
#[test]
fn controlled_depth_and_one_hop_per_pass() {
    let first_files = files(&["main.js"]);
    let first_ctx = CpgContext::build(&first_files, None);
    let first = plan(vec![record(1, &["a.js"])], &first_ctx);
    let loaded = files(&["main.js", "a.js", "b.js", "c.js"]);
    let ctx = CpgContext::build(&loaded, None);
    let mut calls = 0;
    let second = plan_round(
        2,
        2,
        &collection(vec![]),
        &ctx,
        &first,
        &BTreeSet::new(),
        &mut |_, _| {
            calls += 1;
            Some(("b.js".into(), "g".into()))
        },
    );
    assert_eq!(calls, 1);
    assert_eq!(second.open.len(), 1);
    assert_eq!(second.open[0].member.as_deref(), Some("g"));
    assert_eq!(second.open[0].depth, 2);
    assert!(second.load_order.is_empty());
    let bounded = plan_round(
        2,
        1,
        &collection(vec![]),
        &ctx,
        &first,
        &BTreeSet::new(),
        &mut |_, _| Some(("b.js".into(), "g".into())),
    );
    assert!(bounded.open.is_empty());
    assert!(bounded
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::DepthBound));
}
#[test]
fn controlled_barrel_cycle_terminates_without_reread() {
    let initial = files(&["main.js"]);
    let ctx = CpgContext::build(&initial, None);
    let first = plan(vec![record(1, &["a.js"])], &ctx);
    let loaded = files(&["main.js", "a.js", "b.js", "c.js"]);
    let ctx = CpgContext::build(&loaded, None);
    let second = plan_round(
        2,
        4,
        &collection(vec![]),
        &ctx,
        &first,
        &BTreeSet::new(),
        &mut |_, _| Some(("b.js".into(), "f".into())),
    );
    let third = plan_round(
        3,
        4,
        &collection(vec![]),
        &ctx,
        &second,
        &BTreeSet::new(),
        &mut |_, _| Some(("c.js".into(), "f".into())),
    );
    let fourth = plan_round(
        4,
        4,
        &collection(vec![]),
        &ctx,
        &third,
        &BTreeSet::new(),
        &mut |_, _| Some(("b.js".into(), "f".into())),
    );
    assert!(fourth.open.is_empty());
    assert!(fourth.load_order.is_empty());
    assert!(fourth
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::BarrelCycle));
}
#[test]
fn controlled_loaded_target_without_resolution_is_loaded_unresolved() {
    let initial = files(&["main.js"]);
    let ctx = CpgContext::build(&initial, None);
    let first = plan(vec![record(1, &["a.js"])], &ctx);
    let loaded = files(&["main.js", "a.js"]);
    let ctx = CpgContext::build(&loaded, None);
    let result = plan_round(
        2,
        2,
        &collection(vec![]),
        &ctx,
        &first,
        &BTreeSet::new(),
        &mut |_, _| None,
    );
    assert!(result.open.is_empty());
    assert!(result
        .dispositions
        .iter()
        .any(|(_, _, d)| *d == Disposition::LoadedUnresolved));
}
