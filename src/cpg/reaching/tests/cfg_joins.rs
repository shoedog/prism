use super::*;

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
fn exception_bypass_cannot_supply_exact_when_the_safe_path_kills() {
    let parsed = parsed(
        "def f():\n    x = source()\n    try:\n        x = clean()\n    except ValueError:\n        recover()\n    use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let safe_kill = def(&parsed, 1, "x", 4, 0, false);
    let use_edge = edge(&parsed, &func, &original, 7);
    let outcome = run(&parsed, &[original, safe_kill], &[use_edge.clone()]).0;
    assert_label(
        &outcome,
        &use_edge,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn finally_bypass_cannot_supply_exact_when_unflagged_path_kills() {
    let parsed = parsed(
        "def f():\n    x = 1\n    try:\n        x = 2\n    finally:\n        use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let safe_kill = def(&parsed, 1, "x", 4, 0, false);
    let use_edge = edge(&parsed, &func, &original, 6);
    let outcome = run(&parsed, &[original, safe_kill], &[use_edge.clone()]).0;
    assert_label(
        &outcome,
        &use_edge,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn surviving_unflagged_route_remains_exact_with_a_finally_bypass() {
    let parsed = parsed(
        "def f():\n    x = 1\n    try:\n        observe()\n    finally:\n        use(x)\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let use_edge = edge(&parsed, &func, &original, 6);
    let outcome = run(&parsed, &[original], &[use_edge.clone()]).0;
    assert_label(&outcome, &use_edge, FlowConfidence::Exact);
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
