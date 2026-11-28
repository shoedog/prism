use super::*;
use crate::resolution::{DropReason, ResolutionKind, ResolutionOutcome, ResolvedCallee};

fn recompute(
    fixture: &mut Fixture,
    parsed: &BTreeMap<String, ParsedFile>,
    supplied: SuppliedCall<'_>,
) {
    assert!(fixture
        .graph
        .calls
        .entry(supplied.site.caller.clone())
        .or_default()
        .insert(supplied.site.clone()));
    fixture.index = compute_index(&fixture.graph, parsed, Some(supplied));
    fixture.targets("good('control')", &["verified"]);
}

fn parse(fixture: &Fixture, language: Language) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(
        "m.js".into(),
        ParsedFile::parse("m.js", &fixture.source, language).unwrap(),
    )])
}

#[test]
fn refuses_unproved_holder_outcomes() {
    assert_holder_outcomes(false);
}

#[test]
fn refuses_unproved_holder_outcomes_at_relay() {
    assert_holder_outcomes(true);
}

fn assert_holder_outcomes(relay: bool) {
    for failure in ["dropped", "name_only", "multi_target", "wrong_holder"] {
        let mut fixture = direct_case(
            "function holder(cb) { cb('mixed'); } function forward(value) { holder(value); }",
            "function decoy() {} holder(verified); holder(stale); forward(stale);",
            Language::JavaScript,
        );
        let (subject, other) = if relay {
            ("forward(stale)", "holder(stale)")
        } else {
            ("holder(stale)", "forward(stale)")
        };
        fixture.targets("cb('mixed')", &["stale", "verified"]);
        let site = fixture.site(subject).unwrap().clone();
        let other = fixture.site(other).unwrap().clone();
        let outcome = fixture.graph.resolve_call_site_full(&site);
        let [exact] = outcome.resolved.as_slice() else {
            panic!("expected singleton Exact for {subject}: {outcome:?}");
        };
        assert_eq!(exact.confidence, ResolutionConfidence::Exact);
        assert_eq!(exact.target.name, if relay { "forward" } else { "holder" });
        let holder = exact.target.clone();
        let decoy = fixture.graph.functions["decoy"][0].clone();
        let exact = ResolvedCallee {
            target: &holder,
            ..exact.clone()
        };
        let wrong = ResolvedCallee {
            target: &decoy,
            ..exact.clone()
        };
        let outcome = match failure {
            "dropped" => ResolutionOutcome::dropped(DropReason::UnknownName),
            "name_only" => ResolutionOutcome::hit(vec![ResolvedCallee {
                confidence: ResolutionConfidence::NameOnly,
                ..exact
            }]),
            "multi_target" => ResolutionOutcome::hit(vec![wrong, exact]),
            "wrong_holder" => ResolutionOutcome::hit(vec![wrong]),
            _ => unreachable!(),
        };
        let sites = fixture.graph.calls.get_mut(&site.caller).unwrap();
        assert!(sites.remove(&site));
        assert!(sites.remove(&other));
        let parsed = parse(&fixture, Language::JavaScript);
        recompute(
            &mut fixture,
            &parsed,
            SuppliedCall {
                site: &site,
                outcome,
                arguments: None,
            },
        );
        fixture.targets("cb('mixed')", &["verified"]);
        assert_eq!(
            fixture.index.refusal_count("holder_outcome"),
            1,
            "{subject}: {failure}"
        );
    }
}

#[test]
fn refuses_existing_derived_ground_during_recompute() {
    for (origin, kind, pre_resolved) in [
        (CallSiteOrigin::IndirectResolution, CallKind::Call, false),
        (CallSiteOrigin::Source, CallKind::Call, true),
        (CallSiteOrigin::Source, CallKind::MacroInvocation, false),
    ] {
        let mut fixture = direct_case(
            "function holder(cb) { cb('mixed'); }",
            "holder(verified); holder(stale);",
            Language::JavaScript,
        );
        fixture.targets("cb('mixed')", &["stale", "verified"]);
        let ordinary = fixture.site("holder(stale)").unwrap().clone();
        let holder = fixture.graph.functions["holder"][0].clone();
        assert!(fixture
            .graph
            .calls
            .get_mut(&ordinary.caller)
            .unwrap()
            .remove(&ordinary));
        let site = CallSite {
            origin,
            kind,
            pre_resolved_target: pre_resolved.then(|| holder.clone()),
            ..ordinary
        };
        let outcome = ResolutionOutcome::hit(vec![ResolvedCallee {
            target: &holder,
            confidence: ResolutionConfidence::Exact,
            kind: ResolutionKind::FreeSingle,
        }]);
        let parsed = parse(&fixture, Language::JavaScript);
        recompute(
            &mut fixture,
            &parsed,
            SuppliedCall {
                site: &site,
                outcome,
                arguments: None,
            },
        );
        fixture.targets("cb('mixed')", &["verified"]);
        assert_eq!(
            fixture.index.refusal_count("source"),
            1,
            "{origin:?} {kind:?} {pre_resolved}"
        );
    }
}

