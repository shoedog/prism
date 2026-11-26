use super::*;
use std::collections::BTreeMap;
use std::process::Command;

fn ledger(test: &str, id: &str, run: impl FnOnce(&Row, &Fixture)) -> Option<String> {
    const CHILD: &str = "PRISM_PARITY_LEDGER_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let row = rows().into_iter().find(|row| row.id == id).unwrap();
        run(&row, &Fixture::load(&row));
        return None;
    }
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("taint_parity_test::ledger_checks::{test}"),
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("PRISM_TAINT_CROSSING_TRACE", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8(child.stdout).unwrap();
    let stderr = String::from_utf8(child.stderr).unwrap();
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{stderr}"
    );
    Some(stderr)
}

fn walk_id(line: &str) -> &str {
    line.split_whitespace()
        .find_map(|field| field.strip_prefix("walk_id="))
        .unwrap_or_else(|| panic!("{line}"))
}

#[test]
fn wrapper_ledger_keeps_its_assignment_separate_from_return_input() {
    let Some(stderr) = ledger(
        "wrapper_ledger_keeps_its_assignment_separate_from_return_input",
        "tr2-js-wrapper",
        |row, fixture| {
            assert_eq!(review(row, fixture, true), Finding::Unlabeled);
            let two_sites = Fixture::load_with(row, |_, source| {
                *source = source.replace("exec(y);", "exec(y);\n  wrap('fixed');");
            });
            assert_eq!(review(row, &two_sites, true), Finding::Unlabeled);
            assert_eq!(walk(row, &two_sites).reaches, Verdict::Reached);
        },
    ) else {
        return;
    };
    let mut returns = BTreeMap::<usize, Vec<&str>>::new();
    for line in stderr.lines().filter(|line| {
        line.starts_with("taint-crossing ") && line.contains(":y:Def\" relation=ReturnFlow ")
    }) {
        returns
            .entry(walk_id(line).parse().unwrap())
            .or_default()
            .push(line);
    }
    assert_eq!(returns.len(), 3, "{stderr}");
    for lines in returns.values() {
        assert!(
            lines.iter().all(|line| {
                line.contains("site=163..170 slot=0 grades=[Exact] edge_grade=None ")
            }),
            "{stderr}"
        );
    }
    let inputs: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("taint-return-input "))
        .collect();
    assert!(!inputs.is_empty(), "{stderr}");
    assert!(
        inputs
            .iter()
            .all(|line| { line.contains("site=- slot=- grades=[] edge_grade=None ") }),
        "{stderr}"
    );
    assert!(
        stderr.lines().any(|line| {
            line.starts_with("taint-crossing ") && line.contains("edge_grade=Some(Exact) ")
        }),
        "{stderr}"
    );
}

fn assert_return_sites(
    test: &str,
    id: &str,
    nav_verdict: Verdict,
    review_finding: Finding,
    reason: Option<&str>,
    expected: &[(&str, &str)],
) {
    let Some(stderr) = ledger(test, id, |row, fixture| {
        assert_eq!(review(row, fixture, true), review_finding);
        assert_eq!(walk(row, fixture).reaches, nav_verdict);
    }) else {
        return;
    };
    let mut walks = BTreeMap::<usize, BTreeSet<&str>>::new();
    for line in stderr.lines() {
        if !line.contains(" relation=ReturnFlow ")
            || line.rsplit_once(" reason=").map(|(_, value)| value) != reason
        {
            continue;
        }
        for &(target, span) in expected {
            if line.contains(&format!(":{target}:Def\" relation=ReturnFlow ")) {
                assert!(
                    line.contains(&format!(
                        "site={span} slot=0 grades=[Exact] edge_grade=None "
                    )),
                    "{id}: {line}\n{stderr}"
                );
                walks
                    .entry(walk_id(line).parse().unwrap())
                    .or_default()
                    .insert(target);
            }
        }
    }
    let targets = expected
        .iter()
        .map(|(target, _)| *target)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        walks.values().collect::<Vec<_>>(),
        [&targets, &targets],
        "{stderr}"
    );
}

#[test]
fn review_and_navigation_distinguish_equal_budget_return_sites() {
    assert_return_sites(
        "review_and_navigation_distinguish_equal_budget_return_sites",
        "a1-js-equal-budget-return-sites",
        Verdict::Reached,
        Finding::Unlabeled,
        None,
        &[("first", "135..148"), ("second", "167..180")],
    );
}

#[test]
fn depth_cap_return_refusal_names_the_assignment_in_both_clients() {
    assert_return_sites(
        "depth_cap_return_refusal_names_the_assignment_in_both_clients",
        "s2-js-three-ascents",
        Verdict::BoundaryExited,
        Finding::None,
        Some("depth_cap"),
        &[("result", "241..246")],
    );
}

#[test]
fn self_return_refusal_names_the_recursive_assignment_in_both_clients() {
    assert_return_sites(
        "self_return_refusal_names_the_recursive_assignment_in_both_clients",
        "s1-js-self-return",
        Verdict::BoundaryExited,
        Finding::None,
        Some("self_return"),
        &[("y", "136..142")],
    );
}

#[test]
fn discarded_call_cannot_lend_its_site_to_the_receiving_assignment() {
    assert_return_sites(
        "discarded_call_cannot_lend_its_site_to_the_receiving_assignment",
        "tr2-js-discard",
        Verdict::Reached,
        Finding::Unlabeled,
        None,
        &[("y", "137..148")],
    );
}

