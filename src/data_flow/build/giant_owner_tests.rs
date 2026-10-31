//! Differentials for the giant-owner DFG paths: the per-owner shadow index, the
//! shortened `store_supplemental_exact` and the bulk owner labels, each against
//! the code they replaced, at the graph level (serialized bytes) on one and
//! four threads.
use super::owner_labels::{self, Labels};
use super::Producer;
use crate::access_path::AccessPath;
use crate::ast::member_reference_index::identifiers::shadow_index;
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::data_flow::{
    DataFlowGraph, ExactFlowEdge, ExactVarLocation, FlowEdge, VarAccessKind, VarLocation,
};
use crate::languages::Language;
use std::collections::BTreeMap;

/// goyacc shape: one method, one huge switch, a reused name per block.
fn switch_giant(cases: usize) -> String {
    let names = ["x", "list", "stmt", "expr", "tmp"];
    let mut s = String::from(
        "package p\n\ntype S struct {\n\tunion interface{}\n\ts string\n\tn int\n}\n\ntype P struct {\n\tstack [8]S\n}\n\nfunc mk(a string) string { return a }\n\nfunc (r *P) Parse(l int) int {\n\tvar yyn int\n\tvar yyVAL S\n\tvar yyDollar []S\n\t_ = yyDollar\n\tyyS := r.stack[:]\n\tyypt := 0\n\t_ = yypt\n\tswitch yyn {\n",
    );
    for i in 1..=cases {
        let nm = names[i % names.len()];
        s += &format!("\tcase {i}:\n\t\tyyDollar = yyS[yypt-1 : yypt+1]\n\t\t{{\n");
        match i % 4 {
            0 => s += &format!("\t\t\t{nm} := mk(yyDollar[1].s)\n\t\t\tyyVAL.union = {nm}\n"),
            1 => s += "\t\t\tyyVAL.union = yyDollar[1].union\n",
            2 => s += &format!(
                "\t\t\t{nm}, ok := yyDollar[1].union.(string)\n\t\t\tif ok {{\n\t\t\t\tyyVAL.s = {nm}\n\t\t\t}}\n"
            ),
            _ => s += &format!(
                "\t\t\tfn := func() string {{\n\t\t\t\t{nm} := yyDollar[1].s\n\t\t\t\treturn {nm}\n\t\t\t}}\n\t\t\tyyVAL.s = fn()\n"
            ),
        }
        s += "\t\t}\n";
    }
    s + "\t}\n\treturn yyVAL.n\n}\n"
}

/// More definitions than `RD_MAX_DEFS`, plus one name with many definitions
/// and uses so the owner also has well over `BULK_MIN` edges.
fn over_cap(distinct: usize, repeats: usize) -> String {
    let mut s = String::from("package p\n\nfunc use(a ...int) {}\n\nfunc big() int {\n\tx := 0\n");
    for i in 0..distinct {
        s += &format!("\tv{i} := {i}\n\tuse(v{i})\n");
    }
    for i in 0..repeats {
        s += &format!("\tx = x + {i}\n\tuse(x)\n");
    }
    s + "\treturn x\n}\n"
}

/// Straight line: RD is available and exact read candidates exist.
fn straight(repeats: usize) -> String {
    let mut s = String::from("package p\n\nfunc use(a ...int) {}\n\nfunc line() int {\n\tx := 0\n");
    for i in 0..repeats {
        s += &format!("\tx = {i} // padding padding padding\n\tuse(x, x) // padding padding\n");
    }
    s + "\treturn x\n}\n"
}

struct Built {
    bytes: Vec<u8>,
    labels: usize,
    over_cap: usize,
    without_cfg: usize,
    exact: usize,
    index_builds: usize,
    bulk_runs: usize,
}

fn build(source: &str, reference: bool, threads: usize) -> Built {
    build_as(source, "sql.go", Language::Go, reference, threads)
}