#[test]
fn supplied_constructor_refuses_even_with_exact_target_and_argument_occurrences() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let mut fixture = direct_case(
            "class Runner { constructor(cb, input) { cb(input); } }\nclass Other { constructor(unused) {} }",
            "new Runner(stale, verified);",
            language,
        );
        let invocation = "new Runner(stale, verified)";
        assert!(fixture.site(invocation).is_none(), "{language:?}");
        fixture.refuses("cb(input)", "no_caller");
        let start_byte = fixture.source.find(invocation).unwrap();
        let end_byte = start_byte + invocation.len();
        let parsed = parse(&fixture, language);
        let caller = &parsed["m.js"];
        assert!(caller
            .call_argument_texts_and_spans_of_call(start_byte, end_byte, "Runner")
            .is_empty());
        let node = caller
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .unwrap();
        assert_eq!(node.kind(), "new_expression");
        let argument_list = node.child_by_field_name("arguments").unwrap();
        let arguments: Vec<_> = param_slot::children(argument_list)
            .map(|argument| (argument, argument.byte_range()))
            .collect();
        assert_eq!(
            arguments
                .iter()
                .map(|(_, span)| &fixture.source[span.clone()])
                .collect::<Vec<_>>(),
            ["stale", "verified"],
        );
        let site = CallSite {
            callee_name: "Runner".into(),
            start_byte,
            end_byte,
            arg_count: Some(arguments.len()),
            local_binding: JsLocalBinding::Unchecked,
            ..fixture.site("control(verified)").unwrap().clone()
        };
        let real = fixture.graph.functions["constructor"]
            .iter()
            .find(|id| id.start_line == site_line(&fixture.source, "constructor(cb, input)"))
            .unwrap()
            .clone();
        let wrong = fixture.graph.functions["constructor"]
            .iter()
            .find(|id| *id != &real)
            .unwrap()
            .clone();
        let mut bad_spans = arguments.clone();
        bad_spans[0].1 = arguments[1].1.clone();
        let mut reversed = arguments.clone();
        reversed.reverse();
        for (target, arguments, refusal) in [
            (&real, arguments.as_slice(), "source"),
            (&wrong, arguments.as_slice(), "source"),
            (&real, bad_spans.as_slice(), "source"),
            (&real, reversed.as_slice(), "source"),
        ] {
            let outcome = ResolutionOutcome::hit(vec![ResolvedCallee {
                target,
                confidence: ResolutionConfidence::Exact,
                kind: ResolutionKind::FreeSingle,
            }]);
            recompute(
                &mut fixture,
                &parsed,
                SuppliedCall {
                    site: &site,
                    outcome,
                    arguments: Some(arguments),
                },
            );
            fixture.refuses("cb(input)", refusal);
            assert!(fixture
                .graph
                .calls
                .get_mut(&site.caller)
                .unwrap()
                .remove(&site));
        }
    }
}

fn site_line(source: &str, text: &str) -> usize {
    source[..source.find(text).unwrap()].lines().count()
}

#[test]
fn refuses_missing_and_nonsource_ground() {
    let fixture = direct_case(
        "function holder(cb) { cb('mixed'); }",
        "holder(stale); holder(verified); holder`tag`;",
        Language::JavaScript,
    );
    fixture.targets("cb('mixed')", &["stale", "verified"]);
    let tag_start = fixture.source.find("holder`tag`").unwrap();
    assert!(fixture
        .graph
        .calls
        .values()
        .flatten()
        .any(|site| site.start_byte == tag_start && site.origin == CallSiteOrigin::Source));
    assert_eq!(fixture.index.refusal_count("source"), 1);
    let mut graph = fixture.graph;
    let start = fixture.source.find("holder(stale)").unwrap();
    let caller = graph
        .calls
        .values()
        .flatten()
        .find(|site| site.start_byte == start)
        .unwrap()
        .clone();
    let sites = graph.calls.get_mut(&caller.caller).unwrap();
    assert!(sites.remove(&caller));
    sites.insert(CallSite {
        origin: CallSiteOrigin::IndirectResolution,
        ..caller
    });
    let parsed = ParsedFile::parse("m.js", &fixture.source, Language::JavaScript).unwrap();
    let index = compute(&graph, &BTreeMap::from([("m.js".to_string(), parsed)]));
    let modified = Fixture {
        graph,
        index,
        source: fixture.source,
    };
    modified.targets("cb('mixed')", &["verified"]);
    modified.targets("good('control')", &["verified"]);
    assert_eq!(modified.index.refusal_count("source"), 2);

    let setter = direct_case(
        "class Service { set action(cb) { cb('setter'); } }",
        "const service = new Service(); service.action = stale;",
        Language::JavaScript,
    );
    let write = setter.source.find("service.action = stale").unwrap();
    assert!(!setter
        .graph
        .calls
        .values()
        .flatten()
        .any(|site| site.start_byte == write));
    setter.refuses("cb('setter')", "no_caller");
}
