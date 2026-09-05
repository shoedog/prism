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
    kind: VarAccessKind,
) -> VarLocation {
    VarLocation {
        file: parsed.path.clone(),
        function: "f".to_string(),
        function_start_line: parsed.node_line_range(func).0,
        line,
        path: AccessPath::from_expr(path),
        start_byte,
        end_byte: start_byte + path.rsplit('.').next().unwrap().len(),
        kind,
    }
}

fn edge(parsed: &ParsedFile, func: &Node<'_>, def: &DefSite, use_line: usize) -> FlowEdge {
    let name = def.path.base.as_str();
    FlowEdge {
        from: location(
            parsed,
            func,
            &def.path.to_string(),
            def.line,
            def.start_byte,
            VarAccessKind::Def,
        ),
        to: location(
            parsed,
            func,
            &def.path.to_string(),
            use_line,
            byte_on_line(parsed, use_line, name, 0),
            VarAccessKind::Use,
        ),
    }
}

fn run(parsed: &ParsedFile, defs: &[DefSite], edges: &[FlowEdge]) -> (RdOutcome, RdFileStats) {
    let func = function(parsed);
    let outcome = reaching_definitions(parsed, &func, defs, edges);
    let mut stats = RdFileStats::default();
    match outcome {
        RdOutcome::Unavailable(reason) if reason.is_def_cap() || reason.is_line_cap() => {
            stats.functions_over_cap = 1;
        }
        RdOutcome::Unavailable(RdUnavailable::NoCfgEdges) => stats.functions_without_cfg = 1,
        _ => {}
    }
    (outcome, stats)
}

fn assert_label(outcome: &RdOutcome, edge: &FlowEdge, expected: FlowConfidence) {
    let RdOutcome::Available(result) = outcome else {
        panic!("expected Available with {expected:?}, got {outcome:?}");
    };
    assert_eq!(result.labels.get(&(edge.from.clone(), edge.to.clone())), Some(&expected));
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
    let parsed = parsed("def f():\n    x = source()\n    x = clean()\n    use(x)\n", Language::Python);
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
    let RdOutcome::Available(result) = outcome else { unreachable!() };
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
fn an_unreachable_kill_does_not_supply_the_payload() {
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
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 2 }),
    );
}

#[test]
fn over_cap_statement_lines_returns_unavailable_for_the_lines_reason() {
    let parsed = parsed(&calls_source(RD_MAX_LINES + 1), Language::Python);
    let defs = collect_defs(&parsed);
    let (outcome, stats) = run(&parsed, &defs, &[]);
    assert_eq!(defs.len(), 0, "fixture must generate zero Defs");
    assert_eq!(function(&parsed).start_position().row + RD_MAX_LINES + 1, RD_MAX_LINES + 1);
    assert!(matches!(outcome, RdOutcome::Unavailable(reason) if reason.is_line_cap()));
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
    assert!(matches!(outcome, RdOutcome::Unavailable(reason) if reason.is_def_cap()));
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
    assert!(matches!(run(&parsed, &defs, &[]).0, RdOutcome::Available(_)));
}

#[test]
fn cap_poles_remain_runnable() {
    let defs_parsed = parsed(&assignments_source(RD_MAX_DEFS), Language::Python);
    let defs = collect_defs(&defs_parsed);
    assert_eq!(defs.len(), RD_MAX_DEFS);
    assert!(matches!(run(&defs_parsed, &defs, &[]).0, RdOutcome::Available(_)));

    let lines_parsed = parsed(&calls_source(RD_MAX_LINES), Language::Python);
    assert_eq!(collect_defs(&lines_parsed).len(), 0);
    assert_eq!(
        lines_parsed
            .statements_in_function(&function(&lines_parsed))
            .len(),
        RD_MAX_LINES
    );
    assert!(matches!(run(&lines_parsed, &[], &[]).0, RdOutcome::Available(_)));
}

#[test]
fn zero_cfg_edge_function_returns_unavailable_for_the_no_cfg_reason() {
    let parsed = parsed("def f():\n    pass\n", Language::Python);
    let (outcome, stats) = run(&parsed, &[], &[]);
    assert_eq!(outcome, RdOutcome::Unavailable(RdUnavailable::NoCfgEdges));
    assert_eq!(stats.functions_without_cfg, 1);
    assert_eq!(stats.functions_over_cap, 0);
}

