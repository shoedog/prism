use super::*;

fn definition_on_line(defs: &[DefSite], line: usize) -> DefSite {
    defs.iter()
        .find(|def| def.path == AccessPath::simple("x") && def.line == line)
        .unwrap_or_else(|| panic!("fixture must define x on line {line}"))
        .clone()
}

#[test]
fn inner_block_assignment_is_killed_by_later_outer_redeclaration() {
    let parsed = parsed(
        "fn f() {\n    let mut x = 0;\n    {\n        x = source();\n    }\n    let x = 0;\n    sink(x);\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let nested_assignment = definition_on_line(&defs, 4);
    let use_edge = edge(&parsed, &func, &nested_assignment, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&use_edge)).0;
    assert_label(
        &outcome,
        &use_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
    );
}

#[test]
fn loop_initializer_kills_nested_assignment_on_next_iteration() {
    let parsed = parsed(
        "fn f() {\n    loop {\n        let mut x = 0;\n        sink(x);\n        {\n            x = source();\n        }\n        tick();\n    }\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let nested_assignment = definition_on_line(&defs, 6);
    let next_iteration_use = edge(&parsed, &func, &nested_assignment, 4);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&next_iteration_use)).0;
    assert_label(
        &outcome,
        &next_iteration_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
}

#[test]
fn assignment_to_inner_shadow_does_not_kill_outer_binding() {
    let parsed = parsed(
        "fn f() {\n    let mut x = source();\n    {\n        let mut x = 0;\n        x = clean();\n    }\n    sink(x);\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(&outcome, &outer_use, FlowConfidence::Exact);
}

#[test]
fn go_header_binding_ends_at_implicit_scope_exit() {
    let parsed = parsed(
        "package main\nfunc f() {\n\tv := outer()\n\tif v := source(); v != nil {\n\t\tsink(v)\n\t}\n\tsink(v)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("v") && def.line == 3)
        .expect("fixture must define outer v")
        .clone();
    let header = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("v") && def.line == 4)
        .expect("fixture must define header v")
        .clone();
    let outer_edge = edge(&parsed, &func, &outer, 7);
    let header_edge = edge(&parsed, &func, &header, 7);
    let outcome = run(&parsed, &defs, &[outer_edge.clone(), header_edge.clone()]).0;
    assert_label(&outcome, &outer_edge, FlowConfidence::Exact);
    assert_label(
        &outcome,
        &header_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
    );
}

#[test]
fn exception_before_assignment_makes_kill_witness_inadmissible() {
    let parsed = parsed(
        "def f():\n    try:\n        x = source()\n        x = may_throw()\n        raise RuntimeError()\n    except:\n        sink(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let original = definition_on_line(&defs, 3);
    let handler_use = edge(&parsed, &func, &original, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&handler_use)).0;
    assert_label(
        &outcome,
        &handler_use,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn cross_arm_kill_that_cannot_reach_use_is_not_a_witness() {
    let parsed = parsed(
        "def f(c):\n    if c:\n        x = source()\n        x = clean()\n    else:\n        sink(x)\n    done()\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let original = definition_on_line(&defs, 3);
    let cross_arm_use = edge(&parsed, &func, &original, 6);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&cross_arm_use)).0;
    assert_label(
        &outcome,
        &cross_arm_use,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn unresolved_binding_uses_flat_path_kill_equation() {
    let parsed = parsed(
        "function f() {\n  x = source();\n  x = clean();\n  sink(x);\n}\n",
        Language::TypeScript,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let original = definition_on_line(&defs, 2);
    let use_edge = edge(&parsed, &func, &original, 4);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&use_edge)).0;
    assert_label(
        &outcome,
        &use_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
}

#[test]
fn python_inner_block_assignment_reuses_function_binding() {
    let parsed = parsed(
        "def f(flag):\n    x = source()\n    if flag:\n        x = clean()\n    else:\n        return\n    sink(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(
        &outcome,
        &outer_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
}

#[test]
fn go_inner_block_declaration_does_not_kill_outer_binding() {
    let parsed = parsed(
        "package main\nfunc f() {\n\tx := source()\n\t{\n\t\tx := clean()\n\t\tsink(x)\n\t}\n\tsink(x)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 3);
    let outer_use = edge(&parsed, &func, &outer, 8);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(&outcome, &outer_use, FlowConfidence::Exact);
}

#[test]
fn rust_inner_block_declaration_does_not_kill_outer_binding() {
    let parsed = parsed(
        "fn f() {\n    let x = source();\n    {\n        let x = clean();\n        sink(x);\n    }\n    sink(x);\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(&outcome, &outer_use, FlowConfidence::Exact);
}

fn assert_rust_unclassified_pattern_uses_flat_fallback(
    source: &str,
    use_line: usize,
    kill_line: u32,
) {
    let parsed = parsed(source, Language::Rust);
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .unwrap_or_else(|| panic!("fixture must define value on line 2"))
        .clone();
    let outer_use = edge(&parsed, &func, &outer, use_line);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(
        &outcome,
        &outer_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line }),
    );
}

#[test]
fn rust_if_let_pattern_uncertainty_uses_flat_path_fallback() {
    assert_rust_unclassified_pattern_uses_flat_fallback(
        "fn f(opt: Option<i32>) {\n    let value = source();\n    if let Some(value) = opt {\n        {\n            let value = clean();\n        }\n        sink(value);\n    }\n}\n",
        7,
        5,
    );
}

#[test]
fn rust_match_arm_pattern_uncertainty_uses_flat_path_fallback() {
    assert_rust_unclassified_pattern_uses_flat_fallback(
        "fn f(opt: Option<i32>) {\n    let value = source();\n    match opt {\n        Some(value) => {\n            {\n                let value = clean();\n            }\n            sink(value);\n        }\n        None => {}\n    }\n}\n",
        8,
        6,
    );
}

#[test]
fn rust_while_let_pattern_uncertainty_uses_flat_path_fallback() {
    assert_rust_unclassified_pattern_uses_flat_fallback(
        "fn f(mut opt: Option<i32>) {\n    let value = source();\n    while let Some(value) = opt.take() {\n        {\n            let value = clean();\n        }\n        sink(value);\n    }\n}\n",
        7,
        5,
    );
}

#[test]
fn rust_classified_blocks_keep_per_binding_ownership() {
    let parsed = parsed(
        "fn f() {\n    let value = source();\n    {\n        let value = clean();\n        sink(value);\n    }\n    sink(value);\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .unwrap_or_else(|| panic!("fixture must define value on line 2"))
        .clone();
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(&outcome, &outer_use, FlowConfidence::Exact);
}

#[test]
fn javascript_let_inner_block_declaration_does_not_kill_outer_binding() {
    let parsed = parsed(
        "function f() {\n  let x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        Language::JavaScript,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(&outcome, &outer_use, FlowConfidence::Exact);
}

#[test]
fn javascript_repeated_var_reuses_function_binding() {
    let parsed = parsed(
        "function f() {\n  var x = source();\n  var x = clean();\n  sink(x);\n}\n",
        Language::JavaScript,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let first = definition_on_line(&defs, 2);
    let second = definition_on_line(&defs, 3);
    let first_edge = edge(&parsed, &func, &first, 4);
    let second_edge = edge(&parsed, &func, &second, 4);
    let outcome = run(&parsed, &defs, &[first_edge.clone(), second_edge.clone()]).0;
    assert_label(
        &outcome,
        &first_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
    assert_label(&outcome, &second_edge, FlowConfidence::Exact);
}

#[test]
fn javascript_var_redeclaration_reuses_parameter_binding() {
    let parsed = parsed(
        "function f(x) {\n  var x = source();\n  sink(x);\n}\n",
        Language::JavaScript,
    );
    let func = function(&parsed);
    let parameter = def(&parsed, 0, "x", 1, 0, false);
    let redeclaration = def(&parsed, 1, "x", 2, 0, false);
    let parameter_edge = edge(&parsed, &func, &parameter, 3);
    let redeclaration_edge = edge(&parsed, &func, &redeclaration, 3);
    let outcome = run(
        &parsed,
        &[parameter, redeclaration],
        &[parameter_edge.clone(), redeclaration_edge.clone()],
    )
    .0;
    assert_label(
        &outcome,
        &parameter_edge,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 2 }),
    );
    assert_label(&outcome, &redeclaration_edge, FlowConfidence::Exact);
}

#[test]
fn unresolved_pair_uses_every_same_path_definition_as_a_kill() {
    let parsed = parsed(
        "function f() {\n  x = source();\n  {\n    let x = clean();\n    sink(x);\n  }\n  sink(x);\n}\n",
        Language::TypeScript,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let original = definition_on_line(&defs, 2);
    let outer_use = edge(&parsed, &func, &original, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(
        &outcome,
        &outer_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
}

#[test]
fn javascript_and_typescript_lexical_loop_headers_end_at_loop_exit() {
    let loops = [
        "for (let x = 0; flag; tick())",
        "for (let x in values)",
        "for (let x of values)",
    ];
    for language in [Language::JavaScript, Language::TypeScript] {
        for loop_header in loops {
            let source = format!(
                "function f(flag, values) {{\n  let x = source();\n  {loop_header} {{\n    sink(x);\n  }}\n  sink(x);\n}}\n"
            );
            let parsed = parsed(&source, language);
            let func = function(&parsed);
            let defs = collect_defs(&parsed);
            let outer = definition_on_line(&defs, 2);
            let outer_use = edge(&parsed, &func, &outer, 6);
            let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
            let RdOutcome::Available(result) = outcome else {
                panic!("{language:?} {loop_header}: reaching definitions unavailable");
            };
            assert_eq!(
                result
                    .labels
                    .get(&(outer_use.from.clone(), outer_use.to.clone())),
                Some(&FlowConfidence::Exact),
                "{language:?} {loop_header}"
            );
        }
    }
}

#[test]
fn python_comprehension_target_masks_outer_binding() {
    let parsed = parsed(
        "def f(values):\n    x = source()\n    ys = [x for x in values]\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let comprehension_use = edge(&parsed, &func, &outer, 3);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&comprehension_use)).0;
    assert_label(
        &outcome,
        &comprehension_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
}

#[test]
fn python_comprehension_first_iterable_uses_outer_binding() {
    let parsed = parsed(
        "def f():\n    x = source()\n\n\n    ys = [x for x in x]\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let mut first_iterable_use = edge(&parsed, &func, &outer, 5);
    first_iterable_use.to.start_byte = byte_on_line(&parsed, 5, "x", 2);
    first_iterable_use.to.end_byte = first_iterable_use.to.start_byte + 1;
    let outcome = run(&parsed, &defs, std::slice::from_ref(&first_iterable_use)).0;
    assert_label(&outcome, &first_iterable_use, FlowConfidence::Exact);
}

#[test]
fn python_multiline_comprehension_first_iterable_uses_outer_binding() {
    let parsed = parsed(
        "def f():\n    x = source()\n    ys = [\n        x\n        for x in x\n    ]\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let mut first_iterable_use = edge(&parsed, &func, &outer, 5);
    first_iterable_use.to.start_byte = byte_on_line(&parsed, 5, "x", 1);
    first_iterable_use.to.end_byte = first_iterable_use.to.start_byte + 1;
    let outcome = run(&parsed, &defs, std::slice::from_ref(&first_iterable_use)).0;
    assert_label(&outcome, &first_iterable_use, FlowConfidence::Exact);
}

#[test]
fn python_comprehension_later_iterable_uses_earlier_target_binding() {
    let parsed = parsed(
        "def f(xs):\n    x = source()\n    ys = [\n        y\n        for x in xs\n        for y in x\n    ]\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 2);
    let second_iterable_use = edge(&parsed, &func, &outer, 6);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&second_iterable_use)).0;
    assert_label(
        &outcome,
        &second_iterable_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
    );
}

#[test]
fn unclassified_language_binding_construct_uses_flat_path_fallback() {
    let parsed = parsed(
        "class C {\n  void f() {\n    int x = source();\n    {\n      int x = clean();\n    }\n    sink(x);\n  }\n}\n",
        Language::Java,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, 3);
    let outer_use = edge(&parsed, &func, &outer, 7);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
    assert_label(
        &outcome,
        &outer_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
    );
}

#[test]
fn go_range_target_uncertainty_uses_flat_path_fallback() {
    let parsed = parsed(
        "func f(items []int) {\n\tvalue := source()\n\tfor _, value := range items {\n\t\t{\n\t\t\tvalue := clean()\n\t\t\tconsume(value)\n\t\t}\n\t\tsink(value)\n\t}\n\tconsume(value)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .expect("fixture must define outer value")
        .clone();
    let range_use = edge(&parsed, &func, &outer, 8);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&range_use)).0;
    assert_label(
        &outcome,
        &range_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
    );
}

#[test]
fn javascript_and_typescript_destructuring_uncertainty_use_flat_path_fallback() {
    for language in [Language::JavaScript, Language::TypeScript] {
        let parsed = parsed(
            "function f(obj) {\n  const value = source();\n  {\n    const { value } = obj;\n    consume(value);\n  }\n  sink(value);\n}\n",
            language,
        );
        let func = function(&parsed);
        let defs = collect_defs(&parsed);
        let outer = defs
            .iter()
            .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
            .unwrap_or_else(|| panic!("{language:?}: fixture must define outer value"))
            .clone();
        let outer_use = edge(&parsed, &func, &outer, 7);
        let outcome = run(&parsed, &defs, std::slice::from_ref(&outer_use)).0;
        let expected = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 });
        let RdOutcome::Available(result) = outcome else {
            panic!("{language:?}: expected Available with {expected:?}, got {outcome:?}");
        };
        assert_eq!(
            result
                .labels
                .get(&(outer_use.from.clone(), outer_use.to.clone())),
            Some(&expected),
            "{language:?}"
        );
    }
}

#[test]
fn python_except_and_with_alias_uncertainty_use_flat_path_fallback() {
    let except_parsed = parsed(
        "def f():\n    try:\n        may_throw()\n    except E as value:\n        consume(value)\n",
        Language::Python,
    );
    let except_func = function(&except_parsed);
    let except_defs = collect_defs(&except_parsed);
    let except_facts = scope::BindingFacts::new(&except_parsed, &except_func, &except_defs);
    assert_eq!(except_facts.unclassified_binding_lines("value"), vec![4]);

    let with_conditional = parsed(
        "def f(flag):\n    value = source()\n    with resource() as value:\n        if flag:\n            value = clean()\n        sink(value)\n",
        Language::Python,
    );
    let func = function(&with_conditional);
    let defs = collect_defs(&with_conditional);
    let with_facts = scope::BindingFacts::new(&with_conditional, &func, &defs);
    assert_eq!(
        with_facts.unclassified_binding_lines("value"),
        vec![3, 4, 5, 6]
    );
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .expect("fixture must define outer value")
        .clone();
    let killed_use = edge(&with_conditional, &func, &outer, 6);
    let outcome = run(&with_conditional, &defs, std::slice::from_ref(&killed_use)).0;
    assert_label(&outcome, &killed_use, FlowConfidence::Exact);

    let parsed = parsed(
        "def f():\n    value = source()\n    with resource() as value:\n        value = clean()\n        sink(value)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .expect("fixture must define outer value")
        .clone();
    let killed_use = edge(&parsed, &func, &outer, 5);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&killed_use)).0;
    assert_label(
        &outcome,
        &killed_use,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
}

#[test]
fn unclassified_introduction_after_use_does_not_demote_reaching_edge() {
    let parsed = parsed(
        "func f(items []int) {\n\tvalue := source()\n\tconsume(value)\n\tfor _, value := range items {\n\t\tconsume(value)\n\t}\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = defs
        .iter()
        .find(|def| def.path == AccessPath::simple("value") && def.line == 2)
        .expect("fixture must define outer value")
        .clone();
    let use_before_range = edge(&parsed, &func, &outer, 3);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&use_before_range)).0;
    assert_label(&outcome, &use_before_range, FlowConfidence::Exact);
}
