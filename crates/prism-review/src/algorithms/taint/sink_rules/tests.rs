use super::{call_operand, collect_rules, InlineOccurrence, NameSinks, Occurrence, Witness};
use prism::access_path::AccessPath;
use prism::ast::tree_helpers::{call_arg_node, collect_calls};
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use prism::frameworks::{SanitizerCategory, SinkPattern};
use prism::languages::Language;
use prism::rule_registry::{CONFIGURED_SINK, FORMAT_WRAPPER, JS_EVAL, TAINT_WRITE_DATA};
use prism::slice::{attach_rules, SliceFinding};
use std::collections::BTreeSet;

fn path_to(location: VarLocation) -> FlowPath {
    let mut from = location.clone();
    from.kind = VarAccessKind::Def;
    from.line = 1;
    FlowPath {
        edges: vec![FlowEdge { from, to: location }],
        cleansed_for: BTreeSet::new(),
    }
}

fn use_at(parsed: &ParsedFile, before: &str, name: &str, line: usize) -> VarLocation {
    let start = parsed.source.find(before).unwrap() + before.len();
    assert_eq!(&parsed.source[start..start + name.len()], name);
    VarLocation {
        file: parsed.path.clone(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple(name),
        start_byte: start,
        end_byte: start + name.len(),
        kind: VarAccessKind::Use,
    }
}

#[test]
fn combines_paths_and_inline_on_final_admitted_key() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { db.query('select ?', inner(x), y); db.query('select ?', {bind: x}); inline(source()); }",
        Language::JavaScript,
    )
    .unwrap();
    let x = defined_path(&parsed, "x,", "inner(", "x");
    let mut y_cleansed = defined_path(&parsed, "y)", "inner(x), ", "y");
    y_cleansed.cleansed_for.insert(SanitizerCategory::Sqli);
    let y_live = defined_path(&parsed, "y)", "inner(x), ", "y");
    let bound = defined_path(&parsed, "x,", "bind: ", "x");
    let mut calls = Vec::new();
    collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
    let inline_call = calls
        .into_iter()
        .find(|call| parsed.node_text(call) == "inline(source())")
        .unwrap();
    let source = call_arg_node(&inline_call, 0).unwrap();
    let inline = InlineOccurrence {
        admission_key: (&parsed.path, 1),
        occurrence: Occurrence {
            call: inline_call,
            callee: parsed.call_function_name(&inline_call).unwrap(),
            operand: source,
            slot: 0,
        },
        source,
    };
    let paths = [&x, &x, &y_cleansed, &y_live, &bound];
    static QUERY: SinkPattern = SinkPattern {
        call_path: "query",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[1, 2],
        semantic_check: None,
    };
    let cpg = CodePropertyGraph::empty();
    let mut visited = Vec::new();
    let mut project = |occ: Occurrence<'_>, witness: Witness<'_, '_>, rows: &mut Vec<_>| {
        let name = parsed.node_text(&occ.callee);
        match witness {
            Witness::Path { path, reached } => {
                assert!(!reached.byte_range().is_empty());
                visited.push((name, occ.slot, parsed.node_text(&occ.operand)));
                if name == "inner" {
                    let callee = (occ.callee.start_byte(), occ.callee.end_byte());
                    let operand = (occ.operand.start_byte(), occ.operand.end_byte());
                    assert!(!occ.passes_flat_safe_ranges(&[callee]));
                    assert!(occ.passes_flat_safe_ranges(&[operand]));
                    rows.push((&JS_EVAL, true));
                }
                if occ.matches_path_pattern(&parsed, &cpg, 1, path, &QUERY) {
                    assert_ne!(parsed.node_text(&occ.operand), "{bind: x}");
                    assert!(!std::ptr::eq(path, &y_cleansed));
                    rows.push((&TAINT_WRITE_DATA, false));
                }
            }
            Witness::Inline(source) => {
                assert_eq!(parsed.node_text(&source), "source()");
                rows.push((&JS_EVAL, false));
            }
        }
    };
    assert!(collect_rules(&parsed, 1, &BTreeSet::new(), paths, [inline], &mut project).is_empty());
    let admitted = BTreeSet::from([(parsed.path.clone(), 1)]);
    let rejected_inline = InlineOccurrence {
        source: inline.occurrence.callee,
        ..inline
    };
    let rows = collect_rules(
        &parsed,
        1,
        &admitted,
        paths,
        [inline, rejected_inline],
        &mut project,
    );
    assert_eq!(rows.len(), 6);
    assert_eq!(
        visited,
        [
            ("inner", 0, "x"),
            ("query", 1, "inner(x)"),
            ("inner", 0, "x"),
            ("query", 1, "inner(x)"),
            ("query", 2, "y"),
            ("query", 2, "y"),
            ("query", 1, "{bind: x}"),
        ]
    );
    let mut finding = SliceFinding {
        severity: "warning".into(),
        ..SliceFinding::default()
    };
    attach_rules(&mut finding, rows);
    assert_eq!(
        finding.rules,
        [JS_EVAL.rule_ref(), TAINT_WRITE_DATA.rule_ref()]
    );
}

