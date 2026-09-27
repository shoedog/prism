use super::*;
#[test]
fn go_range_short_same_name_iterable_reads_outer_then_shadows_body() {
    let parsed = parsed(
        "func f(items []int) {\n  x := source()\n  for _, x := range selectItems(x) {\n    observe(x)\n  }\n  sink(x)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, "x", 2);
    let iterable_use = edge(&parsed, &func, &outer, 3);
    let inside = edge(&parsed, &func, &outer, 4);
    let after = edge(&parsed, &func, &outer, 6);
    assert_eq!(iterable_use.to.start_byte, byte_on_line(&parsed, 3, "x", 1));
    let outcome = run(
        &parsed,
        &defs,
        &[iterable_use.clone(), inside.clone(), after.clone()],
    )
    .0;
    assert_label(&outcome, &iterable_use, FlowConfidence::Exact);
    assert_label(
        &outcome,
        &inside,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
    assert_label(&outcome, &after, FlowConfidence::Exact);
}

#[test]
fn go_range_short_unresolved_outer_is_shadowed_only_inside() {
    let parsed = parsed(
        "func f(items []int) {\n  x = source()\n  for _, x := range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, "x", 2);
    let inside = edge(&parsed, &func, &outer, 4);
    let zero_iteration_exit = edge(&parsed, &func, &outer, 6);
    let outcome = run(
        &parsed,
        &defs,
        &[inside.clone(), zero_iteration_exit.clone()],
    )
    .0;
    assert_label(
        &outcome,
        &inside,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
    );
    assert_label(&outcome, &zero_iteration_exit, FlowConfidence::Exact);
}

#[test]
fn go_range_short_unresolved_outer_conditional_bypass_is_exact() {
    let parsed = parsed(
        "func f(items []int, c bool) {\n  x = source()\n  if c {\n    for _, x := range items {\n      observe(x)\n    }\n  }\n  sink(x)\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let outer = definition_on_line(&defs, "x", 2);
    let inside = edge(&parsed, &func, &outer, 5);
    let bypass = edge(&parsed, &func, &outer, 8);
    let outcome = run(&parsed, &defs, &[inside.clone(), bypass.clone()]).0;
    assert_label(
        &outcome,
        &inside,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
    );
    assert_label(&outcome, &bypass, FlowConfidence::Exact);
}