fn build_as(
    source: &str,
    file: &str,
    language: Language,
    reference: bool,
    threads: usize,
) -> Built {
    let files = BTreeMap::from([(
        file.to_string(),
        ParsedFile::parse(file, source, language).unwrap(),
    )]);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .start_handler(move |_| {
            shadow_index::set_mode_for_tests(Some(reference));
            owner_labels::set_reference_paths(reference);
        })
        .build()
        .unwrap();
    let dfg = pool.install(|| DataFlowGraph::build_on_current_pool(&files));
    let stats = dfg.rd_function_stats().values().fold((0, 0), |a, s| {
        (a.0 + s.functions_over_cap, a.1 + s.functions_without_cfg)
    });
    Built {
        bytes: bincode::serialize(&dfg).unwrap(),
        labels: dfg.labels().count(),
        exact: dfg.labels().filter(|(_, c)| c.is_exact()).count(),
        over_cap: stats.0,
        without_cfg: stats.1,
        index_builds: shadow_index::tree_builds_on(&pool),
        bulk_runs: owner_labels::bulk_runs_on(&pool),
    }
}

fn assert_same(name: &str, source: &str) -> (usize, usize, usize) {
    for threads in [1, 4] {
        let old = build(source, true, threads);
        let new = build(source, false, threads);
        assert_eq!(old.index_builds, 0, "{name}: reference must not index");
        assert_eq!(old.bulk_runs, 0, "{name}: reference must not bulk");
        assert!(new.index_builds > 0, "{name}: index never ran");
        assert!(
            old.bytes == new.bytes,
            "{name} threads={threads}: serialized graphs differ ({} vs {} bytes)",
            old.bytes.len(),
            new.bytes.len()
        );
        assert_eq!(old.labels, new.labels);
        assert_eq!(
            (old.over_cap, old.without_cfg),
            (new.over_cap, new.without_cfg)
        );
        if threads == 4 {
            return (new.labels, new.over_cap, new.without_cfg);
        }
    }
    unreachable!("threads include 4")
}

#[test]
fn switch_giant_graph_matches_the_old_paths() {
    // 90 cases stay under the definition cap, so RD runs on `Parse` and its
    // labels take the available-path merge (`mk` is the one owner without CFG).
    let source = switch_giant(90);
    assert!(source.len() > shadow_index_min());
    let (labels, over_cap, without_cfg) = assert_same("switch", &source);
    assert!(labels >= owner_labels::BULK_MIN, "labels={labels}");
    assert_eq!((over_cap, without_cfg), (0, 1));
    let new = build(&source, false, 1);
    assert!(new.exact > 0, "RD classified edges of Parse");
    assert!(
        new.bulk_runs > 0,
        "available-path labels went through the merge"
    );
}

#[test]
fn over_cap_graph_matches_the_old_paths() {
    let source = over_cap(2100, 70);
    assert!(source.len() > shadow_index_min());
    let (labels, over_cap, _) = assert_same("over-cap", &source);
    assert!(labels >= owner_labels::BULK_MIN, "labels={labels}");
    assert_eq!(over_cap, 1, "RD must refuse this owner by definition cap");
    assert!(
        build(&source, false, 1).bulk_runs > 0,
        "fallback labels of the refused owner were built in bulk"
    );
}

#[test]
fn straight_line_graph_matches_the_old_paths() {
    let source = straight(200);
    assert!(source.len() > shadow_index_min());
    let (labels, over_cap, _) = assert_same("straight", &source);
    assert_eq!(over_cap, 0);
    assert!(labels >= owner_labels::BULK_MIN, "labels={labels}");
    let new = build(&source, false, 1);
    // `line` is the only owner with statements to reach: RD classified edges.
    assert!(new.exact > 0, "RD must be available for this owner");
    assert!(
        new.bulk_runs > 0,
        "available-path labels went through append"
    );
}

