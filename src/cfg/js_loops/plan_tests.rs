//! #693/#697 option B, slice 2: the plan's verdict per tree, its counters and its one-plan consistency. The
//! sources are the hand-written fixtures of `cpg::reaching::loop_plan_tests` (whose label and CFG expectations
//! are RED on 874ce2c3); here the reason each is refused is pinned.
use super::{LoopPlan, Why};
use crate::ast::ParsedFile;
use crate::cpg::loop_plan_tests::{ADMITTED, REFUSED};
use crate::languages::Language;

fn plan(src: &str) -> LoopPlan {
    ParsedFile::parse("a.js", src, Language::JavaScript)
        .unwrap()
        .js_loop_plan()
        .clone()
}

fn reason(w: &Option<Why>) -> String {
    match w {
        None => "admitted".into(),
        Some(Why::Legacy(l)) => format!("Legacy({l:?})"),
        Some(Why::Placement) => "Placement".into(),
        Some(Why::Unresolved(u)) => format!("Unresolved({u:?})"),
        Some(Why::TryFlow) => "TryFlow".into(),
        Some(Why::OwnerSibling) => "OwnerSibling".into(),
    }
}

#[test]
fn each_refusal_fixture_is_refused_for_its_reason() {
    for c in REFUSED {
        let p = plan(c.src);
        assert_eq!(p.verdicts.len(), 1, "{}", c.name);
        let v = &p.verdicts[0];
        assert_eq!(reason(&v.why), c.why, "{}: {}", c.name, v.first);
        assert!(
            !v.first.is_empty(),
            "{}: the trace names an offender",
            c.name
        );
        assert!(p.refuses_owner(v.owner), "{}: the owner is refused", c.name);
    }
}

#[test]
fn the_admitted_controls_are_admitted_and_refuse_no_owner() {
    for (name, src, _) in ADMITTED {
        let p = plan(src);
        assert_eq!(p.verdicts.len(), 1, "{name}");
        assert_eq!(reason(&p.verdicts[0].why), "admitted", "{name}");
        assert!(!p.refuses_owner(p.verdicts[0].owner), "{name}");
    }
}

#[test]
fn placement_is_the_primary_reason_only_when_every_record_is_resolved() {
    // N1, Prettier and sol's two fixtures have no unresolved record (all are owned or independent), so the
    // placement rule alone refuses them; a head collapse is also a placement violation (the body occurrence is
    // placed at the head) and is reported as both.
    let collapse = plan(
        REFUSED
            .iter()
            .find(|c| c.name == "head_collapse")
            .unwrap()
            .src,
    );
    assert_eq!(collapse.verdicts[0].overlaps, ["placement", "unresolved"]);
    let n1 = plan(REFUSED[0].src);
    assert_eq!(n1.verdicts[0].overlaps, ["placement"]);
}

#[test]
fn counters_name_every_tree_and_reason_of_a_file() {
    let src = format!(
        "{}{}{}{}",
        ADMITTED[0].1,
        REFUSED[0].src,
        REFUSED
            .iter()
            .find(|c| c.name == "head_collapse")
            .unwrap()
            .src,
        // The initializer writes `a`, the body too, neither test nor update does (`n--`): main's own fallback,
        // so legacy; its owner is label-refused too (fold 2).
        "function h(n) {\n  let a = 0;\n  for (a = 1; a < n; n--) {\n    a = 2;\n  }\n}\n"
    );
    let p = plan(&src);
    let c = p.counters();
    let want = [
        ("trees", 4),
        ("admitted", 1),
        ("refused.placement", 1),
        ("refused.unresolved.head_collapse", 1),
        ("refused.legacy.for_init", 1),
        ("overlap.placement", 2),
        ("overlap.unresolved", 1),
        ("owners_refused", 3),
    ];
    assert_eq!(c.len(), want.len(), "{c:?}");
    for (k, n) in want {
        assert_eq!(c.get(k), Some(&n), "{k}: {c:?}");
    }
}