fn assert_capture(source: &str, language: Language, def_line: usize, use_line: usize) {
    let parsed = parsed(source, language);
    let func = function(&parsed);
    let outer = def(&parsed, 0, "x", def_line, 0, false);
    let capture = edge(&parsed, &func, &outer, use_line);
    let outcome = run(&parsed, &[outer], &[capture.clone()]).0;
    assert_label(
        &outcome,
        &capture,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn python_lambda_capture_timing_is_unknown() {
    assert_capture(
        "def f():\n    x = 1\n    delayed = lambda: use(x)\n    x = 2\n    return delayed\n",
        Language::Python,
        2,
        3,
    );
}

#[test]
fn python_nested_def_capture_timing_is_unknown() {
    assert_capture(
        "def f():\n    x = 1\n    def inner():\n        use(x)\n    x = 2\n    return inner\n",
        Language::Python,
        2,
        4,
    );
}

#[test]
fn go_defer_function_literal_capture_timing_is_unknown() {
    assert_capture(
        "package p\nfunc f() {\n    x := 1\n    defer func() { use(x) }()\n    x = 2\n}\n",
        Language::Go,
        3,
        4,
    );
}

#[test]
fn go_statement_function_literal_capture_timing_is_unknown() {
    assert_capture(
        "package p\nfunc f() {\n    x := 1\n    go func() { use(x) }()\n    x = 2\n}\n",
        Language::Go,
        3,
        4,
    );
}

#[test]
fn javascript_arrow_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x = 1;\n  const delayed = () => use(x);\n  x = 2;\n  return delayed;\n}\n",
        Language::JavaScript,
        2,
        3,
    );
}

#[test]
fn javascript_function_expression_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x = 1;\n  const delayed = function () { use(x); };\n  x = 2;\n  return delayed;\n}\n",
        Language::JavaScript,
        2,
        3,
    );
}

#[test]
fn typescript_arrow_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x: number = 1;\n  const delayed = () => use(x);\n  x = 2;\n  return delayed;\n}\n",
        Language::TypeScript,
        2,
        3,
    );
}

#[test]
fn typescript_function_expression_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x: number = 1;\n  const delayed = function () { use(x); };\n  x = 2;\n  return delayed;\n}\n",
        Language::TypeScript,
        2,
        3,
    );
}

#[test]
fn rust_closure_capture_timing_is_unknown() {
    assert_capture(
        "fn f() {\n    let mut x = 1;\n    let delayed = || use_it(x);\n    x = 2;\n    drop(delayed);\n}\n",
        Language::Rust,
        2,
        3,
    );
}

#[test]
fn go_defer_argument_is_evaluated_now() {
    let parsed = parsed(
        "package p\nfunc f() {\n    x := 1\n    defer use(x)\n    x = 2\n    return\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 3, 0, false);
    let later = def(&parsed, 1, "x", 5, 0, false);
    let immediate = edge(&parsed, &func, &original, 4);
    let outcome = run(&parsed, &[original, later], &[immediate.clone()]).0;
    assert_label(&outcome, &immediate, FlowConfidence::Exact);
}

#[test]
fn try_header_exception_join_is_cfg_incomplete() {
    let parsed = parsed(
        "def f():\n    x = source()\n    try:\n        x = clean()\n        raise ValueError()\n    except ValueError:\n        use(x)\n    done()\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let edge = edge(&parsed, &func, &original, 7);
    let outcome = run(&parsed, &[original], &[edge.clone()]).0;
    assert_label(
        &outcome,
        &edge,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn go_return_to_defer_join_is_cfg_incomplete() {
    let parsed = parsed(
        "package p\nfunc f() {\n    x := 1\n    defer use(x)\n    x = 2\n    return\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let later = def(&parsed, 0, "x", 5, 0, false);
    let deferred = edge(&parsed, &func, &later, 4);
    let outcome = run(&parsed, &[later], &[deferred.clone()]).0;
    assert_label(
        &outcome,
        &deferred,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn branch_arm_fallthrough_join_is_cfg_incomplete() {
    let parsed = parsed(
        "def f(c):\n    if c:\n        x = clean()\n    else:\n        use(x)\n    done()\n",
        Language::Python,
    );
    let func = function(&parsed);
    let branch = def(&parsed, 0, "x", 3, 0, false);
    let cross_arm = edge(&parsed, &func, &branch, 5);
    let outcome = run(&parsed, &[branch], &[cross_arm.clone()]).0;
    assert_label(
        &outcome,
        &cross_arm,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}
