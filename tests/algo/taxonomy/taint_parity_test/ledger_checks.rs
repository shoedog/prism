use super::*;
use std::collections::BTreeMap;
use std::process::Command;

/// The navigation walks' ledger lines only: review mode prints no site, slot or grades, so the
/// site-field assertions below read the lines that carry them.
fn ledger(test: &str, id: &str, run: impl FnOnce(&Row, &Fixture)) -> Option<String> {
    ledger_raw(test, id, run).map(|stderr| navigation_lines(&stderr))
}

fn navigation_lines(stderr: &str) -> String {
    stderr
        .lines()
        .filter(|line| !line.starts_with("taint-") || line.contains(" site="))
        .collect::<Vec<_>>()
        .join("\n")
}

fn ledger_raw(test: &str, id: &str, run: impl FnOnce(&Row, &Fixture)) -> Option<String> {
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
    assert_eq!(returns.len(), 1, "{stderr}");
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
    let Some(raw) = ledger_raw(test, id, |row, fixture| {
        assert_eq!(review(row, fixture, true), review_finding);
        assert_eq!(walk(row, fixture).reaches, nav_verdict);
    }) else {
        return;
    };
    let stderr = navigation_lines(&raw);
    if let Some(reason) = reason {
        // Review mode keeps the refusal's reason on the same hop but prints no site fields.
        let review_lines: Vec<_> = raw
            .lines()
            .filter(|line| {
                line.starts_with("taint-refusal ")
                    && line.contains(" relation=ReturnFlow ")
                    && line.ends_with(&format!(" reason={reason}"))
                    && !line.contains(" site=")
            })
            .collect();
        for &(target, _) in expected {
            assert!(
                review_lines
                    .iter()
                    .any(|line| line.contains(&format!(":{target}:Def\" relation=ReturnFlow "))),
                "{id}: no review record for {target} with reason={reason}\n{raw}"
            );
        }
        assert!(
            review_lines
                .iter()
                .all(|line| !line.contains(" slot=") && !line.contains(" grades=")),
            "{id}: {review_lines:#?}"
        );
    }
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
    assert_eq!(walks.values().collect::<Vec<_>>(), [&targets], "{stderr}");
}

#[test]
fn navigation_distinguishes_equal_budget_return_sites() {
    assert_return_sites(
        "navigation_distinguishes_equal_budget_return_sites",
        "a1-js-equal-budget-return-sites",
        Verdict::Reached,
        Finding::Unlabeled,
        None,
        &[("first", "135..148"), ("second", "167..180")],
    );
}

#[test]
fn navigation_names_the_depth_cap_assignment_and_review_prints_no_site() {
    assert_return_sites(
        "navigation_names_the_depth_cap_assignment_and_review_prints_no_site",
        "s2-js-three-ascents",
        Verdict::BoundaryExited,
        Finding::None,
        Some("depth_cap"),
        &[("result", "241..246")],
    );
}