#[test]
fn the_plan_is_deterministic_and_one_per_file() {
    let parsed = ParsedFile::parse("a.js", REFUSED[0].src, Language::JavaScript).unwrap();
    let a = parsed.js_loop_plan();
    assert!(std::ptr::eq(a, parsed.js_loop_plan()));
    assert_eq!(a.counters(), plan(REFUSED[0].src).counters());
}

#[test]
fn only_the_owner_of_a_refused_tree_is_refused() {
    // The callback's loop is refused (head collapse); the enclosing function holds no loop tree of its own,
    // so the arrow's owner is refused (main's labels) and the function's is not.
    let src = REFUSED
        .iter()
        .find(|c| c.name == "callback_head_collapse")
        .unwrap()
        .src;
    let p = plan(src);
    assert_eq!(p.verdicts.len(), 1);
    let outer = (0, src.trim_end().len());
    assert_ne!(p.verdicts[0].owner, outer, "the owner is the arrow");
    assert!(p.refuses_owner(p.verdicts[0].owner));
    assert!(!p.refuses_owner(outer));
}

#[test]
fn a_placement_check_that_cannot_be_evaluated_is_a_violation() {
    // A `do` whose condition has no execution point of its own (a one-line `do`) has no head to place
    // against: fail closed.
    let src = "function f(c) {\n  let z = 1;\n  do { x = g(); } while (c());\n  sink(x, z);\n}\n";
    let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let v = &parsed.js_loop_plan().verdicts[0];
    let message = super::super::placement::violation(&parsed, owner, v.root).unwrap();
    assert!(message.contains("no execution point"), "{message}");
    assert!(parsed.js_loop_plan().refuses_owner(v.owner));
    // No loop spans a range that is not one: also a violation, never an admission.
    assert!(super::super::placement::violation(&parsed, owner, (0, 3)).is_some());
}

#[test]
fn an_assumption_is_about_one_file_only() {
    // A rayon join can run another file's task on the thread that holds an assumption: that file must be
    // answered by its own plan, not by the assumption made for a different file.
    let admitted_src = ADMITTED[0].1;
    let a = ParsedFile::parse("a.js", admitted_src, Language::JavaScript).unwrap();
    let b = ParsedFile::parse("b.js", admitted_src, Language::JavaScript).unwrap();
    let root = a.js_loop_plan().verdicts[0].root;
    assert!(super::admits(&b, root), "b's own plan admits it");
    super::with_admission(&a, &std::collections::BTreeSet::new(), || {
        assert!(!super::admits(&a, root), "the assumption holds for a");
        assert!(super::admits(&b, root), "and not for b");
    });
}

#[test]
fn an_admissible_loop_beside_a_refused_one_in_the_same_owner_is_refused_with_it() {
    // Final policy: a refused owner keeps main's geometry whole. `while (a())` alone is admissible; the second
    // loop is a head collapse, so the owner is refused and the first loop is `OwnerSibling`.
    let src = "function f(a, c) {\n  let z = 1;\n  while (a()) {\n    work();\n  }\n  while (c()) x = g();\n  sink(x, z);\n}\n";
    let p = plan(src);
    let mut why: Vec<String> = p.verdicts.iter().map(|v| reason(&v.why)).collect();
    why.sort();
    assert_eq!(why, ["OwnerSibling", "Unresolved(HeadCollapse)"], "{why:?}");
    assert!(p.refuses_owner(p.verdicts[0].owner));
    let main = crate::cfg::js_loops::with_main_loop_cfg(|| {
        let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
        crate::cfg::build_cfg_edges_with_arms(&parsed)
    });
    let now = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    assert_eq!(
        format!("{:?}", crate::cfg::build_cfg_edges_with_arms(&now)),
        format!("{main:?}")
    );
}

// Perf slice (#785): the speed gates change cost, never output.

