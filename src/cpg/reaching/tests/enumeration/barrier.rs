use super::super::grammar_lint::{visibility_span, ByteSpan, SpanKind};
use super::case::{Case, CaseClass};
use crate::cpg::{CodePropertyGraph, DfgLabelStats, FlowConfidence, FlowDoubt};
use crate::data_flow::VarLocation;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[test]
fn every_adjacent_pair_is_ordered() {
    let ordered = [
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 11 }),
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 37 }),
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
        FlowConfidence::NameOnly(FlowDoubt::SameLine),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
    ];

    for pair in ordered.windows(2) {
        let better = pair[0];
        let worse = pair[1];
        assert_eq!(better.worst(worse), worse, "{better:?} < {worse:?}");
        assert_eq!(worse.worst(better), worse, "{worse:?} > {better:?}");
    }
}

fn site(location: &VarLocation) -> String {
    format!(
        "{}:{}:{}:{}:{}:{:?}",
        location.file,
        location.function,
        location.function_start_line,
        location.line,
        location.path,
        location.kind
    )
}

fn real_folded_paths(fixture: &str) -> BTreeMap<(String, String), FlowConfidence> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("eval/fixtures")
        .join(fixture);
    let repo = crate::repo_loader::load_repo(&root).unwrap();
    let cpg = crate::build_pool::install(|| CodePropertyGraph::build(&repo.files));
    let defs: BTreeSet<VarLocation> = cpg
        .dfg
        .defs()
        .flat_map(|(_, locations)| locations)
        .cloned()
        .collect();
    let mut paths = BTreeMap::new();

    for from in defs {
        let (reachable, labels) = cpg.dfg_forward_reachable_labeled(&from);
        for to in reachable {
            let target = cpg.var_node_for_location(&to).unwrap();
            paths.insert((site(&from), site(&to)), labels[&target]);
        }
    }
    paths
}

#[test]
fn ruled_path_label_delta_is_exact() {
    type Row = (
        &'static str,
        &'static str,
        &'static str,
        FlowConfidence,
        FlowConfidence,
    );
    const ALIAS: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);
    const CFG: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    macro_rules! row {
        ($fixture:literal, $from:literal, $to:literal) => {
            ($fixture, $from, $to, ALIAS, CFG)
        };
    }

    let expected: [Row; 31] = [
        row!(
            "go/dfg_reaching_alias_conservative",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:p:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:p:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:3:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:p.x:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:4:q.x:Def"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:3:q:Def",
            "main.go:f:2:6:p:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:5:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:p:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:q:Def",
            "main.go:f:2:5:q.x:Use"
        ),
        row!(
            "go/dfg_reaching_alias_unstable",
            "main.go:f:2:6:q:Def",
            "main.go:f:2:6:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_conservative",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:p:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:p:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:2:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:p.x:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:3:q.x:Def"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:2:q:Def",
            "a.py:f:1:5:p:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:4:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:p:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:q:Def",
            "a.py:f:1:4:q.x:Use"
        ),
        row!(
            "python/dfg_reaching_alias_unstable",
            "a.py:f:1:5:q:Def",
            "a.py:f:1:5:q:Use"
        ),
        row!(
            "rust/dfg_reaching_alias_conservative",
            "main.rs:f:1:2:q:Def",
            "main.rs:f:1:4:q.x:Use"
        ),
    ];

    let mut snapshots = BTreeMap::new();
    let observed: Vec<Row> = expected
        .iter()
        .map(|&(fixture, from, to, old, new)| {
            let paths = snapshots
                .entry(fixture)
                .or_insert_with(|| real_folded_paths(fixture));
            let actual = paths
                .get(&(from.to_string(), to.to_string()))
                .copied()
                .unwrap_or_else(|| panic!("missing real folded path: {fixture}: {from} -> {to}"));
            assert_ne!(old, new, "delta row must change: {fixture}: {from} -> {to}");
            (fixture, from, to, old, actual)
        })
        .collect();

    assert_eq!(observed, expected);
}

