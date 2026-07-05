use prism::navigation::types::{
    QueryError, Reachability, ReasoningWarning, SymbolRef, WarningKind,
};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;
use prism::repo_loader::load_repo;
use std::sync::Arc;
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    session: NavigationSession,
}

fn fixture(files: &[(&str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    for (file, src) in files {
        let path = dir.path().join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    Fixture {
        _dir: dir,
        session: NavigationSession { repo, index },
    }
}

#[test]
fn witness_mode_reports_reached_sink_and_witness_graph() {
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    value = user\n    sink(value)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::Reached));
    assert!(reasoning
        .per_sink
        .iter()
        .any(|sink| sink.reachability == Reachability::Reached));
    assert!(evidence
        .graph
        .as_ref()
        .is_some_and(|graph| !graph.nodes.is_empty() && !graph.edges.is_empty()));
    let graph = evidence.graph.as_ref().expect("witness graph");
    let graph_node = reasoning
        .per_sink
        .iter()
        .flat_map(|sink| &sink.sources)
        .find_map(|source| source.graph_node)
        .expect("source should point at sink endpoint in witness graph");
    assert!(
        matches!(
            &graph.nodes[graph_node].symbol,
            Some(SymbolRef::Variable { path, access, line, .. })
                if path == "value" && access == "use" && *line == 4
        ),
        "graph_node must identify the reached sink endpoint, not just the last merged witness node"
    );
}

#[test]
fn same_line_body_def_on_function_start_line_reaches_instead_of_boundary() {
    let fixture = fixture(&[("app.py", "def f(u): y = u; sink(y)\n")]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 1,
        }]),
    )
    .expect("taint_reaches");

    let y_sinks = evidence
        .reasoning
        .as_ref()
        .expect("reasoning summary")
        .per_sink
        .iter()
        .filter(|sink| {
            matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. } if path == "y" && access == "use"
            )
        })
        .collect::<Vec<_>>();
    assert!(!y_sinks.is_empty());
    assert!(
        y_sinks
            .iter()
            .any(|sink| sink.reachability == Reachability::Reached),
        "same-line body local must be reached, not treated as a parameter boundary"
    );
}

#[test]
fn same_line_collapsed_occurrence_fails_open_with_warning() {
    let fixture = fixture(&[("app.py", "def f(u):\n    sink(y); y = u; baz(y)\n")]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::OrderingUnavailable { .. })
        )
    }));

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    let y_uses = reasoning
        .per_sink
        .iter()
        .filter(|sink| {
            matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. } if path == "y" && access == "use"
            )
        })
        .collect::<Vec<_>>();
    assert!(!y_uses.is_empty(), "line sink should include y uses");
    assert!(
        y_uses
            .iter()
            .any(|sink| sink.reachability == Reachability::Reached),
        "collapsed same-line uses must fail open to avoid a false NotReached verdict"
    );
}

#[test]
fn same_line_loop_carried_field_occurrence_fails_open_with_warning() {
    let fixture = fixture(&[(
        "app.py",
        "def f(cond):\n    user = input()\n    while cond:\n        sink(o.data); o.data = user\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::OrderingUnavailable { .. })
        )
    }));
    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(
        reasoning.per_sink.iter().any(|sink| {
            matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. }
                    if path == "o.data"
                        && access == "use"
                        && sink.reachability == Reachability::Reached
            )
        }),
        "same-line loop-carried field flow must fail open instead of reporting NotReached"
    );
}