/// JS/TS are the only languages with exact read candidates, so the nonempty
/// `store_supplemental_exact` path is reached from JavaScript owners only.
fn js_exact(distinct: usize, repeats: usize) -> String {
    let mut s = String::from("function use(...a) {}\nfunction line() {\n  let x = 0;\n");
    for i in 0..distinct {
        s += &format!("  const v{i} = {i};\n  use(v{i}, v{i});\n");
    }
    for i in 0..repeats {
        s += &format!("  x = {i};\n  use(x, x);\n");
    }
    s + "  return x;\n}\n"
}

fn assert_js_same(name: &str, source: &str) -> Built {
    let mut last = None;
    for threads in [1, 4] {
        let old = build_as(source, "a.js", Language::JavaScript, true, threads);
        let new = build_as(source, "a.js", Language::JavaScript, false, threads);
        assert_eq!(old.bulk_runs, 0, "{name}");
        assert!(
            old.bytes == new.bytes,
            "{name} threads={threads}: graphs differ"
        );
        last = Some(new);
    }
    last.unwrap()
}

#[test]
fn js_exact_side_table_matches_the_old_paths() {
    // Available path: RD runs, exact candidates are classified and stored.
    let small = assert_js_same("js-available", &js_exact(40, 70));
    assert!(small.exact > 0);
    let files = BTreeMap::from([(
        "a.js".to_string(),
        ParsedFile::parse("a.js", &js_exact(40, 70), Language::JavaScript).unwrap(),
    )]);
    let dfg = DataFlowGraph::build(&files);
    assert!(
        dfg.exact_labels().count() > 0,
        "available path stores exact labels"
    );
    // Unavailable path (over the definition cap) with real exact candidates:
    // the per-edge fallback label map is still built and consulted.
    let big_source = js_exact(2100, 70);
    let big = assert_js_same("js-over-cap", &big_source);
    assert_eq!(big.over_cap, 1);
    let files = BTreeMap::from([(
        "a.js".to_string(),
        ParsedFile::parse("a.js", &big_source, Language::JavaScript).unwrap(),
    )]);
    let dfg = DataFlowGraph::build(&files);
    assert!(
        dfg.exact_labels().count() > 0,
        "over-cap path stores exact labels"
    );
}

fn shadow_index_min() -> usize {
    8 * 1024
}

fn loc(line: usize, name: &str, kind: VarAccessKind, byte: usize) -> VarLocation {
    VarLocation {
        file: "a.go".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple(name.to_string()),
        start_byte: byte,
        end_byte: byte + 1,
        kind,
    }
}

fn edge(from_line: usize, to_line: usize, name: &str, byte: usize) -> FlowEdge {
    FlowEdge {
        from: loc(from_line, name, VarAccessKind::Def, byte),
        to: loc(to_line, name, VarAccessKind::Use, byte),
    }
}

fn per_edge(edges: &[FlowEdge], label: FlowConfidence) -> Labels {
    let mut labels = Labels::new();
    for e in edges {
        Producer::insert_label(&mut labels, (e.from.clone(), e.to.clone()), label);
    }
    labels
}

fn keys_with_bytes(labels: &Labels) -> Vec<(usize, usize, usize, usize, FlowConfidence)> {
    labels
        .iter()
        .map(|((f, t), l)| (f.line, t.line, f.start_byte, t.start_byte, *l))
        .collect()
}

