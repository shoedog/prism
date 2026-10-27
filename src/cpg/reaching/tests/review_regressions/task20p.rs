use super::*;

#[test]
fn python_finally_self_read_uses_the_preceding_kill() {
    let parsed = parsed(
        "def f():\n    x = source()\n    try:\n        x = h1()\n        w = k()\n        x = h2()\n    finally:\n        x = x + 1\n    sink(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let first_try_assignment = definition_on_line(&defs, 4);
    let finally_read = edge(&parsed, &func, &first_try_assignment, 8);
    let outcome = run(&parsed, &defs, std::slice::from_ref(&finally_read)).0;
    assert_label(
        &outcome,
        &finally_read,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
    );
}

#[test]
fn python_finally_dominant_kill_attribution_c27() {
    let parsed = parsed(
        "def f():\n    x = source()\n    try:\n        x = h1()\n        w = k()\n        x = h2()\n        done()\n    finally:\n        x = x + 1\n    sink(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let defs = collect_defs(&parsed);
    let initial = definition_on_line(&defs, 2);
    let first_try_assignment = definition_on_line(&defs, 4);
    let first_to_finally = edge(&parsed, &func, &first_try_assignment, 9);
    let initial_to_after = edge(&parsed, &func, &initial, 10);
    let first_to_after = edge(&parsed, &func, &first_try_assignment, 10);
    let outcome = run(
        &parsed,
        &defs,
        &[
            first_to_finally.clone(),
            initial_to_after.clone(),
            first_to_after.clone(),
        ],
    )
    .0;
    assert_label(
        &outcome,
        &first_to_finally,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 }),
    );
    assert_label(
        &outcome,
        &initial_to_after,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 9 }),
    );
    assert_label(
        &outcome,
        &first_to_after,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 9 }),
    );
}