#[test]
fn identifier_substring_in_callee_name_does_not_block_reachability() {
    let fixture = fixture(&[("app.py", "def f(u):\n    id = u; valid(id)\n")]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(reasoning.per_sink.iter().any(|sink| {
        matches!(
            &sink.sink,
            SymbolRef::Variable { path, access, .. }
                if path == "id" && access == "use" && sink.reachability == Reachability::Reached
        )
    }));
}

#[test]
fn callee_identifier_collision_is_not_reported_as_sink() {
    let fixture = fixture(&[(
        "app.py",
        "def f(sink):\n    clean = 'ok'\n    sink(clean)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 3,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::NotReached));
    assert!(
        reasoning.per_sink.iter().all(|sink| {
            !matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. } if path == "sink" && access == "use"
            )
        }),
        "the call callee identifier must not be treated as the sink argument"
    );
}

#[test]
fn same_name_callee_argument_is_kept_as_sink_argument() {
    let fixture = fixture(&[("app.py", "def f(sink):\n    sink(sink)\n")]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(
        reasoning.per_sink.iter().any(|sink| {
            matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. }
                    if path == "sink"
                        && access == "use"
                        && sink.reachability == Reachability::Reached
            )
        }),
        "a same-name callee/argument occurrence should keep the legitimate argument sink"
    );
}

#[test]
fn same_line_multi_function_body_local_reaches_instead_of_boundary() {
    let fixture = fixture(&[(
        "app.js",
        "function a(u) { x = u; sink(x); } function b(u) { y = u; sink(y); }\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "b".into(),
            file: Some("app.js".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.js".into(),
            line: 1,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(
        reasoning.per_sink.iter().any(|sink| {
            matches!(
                &sink.sink,
                SymbolRef::Variable { path, access, .. }
                    if path == "y" && access == "use" && sink.reachability == Reachability::Reached
            )
        }),
        "body-local y in the second same-line function must be reached, not treated as a parameter boundary"
    );
}

#[test]
fn multiline_function_symbol_seed_resolves_parameter_node() {
    let fixture = fixture(&[("app.py", "def f(\n    user\n):\n    sink(user)\n")]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.py".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("multiline parameter symbol seed should resolve");

    assert_eq!(
        evidence.reasoning.as_ref().unwrap().reachability,
        Some(Reachability::Reached)
    );
}

#[test]
fn cross_function_sink_reached_via_exact_descent() {
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    g(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::Reached));
    assert!(!evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
        )
    }));
    let graph = evidence.graph.as_ref().expect("witness graph");
    assert!(
        graph.edges.iter().any(|edge| edge.kind == "CallDescent"),
        "descent edge should be represented in witness graph"
    );
}

#[test]
fn name_only_backed_hop_stays_boundary_exited() {
    let fixture = fixture(&[
        ("app.py",
         "class A:\n    def m(self, p):\n        sink(p)\n\ndef f(obj):\n    user = input()\n    obj.m(user)\n",
        )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 6,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 3,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::BoundaryExited));
    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
        )
    }));
}

#[test]
fn descent_depth_bound_yields_boundary_exited() {
    let fixture = fixture(&[
        ("app.py",
         "def h(p):\n    sink(p)\n\ndef g(p):\n    h(p)\n\ndef f(p):\n    g(p)\n\ndef top():\n    user = input()\n    f(user)\n",
        )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 11,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::BoundaryExited));
    assert!(reasoning.per_sink.iter().all(|sink| {
        matches!(
            &sink.sink,
            SymbolRef::Variable { path, .. } if path == "p"
                && sink.sources.iter().any(|source| source.reachability == Reachability::BoundaryExited)
        )
    }));
    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
        )
    }));
}

#[test]
fn descended_witness_surface() {
    // T4b: the witness graph carries a `CallDescent` edge and `descent_depth` records the number of
    // descent hops on the winning witness chain — both Stage B (additive) surfaces.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    g(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert_eq!(source.descent_depth, 1);

    let graph = evidence.graph.as_ref().expect("witness graph");
    assert!(
        graph.edges.iter().any(|e| e.kind == "CallDescent"),
        "witness graph must contain the descent edge: {:?}",
        graph.edges
    );
}

#[test]
fn frontier_mode_has_descended_frontier() {
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    g(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        None,
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(reasoning.reachability.is_none());
    assert!(reasoning.per_sink.is_empty());
    assert!(
        evidence.items.iter().any(
            |item| matches!(&item.symbol, Some(SymbolRef::Variable { path, .. }) if path == "p")
        ),
        "frontier should include reached callee sink argument through descent"
    );
    assert_eq!(evidence.graph, None);
    assert!(!evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
        )
    }));
}

