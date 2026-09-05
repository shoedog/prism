use super::*;
use crate::cpg::FlowDoubt;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;

fn parsed(source: &str, language: Language) -> ParsedFile {
    let extension = match language {
        Language::Python => "py",
        Language::JavaScript => "js",
        Language::TypeScript => "ts",
        Language::Go => "go",
        Language::Rust => "rs",
        _ => "txt",
    };
    ParsedFile::parse(&format!("test.{extension}"), source, language).unwrap()
}

fn function(parsed: &ParsedFile) -> Node<'_> {
    parsed
        .all_functions()
        .into_iter()
        .min_by_key(|node| node.start_byte())
        .expect("fixture must contain a function")
}

fn byte_on_line(parsed: &ParsedFile, line: usize, needle: &str, occurrence: usize) -> usize {
    let start = parsed.line_start_byte(line);
    let end = parsed
        .source
        .as_bytes()
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, byte)| (*byte == b'\n').then_some(index))
        .unwrap_or(parsed.source.len());
    start
        + parsed.source[start..end]
            .match_indices(needle)
            .nth(occurrence)
            .unwrap_or_else(|| panic!("{needle:?} occurrence {occurrence} missing on line {line}"))
            .0
}

fn def(
    parsed: &ParsedFile,
    id: u32,
    path: &str,
    line: usize,
    occurrence: usize,
    alias_derived: bool,
) -> DefSite {
    DefSite {
        id: DefId(id),
        path: AccessPath::from_expr(path),
        line,
        start_byte: byte_on_line(parsed, line, path.rsplit('.').next().unwrap(), occurrence),
        alias_derived,
    }
}

fn location(
    parsed: &ParsedFile,
    func: &Node<'_>,
    path: &str,
    line: usize,
    start_byte: usize,
    end_byte: usize,
    kind: VarAccessKind,
) -> VarLocation {
    VarLocation {
        file: parsed.path.clone(),
        function: "f".to_string(),
        function_start_line: parsed.node_line_range(func).0,
        line,
        path: AccessPath::from_expr(path),
        start_byte,
        end_byte,
        kind,
    }
}

fn edge(parsed: &ParsedFile, func: &Node<'_>, def: &DefSite, use_line: usize) -> FlowEdge {
    let name = def.path.base.as_str();
    let use_path = def.path.clone();
    let use_lines = BTreeSet::from([use_line]);
    let use_span = parsed
        .rvalue_identifier_spans_on_lines(func, &use_lines)
        .into_iter()
        .find(|span| span.line == use_line && span.path == use_path);
    let (use_start, use_end) = use_span
        .map(|span| (span.start_byte, span.end_byte))
        .unwrap_or_else(|| {
            let anchor = parsed.line_start_byte(use_line);
            (anchor, anchor)
        });
    FlowEdge {
        from: location(
            parsed,
            func,
            &def.path.to_string(),
            def.line,
            def.start_byte,
            def.start_byte + name.len(),
            VarAccessKind::Def,
        ),
        to: location(
            parsed,
            func,
            &def.path.to_string(),
            use_line,
            use_start,
            use_end,
            VarAccessKind::Use,
        ),
    }
}

fn run(parsed: &ParsedFile, defs: &[DefSite], edges: &[FlowEdge]) -> (RdOutcome, RdFileStats) {
    let func = function(parsed);
    let function_name = parsed
        .language
        .function_name(&func)
        .map(|name| parsed.node_text(&name).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string());
    let (function_start_line, _) = parsed.node_line_range(&func);
    let outcome = reaching_definitions(parsed, &func, defs, edges);
    let mut stats = RdFileStats::default();
    match outcome {
        RdOutcome::Unavailable(reason) if reason.is_def_cap() || reason.is_line_cap() => {
            stats.record_over_cap(function_name, function_start_line);
        }
        RdOutcome::Unavailable(RdUnavailable::NoCfgEdges) => {
            stats.record_without_cfg(function_name, function_start_line);
        }
        _ => {}
    }
    (outcome, stats)
}

fn assert_label(outcome: &RdOutcome, edge: &FlowEdge, expected: FlowConfidence) {
    let RdOutcome::Available(result) = outcome else {
        panic!("expected Available with {expected:?}, got {outcome:?}");
    };
    assert_eq!(
        result.labels.get(&(edge.from.clone(), edge.to.clone())),
        Some(&expected)
    );
}