#[test]
fn the_loop_keyword_gate_never_hides_a_loop() {
    use crate::cfg::js_loops::may_hold_loop;
    for src in [
        "for(;;){}",
        "x;for (a of b) {}",
        "{while(c){}}",
        "do{}while(c)",
        "lbl:for(;;);",
        "a=1\nfor(;;)",
        "\u{ff1b}for(;;)",
        "if(x)do y();while(z)",
    ] {
        assert!(may_hold_loop(src.as_bytes()), "{src}");
    }
    // Conservative cases: a keyword in a string, as a property, minified, or after a statement end still pass.
    for src in [
        "for(;;){",
        "do{}while(0)",
        "a.for(1)",
        "s = 'while the loop runs'",
        "x=1;do{x++}while(x<3)",
    ] {
        assert!(may_hold_loop(src.as_bytes()), "{src}");
    }
    for src in [
        "",
        "$for = 1; a.$while = 2",
        "\\u0066or(;;){}",
        "format(x); forEach(y); document.undo(); window.width = 1",
        "let do_it = 1; $for = 2; _while = 3; formdo(x)",
        "x = 1; return y;",
    ] {
        assert!(!may_hold_loop(src.as_bytes()), "{src}");
    }
    // Every fixture owner that holds a loop passes the gate (so the gate never changes a plan).
    for c in REFUSED {
        assert!(may_hold_loop(c.src.as_bytes()), "{}", c.name);
        let p = plan(c.src);
        assert!(!p.verdicts.is_empty(), "{}", c.name);
    }
    for (name, src, _) in ADMITTED {
        assert!(may_hold_loop(src.as_bytes()), "{name}");
    }
}

#[test]
fn a_file_without_a_loop_has_an_empty_plan_and_without_try_no_regions() {
    let src = "function f(a) {\n  let x = 0;\n  if (a) {\n    x = 1;\n  }\n  return x;\n}\n";
    let p = plan(src);
    assert!(p.verdicts.is_empty() && p.counters().is_empty());
    let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    assert!(super::tryflow::Regions::of(&parsed).0.is_empty());
    let with_try = ParsedFile::parse(
        "a.js",
        "function f() {\n  try {\n    g();\n  } catch (e) {}\n}\n",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(super::tryflow::Regions::of(&with_try).0, [(2, 4)]);
}

#[test]
fn the_plans_standing_builds_are_the_fresh_builds() {
    use crate::cfg::{build_function_cfg, EdgeSink};
    let mut sources: Vec<&str> = ADMITTED.iter().map(|(_, s, _)| *s).collect();
    sources.extend(REFUSED.iter().map(|c| c.src));
    sources.extend(
        crate::cpg::loop_plan_tests::LEGACY_REFUSED
            .iter()
            .map(|(_, _, s)| *s),
    );
    let mut replayed = 0;
    for src in sources {
        let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
        let plan = parsed.js_loop_plan();
        for owner in parsed.all_functions() {
            let Some(standing) = plan.replay((owner.start_byte(), owner.end_byte())) else {
                continue;
            };
            let mut sink = EdgeSink::new();
            build_function_cfg(owner, &parsed, &mut sink);
            let fresh: Vec<_> = sink.drain().collect();
            assert_eq!(standing, fresh.as_slice(), "{src}");
            replayed += 1;
        }
    }
    assert!(replayed > 0, "the fixtures exercise a replayed build");
}

#[test]
fn a_loop_recovered_from_a_parse_error_is_planned_like_the_ungated_walk() {
    // `1while` and `1for` are no keyword to the text gate (a digit precedes), but tree-sitter recovers a loop
    // statement in a body with an error: the gate is bypassed there (9af2aeb7 counts one tree, refused).
    for src in [
        "function f() {\n  let x = 0;\n  1while ((x = next())) {\n    x = g();\n  }\n  sink(x);\n}\n",
        "function f(xs) {\n  let x = 0;\n  1for (const y of xs) {\n    x = y;\n  }\n  sink(x);\n}\n",
    ] {
        let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
        let body = parsed.all_functions()[0].child_by_field_name("body").unwrap();
        assert!(body.has_error());
        assert!(!crate::cfg::js_loops::may_hold_loop(
            &src.as_bytes()[body.byte_range()]
        ));
        let counters = parsed.js_loop_plan().counters();
        assert_eq!(counters.get("trees"), Some(&1), "{counters:?}");
        assert_eq!(counters.get("owners_refused"), Some(&1), "{counters:?}");
    }
}