#[test]
fn coalesced_reads_do_not_cross_a_binding_or_write() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { one(x); two(x, x); { let x = 'clean'; shadow(x); } three(y); y = 'clean'; rebound(y); }",
        Language::JavaScript,
    )
    .unwrap();
    let x = defined_path(&parsed, "x,", "one(", "x");
    let y = defined_path(&parsed, "y)", "three(", "y");
    let mut slots = Vec::new();
    let rows = collect_rules(
        &parsed,
        1,
        &BTreeSet::from([(parsed.path.clone(), 1)]),
        [&x, &y],
        [],
        |occ, _, _| slots.push((parsed.node_text(&occ.callee), occ.slot)),
    );
    assert!(rows.is_empty());
    assert_eq!(slots, [("one", 0), ("two", 0), ("two", 1), ("three", 0)]);
}

#[test]
fn only_located_arguments_can_contribute_name_detector_rows() {
    let parsed = ParsedFile::parse(
        "a.py",
        "def f(x):\n    x.report('ok')\n    report_extra(x); report('%s', x)\n",
        Language::Python,
    )
    .unwrap();
    let path = defined_path(&parsed, "x)", "    ", "x");
    let mut other_file = use_at(&parsed, "report_extra(", "x", 3);
    other_file.file = "other.py".into();
    let mut spanless = use_at(&parsed, "'%s', ", "x", 3);
    spanless.start_byte = 0;
    spanless.end_byte = 0;
    let other_file = path_to(other_file);
    let spanless = path_to(spanless);
    let configured = ["report".into()];
    let wrappers = ["=report".into()];
    let names = NameSinks::new(&configured, &wrappers);
    let admitted = BTreeSet::from([(parsed.path.clone(), 2), (parsed.path.clone(), 3)]);
    let mut project = |occ: Occurrence<'_>, _, rows: &mut Vec<_>| {
        names.select(parsed.node_text(&occ.callee), rows)
    };
    assert!(collect_rules(
        &parsed,
        2,
        &admitted,
        [&path, &other_file, &spanless],
        [],
        &mut project,
    )
    .is_empty());
    assert!(collect_rules(
        &parsed,
        3,
        &admitted,
        [&other_file, &spanless],
        [],
        &mut project
    )
    .is_empty());

    let path = defined_path(&parsed, "x)", "report_extra(", "x");
    let rows = collect_rules(&parsed, 3, &admitted, [&path], [], &mut project);
    assert_eq!(
        rows.iter().map(|(row, _)| row.id()).collect::<Vec<_>>(),
        [
            CONFIGURED_SINK.id(),
            CONFIGURED_SINK.id(),
            FORMAT_WRAPPER.id()
        ]
    );
    let exact_pattern = ["=report".into()];
    let exact = NameSinks::new(&exact_pattern, &[]);
    let rows = collect_rules(&parsed, 3, &admitted, [&path], [], |occ, _, rows| {
        exact.select(parsed.node_text(&occ.callee), rows)
    });
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.id(), CONFIGURED_SINK.id());
    assert!(!rows[0].1);
}

fn defined_path(parsed: &ParsedFile, definition: &str, before: &str, name: &str) -> FlowPath {
    let line = |marker| {
        let byte = parsed.source.find(marker).unwrap();
        parsed.source[..byte]
            .bytes()
            .filter(|&b| b == b'\n')
            .count()
            + 1
    };
    let mut path = path_to(use_at(parsed, before, name, line(before)));
    let from = &mut path.edges[0].from;
    from.line = line(definition);
    from.start_byte = parsed.source.find(definition).unwrap();
    from.end_byte = from.start_byte + name.len();
    path
}

fn visits<'a>(parsed: &'a ParsedFile, path: &FlowPath) -> Vec<(&'a str, usize)> {
    let line = path.edges[0].to.line;
    let mut visits = Vec::new();
    collect_rules(
        parsed,
        line,
        &BTreeSet::from([(parsed.path.clone(), line)]),
        [path],
        [],
        |occ, _, _| visits.push((parsed.node_text(&occ.callee), occ.slot)),
    );
    visits
}

