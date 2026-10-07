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
    let outcome = run(&parsed, &[alias, direct], std::slice::from_ref(&alias_edge)).0;
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
    let outcome = run(
        &parsed,
        &[original, branch],
        std::slice::from_ref(&original_edge),
    )
    .0;
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
    let outcome = run(&parsed, &[later], std::slice::from_ref(&loop_edge)).0;
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
        std::slice::from_ref(&original_edge),
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
    let outcome = run(
        &parsed,
        &[original, unreachable],
        std::slice::from_ref(&original_edge),
    )
    .0;
    assert_label(
        &outcome,
        &original_edge,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

mod captures;
mod cfg_joins;
mod limits;
mod review_regressions;