#[test]
fn fallback_labels_equal_per_edge_insertion_for_every_edge_order() {
    let label = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    let ascending: Vec<_> = (1..40)
        .flat_map(|d| (d..d + 5).map(move |u| edge(d, u, "x", 10 * d + u)))
        .collect();
    let mut shuffled = ascending.clone();
    shuffled.reverse();
    shuffled.rotate_left(17);
    // Same identity, different bytes: the first inserted key must survive.
    let mut duplicated = ascending.clone();
    for e in ascending.iter().step_by(3) {
        let mut twin = e.clone();
        twin.from.start_byte += 1000;
        twin.to.start_byte += 1000;
        duplicated.push(twin);
    }
    duplicated.rotate_left(5);
    let mut mixed = ascending.clone();
    mixed.push(edge(3, 4, "y", 1)); // another path, same lines
    mixed.push(edge(2, 90, "x", 2));
    // Same lines and path, other access kinds: distinct identities.
    let mut kinds = edge(3, 4, "x", 31);
    kinds.from.kind = VarAccessKind::Use;
    mixed.push(kinds);
    let mut kinds = edge(3, 4, "x", 32);
    kinds.to.kind = VarAccessKind::Def;
    mixed.push(kinds);
    for (name, edges) in [
        ("ascending", ascending),
        ("shuffled", shuffled),
        ("duplicated", duplicated),
        ("mixed", mixed),
    ] {
        let want = per_edge(&edges, label);
        let got = Producer::fallback_owner_labels(&edges, label);
        assert_eq!(got, want, "{name}");
        assert_eq!(
            keys_with_bytes(&got),
            keys_with_bytes(&want),
            "{name}: kept keys"
        );
    }
    // Edges that do not share an owner prefix take the full comparison.
    let mut foreign = vec![edge(3, 4, "x", 1), edge(3, 4, "x", 2)];
    foreign[0].from.function = "g".into();
    foreign[0].to.function = "g".into();
    assert_eq!(
        Producer::fallback_owner_labels(&foreign, label),
        per_edge(&foreign, label)
    );
    assert_eq!(Producer::fallback_owner_labels(&foreign, label).len(), 2);
    assert!(Producer::fallback_owner_labels(&[], label).is_empty());
}

#[test]
fn merging_owner_labels_equals_per_key_insertion() {
    let label = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    let big: Vec<_> = (1..80)
        .flat_map(|d| (d..d + 60).map(move |u| edge(d, u, "x", u)))
        .collect();
    assert!(big.len() >= owner_labels::BULK_MIN);
    let mut other = big.clone();
    for e in &mut other {
        e.from.function = "a_other".into();
        e.to.function = "a_other".into();
    }
    let other_labels = per_edge(&other, label);
    let batch = per_edge(&big, FlowConfidence::Exact);

    // Unique owner, existing labels sort both before and after: append.
    let mut merged = other_labels.clone();
    Producer::merge_owner_labels(&mut merged, batch.clone(), true);
    let mut want = other_labels.clone();
    for (k, l) in batch.clone() {
        Producer::insert_label(&mut want, k, l);
    }
    assert_eq!(merged, want);

    // Non-unique owner with colliding keys: values combine by `worst`, the
    // existing key is kept.
    let mut colliding = per_edge(&big, label);
    let mut want = colliding.clone();
    Producer::merge_owner_labels(&mut colliding, batch.clone(), false);
    for (k, l) in batch.clone() {
        Producer::insert_label(&mut want, k, l);
    }
    assert_eq!(colliding, want);
    assert!(
        colliding.values().all(|l| *l == label),
        "worst keeps NameOnly"
    );

    // Empty destination adopts the batch; empty batch changes nothing.
    let mut empty = Labels::new();
    Producer::merge_owner_labels(&mut empty, batch.clone(), false);
    assert_eq!(empty, batch);
    let mut same = batch.clone();
    Producer::merge_owner_labels(&mut same, Labels::new(), true);
    assert_eq!(same, batch);

    // Below BULK_MIN the per-key path is used and agrees.
    let small: Vec<_> = big.iter().take(10).cloned().collect();
    let mut dest = other_labels.clone();
    let mut want = other_labels;
    Producer::merge_owner_labels(&mut dest, per_edge(&small, label), true);
    for (k, l) in per_edge(&small, label) {
        Producer::insert_label(&mut want, k, l);
    }
    assert_eq!(dest, want);
}

