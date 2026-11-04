use super::*;
use crate::cpg::reaching::{deduplicate_definitions, DefId};

const SOURCE: &str = "package p\nfunc f() {\np := T{}\np.f = 1\nuse(p)\n}\n";

fn run(flagged: bool, no_route: bool, unmapped: bool) -> Option<RecallResult> {
    run_raw(flagged, no_route, unmapped, false)
}

fn run_raw(flagged: bool, no_route: bool, unmapped: bool, duplicate: bool) -> Option<RecallResult> {
    let parsed = ParsedFile::parse("a.go", SOURCE, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let spans = parsed.assignment_lvalue_spans_on_lines(&owner, &(2..=6).collect());
    let mut raw: Vec<_> = spans
        .iter()
        .enumerate()
        .map(|(i, s)| DefSite {
            id: DefId(i as u32),
            path: s.path.clone(),
            line: s.line,
            start_byte: s.start_byte,
            alias_derived: false,
        })
        .collect();
    if duplicate {
        raw.extend(raw.clone());
    }
    let defs = deduplicate_definitions(&raw);
    let bindings = BindingFacts::new(&parsed, &owner, &defs, std::iter::empty());
    let mapped: Vec<_> = defs
        .iter()
        .map(|d| if unmapped { None } else { Some(d.line - 2) })
        .collect();
    let lines = BTreeMap::from([(3, 1), (4, 2), (5, 3)]);
    let spans = super::super::statement_line_spans(&parsed, &owner);
    let mut lookup = StatementLookup::new(&spans);
    let successors = vec![
        vec![(1, false)],
        vec![(2, false)],
        if no_route { vec![] } else { vec![(3, flagged)] },
        vec![],
    ];
    let mut predecessors = vec![vec![]; 4];
    for (from, edges) in successors.iter().enumerate() {
        for &(to, _) in edges {
            predecessors[to].push(from);
        }
    }
    let mut reachable = BitSet::new(4);
    for n in 0..4 {
        reachable.insert(n);
    }
    let lines = if unmapped {
        BTreeMap::from([(3, 1), (4, 2)])
    } else {
        lines
    };
    analyze(
        &parsed,
        owner,
        &raw,
        &defs,
        &mapped,
        &lines,
        &mut lookup,
        &successors,
        &predecessors,
        &reachable,
        &bindings,
    )
}

#[test]
fn go_field_recall_flagged_route_is_uncertain_and_no_route_is_absent() {
    let exact = run(false, false, false).unwrap();
    assert!(!exact.edges.is_empty());
    assert!(exact.edges.iter().all(|(_, l)| *l == FlowConfidence::Exact));
    let uncertain = run(true, false, false).unwrap();
    assert!(uncertain
        .edges
        .iter()
        .all(|(_, l)| *l == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)));
    assert!(run(false, true, false).is_none());
    let missing = run(false, false, true).unwrap();
    assert!(missing
        .edges
        .iter()
        .all(|(_, l)| *l == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)));
}

#[test]
fn go_field_recall_supplemental_uses_admitted_once_and_never_on_cap() {
    let parsed = ParsedFile::parse("a.go", SOURCE, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let admitted = run(false, false, false).unwrap();
    let mut uses = BTreeMap::new();
    let mut edges = vec![];
    let mut labels = BTreeMap::new();
    let mut stats = RdFileStats::default();
    let capped = with_cap(0, || run(false, false, false)).unwrap();
    assert!(capped.edges.is_empty());
    capped.emit(&mut uses, &mut edges, &mut labels, &mut stats, owner, "f");
    assert!(uses.is_empty());
    assert!(edges.is_empty());
    admitted
        .clone()
        .emit(&mut uses, &mut edges, &mut labels, &mut stats, owner, "f");
    assert_eq!(uses.values().map(Vec::len).sum::<usize>(), 1);
    admitted.emit(&mut uses, &mut edges, &mut labels, &mut stats, owner, "f");
    assert_eq!(uses.values().map(Vec::len).sum::<usize>(), 1);
}

#[test]
fn go_field_recall_missing_cfg_records_skip_and_no_edges() {
    let parsed = ParsedFile::parse("a.go", SOURCE, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let from = run(false, false, false).unwrap().edges.remove(0).0.from;
    let defs = [DefSite {
        id: DefId(0),
        path: from.path,
        line: from.line,
        start_byte: from.start_byte,
        alias_derived: false,
    }];
    let cfg = super::super::FileCfgEdgeIndex::new(&[]);
    let outcome =
        super::super::reaching_definitions_with_exact(&parsed, &owner, &defs, &[], &[], &cfg);
    assert_eq!(
        outcome,
        super::super::RdOutcome::Unavailable(RdUnavailable::NoCfgEdges)
    );
    let mut stats = RdFileStats::default();
    record_skip(&parsed, owner, &defs, RdUnavailable::NoCfgEdges, &mut stats);
    assert_eq!(stats.recall.functions_without_cfg(), 1);
    assert_eq!(stats.recall.candidates(), 0);
}

#[test]
fn go_field_recall_budget_counts_duplicate_raw_sources() {
    let single = run(false, false, false).unwrap().edges.len();
    let duplicate = run_raw(false, false, false, true).unwrap();
    assert_eq!(duplicate.edges.len(), single * 2);
    let capped = with_cap(single * 2 - 1, || run_raw(false, false, false, true)).unwrap();
    assert_eq!(capped.status, Status::AdditionCap(single * 2));
    assert!(capped.edges.is_empty());
}