#[test]
fn refuses_shadowed_initial_span() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x) {\n  { const x = 'safe'; res.send(x); } observe(x);\n  confirm(x);\n}",
        Language::JavaScript,
    )
    .unwrap();
    assert!(visits(&parsed, &defined_path(&parsed, "x)", "res.send(", "x")).is_empty());
    assert_eq!(
        visits(&parsed, &defined_path(&parsed, "x)", "confirm(", "x")),
        [("confirm", 0)]
    );
}

#[test]
fn every_destructuring_write_blocks_coalescing_in_both_directions() {
    let mut failures = Vec::new();
    for write in [
        "({x} = o);",
        "({x = d} = o);",
        "var {x} = o;",
        "for ({x} of xs);",
        "for (var {x} of xs);",
    ] {
        let source = format!("function f(x) {{\n  observe(x); {write} res.send(x);\n}}");
        let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{write}");
        let forward = visits(&parsed, &defined_path(&parsed, "x)", "observe(", "x"));
        let backward = visits(&parsed, &defined_path(&parsed, "x)", "res.send(", "x"));
        if forward != [("observe", 0)] || !backward.is_empty() {
            failures.push(format!("{write}: {forward:?}, {backward:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn captured_write_blocks_reuse_and_cannot_validate_a_late_representative() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f() {\n  let x = source();\n  const reset = () => { x = 'safe'; };\n  observe(x); reset(); res.send(x);\n}",
        Language::JavaScript,
    )
    .unwrap();
    assert!(visits(
        &parsed,
        &defined_path(&parsed, "x = source", "res.send(", "x")
    )
    .is_empty());
    assert!(!visits(
        &parsed,
        &defined_path(&parsed, "x = source", "observe(", "x")
    )
    .contains(&("send", 0)));
}

#[test]
fn immutable_value_reaches_nested_arguments() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f() {\n  const x = source();\n  observe(x); inspect('x'); /* x */ res.send(eval(x));\n}",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(
        visits(
            &parsed,
            &defined_path(&parsed, "x = source", "observe(", "x")
        ),
        [("observe", 0), ("eval", 0), ("send", 0)]
    );
}

#[test]
fn ri15_assignment_wrapped_calls_share_the_reached_value() {
    let parsed = ParsedFile::parse(
        "a.go",
        r#"package p
func f(name string) {
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/safe/") {
        return
    }
    _ = os.WriteFile(cleaned, []byte("data"), 0644); _ = fmt.Sprintf(cleaned)
}
"#,
        Language::Go,
    )
    .unwrap();
    assert_eq!(
        visits(
            &parsed,
            &defined_path(&parsed, "cleaned :=", "WriteFile(", "cleaned")
        ),
        [("WriteFile", 0), ("Sprintf", 0)]
    );
}

#[test]
fn exact_asserted_members_use_the_core_path() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for expr in [
            "runtime.X",
            "(runtime as any).X",
            "(runtime satisfies any).X",
            "((runtime as any) satisfies any).X",
            "(/* before */ runtime /* value */ as any).X",
        ] {
            let source = format!("function f(runtime: any) {{\n  fetch({expr});\n}}");
            let parsed = ParsedFile::parse("a.ts", &source, language).unwrap();
            assert!(!parsed.tree.root_node().has_error(), "{language:?}: {expr}");
            let mut path = defined_path(&parsed, "runtime:", "fetch(", expr);
            let field = AccessPath::from_expr("runtime.X");
            assert!(parsed
                .rvalue_identifier_spans_on_lines(&parsed.all_functions()[0], &BTreeSet::from([2]))
                .iter()
                .any(|span| span.start_byte == path.edges[0].to.start_byte
                    && span.end_byte == path.edges[0].to.end_byte
                    && span.path == field));
            path.edges[0].from.path = AccessPath::simple("runtime");
            path.edges[0].from.end_byte = path.edges[0].from.start_byte + "runtime".len();
            path.edges[0].to.path = field;
            let actual = visits(&parsed, &path);
            if actual != [("fetch", 0)] {
                failures.push((language, expr));
            }
            path.edges[0].to.path = AccessPath::from_expr("runtime.Y");
            assert!(visits(&parsed, &path).is_empty(), "{expr}");
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn semantic_slot_ignores_comment_trivia() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x) {\n  el.insertAdjacentHTML(/* position */ x, 'safe'); el.insertAdjacentHTML('beforeend', /* html */ x);\n}",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(
        visits(
            &parsed,
            &defined_path(&parsed, "x)", "/* position */ ", "x")
        ),
        [("insertAdjacentHTML", 0), ("insertAdjacentHTML", 1)]
    );
}

