use prism::navigation::types::{Reason, SymbolRef, WarningKind};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        std::fs::write(dir.path().join(name), src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn callers_reports_caller_and_call_site_line() {
    let s = session(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    let ev = queries::callers(&s, Some("target"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| matches!(&i.symbol,
        Some(SymbolRef::Function { name, .. }) if name == "caller")));
    assert!(ev.items.iter().any(|i| i.why.iter().any(|r|
        matches!(r, Reason::CalledBy { caller, call_site_line } if caller == "caller" && *call_site_line == 5))));
}

#[test]
fn callers_uses_incoming_index_not_reversed_outgoing_calls() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = NavigationIndex::build(&repo).with_modified_cpg_for_testing(|cpg| {
        cpg.call_graph.calls.clear();
    });
    let s = NavigationSession {
        repo,
        index: Arc::new(index),
    };

    let callers = queries::callers(&s, Some("target"), None, None, 1).unwrap();
    assert!(
        callers.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, .. }) if name == "caller")),
        "callers should still use the incoming callers map when outgoing calls are absent"
    );

    let callees = queries::callees(&s, Some("caller"), None, None, 1).unwrap();
    assert!(
        !callees.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, .. }) if name == "target")),
        "callees should reflect the empty outgoing calls map in this fixture"
    );
}

#[test]
fn callers_preserves_duplicate_incoming_sites_from_callers_map() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )
    .unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = NavigationIndex::build(&repo).with_modified_cpg_for_testing(|cpg| {
        let sites = cpg
            .call_graph
            .callers
            .get_mut("target")
            .expect("target caller bucket");
        let duplicate = sites[0].clone();
        sites.push(duplicate);
    });
    let s = NavigationSession {
        repo,
        index: Arc::new(index),
    };

    let callers = queries::callers(&s, Some("target"), None, None, 1).unwrap();
    let caller_count = callers
        .items
        .iter()
        .filter(|i| {
            matches!(&i.symbol,
            Some(SymbolRef::Function { name, .. }) if name == "caller")
        })
        .count();
    assert_eq!(
        caller_count, 2,
        "incoming index must preserve callers-map multiplicity even when resolution is memoized"
    );
}

#[test]
fn callers_direct_hit_scores_1_0_and_hop2_decays() {
    // A() called by B() called by C() — transitive (R3-M5).
    let s = session(&[(
        "a.py",
        "def a():\n    return 1\n\ndef b():\n    return a()\n\ndef c():\n    return b()\n",
    )]);
    let ev = queries::callers(&s, Some("a"), None, None, 2).unwrap();
    let b = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="b"))
        .unwrap();
    assert_eq!(b.score, 1.0); // direct caller
    let c = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="c"))
        .unwrap();
    assert_eq!(c.score, 0.5); // hop-2 caller
}

#[test]
fn callers_depth_zero_has_no_expansion() {
    let s = session(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    let ev = queries::callers(&s, Some("target"), None, None, 0).unwrap();
    assert!(ev.items.is_empty());
}

// P3: `poll` must stay OVER the R6 fanout cap (4 owners: A/B/C/D) so these
// fixtures keep exercising the still-dropped + warned collision path — a
// 2-owner pool now resolves to a labeled candidate edge instead (see
// r6_candidate_test::python_two_owner_untyped_receiver_yields_candidate_hit).
#[test]
fn callers_query_emits_collision_warning_for_dropped_sites() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("c.py", "class C:\n    def poll(self):\n        return 3\n"),
        ("d.py", "class D:\n    def poll(self):\n        return 4\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let ev = queries::callers(&s, Some("poll"), Some("a.py"), None, 1).unwrap();
    assert!(!ev.items.iter().any(|i| {
        matches!(
            &i.symbol,
            Some(SymbolRef::Function { file, name, .. }) if file == "main.py" && name == "drive"
        )
    }));
    assert!(ev.warnings.iter().any(|w| {
        matches!(w.kind, WarningKind::Collision)
            && w.message.contains('1')
            && w.message
                .contains("unknown receiver type across multiple owner types")
    }));
}

#[test]
fn ego_graph_emits_collision_warning_for_seed() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("c.py", "class C:\n    def poll(self):\n        return 3\n"),
        ("d.py", "class D:\n    def poll(self):\n        return 4\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let ev = queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 1, &["Call"]).unwrap();
    assert!(ev.warnings.iter().any(|w| {
        matches!(w.kind, WarningKind::Collision)
            && w.message.contains('1')
            && w.message
                .contains("unknown receiver type across multiple owner types")
    }));
}

#[test]
fn ego_graph_does_not_emit_collision_warning_when_call_edges_not_collected() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("c.py", "class C:\n    def poll(self):\n        return 3\n"),
        ("d.py", "class D:\n    def poll(self):\n        return 4\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let dataflow_only =
        queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 1, &["DataFlow"]).unwrap();
    assert!(!dataflow_only
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::Collision)));

    let zero_hops = queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 0, &["Call"]).unwrap();
    assert!(!zero_hops
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::Collision)));
}

