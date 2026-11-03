use super::*;

fn check_fold_cycle(source: &str, carried: bool) {
    let parsed = parsed(source, Language::Go);
    let func = function(&parsed);
    let line = |marker| source.lines().position(|l| l.contains(marker)).unwrap() + 1;
    let definition = def(&parsed, 0, "x", line("// def"), 0, false);
    let flow = edge(&parsed, &func, &definition, line("// use"));
    let outcome = run(&parsed, &[definition], std::slice::from_ref(&flow)).0;
    assert_label(&outcome, &flow, FlowConfidence::Exact);
    let RdOutcome::Available(result) = outcome else {
        unreachable!()
    };
    assert_eq!(
        result.loop_carried_edges.contains(&(flow.from, flow.to)),
        carried
    );
}

#[test]
fn go_fold_rd_acyclic_defer() {
    check_fold_cycle(
        include_str!("../../../../tests/fixtures/go_loop_cycles/acyclic_defer.go"),
        false,
    );
}

#[test]
fn go_fold_rd_cyclic_goto() {
    check_fold_cycle(
        include_str!("../../../../tests/fixtures/go_loop_cycles/cyclic_goto.go"),
        true,
    );
}

#[test]
fn go_fold_rd_cyclic_defer() {
    check_fold_cycle(
        include_str!("../../../../tests/fixtures/go_loop_cycles/cyclic_defer.go"),
        true,
    );
}

#[test]
fn go_round2_rd_backward_goto_requires_cycle() {
    for (source, from, to, carried) in [
        ("package p\nfunc f(x int) int {\ngoto Assign\nRead:\nreturn x\nAssign:\nx = 1\ngoto Read\n}\n", 7, 5, false),
        ("package p\nfunc f(x int, c bool) int {\nRead:\nuse(x)\nx = 1\nif c {\ngoto Read\n}\nreturn x\n}\n", 5, 4, true),
    ] {
        let parsed = parsed(source, Language::Go);
        let func = function(&parsed);
        let definition = def(&parsed, 0, "x", from, 0, false);
        let flow = edge(&parsed, &func, &definition, to);
        let outcome = run(&parsed, &[definition], std::slice::from_ref(&flow)).0;
        assert_label(&outcome, &flow, FlowConfidence::Exact);
        let RdOutcome::Available(result) = outcome else { unreachable!() };
        assert_eq!(result.loop_carried_edges.contains(&(flow.from, flow.to)), carried);
    }
}
