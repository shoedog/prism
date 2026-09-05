use super::*;

#[test]
fn over_cap_statement_lines_returns_unavailable_for_the_lines_reason() {
    let parsed = parsed(&calls_source(RD_MAX_LINES + 1), Language::Python);
    let defs = collect_defs(&parsed);
    let (outcome, stats) = run(&parsed, &defs, &[]);
    assert_eq!(defs.len(), 0, "fixture must generate zero Defs");
    assert_eq!(
        parsed.statements_in_function(&function(&parsed)).len(),
        RD_MAX_LINES + 1
    );
    assert!(matches!(
        outcome,
        RdOutcome::Unavailable(RdUnavailable::StatementLinesCapExceeded { actual })
            if actual == RD_MAX_LINES + 1
    ));
    assert_eq!(stats.functions_over_cap, 1);
}

#[test]
fn over_cap_defs_returns_unavailable_for_the_defs_reason() {
    let parsed = parsed(&assignments_source(RD_MAX_DEFS + 1), Language::Python);
    let defs = collect_defs(&parsed);
    let stmt_count = parsed.statements_in_function(&function(&parsed)).len();
    let (outcome, stats) = run(&parsed, &defs, &[]);
    assert!(defs.len() > RD_MAX_DEFS);
    assert!(stmt_count <= RD_MAX_LINES);
    assert!(matches!(
        outcome,
        RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded { actual })
            if actual == RD_MAX_DEFS + 1
    ));
    assert_eq!(stats.functions_over_cap, 1);
}

#[test]
fn a_long_span_with_few_statements_and_few_defs_is_not_unavailable() {
    let mut source = String::from("def f():\n");
    for _ in 0..4980 {
        source.push_str("    # gap\n");
    }
    for index in 0..10 {
        source.push_str(&format!("    v{index} = 1\n    g()\n"));
    }
    let parsed = parsed(&source, Language::Python);
    let defs = collect_defs(&parsed);
    assert!(parsed.node_line_range(&function(&parsed)).1 > RD_MAX_LINES);
    assert!(parsed.statements_in_function(&function(&parsed)).len() < 30);
    assert!(defs.len() < 20);
    assert!(matches!(
        run(&parsed, &defs, &[]).0,
        RdOutcome::Available(_)
    ));
}

#[test]
fn cap_poles_remain_runnable() {
    let defs_parsed = parsed(&assignments_source(RD_MAX_DEFS), Language::Python);
    let defs = collect_defs(&defs_parsed);
    assert_eq!(defs.len(), RD_MAX_DEFS);
    assert!(matches!(
        run(&defs_parsed, &defs, &[]).0,
        RdOutcome::Available(_)
    ));

    let lines_parsed = parsed(&calls_source(RD_MAX_LINES), Language::Python);
    assert_eq!(collect_defs(&lines_parsed).len(), 0);
    assert_eq!(
        lines_parsed
            .statements_in_function(&function(&lines_parsed))
            .len(),
        RD_MAX_LINES
    );
    assert!(matches!(
        run(&lines_parsed, &[], &[]).0,
        RdOutcome::Available(_)
    ));
}

#[test]
fn zero_cfg_edge_function_returns_unavailable_for_the_no_cfg_reason() {
    let parsed = parsed("def f():\n    pass\n", Language::Python);
    let (outcome, stats) = run(&parsed, &[], &[]);
    assert_eq!(outcome, RdOutcome::Unavailable(RdUnavailable::NoCfgEdges));
    assert_eq!(stats.functions_without_cfg, 1);
    assert_eq!(stats.functions_over_cap, 0);
}

#[test]
fn definitions_cap_has_precedence_when_both_caps_are_exceeded() {
    let parsed = parsed(&calls_source(RD_MAX_LINES + 1), Language::Python);
    let defs: Vec<DefSite> = (0..=RD_MAX_DEFS)
        .map(|index| DefSite {
            id: DefId(index as u32),
            path: AccessPath::simple(format!("v{index}")),
            line: 1,
            start_byte: 0,
            alias_derived: false,
        })
        .collect();
    assert!(matches!(
        run(&parsed, &defs, &[]).0,
        RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded { actual })
            if actual == RD_MAX_DEFS + 1
    ));
}

#[test]
fn continuation_line_maps_to_its_containing_statement() {
    let parsed = parsed(
        "def f():\n    x = source()\n    use(\n        x\n    )\n    done()\n",
        Language::Python,
    );
    let func = function(&parsed);
    let source = def(&parsed, 0, "x", 2, 0, false);
    let continuation = edge(&parsed, &func, &source, 4);
    let outcome = run(&parsed, &[source], &[continuation.clone()]).0;
    assert_label(&outcome, &continuation, FlowConfidence::Exact);
}

#[test]
fn endpoint_without_a_containing_statement_is_cfg_incomplete() {
    let parsed = parsed(
        "def f():\n    x = source()\n    # x is not an executable read\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let source = def(&parsed, 0, "x", 2, 0, false);
    let unmapped = edge(&parsed, &func, &source, 3);
    let outcome = run(&parsed, &[source], &[unmapped.clone()]).0;
    assert_label(
        &outcome,
        &unmapped,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn rust_inner_statements_win_over_the_containing_if_span() {
    let parsed = parsed(
        "fn f(c: bool) {\n    if c {\n        let x = 1;\n        use_it(x);\n    }\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let source = def(&parsed, 0, "x", 3, 0, false);
    let use_edge = edge(&parsed, &func, &source, 4);
    let outcome = run(&parsed, &[source], &[use_edge.clone()]).0;
    assert_label(&outcome, &use_edge, FlowConfidence::Exact);
}

#[test]
fn labels_have_exactly_the_supplied_edge_keys() {
    let parsed = parsed("def f():\n    x = source()\n    use(x)\n", Language::Python);
    let func = function(&parsed);
    let source = def(&parsed, 0, "x", 2, 0, false);
    let supplied = edge(&parsed, &func, &source, 3);
    let outcome = run(&parsed, &[source], &[supplied.clone()]).0;
    let RdOutcome::Available(result) = outcome else {
        panic!("expected Available, got {outcome:?}");
    };
    assert_eq!(result.labels.len(), 1);
    assert_eq!(
        result.labels.keys().next(),
        Some(&(supplied.from, supplied.to))
    );
}

#[test]
fn rd_file_stats_round_trip_through_serde() {
    let mut expected = RdFileStats::default();
    expected.record_over_cap("cap_a".into(), 1);
    expected.record_over_cap("cap_b".into(), 4);
    expected.record_without_cfg("cfg_a".into(), 8);
    expected.record_without_cfg("cfg_b".into(), 11);
    expected.record_without_cfg("cfg_c".into(), 14);
    assert_eq!(expected.functions_over_cap, 2);
    assert_eq!(expected.functions_without_cfg, 3);
    let bytes = bincode::serialize(&expected).unwrap();
    let mut restored = bincode::deserialize::<RdFileStats>(&bytes).unwrap();
    assert_eq!(restored, expected);

    restored.record_without_cfg("cfg_a".into(), 8);
    assert_eq!(restored.functions_without_cfg, 3);
    restored.record_without_cfg("cfg_d".into(), 17);
    assert_eq!(restored.functions_without_cfg, 4);
}