#[test]
fn field_qualified_call_argument_reports_boundary_exited() {
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    o.data = input()\n    g(o.data)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert_eq!(reasoning.reachability, Some(Reachability::Reached));
    assert!(
        !evidence.warnings.iter().any(|warning| {
            matches!(
                &warning.kind,
                WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
            )
        }),
        "field-qualified arg->param flow may descend, so no boundary warning"
    );
}

#[test]
fn sink_resolution_errors_are_hard_failures() {
    let fixture = fixture(&[("app.py", "def f():\n    user = input()\n")]);
    let error = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 99,
        }]),
    )
    .expect_err("bad sink should be a hard error");

    assert!(matches!(error, QueryError::LocationOutOfRange { .. }));
}

// --- P10: path-proven sanitizer verdicts -----------------------------------------------------

fn sink_source<'a>(
    evidence: &'a prism::navigation::types::Evidence,
) -> &'a prism::navigation::types::SinkSourceResult {
    evidence
        .reasoning
        .as_ref()
        .expect("reasoning summary")
        .per_sink
        .first()
        .expect("one sink")
        .sources
        .first()
        .expect("one source")
}

#[test]
fn path_proven_sanitizer_downgrades_verdict_to_sanitized() {
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    safe = html.escape(user)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert_eq!(source.sanitized_by.len(), 1, "{:?}", source.sanitized_by);
    let site = &source.sanitized_by[0];
    assert_eq!(site.callee_text, "html.escape");
    assert_eq!(site.file, "app.py");
    assert_eq!(site.line, 3);

    let graph = evidence.graph.as_ref().expect("witness graph");
    assert!(
        graph.nodes.iter().any(|n| matches!(
            &n.symbol,
            Some(SymbolRef::Statement { kind, line, .. }) if kind == "SanitizerCall" && *line == 3
        )),
        "witness graph must contain the sanitizer call site node: {:?}",
        graph.nodes
    );
    assert!(
        graph.edges.iter().any(|e| e.kind == "SanitizedBy"),
        "witness graph must contain a SanitizedBy edge: {:?}",
        graph.edges
    );

    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::Cleansed { .. })
        ) && warning
            .message
            .contains("sanitizes this path via html.escape")
    }));
}

#[test]
fn raw_variable_bypassing_sanitizer_stays_reached() {
    // `safe` is sanitized, but the sink consumes the RAW `user` — the sanitizer sits in the
    // function body but not on THIS path. Must not be a false negative (verdict stays Reached).
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    safe = html.escape(user)\n    sink(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
    // Function-body presence still fires the advisory Cleansed warning (unchanged behavior).
    assert!(evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::Cleansed { .. })
        )
    }));
}

#[test]
fn multi_hop_assignment_chain_through_sanitizer_is_sanitized() {
    // `a = escape(u); b = a; sink(b)` — the sanitizer transition is one hop back from the sink;
    // the rest of the chain is plain AssignmentPropagation. Verdict must still flip via chain
    // transitivity.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    u = input()\n    a = html.escape(u)\n    b = a\n    sink(b)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert_eq!(source.sanitized_by.len(), 1, "{:?}", source.sanitized_by);
    assert_eq!(source.sanitized_by[0].callee_text, "html.escape");
}