#[test]
fn name_selection_uses_only_a_verified_callee_token() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, table, audit) {\n  table[audit](x); audit(x);\n  const fn = new Function('return ' + x);\n}",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(
        visits(&parsed, &defined_path(&parsed, "x,", "table[audit](", "x")),
        [("audit", 0)]
    );
    assert_eq!(
        visits(&parsed, &defined_path(&parsed, "x,", "'return ' + ", "x")),
        [("Function", 0)]
    );
}

#[test]
fn call_layouts_use_grammar_specific_callee_and_arguments() {
    let mut failures = Vec::new();
    for (language, source, before, name, expected) in [
        (
            Language::Java,
            "class A { void f(String x) { new p.Builder(\"ok\", /* value */ x); } }",
            "\"ok\", /* value */ ",
            "x",
            ("Builder", 1),
        ),
        (
            Language::Cpp,
            "void f(int x) { new ns::Builder(0, /* value */ x); }",
            "/* value */ ",
            "x",
            ("Builder", 1),
        ),
        (Language::Bash, "audit safe \"$x\"", "$", "x", ("audit", 1)),
        (
            Language::Terraform,
            "value = audit(\"ok\", /* value */ local.x)\n",
            "/* value */ ",
            "local.x",
            ("audit", 1),
        ),
    ] {
        let parsed = ParsedFile::parse("fixture", source, language).unwrap();
        assert!(
            !parsed.tree.root_node().has_error(),
            "{language:?}: {source}"
        );
        let location = use_at(&parsed, before, name, 1);
        let reached = parsed
            .tree
            .root_node()
            .named_descendant_for_byte_range(location.start_byte, location.end_byte)
            .unwrap();
        let call = std::iter::successors(reached.parent(), tree_sitter::Node::parent)
            .find(|node| parsed.language.is_call_node(node.kind()))
            .unwrap();
        let actual = call_operand(&parsed, call, reached)
            .map(|occ| (parsed.node_text(&occ.callee), occ.slot));
        if actual != Some(expected) {
            failures.push(format!("{language:?}: {actual:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn unrelated_inline_call_cannot_supply_a_rule() {
    let parsed = ParsedFile::parse(
        "a.py",
        "other()\ninline(\n    source()\n)\n",
        Language::Python,
    )
    .unwrap();
    let call = parsed
        .tree
        .root_node()
        .named_child(1)
        .unwrap()
        .named_child(0)
        .unwrap();
    let source = call_arg_node(&call, 0).unwrap();
    let inline = InlineOccurrence {
        admission_key: (&parsed.path, 2),
        occurrence: Occurrence {
            call,
            callee: parsed.call_function_name(&call).unwrap(),
            operand: source,
            slot: 0,
        },
        source,
    };
    let admitted = [1, 2, 3]
        .map(|line| (parsed.path.clone(), line))
        .into_iter()
        .collect();
    let project = |_, _, rows: &mut Vec<_>| rows.push((&JS_EVAL, false));
    assert!(collect_rules(&parsed, 1, &admitted, [], [inline], project).is_empty());
    assert!(collect_rules(&parsed, 3, &admitted, [], [inline], project).is_empty());
    let wrong_slot = InlineOccurrence {
        occurrence: Occurrence {
            slot: 1,
            ..inline.occurrence
        },
        ..inline
    };
    assert!(collect_rules(&parsed, 2, &admitted, [], [wrong_slot], project).is_empty());
    assert_eq!(
        collect_rules(&parsed, 2, &admitted, [], [inline], project).len(),
        1
    );
}

#[test]
fn assignment_target_is_not_a_reached_read() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x) {\n  res.send(x = 'safe');\n}\n",
        Language::JavaScript,
    )
    .unwrap();
    let mut path = path_to(use_at(&parsed, "res.send(", "x", 2));
    let definition = parsed.source.find("x)").unwrap();
    path.edges[0].from.start_byte = definition;
    path.edges[0].from.end_byte = definition + 1;
    let mut visited = Vec::new();
    collect_rules(
        &parsed,
        2,
        &BTreeSet::from([(parsed.path.clone(), 2)]),
        [&path],
        [],
        |occ, _, _| visited.push(parsed.node_text(&occ.callee)),
    );
    assert!(visited.is_empty(), "{visited:?}");
}