#[test]
fn navigation_names_the_self_return_assignment_and_review_prints_no_site() {
    assert_return_sites(
        "navigation_names_the_self_return_assignment_and_review_prints_no_site",
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
            "- slot=- grades=[] edge_grade=None",
            "- slot=- grades=[] edge_grade=None",
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
fn discarded_multiline_python_call_cannot_name_the_shortcut_site() {
    let Some(stderr) = ledger(
        "discarded_multiline_python_call_cannot_name_the_shortcut_site",
        "s2-py-discard-multiline-return",
        |row, fixture| {
            assert_eq!(walk(row, fixture).reaches, Verdict::NotReached);
            assert_eq!(review(row, fixture, true), Finding::None);
        },
    ) else {
        return;
    };
    let cuts: Vec<_> = stderr
        .lines()
        .filter(|line| {
            line.contains(":t:Use\" to=")
                && line.contains(":fixed:Def\" relation=AssignmentPropagation ")
                && line.ends_with("reason=shortcut_suppressed")
        })
        .collect();
    assert_eq!(
        cuts.iter()
            .map(|line| walk_id(line))
            .collect::<BTreeSet<_>>()
            .len(),
        1,
        "{stderr}"
    );
    assert!(
        cuts.iter()
            .all(|line| line.contains("site=- slot=- grades=[] ")),
        "{cuts:#?}"
    );
}

fn shortcut_source(lang: &str, adjacent: bool, assigned: bool) -> (&'static str, String) {
    let (file, head, binding, tail) = match lang {
        "go" => (
            "a.go",
            "package main\nfunc identity(v string) string { return v }\nfunc entry() {\n  t := acquire()\n  ",
            "result := ",
            "\n  sink(result)\n}\n",
        ),
        "javascript" => (
            "a.js",
            "function identity(v) { return v; }\nfunction entry() {\n  const t = acquire();\n  ",
            "const result = ",
            ";\n  sink(result);\n}\n",
        ),
        "python" => (
            "a.py",
            "def identity(v):\n    return v\n\ndef entry():\n    t = acquire()\n    ",
            "result = ",
            "\n    sink(result)\n",
        ),
        "tsx" | "typescript" => (
            if lang == "tsx" { "a.tsx" } else { "a.ts" },
            "function identity(v: string): string { return v; }\nfunction entry() {\n  const t = acquire();\n  ",
            "const result = ",
            ";\n  sink(result);\n}\n",
        ),
        _ => panic!("add a shortcut fixture for {lang}"),
    };
    let (discard, receive) = if assigned {
        ("\"fixed\"", "t")
    } else {
        ("t", "\"fixed\"")
    };
    let discarded_call = if adjacent {
        let comma = if lang == "go" { "," } else { "" };
        format!("identity(\n        {discard}{comma})")
    } else {
        format!("identity({discard})")
    };
    (
        file,
        format!("{head}{discarded_call}; {binding}identity({receive}){tail}"),
    )
}

#[test]
fn shortcut_site_requires_the_same_call_in_every_matrix_language() {
    use prism::cpg::{ReturnFlowMode, VarAccess};
    use prism::reasoning::order::AstOrderView;

    let Some(stderr) = ledger(
        "shortcut_site_requires_the_same_call_in_every_matrix_language",
        "x01",
        |_, _| {
            let languages: BTreeSet<_> = rows().into_iter().map(|row| row.lang).collect();
            for lang in languages {
                for adjacent in [false, true] {
                    for assigned in [false, true] {
                        let (file, source) = shortcut_source(&lang, adjacent, assigned);
                        let parsed = ParsedFile::parse(
                            file,
                            &source,
                            prism::languages::Language::from_path(file).unwrap(),
                        )
                        .unwrap();
                        assert_eq!(parsed.parse_error_count, 0, "{source}");
                        let files = [(file.into(), parsed)].into();
                        let ctx = CpgContext::build(&files, None);
                        let cpg = &ctx.cpg;
                        let order = AstOrderView::new(cpg, &files);
                        let root_start =
                            source.find("t =").or_else(|| source.find("t :=")).unwrap();
                        let root = cpg.graph.node_indices().find(|&node| {
                            matches!(cpg.node(node), CpgNode::Variable { start_byte, access: VarAccess::Def, .. }
                                if *start_byte == root_start)
                        }).unwrap_or_else(|| panic!("{source}"));
                        let call = if assigned {
                            "identity(t)"
                        } else {
                            "identity(\"fixed\")"
                        };
                        let start = source.rfind(call).unwrap();
                        let end = start + call.len();
                        eprintln!("case {lang} adjacent={adjacent} assigned={assigned} expected={start}..{end}");
                        cpg.taint_trace_nodes_with_mode(&[root], Some(&order), ReturnFlowMode::On);
                    }
                }
            }
        },
    ) else {
        return;
    };
    let mut mismatches = Vec::new();
    let mut cases = 0;
    for case in stderr.split("case ").skip(1) {
        cases += 1;
        let (header, lines) = case.split_once('\n').unwrap();
        let cuts: Vec<_> = lines
            .lines()
            .filter(|line| {
                line.contains(":t:Use\" to=")
                    && line.contains(":result:Def\" relation=AssignmentPropagation ")
                    && line.ends_with("reason=shortcut_suppressed")
            })
            .collect();
        let expected = if header.contains("assigned=true") {
            format!(
                "site={} slot=0 grades=[Exact] ",
                header.split_once("expected=").unwrap().1
            )
        } else {
            "site=- slot=- grades=[] ".into()
        };
        if cuts.len() != 1 || !cuts[0].contains(&expected) {
            mismatches.push(format!(
                "{header}: expected {expected:?}, got {cuts:?}\n{lines}"
            ));
        }
    }
    assert_eq!(
        cases,
        4 * rows()
            .into_iter()
            .map(|row| row.lang)
            .collect::<BTreeSet<_>>()
            .len()
    );
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn trace_review_without_return_flow_logs_proved_descents() {
    let Some(stderr) = ledger_raw(
        "trace_review_without_return_flow_logs_proved_descents",
        "tr2-js-wrapper",
        |row, fixture| {
            let ctx = fixture.context();
            let (_, trace) = slice_with_trace(&ctx, &fixture.diff(), &row.config(false)).unwrap();
            assert!(!trace.roots.is_empty());
        },
    ) else {
        return;
    };
    assert!(
        stderr.lines().any(|line| {
            line.starts_with("taint-crossing ")
                && line.contains("relation=CallDescent edge_grade=Some(Exact) ")
        }),
        "{stderr}"
    );
    assert!(!stderr.contains("relation=ReturnFlow "), "{stderr}");
}

#[test]
fn call_descent_requires_the_target_formal_at_the_actual_slot() {
    use prism::cpg::{CpgEdge, FlowConfidence, VarAccess};
    use prism::reasoning::order::AstOrderView;

    const SOURCE: &str = "function bind(x, y) {\n  const local = x;\n  sink(y);\n}\n\
        function entry(t) {\n  bind(t, 42);\n}\n";
    let Some(stderr) = ledger(
        "call_descent_requires_the_target_formal_at_the_actual_slot",
        "x01",
        |row, _| {
            let fixture = Fixture::load_with(row, |_, source| *source = SOURCE.into());
            let CpgContext { mut cpg, files, .. } = fixture.context();
            let occurrence =
                |byte, access| {
                    cpg.graph.node_indices().find(|&node| {
                    matches!(cpg.node(node), CpgNode::Variable { start_byte, access: kind, .. }
                        if (*start_byte, *kind) == (byte, access))
                }).unwrap()
                };
            let from = occurrence(SOURCE.find("bind(t").unwrap() + 5, VarAccess::Use);
            let unrelated = ["y)", "local ="]
                .map(|name| occurrence(SOURCE.find(name).unwrap(), VarAccess::Def));
            for to in unrelated {
                cpg.graph
                    .add_edge(from, to, CpgEdge::DataFlow(FlowConfidence::Exact));
            }
            let order = AstOrderView::new(&cpg, files);
            cpg.taint_trace_nodes(&[from], Some(&order));
        },
    ) else {
        return;
    };
    let call = SOURCE.find("bind(t, 42)").unwrap();
    for (target, fields) in [
        (
            "x",
            format!(
                "site={call}..{} slot=0 grades=[Exact] ",
                call + "bind(t, 42)".len()
            ),
        ),
        ("y", "site=- slot=- grades=[] ".into()),
        ("local", "site=- slot=- grades=[] ".into()),
    ] {
        let line = stderr
            .lines()
            .find(|line| {
                line.starts_with("taint-crossing ")
                    && line.contains(&format!(":{target}:Def\" relation=CallDescent "))
            })
            .unwrap_or_else(|| panic!("{stderr}"));
        assert!(line.contains(&fields), "{line}");
    }
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

#[test]
fn review_ledger_drops_site_slot_and_grades_but_keeps_reasons() {
    let Some(stderr) = ledger_raw(
        "review_ledger_drops_site_slot_and_grades_but_keeps_reasons",
        "s2-js-alias-same-line",
        |row, fixture| {
            let _ = review(row, fixture, true);
        },
    ) else {
        return;
    };
    let lines: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("taint-"))
        .collect();
    assert!(!lines.is_empty(), "{stderr}");
    assert!(
        lines.iter().all(|line| {
            !line.contains(" site=") && !line.contains(" slot=") && !line.contains(" grades=")
        }),
        "{stderr}"
    );
    assert!(
        lines.iter().all(|line| line.contains(" edge_grade=")),
        "{stderr}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("taint-refusal ") && line.contains(" reason=")),
        "{stderr}"
    );
    // The alias hop's refusal keeps its reason in review mode.
    assert!(
        lines.iter().any(|line| {
            line.starts_with("taint-refusal ")
                && line.contains(":164..165#")
                && line.ends_with(" reason=no_matching_site")
        }),
        "{stderr}"
    );
}

fn assert_js12_review_has_no_site_fields(test: &str, return_flow: bool) {
    let Some(stderr) = ledger_raw(test, "js12", |row, fixture| {
        let _ = review(row, fixture, return_flow);
    }) else {
        return;
    };
    let lines: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("taint-"))
        .collect();
    assert!(
        lines.iter().all(|line| {
            !line.contains(" site=") && !line.contains(" slot=") && !line.contains(" grades=")
        }),
        "{stderr}"
    );
    assert!(
        lines.iter().any(|line| {
            // With return flow on, the NameOnly descent is refused; with it off, review admits it.
            line.starts_with(if return_flow {
                "taint-refusal "
            } else {
                "taint-crossing "
            }) && line.contains(" relation=CallDescent ")
                && line.contains(" edge_grade=Some(NameOnly(CallNameOnly))")
        }),
        "{stderr}"
    );
}

#[test]
fn nameonly_refusal_prints_no_site_fields_in_review_with_return_flow() {
    assert_js12_review_has_no_site_fields(
        "nameonly_refusal_prints_no_site_fields_in_review_with_return_flow",
        true,
    );
}

#[test]
fn nameonly_refusal_prints_no_site_fields_in_review_without_return_flow() {
    assert_js12_review_has_no_site_fields(
        "nameonly_refusal_prints_no_site_fields_in_review_without_return_flow",
        false,
    );
}

#[test]
fn navigation_keeps_the_nameonly_site_and_grade_of_js12() {
    let Some(stderr) = ledger(
        "navigation_keeps_the_nameonly_site_and_grade_of_js12",
        "js12",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited),
    ) else {
        return;
    };
    assert!(
        stderr.lines().any(|line| {
            line.starts_with("taint-refusal ")
                && line.contains(" relation=CallDescent site=59..73 ")
                && line.contains(" grades=[NameOnly] ")
                && line.ends_with(" reason=not_exact")
        }),
        "{stderr}"
    );
}

#[test]
fn navigation_ledger_keeps_site_slot_and_grades() {
    // The raw ledger, unfiltered: a navigation-only run prints the fields on every record.
    let Some(stderr) = ledger_raw(
        "navigation_ledger_keeps_site_slot_and_grades",
        "s2-js-alias-same-line",
        |row, fixture| assert_eq!(walk(row, fixture).reaches, Verdict::BoundaryExited),
    ) else {
        return;
    };
    let lines: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("taint-crossing ") || line.starts_with("taint-refusal "))
        .collect();
    assert!(!lines.is_empty(), "{stderr}");
    assert!(
        lines.iter().all(|line| {
            line.contains(" site=") && line.contains(" slot=") && line.contains(" grades=[")
        }),
        "{stderr}"
    );
}
