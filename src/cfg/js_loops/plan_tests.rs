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
        assert!(p.refuses_owner(v.owner), "{}: O1 refuses the owner", c.name);
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
        // so legacy, with no label refusal.
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
        ("owners_refused", 2),
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
    // The callback's loop is refused; the enclosing function holds no loop tree of its own, so O1 refuses
    // the arrow's labels and leaves the function's.
    let src = REFUSED
        .iter()
        .find(|c| c.name == "foreign_from_enclosing_callable")
        .unwrap()
        .src;
    let p = plan(src);
    assert_eq!(p.verdicts.len(), 1);
    let outer = (0, src.trim_end().len());
    assert_ne!(p.verdicts[0].owner, outer, "the owner is the arrow");
    assert!(p.refuses_owner(p.verdicts[0].owner));
    assert!(!p.refuses_owner(outer));
}
