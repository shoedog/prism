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
