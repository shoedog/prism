use super::*;
use prism::cpg::{LossReason, Relation, ReturnFlowMode};
use prism::navigation::types::{ReasoningWarning, WarningKind};
use prism::reasoning::order::AstOrderView;
use prism::reasoning::types::SinkSourceResult;

pub(super) fn check_walk_reason(row: &Row, source: &SinkSourceResult) {
    if row.pending_issue(Level::Reasons).is_some() {
        return;
    }
    if source.reachability != Reachability::BoundaryExited {
        assert_eq!(source.reason, None, "{}", row.id);
        return;
    }
    let Some((_, Some(reasons))) = active_cell(row, enabled_groups().contains(&row.group)) else {
        return;
    };
    let expected: Vec<_> = reasons
        .walk
        .iter()
        .filter(|reason| {
            matches!(
                reason.as_str(),
                "target_not_binding"
                    | "no_matching_site"
                    | "ambiguous_site"
                    | "not_exact"
                    | "unproven_slot"
                    | "depth_cap"
                    | "policy_ceiling"
                    | "self_param"
                    | "self_return"
            )
        })
        .collect();
    if !expected.is_empty() {
        let reason = source
            .reason
            .expect("refused hop reason for the marked pair");
        assert!(
            expected.contains(&&reason.to_string()),
            "{}: expected {expected:?}, got {reason}",
            row.id
        );
    }
}

pub(super) fn check_deferred_refusals(
    row: &Row,
    session: &NavigationSession,
    summary: &prism::reasoning::types::ReasoningSummary,
) {
    if row.group != "option_b" {
        return;
    }
    let Some(current) = current_cell(row, enabled_groups().contains(&row.group)) else {
        return;
    };
    assert_eq!(
        summary.reachability.map(Verdict::from),
        Some(current.expect.walk.reaches),
        "{}: line verdict",
        row.id
    );
    let resolved = resolve(session, &[row.source_seed()], SeedRole::Source).unwrap();
    let mut roots = resolved
        .seeds
        .iter()
        .flat_map(|seed| seed.nodes.iter().zip(&seed.locations))
        .filter(|(_, loc)| {
            loc.file == row.src.file && [loc.start_byte, loc.end_byte] == row.src.bytes
        });
    let (&root, _) = roots.next().expect("marked source occurrence");
    assert!(
        roots.next().is_none(),
        "{}: ambiguous marked source",
        row.id
    );
    let cpg = session.index.cpg();
    let order = AstOrderView::new(cpg, &session.repo.files);
    let trace = cpg.taint_trace_nodes_with_mode(&[root], Some(&order), ReturnFlowMode::On);
    for reason in &current.reasons.as_ref().expect("live refusal reasons").walk {
        assert!(
            trace
                .boundary
                .iter()
                .any(|edge| edge.root == root && edge.reason.to_string() == *reason),
            "{}: missing live {reason} refusal for the marked root",
            row.id
        );
    }
}

fn row(id: &str) -> Row {
    rows().into_iter().find(|row| row.id == id).unwrap()
}

#[test]
fn f01_checks_each_roots_own_reason() {
    let mut row = row("f01");
    row.group = "reasons".into();
    row.current = None;
    row.sink.line = 6;
    row.sink.operand = [96, 97];
    let original = Fixture::load(&row);
    assert_eq!(walk(&row, &original).reaches, Verdict::BoundaryExited);
    let original_root = row.src.bytes;
    row.src.line = 16;
    row.src.bytes = [208, 209];
    assert_eq!(walk(&row, &original).reaches, Verdict::BoundaryExited);

    row.src.line = 15;
    row.src.bytes = original_root;
    let fixture = Fixture::load_with(&row, |_, source| {
        *source = source.replace(
            "const t = getInput();",
            "const t = getInput(); const u = 0;",
        );
    });
    assert_eq!(walk(&row, &fixture).reaches, Verdict::BoundaryExited);
    let text = &fixture.repo.files[&row.src.file].source;
    let unrelated = text.find("u = 0").unwrap();
    row.src.bytes = [unrelated, unrelated + 1];
    assert_eq!(walk(&row, &fixture).reaches, Verdict::NotReached);
}

fn fan_reaches(row: &Row, fixture: &Fixture, return_flow: bool) -> bool {
    let (_, trace) = slice_with_trace(
        &fixture.context(),
        &fixture.diff(),
        &row.config(return_flow),
    )
    .unwrap();
    trace.roots.iter().any(|root| {
        root.origin == TaintRootOrigin::Explicit
            && root.location.file == row.src.file
            && [root.location.start_byte, root.location.end_byte] == row.src.bytes
            && root.reached.iter().any(|target| {
                target.location.file == row.sink.file
                    && [target.location.start_byte, target.location.end_byte] == row.sink.operand
            })
    })
}

#[test]
fn reason_audit_x01_separates_uncertified_review_assignment_from_trace_cut() {
    let row = row("x01");
    let fixture = Fixture::load(&row);
    assert_eq!(review(&row, &fixture, false), Finding::Unlabeled);
    assert!(fan_reaches(&row, &fixture, false));
    let session = fixture.session();
    let cpg = session.index.cpg();
    let source = &fixture.repo.files[&row.src.file].source;
    let at = |byte| {
        cpg.node_indices().find(|&node| {
        matches!(cpg.node(node), CpgNode::Variable { start_byte, .. } if *start_byte == byte)
    }).unwrap()
    };
    let root = at(row.src.bytes[0]);
    let def = at(source.find("z=forget").unwrap());
    let actual = at(source.find("forget(t)").unwrap() + "forget(".len());
    let legacy = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::Off);
    assert_eq!(
        legacy.parents_by_root[&(root, def)],
        (actual, Relation::AssignmentPropagation)
    );
    assert_eq!(review(&row, &fixture, true), Finding::None);
    let fixed = Fixture::load_with(&row, |_, source| {
        *source = source.replace("forget(t)", "forget(0)")
    });
    assert_eq!(review(&row, &fixed, false), Finding::None);
}