#[test]
fn return_refusal_requires_assignment_and_callee_evidence() {
    use prism::cpg::{CpgEdge, ReturnFlowMode, SameLineOrderView, VarAccess};
    use prism::reasoning::order::AstOrderView;

    const SOURCE: &str = "class K {\n  f(p) {\n    const v = p;\n    return v;\n  }\n}\n\
        function entry(obj, y) {\n  const out = obj.f(y);\n  exec(out);\n}\n\
        function g(v) { return v; }\n\
        function stray(y) {\n  const other = g(y);\n  exec(other);\n}\n";
    let Some(stderr) = ledger(
        "return_refusal_requires_assignment_and_callee_evidence",
        "tr2-js-wrapper",
        |row, _| {
            let fixture = Fixture::load_with(row, |_, source| *source = SOURCE.into());
            let CpgContext { mut cpg, files, .. } = fixture.context();
            let variable = |start, kind| {
                cpg.graph
                    .node_indices()
                    .find(|&n| {
                        matches!(cpg.node(n), CpgNode::Variable { start_byte, access, .. }
                        if (*start_byte, *access) == (start, kind))
                    })
                    .unwrap()
            };
            let from = variable(SOURCE.find("return v").unwrap() + 7, VarAccess::Use);
            let out = variable(SOURCE.find("out =").unwrap(), VarAccess::Def);
            let other = variable(SOURCE.find("other =").unwrap(), VarAccess::Def);
            // Step 5c would decline both edges. Inject them to exercise the ascent gate.
            for to in [out, other] {
                cpg.graph.add_edge(
                    from,
                    to,
                    CpgEdge::ReturnFlow {
                        suppress_shortcut: false,
                    },
                );
            }
            let order = AstOrderView::new(&cpg, files);
            let empty_files = BTreeMap::new();
            let missing = AstOrderView::new(&cpg, &empty_files);
            let views: [Option<&dyn SameLineOrderView>; 3] = [None, Some(&missing), Some(&order)];
            for view in views {
                let trace = cpg.taint_trace_nodes_with_mode(&[from], view, ReturnFlowMode::On);
                assert!(!trace.in_frontier(out) && !trace.in_frontier(other));
            }
        },
    ) else {
        return;
    };
    let site = SOURCE.find("obj.f(y)").unwrap();
    let fields = |reason: &str| {
        stderr
            .lines()
            .filter(move |line| {
                line.rsplit_once(" reason=")
                    .is_some_and(|(_, r)| r == reason)
                    && line.contains(" relation=ReturnFlow ")
            })
            .map(|line| {
                line.split_once(" site=")
                    .unwrap()
                    .1
                    .split_once(" depth=")
                    .unwrap()
                    .0
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        fields("not_exact"),
        [
            "unavailable(review-mode) slot=- grades=[] edge_grade=None",
            "unavailable(assignment-proof) slot=- grades=[] edge_grade=None",
            &format!(
                "{site}..{} slot=0 grades=[NameOnly] edge_grade=None",
                site + "obj.f(y)".len()
            ),
        ],
        "{stderr}"
    );
    assert_eq!(
        fields("no_matching_site"),
        ["- slot=- grades=[] edge_grade=None"; 3],
        "{stderr}"
    );
}

#[test]
fn alias_refusal_omits_the_rejected_sites_grade() {
    let Some(stderr) = ledger(
        "alias_refusal_omits_the_rejected_sites_grade",
        "s2-js-alias-same-line",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited),
    ) else {
        return;
    };
    let line = stderr
        .lines()
        .find(|line| {
            line.starts_with("taint-refusal ") && line.ends_with("reason=no_matching_site")
        })
        .unwrap_or_else(|| panic!("{stderr}"));
    assert!(
        line.contains(":164..165#") && line.contains("site=- slot=- grades=[] "),
        "{line}"
    );
}

#[test]
fn shortcut_refusal_reports_its_certified_grade() {
    let Some(stderr) = ledger(
        "shortcut_refusal_reports_its_certified_grade",
        "x01",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::NotReached),
    ) else {
        return;
    };
    let line = stderr
        .lines()
        .find(|line| {
            line.starts_with("taint-refusal ") && line.ends_with("reason=shortcut_suppressed")
        })
        .unwrap_or_else(|| panic!("{stderr}"));
    assert!(
        line.contains("site=113..122 slot=0 grades=[Exact] "),
        "{line}"
    );
}

#[test]
fn sanitizer_bypass_records_are_countable_per_walk() {
    let Some(stderr) = ledger(
        "sanitizer_bypass_records_are_countable_per_walk",
        "a1-js-sanitizer-boundary-bypass",
        |row, fixture| {
            for _ in 0..2 {
                assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited);
            }
        },
    ) else {
        return;
    };
    let mut walks = BTreeMap::new();
    for line in stderr.lines() {
        let (event, fields) = line.split_once(' ').unwrap();
        let walk = fields
            .strip_prefix("walk=")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        let (kind, crossings, refusals) = walks.entry(walk_id(line)).or_insert((walk, 0, 0));
        assert_eq!(*kind, walk, "{line}");
        match event {
            "taint-crossing" => *crossings += 1,
            "taint-refusal" => *refusals += 1,
            _ => panic!("{line}"),
        }
    }
    let mut counts = walks.into_values().collect::<Vec<_>>();
    counts.sort_unstable();
    assert_eq!(
        counts,
        [
            ("bypass", 2, 1),
            ("bypass", 2, 1),
            ("primary", 3, 1),
            ("primary", 3, 1),
        ],
        "{stderr}"
    );
}