#[test]
fn collision_warning_names_up_to_five_sites_sorted_deterministically() {
    // P3 item 3: 6 dropped call sites (over the R6 fanout cap: A/B/C/D all
    // define `poll`), added out of lexicographic order, to prove the warning
    // both caps the named sites at 5 and sorts them deterministically.
    let mut files: Vec<(String, String)> = vec![
        (
            "a.py".into(),
            "class A:\n    def poll(self):\n        return 1\n".into(),
        ),
        (
            "b.py".into(),
            "class B:\n    def poll(self):\n        return 2\n".into(),
        ),
        (
            "c.py".into(),
            "class C:\n    def poll(self):\n        return 3\n".into(),
        ),
        (
            "d.py".into(),
            "class D:\n    def poll(self):\n        return 4\n".into(),
        ),
    ];
    for name in [
        "f_caller.py",
        "b_caller.py",
        "e_caller.py",
        "a_caller.py",
        "d_caller.py",
        "c_caller.py",
    ] {
        files.push((name.into(), "def drive(x):\n    return x.poll()\n".into()));
    }
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    let s = session(&files);
    let ev = queries::callers(&s, Some("poll"), Some("a.py"), None, 1).unwrap();
    let warning = ev
        .warnings
        .iter()
        .find(|w| matches!(w.kind, WarningKind::Collision))
        .expect("collision warning");
    assert!(
        warning
            .message
            .starts_with("6 same-name receiver call site(s)"),
        "{}",
        warning.message
    );
    let expected_named = [
        "a_caller.py:2",
        "b_caller.py:2",
        "c_caller.py:2",
        "d_caller.py:2",
        "e_caller.py:2",
    ];
    for site in expected_named {
        assert!(warning.message.contains(site), "{}", warning.message);
    }
    assert!(
        !warning.message.contains("f_caller.py"),
        "must cap at 5 named sites: {}",
        warning.message
    );
}

#[test]
fn callers_finds_aliased_import_call_site() {
    // `from util import tick as t; t()` — call-site name is "t", function is "tick".
    let s = session(&[
        ("util.py", "def tick():\n    pass\n"),
        (
            "app.py",
            "from util import tick as t\n\ndef run():\n    t()\n",
        ),
    ]);
    let ev = queries::callers(&s, Some("tick"), Some("util.py"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "callers of tick must include the aliased `t()` call in app.py::run; got {:?}",
        ev.items
    );
}

#[test]
fn callers_alias_resolves_correct_target_not_same_named_other() {
    // Two modules define `tick`; app aliases ONLY util.tick.
    let s = session(&[
        ("util.py", "def tick():\n    pass\n"),
        ("other.py", "def tick():\n    pass\n"),
        (
            "app.py",
            "from util import tick as t\n\ndef run():\n    t()\n",
        ),
    ]);
    // Positive: the alias arm surfaces the `t()` site and it resolves to util.tick.
    let util_ev = queries::callers(&s, Some("tick"), Some("util.py"), None, 1).unwrap();
    assert!(
        util_ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "callers of util.tick must include aliased `t()` in app.py::run; got {:?}",
        util_ev.items
    );
    // Negative: identity backstop — the SAME candidate must NOT attach to other.tick.
    let other_ev = queries::callers(&s, Some("tick"), Some("other.py"), None, 1).unwrap();
    assert!(
        !other_ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, .. }) if name == "run")),
        "alias to util.tick must NOT register as a caller of other.tick; got {:?}",
        other_ev.items
    );
}

#[test]
fn callers_alias_arm_excludes_qualified_method_sites() {
    // `poll` is a method on two classes (multi-owner) AND the alias target name.
    // The qualified `x.poll()` site lives under callers key "poll"; the alias arm
    // keys off a binding whose member == "poll", but must skip qualified sites so it
    // does not feed `x.poll()` into the (non-identity-filtered) collision counter.
    let s = session(&[
        ("util.py", "def poll():\n    pass\n"),
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        (
            "app.py",
            "from util import poll as p\n\ndef run(x):\n    p()\n    return x.poll()\n",
        ),
    ]);
    // alias `p()` resolves to util.poll (free fn) — surfaced as a caller.
    let ev = queries::callers(&s, Some("poll"), Some("util.py"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|i| matches!(&i.symbol,
            Some(SymbolRef::Function { name, file, .. }) if name == "run" && file == "app.py")),
        "alias p() should be a caller of util.poll; got {:?}",
        ev.items
    );
}