pub(super) const CASES: &[Case] = &[
    Case {
        id: "e0b-java-record_pattern",
        language: Language::Java,
        kind: "record_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x)) { sink(x); }\n    return x;\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "e0b-rs-tuple_struct_pattern",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt { observe(x); }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: None,
    },
    Case {
        id: "e0c-1-java-instanceof-same-line-guarded-use",
        language: Language::Java,
        kind: "record_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x) && sink(x)) { }\n    return x;\n}\n",
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "e0c-1-rust-if-let-empty-span",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt {}\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 0)),
    },
    Case {
        id: "e0b-bash-variable_assignment",
        language: Language::Bash,
        kind: "variable_assignment",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    for x in a b; do :; done\n    sink \"$x\"\n}\n",
        expect: &[(
            "2:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
];

const JAVA_NEGATED_CASE: Case = Case {
    id: "java-negated-instanceof-branch-aware",
    language: Language::Java,
    kind: "record_pattern",
    variant: None,
    row_variant: None,
    class: CaseClass::Behavioral,
    src: "int f(Object o) {\n    int x = source();\n    if (!(o instanceof R(var x))) {\n        sink(x);\n    } else {\n        consume(x);\n    }\n    return x;\n}\n",
    expect: &[
        ("2:x", "4:x", FlowConfidence::Exact),
        (
            "2:x",
            "6:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
        ),
        ("2:x", "8:x", FlowConfidence::Exact),
    ],
    expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
};

fn assert_barrier_case(case: &Case) {
    let parsed = super::super::parsed(case.src, case.language);
    let function = super::super::function(&parsed);
    let defs = super::super::collect_defs(&parsed);
    let mut edges = Vec::new();
    for (from, to, _) in case.expect {
        let (def_line, def_name) = from.split_once(':').unwrap();
        let (use_line, use_name) = to.split_once(':').unwrap();
        assert_eq!(def_name, use_name, "{}: expectation path", case.id);
        let def_line: usize = def_line.parse().unwrap();
        let use_line: usize = use_line.parse().unwrap();
        let def = defs
            .iter()
            .find(|def| def.line == def_line && def.path.to_string() == def_name)
            .unwrap_or_else(|| panic!("{}: definition {from:?} missing", case.id));
        edges.push(super::super::edge(&parsed, &function, def, use_line));
    }
    let (outcome, _) = super::super::run(&parsed, &defs, &edges);
    for (edge, (_, _, expected)) in edges.iter().zip(case.expect.iter()) {
        super::super::assert_label(&outcome, edge, *expected);
    }
    if let Some((field, expected)) = case.expect_counter {
        assert_eq!(field, "dfg_label_nameonly_ownership_uncertain");
        let super::super::RdOutcome::Available(result) = outcome else {
            panic!("{}: expected measurable outcome, got {outcome:?}", case.id);
        };
        let mut stats = DfgLabelStats::default();
        for label in result.labels.values().copied() {
            stats.record_label(label);
        }
        assert_eq!(
            stats.dfg_label_nameonly_ownership_uncertain as i64, expected,
            "{}: measured {field}",
            case.id
        );
    }
}

#[test]
fn java_instanceof_pattern_uncertain() {
    assert_barrier_case(&CASES[0]);
}

#[test]
fn rust_if_let_uncertain_outside_scope() {
    assert_barrier_case(&CASES[1]);
}

#[test]
fn java_instanceof_same_line_guarded_use() {
    assert_barrier_case(&CASES[2]);
}

#[test]
fn rust_if_let_empty_span() {
    assert_barrier_case(&CASES[3]);
}

#[test]
fn unknown_scope_fail_closed() {
    assert_barrier_case(&CASES[4]);
}

#[test]
fn java_negated_instanceof_is_branch_aware() {
    assert_barrier_case(&JAVA_NEGATED_CASE);
}

fn ancestor_of_kind<'a>(mut node: tree_sitter::Node<'a>, kind: &str) -> tree_sitter::Node<'a> {
    while let Some(parent) = node.parent() {
        if parent.kind() == kind {
            return parent;
        }
        node = parent;
    }
    panic!("{} has no {kind} ancestor", node.kind());
}

#[test]
fn java_pattern_guard_spans_have_exact_disjoint_byte_ranges() {
    let positive = super::super::parsed(CASES[2].src, Language::Java);
    let binder = super::case::nodes_of_kind(&positive, "record_pattern")[0];
    let owner = ancestor_of_kind(binder, "if_statement");
    let condition = owner.child_by_field_name("condition").unwrap();
    let consequence = owner.child_by_field_name("consequence").unwrap();
    assert_eq!(
        visibility_span(&positive, binder),
        SpanKind::Known(vec![
            ByteSpan {
                start: binder.end_byte(),
                end: condition.end_byte(),
            },
            ByteSpan {
                start: consequence.start_byte(),
                end: consequence.end_byte(),
            },
        ])
    );

    let negated = super::super::parsed(JAVA_NEGATED_CASE.src, Language::Java);
    let binder = super::case::nodes_of_kind(&negated, "record_pattern")[0];
    let owner = ancestor_of_kind(binder, "if_statement");
    let alternative = owner.child_by_field_name("alternative").unwrap();
    assert_eq!(
        visibility_span(&negated, binder),
        SpanKind::Known(vec![ByteSpan {
            start: alternative.start_byte(),
            end: alternative.end_byte(),
        }])
    );
}