fn collect_defs(parsed: &ParsedFile) -> Vec<DefSite> {
    let func = function(parsed);
    let (start, end) = parsed.node_line_range(&func);
    let lines = (start..=end).collect();
    parsed
        .assignment_lvalue_spans_on_lines(&func, &lines)
        .into_iter()
        .enumerate()
        .map(|(index, span)| DefSite {
            id: DefId(index as u32),
            path: span.path,
            line: span.line,
            start_byte: span.start_byte,
            alias_derived: false,
        })
        .collect()
}

fn calls_source(count: usize) -> String {
    let mut source = String::from("def f():\n");
    for _ in 0..count {
        source.push_str("    g()\n");
    }
    source
}

fn assignments_source(count: usize) -> String {
    let mut source = String::from("def f():\n");
    let mut next = 0;
    while next < count {
        source.push_str("    ");
        for slot in 0..3 {
            if next >= count {
                break;
            }
            if slot > 0 {
                source.push_str("; ");
            }
            source.push_str(&format!("v{next} = 1"));
            next += 1;
        }
        source.push('\n');
    }
    source
}

#[test]
fn same_path_redefinition_kills_the_earlier_def() {
    let parsed = parsed(
        "def f():\n    x = source()\n    x = clean()\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let a = def(&parsed, 0, "x", 2, 0, false);
    let b = def(&parsed, 1, "x", 3, 0, false);
    let a_edge = edge(&parsed, &func, &a, 4);
    let b_edge = edge(&parsed, &func, &b, 4);
    let outcome = run(&parsed, &[a, b], &[a_edge.clone(), b_edge.clone()]).0;
    assert_label(
        &outcome,
        &a_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
    assert_label(&outcome, &b_edge, FlowConfidence::Exact);
}

#[test]
fn nested_scope_declaration_does_not_kill_outer_after_scope_exit() {
    let cases = [
        (
            Language::Rust,
            "fn f() {\n    let x = source();\n    {\n        let x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
            2,
            4,
            7,
        ),
        (
            Language::Go,
            "package main\nfunc f() {\n\tx := source()\n\t{\n\t\tx := clean()\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
            3,
            5,
            8,
        ),
        (
            Language::JavaScript,
            "function f() {\n  let x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
            2,
            4,
            7,
        ),
        (
            Language::TypeScript,
            "function f() {\n  let x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
            2,
            4,
            7,
        ),
    ];
    for (language, source, outer_line, inner_line, use_line) in cases {
        let parsed = parsed(source, language);
        let func = function(&parsed);
        let outer = def(&parsed, 0, "x", outer_line, 0, false);
        let inner = def(&parsed, 1, "x", inner_line, 0, false);
        let outer_edge = edge(&parsed, &func, &outer, use_line);
        let outcome = run(&parsed, &[outer, inner], std::slice::from_ref(&outer_edge)).0;
        assert_label(&outcome, &outer_edge, FlowConfidence::Exact);
    }
}

#[test]
fn outer_assignment_after_nested_scope_kills_at_outer_line() {
    let cases = [
        (
            Language::Rust,
            "fn f() {\n    let mut x = source();\n    {\n        let x = clean();\n        sink(x);\n    }\n    x = clean();\n    sink(x);\n}\n",
            2,
            4,
            7,
            8,
        ),
        (
            Language::Go,
            "package main\nfunc f() {\n\tx := source()\n\t{\n\t\tx := clean()\n\t\tsink(x)\n\t}\n\tx = clean()\n\tsink(x)\n}\n",
            3,
            5,
            8,
            9,
        ),
        (
            Language::JavaScript,
            "function f() {\n  let x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  x = clean();\n  sink(x);\n}\n",
            2,
            4,
            7,
            8,
        ),
        (
            Language::TypeScript,
            "function f() {\n  let x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  x = clean();\n  sink(x);\n}\n",
            2,
            4,
            7,
            8,
        ),
    ];
    for (language, source, outer_line, inner_line, assignment_line, use_line) in cases {
        let parsed = parsed(source, language);
        let func = function(&parsed);
        let outer = def(&parsed, 0, "x", outer_line, 0, false);
        let inner = def(&parsed, 1, "x", inner_line, 0, false);
        let assignment = def(&parsed, 2, "x", assignment_line, 0, false);
        let outer_edge = edge(&parsed, &func, &outer, use_line);
        let outcome = run(
            &parsed,
            &[outer, inner, assignment],
            std::slice::from_ref(&outer_edge),
        )
        .0;
        assert_label(
            &outcome,
            &outer_edge,
            FlowConfidence::NameOnly(FlowDoubt::Killed {
                kill_line: assignment_line as u32,
            }),
        );
    }
}

#[test]
fn alias_derived_defs_never_kill_in_v1() {
    let parsed = parsed(
        "def f():\n    p = q\n    p.x = 1\n    q.x = 2\n    use(q.x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let alias = def(&parsed, 0, "q.x", 3, 0, true);
    let direct = def(&parsed, 1, "q.x", 4, 0, false);
    let alias_edge = edge(&parsed, &func, &alias, 5);
    let outcome = run(&parsed, &[alias, direct], &[alias_edge.clone()]).0;
    assert_label(
        &outcome,
        &alias_edge,
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
    );
}

#[test]
fn same_line_defs_never_kill_one_another_by_byte_order() {
    let parsed = parsed("def f():\n    a = 1; a = 2\n    use(a)\n", Language::Python);
    let func = function(&parsed);
    let first = def(&parsed, 0, "a", 2, 0, false);
    let second = def(&parsed, 1, "a", 2, 1, false);
    let first_edge = edge(&parsed, &func, &first, 3);
    let second_edge = edge(&parsed, &func, &second, 3);
    let outcome = run(
        &parsed,
        &[first, second],
        &[first_edge.clone(), second_edge.clone()],
    )
    .0;
    for candidate in [&first_edge, &second_edge] {
        assert_label(
            &outcome,
            candidate,
            FlowConfidence::NameOnly(FlowDoubt::SameLine),
        );
    }
}

#[test]
fn duplicate_definition_observation_is_not_a_same_line_collision() {
    let parsed = parsed(
        "package main\nfunc f() {\n\tx := source()\n\tsink(x)\n\tx = clean()\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let source = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("x") && def.line == 3)
        .expect("short declaration must produce a definition")
        .clone();
    let source_edge = edge(&parsed, &func, &source, 4);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&source_edge)).0;
    assert_label(&outcome, &source_edge, FlowConfidence::Exact);
}

#[test]
fn diamond_kill_on_one_branch_still_reaches() {
    let parsed = parsed(
        "def f(c):\n    x = source()\n    if c:\n        x = clean()\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let branch = def(&parsed, 1, "x", 4, 0, false);
    let original_edge = edge(&parsed, &func, &original, 5);
    let outcome = run(&parsed, &[original, branch], &[original_edge.clone()]).0;
    assert_label(&outcome, &original_edge, FlowConfidence::Exact);
}

#[test]
fn loop_back_edge_makes_a_textually_earlier_use_reachable() {
    let parsed = parsed(
        "def f(c):\n    while c:\n        use(x)\n        x = make()\n    done()\n",
        Language::Python,
    );
    let func = function(&parsed);
    let later = def(&parsed, 0, "x", 4, 0, false);
    let loop_edge = edge(&parsed, &func, &later, 3);
    let outcome = run(&parsed, &[later], &[loop_edge.clone()]).0;
    assert_label(&outcome, &loop_edge, FlowConfidence::Exact);
    let RdOutcome::Available(result) = outcome else {
        unreachable!()
    };
    assert!(result
        .loop_carried_edges
        .contains(&(loop_edge.from, loop_edge.to)));
}

#[test]
fn kill_line_is_the_lowest_reachable_killing_line() {
    let parsed = parsed(
        "def f(c):\n    x = source()\n    if c:\n        x = a()\n    else:\n        x = b()\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let first_kill = def(&parsed, 1, "x", 4, 0, false);
    let second_kill = def(&parsed, 2, "x", 6, 0, false);
    let original_edge = edge(&parsed, &func, &original, 7);
    let outcome = run(
        &parsed,
        &[original, first_kill, second_kill],
        &[original_edge.clone()],
    )
    .0;
    assert_label(
        &outcome,
        &original_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
}

#[test]
fn no_reaching_candidate_without_a_proven_kill_is_cfg_incomplete() {
    let parsed = parsed(
        "def f():\n    x = source()\n    return\n    x = clean()\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let unreachable = def(&parsed, 1, "x", 4, 0, false);
    let original_edge = edge(&parsed, &func, &original, 5);
    let outcome = run(&parsed, &[original, unreachable], &[original_edge.clone()]).0;
    assert_label(
        &outcome,
        &original_edge,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

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

mod captures;
mod cfg_joins;
