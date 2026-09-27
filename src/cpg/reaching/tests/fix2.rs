use super::*;

#[test]
fn go_range_body_reassignment_reaches_body_use() {
    let parsed = parsed(
        "func f(items []int) {\n  for _, x := range items {\n    x = source()\n    sink(x)\n  }\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let source = definition_on_line(&defs, "x", 3);
    let body_use = edge(&parsed, &func, &source, 4);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&body_use)).0;
    assert_label(&outcome, &body_use, FlowConfidence::Exact);
}

#[test]
fn go_range_inner_block_declaration_reaches_inner_use() {
    let parsed = parsed(
        "func f(items []int) {\n  for _, x := range items {\n    {\n      x := source()\n      sink(x)\n    }\n  }\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let source = definition_on_line(&defs, "x", 4);
    let inner_use = edge(&parsed, &func, &source, 5);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&inner_use)).0;
    assert_label(&outcome, &inner_use, FlowConfidence::Exact);
}

#[test]
fn nested_go_range_binder_reaches_its_own_body() {
    let parsed = parsed(
        "func f(outer []int, inner []int) {\n  for _, x := range outer {\n    for _, x := range inner {\n      sink(x)\n    }\n  }\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let mut defs = collect_defs(&parsed);
    let nested = def(&parsed, defs.len() as u32, "x", 3, 0, false);
    defs.push(nested.clone());
    let nested_use = edge(&parsed, &func, &nested, 4);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&nested_use)).0;
    assert_label(&outcome, &nested_use, FlowConfidence::Exact);
}
