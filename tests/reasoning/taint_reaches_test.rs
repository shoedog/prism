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
fn cross_function_sink_reports_boundary_exited_and_warning() {
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

    assert_eq!(
        evidence.reasoning.as_ref().unwrap().reachability,
        Some(Reachability::BoundaryExited)
    );
    assert!(
        evidence.warnings.iter().any(|warning| {
            matches!(
                &warning.kind,
                WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
            )
        }),
        "boundary witness mode should emit an interprocedural warning"
    );
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
    assert_eq!(reasoning.reachability, Some(Reachability::BoundaryExited));
    assert!(
        evidence.warnings.iter().any(|warning| {
            matches!(
                &warning.kind,
                WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary { .. })
            )
        }),
        "field-qualified arg->param flow should be recorded as an interprocedural boundary"
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