#[test]
fn reason_audit_self_return_and_missing_review_return_flow() {
    let row = row("s1-js-self-return");
    let fixture = Fixture::load(&row);
    let session = fixture.session();
    let evidence = taint_reaches(
        &session,
        &[row.source_seed()],
        Some(&[SeedSpec::Loc {
            file: row.sink.file.clone(),
            line: row.sink.line,
        }]),
    )
    .unwrap();
    assert_eq!(
        evidence.reasoning.as_ref().unwrap().reachability,
        Some(Reachability::BoundaryExited)
    );
    assert!(evidence.warnings.iter().any(|warning| {
        warning.message == "taint reached a self-function return boundary at y"
            && matches!(
                &warning.kind,
                WarningKind::Reasoning(ReasoningWarning::InterproceduralBoundary {
                    reason: Some(LossReason::SelfReturn),
                    ..
                })
            )
    }));
    assert_eq!(review(&row, &fixture, false), Finding::None);
    assert_eq!(review(&row, &fixture, true), Finding::None);
    let split = Fixture::load_with(&row, |_, source| {
        *source = source
            .replace(
                "function run(n) {\n  if (n === 0) {",
                "function base(n) {\n  {",
            )
            .replace(
                "  const y = run(0);",
                "}\nfunction run(n) { const y = base(0);",
            );
    });
    let mut split_row = row;
    split_row.sink.line += 1;
    assert_eq!(review(&split_row, &split, false), Finding::None);
    assert_eq!(review(&split_row, &split, true), Finding::Unlabeled);
}

#[test]
fn reason_audit_long_continuation_separates_cfg_and_matcher_admission() {
    let commented = row("s1-js-long-continuation");
    let whitespace = row("d-js-long-continuation-space");
    for (row, kind, bytes, trace_grade) in [
        (&commented, "comment", [99, 123], Finding::None),
        (&whitespace, "identifier", [708, 709], Finding::Exact),
    ] {
        let fixture = Fixture::load(row);
        let call = syntax_node(
            &fixture.repo.files[&row.sink.file],
            row.site.as_ref().unwrap().bytes,
        );
        let selected = prism::ast::tree_helpers::call_arg_node(&call, 0).unwrap();
        assert_eq!(selected.kind(), kind);
        assert_eq!([selected.start_byte(), selected.end_byte()], bytes);
        assert!(!fan_reaches(row, &fixture, false));
        assert!(fan_reaches(row, &fixture, true));
        assert_eq!(review(row, &fixture, false), Finding::None);
        assert_eq!(review(row, &fixture, true), trace_grade);
    }
}

#[test]
fn reason_audit_uncertain_dispatch_and_operand_projection() {
    let uncertain = row("s2-js-mixed-bound-alias");
    let fixture = Fixture::load(&uncertain);
    assert_eq!(review(&uncertain, &fixture, false), Finding::NameOnly);
    assert!(fan_reaches(&uncertain, &fixture, false));
    assert_eq!(review(&uncertain, &fixture, true), Finding::None);
    let fixed = Fixture::load_with(&uncertain, |_, source| {
        *source = source.replace("other.run(t)", "other.run(0)")
    });
    assert_eq!(review(&uncertain, &fixed, false), Finding::None);

    let projection = row("tr5-js-two-roots");
    let fixture = Fixture::load(&projection);
    for return_flow in [false, true] {
        assert!(!fan_reaches(&projection, &fixture, return_flow));
        assert_eq!(review(&projection, &fixture, return_flow), Finding::Exact);
        let (_, trace) = slice_with_trace(
            &fixture.context(),
            &fixture.diff(),
            &projection.config(return_flow),
        )
        .unwrap();
        assert!(trace.roots.iter().any(|root| root.location.path.base == "u"
            && [root.location.start_byte, root.location.end_byte] != projection.src.bytes
            && root.reached.iter().any(|target| [
                target.location.start_byte,
                target.location.end_byte
            ] == projection.sink.operand)));
    }
}

#[test]
fn reason_audit_configuration_admission_and_source_row_mask() {
    let configured = row("e2-flat-configured");
    let fixture = Fixture::load(&configured);
    for return_flow in [false, true] {
        assert!(fan_reaches(&configured, &fixture, return_flow));
        assert_eq!(review(&configured, &fixture, return_flow), Finding::None);
        let mut config = configured.config(return_flow);
        config.extra_sinks.push("=customObserve".into());
        assert_ne!(review_with(&configured, &fixture, &config), Finding::None);
    }
    let mut masked = row("e2-framework-source-guard");
    let fixture = Fixture::load(&masked);
    for return_flow in [false, true] {
        assert_eq!(review(&masked, &fixture, return_flow), Finding::None);
    }
    let split = Fixture::load_with(&masked, |_, source| {
        *source = source.replace("; db.Query(t)", "\n  db.Query(t)")
    });
    masked.sink.line += 1;
    for return_flow in [false, true] {
        assert_eq!(review(&masked, &split, return_flow), Finding::Exact);
    }
}