#[test]
fn sanitizer_in_a_different_function_does_not_affect_verdict() {
    // The sanitizer call lives in `helper`, not `f` (the source's own function) — body-presence
    // detection itself misses it (scoped to the source's enclosing function), so behavior is
    // completely unchanged: no Cleansed warning, no sanitized_by, ordinary Reached.
    let fixture = fixture(&[(
        "app.py",
        "def helper(x):\n    return html.escape(x)\n\ndef f():\n    user = input()\n    sink(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 6,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty());
    assert!(!evidence.warnings.iter().any(|warning| {
        matches!(
            &warning.kind,
            WarningKind::Reasoning(ReasoningWarning::Cleansed { .. })
        )
    }));
}

#[test]
fn frontier_mode_has_no_sanitized_verdict() {
    // Frontier mode (no sinks) has no verdict at all — Sanitized is a witness-mode-only concept.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    safe = html.escape(user)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        None,
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(reasoning.reachability.is_none());
    assert!(reasoning.per_sink.is_empty());
}

#[test]
fn js_declaration_form_path_proven_sanitizer_is_sanitized() {
    // `const safe = escape(user)` — the JS/TS `variable_declarator` declaration form (not an
    // `assignment_expression`); codex catch: the generic assignment helpers alone don't cover
    // this, `declaration_value`/`declaration_name` must.
    let fixture = fixture(&[(
        "app.js",
        "function f() {\n  var user = input();\n  const safe = escape(user);\n  sink(safe);\n}\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.js".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.js".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert_eq!(source.sanitized_by.len(), 1, "{:?}", source.sanitized_by);
    assert_eq!(source.sanitized_by[0].callee_text, "escape");
    assert_eq!(source.sanitized_by[0].file, "app.js");
    assert_eq!(source.sanitized_by[0].line, 3);
}

#[test]
fn js_declaration_form_bypass_stays_reached() {
    let fixture = fixture(&[(
        "app.js",
        "function f() {\n  var user = input();\n  const safe = escape(user);\n  sink(user);\n}\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.js".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.js".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty());
}

#[test]
fn go_paired_check_recognizer_is_excluded_from_verdict() {
    // `filepath.Clean` (Go path-traversal family) carries a `paired_check` — the BLOCKER ruling
    // excludes paired-check recognizers from verdict-affecting `Sanitized` entirely, even when
    // `strings.HasPrefix` is textually present too. Verdict must stay Reached (advisory-only).
    let fixture = fixture(&[(
        "app.go",
        "package m\n\nfunc f(base string, p string) string {\n\tclean := filepath.Clean(p)\n\tif strings.HasPrefix(clean, base) {\n\t\tsink(clean)\n\t}\n\treturn clean\n}\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.go".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.go".into(),
            line: 6,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(
        reasoning.per_sink.iter().any(|sink| sink
            .sources
            .iter()
            .any(|source| source.reachability == Reachability::Reached
                && source.sanitized_by.is_empty())),
        "paired-check recognizers must not produce a Sanitized verdict"
    );
}

// --- F1 BLOCKER: the Use must be inside the sanitizer's DATA-ARGUMENT span, not merely inside
// the call expression -----------------------------------------------------------------------

#[test]
fn second_argument_use_is_not_a_sanitizer_transition() {
    // `ast.rs::rvalue_identifier_spans_on_lines` records EVERY call argument as a Use, not just
    // the first. `user` sits in the SECOND argument position of `escape(other, user)` — the
    // recognizer's data argument is arg[0] (`other`), so the transform never runs on `user`.
    // Pre-fix, any Use inside the RHS call span (including arg[1]) matched the window, producing
    // a false `Sanitized`. Must stay Reached.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    other = 'x'\n    safe = escape(other, user)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

#[test]
fn tainted_callee_identifier_is_not_a_sanitizer_transition() {
    // `escape` is a local variable holding tainted input, then CALLED as `escape(other)` — the DFG
    // records the callee identifier itself as a Use, and pre-fix that Use sat inside the RHS call
    // span too. The callee span is never the data argument, so this must not flip to Sanitized.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    escape = input()\n    safe = escape(other)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}
// --- F2 BLOCKER: language applicability in the VERDICT path --------------------------------

#[test]
fn go_cross_language_recognizer_name_does_not_produce_sanitized_verdict() {
    // Go is `sanitizer_supported` (via the paired-check path family), but `escape` is a bare-name
    // recognizer registered for Python/JS in `active_recognizers()`. `call_path_matches`'s
    // exact-match branch is NOT gated by language, so pre-fix `sanitizer_call_site` matched a Go
    // `escape(user)` call against the Python/JS entry, producing a false Sanitized verdict.
    let fixture = fixture(&[(
        "app.go",
        "package m\n\nfunc f(user string) string {\n\tsafe := escape(user)\n\tsink(safe)\n\treturn safe\n}\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Symbol {
            name: "f".into(),
            file: Some("app.go".into()),
        }],
        Some(&[SeedSpec::Loc {
            file: "app.go".into(),
            line: 5,
        }]),
    )
    .expect("taint_reaches");

    let reasoning = evidence.reasoning.as_ref().expect("reasoning summary");
    assert!(
        reasoning.per_sink.iter().any(|sink| sink
            .sources
            .iter()
            .any(|source| source.reachability == Reachability::Reached
                && source.sanitized_by.is_empty())),
        "a same-name recognizer from another language's table must not produce a Sanitized verdict for Go"
    );
}

// --- F3 BLOCKER: Python `keyword_argument` must not be treated as the data-argument span ---------
//
// `ast.rs::rvalue_identifier_spans_on_lines` records EVERY identifier under a call's argument list
// as a Use — including BOTH the `name` (label) and `value` sides of a Python `keyword_argument`
// node. Pre-fix, "first named child of the argument list" accepted the WHOLE `keyword_argument`
// span, so a Use sitting in the keyword LABEL (which can coincidentally share text with a real
// variable name) or in an unrelated tainted keyword argument would still flip the verdict to
// `Sanitized`. The fix requires: for a keyword form, the Use must sit inside the VALUE span of the
// keyword_argument whose NAME equals the recognizer's `data_param` (`"s"` for `html.escape`).

#[test]
fn python_keyword_label_use_is_not_a_sanitizer_transition() {
    // The tainted variable is named `s` — the SAME text as `html.escape`'s data-parameter name.
    // `html.escape(s=other)` passes untainted `other` as the data value; the only "s"-shaped Use
    // near the call is the keyword LABEL itself, not a reference to the tainted variable. Must stay
    // Reached.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    s = input()\n    other = 'x'\n    safe = html.escape(s=other)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

#[test]
fn python_tainted_non_data_kwarg_is_not_a_sanitizer_transition() {
    // `html.escape(quote=quote, s=other)` — the tainted value flows into `quote`, the CONFIG kwarg
    // (controls quote-character escaping), not `s`, the data kwarg. `other` (the actual `s=` value)
    // is untainted. The sanitizer transform never runs on the tainted value on this path.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    quote = input()\n    other = 'x'\n    safe = html.escape(quote=quote, s=other)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

// --- P14 Stage B: sanitizer contract survives interprocedural descent ----------------------

#[test]
fn sanitize_then_pass_across_descent_is_sanitized() {
    // The sanitizer transition (`safe = html.escape(user)`) is INTRA-caller; `safe` is then passed
    // across an Exact-gated descent into `g`, which sinks the (now-sanitized) parameter directly.
    // The transition is provable independent of the descent hop, so the verdict must stay Sanitized.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    safe = html.escape(user)\n    g(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert_eq!(source.sanitized_by.len(), 1, "{:?}", source.sanitized_by);
    assert_eq!(source.sanitized_by[0].callee_text, "html.escape");
    assert_eq!(
        source.descent_depth, 1,
        "the winning witness chain crosses exactly one descent hop"
    );

    let graph = evidence.graph.as_ref().expect("witness graph");
    assert!(
        graph.edges.iter().any(|e| e.kind == "CallDescent"),
        "witness graph must retain the descent edge alongside the sanitizer step: {:?}",
        graph.edges
    );
}

#[test]
fn callee_body_sanitizer_bypass_stays_reached() {
    // `g` sanitizes its param into an unused local (`safe = html.escape(p)`) but sinks the RAW `p`
    // directly. The sanitizer transition never runs on the value that reaches the sink, so the
    // verdict must stay Reached. This also pins that the arg->param `CallDescent` window itself is
    // never (mis)evaluated as a sanitizer transition.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    safe = html.escape(p)\n    sink(p)\n\ndef f():\n    user = input()\n    g(user)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 6,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 3,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
    assert_eq!(source.descent_depth, 1);
}

#[test]
fn python_keyword_data_arg_is_a_sanitizer_transition() {
    // `html.escape(s=user)` — the tainted value IS passed via the data kwarg by name. This is a
    // genuine sanitizer transition and must flip to Sanitized, symmetric with the positional form.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    safe = html.escape(s=user)\n    sink(safe)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 4,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert_eq!(source.sanitized_by.len(), 1, "{:?}", source.sanitized_by);
    assert_eq!(source.sanitized_by[0].callee_text, "html.escape");
}

// --- P14 fix-wave BLOCKER: bypass-proven `Sanitized` (a `Sanitized` downgrade must be sound
// against every route to the sink, not just whichever first-enqueue-wins chain happened to reach
// it first). See src/reasoning/taint_reaches.rs's `witness_mode` and
// src/reasoning/sanitizer_walk.rs's `sanitizer_bypass_exclusions`. ---

#[test]
fn sanitizer_bypass_via_second_call_stays_reached() {
    // T-F1, THE regression pin. `g(safe)` is called before `g(raw)`; `g`'s single physical
    // parameter node is a confluence point, so first-enqueue-wins dedup means the ONLY witness
    // chain that can ever reach the sink winds through `safe` (the sanitized value) — but
    // `g(raw)` proves an actual, unsanitized route ALSO reaches the sink. Before this fix, the
    // sanitizer transition proven on the (sole, arbitrarily-selected) winning chain downgraded
    // the verdict to `Sanitized` even though `raw` bypasses it entirely: a false `Sanitized`.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    safe = html.escape(user)\n    raw = user\n    g(safe)\n    g(raw)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

#[test]
fn both_calls_sanitized_is_sanitized() {
    // T-F2: `g(safe1); g(safe2)` — BOTH arguments are, SEPARATELY, sanitizer outputs. Only ONE of
    // the two sanitizer transitions can ever appear on any single witness chain to `g`'s shared
    // parameter node (first-enqueue-wins), but excluding BOTH proven transitions (found by
    // scanning the whole first-parent tree, not just the winning chain) disconnects the sink
    // entirely — the verdict must be `Sanitized`.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    sink(p)\n\ndef f():\n    user = input()\n    safe1 = html.escape(user)\n    safe2 = html.escape(user)\n    g(safe1)\n    g(safe2)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 5,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert!(!source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

#[test]
fn callee_internal_sanitizer_covers_all_entries() {
    // T-F3: `g(a); g(b)` — two DIFFERENT callers reach `g`'s shared parameter, but the sanitizer
    // transition sits INSIDE `g`, downstream of the confluence node (`safe = html.escape(p);
    // sink(safe)`). Whichever caller wins the race, its flow still passes through the same proven
    // transition before the sink — cutting it disconnects every entry, so the verdict must be
    // `Sanitized`.
    let fixture = fixture(&[(
        "app.py",
        "def g(p):\n    safe = html.escape(p)\n    sink(safe)\n\ndef f():\n    a = input()\n    b = a\n    g(a)\n    g(b)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 6,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 3,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Sanitized);
    assert!(!source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}

#[test]
fn intra_function_confluence_bypass_stays_reached() {
    // T-F4: the intra-function analogue of T-F1's hazard, with NO interprocedural descent
    // involved at all. `x = safe or raw` gives `x`'s Def node two same-line `Use` parents (`safe`
    // and `raw`) via `AssignmentPropagation` — a genuine confluence node, just like a shared
    // callee parameter. `safe` is sanitized; `raw` is the raw source value; first-enqueue-wins
    // means the ORIGINAL winning chain reaches `x` via `safe` (so the old chain-scoped check would
    // wrongly downgrade to `Sanitized`), but `raw`'s route bypasses the sanitizer just as `g(raw)`
    // does in T-F1. Verdict must stay Reached.
    let fixture = fixture(&[(
        "app.py",
        "def f():\n    user = input()\n    safe = html.escape(user)\n    raw = user\n    x = safe or raw\n    sink(x)\n",
    )]);
    let evidence = taint_reaches(
        &fixture.session,
        &[SeedSpec::Loc {
            file: "app.py".into(),
            line: 2,
        }],
        Some(&[SeedSpec::Loc {
            file: "app.py".into(),
            line: 6,
        }]),
    )
    .expect("taint_reaches");

    let source = sink_source(&evidence);
    assert_eq!(source.reachability, Reachability::Reached);
    assert!(source.sanitized_by.is_empty(), "{:?}", source.sanitized_by);
}