fn exact(line: usize, byte: usize, kind: VarAccessKind) -> ExactVarLocation {
    ExactVarLocation::from_legacy(&loc(line, "x", kind, byte))
}

#[test]
fn supplemental_exact_matches_the_unshortened_body() {
    let nameonly = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    let legacy: Vec<FlowEdge> = (1..6).map(|i| edge(1, 1 + i, "x", i)).collect();
    let mut legacy_labels = per_edge(&legacy, FlowConfidence::Exact);
    // One legacy edge carries a NameOnly label: a classified Exact for the
    // same binding conflicts and must be refused.
    let conflicted = (legacy[2].from.clone(), legacy[2].to.clone());
    legacy_labels.insert(conflicted, nameonly);
    let edge_for = |e: &FlowEdge| ExactFlowEdge::from_refs(&e.from, &e.to);
    let fresh = ExactFlowEdge {
        from: exact(9, 90, VarAccessKind::Def),
        to: exact(10, 100, VarAccessKind::Use),
    };
    let cases: Vec<(&str, BTreeMap<ExactFlowEdge, FlowConfidence>)> = vec![
        ("empty", BTreeMap::new()),
        (
            "fresh",
            BTreeMap::from([(fresh.clone(), FlowConfidence::Exact)]),
        ),
        (
            "already legacy",
            BTreeMap::from([(edge_for(&legacy[0]), FlowConfidence::Exact)]),
        ),
        (
            "conflicting",
            BTreeMap::from([
                (edge_for(&legacy[2]), FlowConfidence::Exact),
                (fresh.clone(), FlowConfidence::Exact),
            ]),
        ),
    ];
    for (name, classified) in cases {
        for seeded in [false, true] {
            let mut want = BTreeMap::new();
            let mut got = BTreeMap::new();
            if seeded {
                want.insert(fresh.clone(), nameonly);
                got.insert(fresh.clone(), nameonly);
            }
            Producer::store_supplemental_exact_frozen(
                &mut want,
                &legacy,
                &legacy_labels,
                classified.clone(),
            );
            Producer::store_supplemental_exact(
                &mut got,
                &legacy,
                &legacy_labels,
                classified.clone(),
            );
            assert_eq!(got, want, "{name} seeded={seeded}");
        }
    }
    // The frozen body really stores something for the fresh case, so the
    // equalities above are not vacuous.
    let mut stored = BTreeMap::new();
    Producer::store_supplemental_exact(
        &mut stored,
        &legacy,
        &legacy_labels,
        BTreeMap::from([(fresh.clone(), FlowConfidence::Exact)]),
    );
    assert_eq!(stored.get(&fresh), Some(&FlowConfidence::Exact));
}

/// Two owners sharing (name, start line) whose labels together pass `BULK_MIN`:
/// the bulk label paths must see the collision and keep the legacy graph.
#[test]
fn owners_sharing_name_and_start_line_keep_the_legacy_graph() {
    let mut source = String::from("fn f() { fn f() {\n let mut x = 0;\n");
    for i in 0..75 {
        source += &format!(" x = {i};\n consume(x);\n");
    }
    source += " }\n let mut x = 1;\n consume(x);\n }\n";
    let parsed = ParsedFile::parse("a.rs", &source, Language::Rust).unwrap();
    let owners = parsed.all_functions();
    assert_eq!(owners.len(), 2);
    assert_eq!(
        parsed.dfg_owner_name(&owners[0]),
        parsed.dfg_owner_name(&owners[1])
    );
    assert_eq!(
        parsed.node_line_range(&owners[0]).0,
        parsed.node_line_range(&owners[1]).0
    );
    for threads in [1, 4] {
        let old = build_as(&source, "a.rs", Language::Rust, true, threads);
        let new = build_as(&source, "a.rs", Language::Rust, false, threads);
        assert!(new.labels >= owner_labels::BULK_MIN, "{}", new.labels);
        assert_eq!(new.bytes, old.bytes, "colliding owners threads={threads}");
    }
}
