use super::*;

fn entry_row(formals: &str, body: &str, actuals: &str, reason: &'static str) {
    let holder = format!("function holder({formals}) {{ {body} cb('subject'); }}");
    direct_case(
        &holder,
        &format!("holder({actuals});"),
        Language::JavaScript,
    )
    .refuses("cb('subject')", reason);
}

#[test]
fn earlier_default_member_assignment() {
    entry_row(
        "a, x=(a.cb=fallback), {cb}=a",
        "",
        "{cb: stale}",
        "default_dependency",
    );
}

#[test]
fn earlier_default_call_or_mutator() {
    for effect in ["m(a)", "a.m()"] {
        entry_row(
            &format!("a, x={effect}, {{cb}}=a"),
            "",
            "{cb: stale, m() { this.cb = fallback; }}",
            "default_dependency",
        );
    }
}

#[test]
fn earlier_default_delete() {
    entry_row(
        "a, x=delete a.cb, {cb}=a",
        "",
        "{cb: stale}",
        "default_dependency",
    );
}

#[test]
fn earlier_default_arguments_alias() {
    entry_row(
        "a, x=(arguments[0].cb=fallback), {cb}=a",
        "",
        "{cb: stale}",
        "stale_path",
    );
}

#[test]
fn earlier_default_closure_called_later() {
    entry_row(
        "p, f=()=>{p.cb=fallback}, x=f()",
        "const {cb}=p;",
        "{cb: stale}",
        "stale_path",
    );
    direct_case(
        "function holder(p, f=()=>0) { const {cb}=p; cb('control snapshot'); }",
        "holder({cb: verified});",
        Language::JavaScript,
    )
    .targets("cb('control snapshot')", &["verified"]);
}

#[test]
fn earlier_sibling_default_mutates_pattern_source() {
    entry_row(
        "a, {x=(a.cb=fallback), cb}=a",
        "",
        "{cb: stale}",
        "path_step",
    );
    direct_case(
        "function holder({cb=verified}) { cb('control pattern'); }",
        "holder({cb: undefined});",
        Language::JavaScript,
    )
    .targets("cb('control pattern')", &["verified"]);
}

#[test]
fn earlier_default_mutates_body_snapshot() {
    entry_row(
        "p, x=(p.cb=fallback)",
        "const {cb}=p;",
        "{cb: stale}",
        "stale_path",
    );
    direct_case(
        "function holder(p, x=0) { const {cb}=p; cb('control snapshot'); }",
        "holder({cb: verified});",
        Language::JavaScript,
    )
    .targets("cb('control snapshot')", &["verified"]);
}

#[test]
fn earlier_default_mutates_array_before_iteration() {
    entry_row(
        "a, x=a.shift(), [cb]=a",
        "",
        "[stale, fallback]",
        "path_step",
    );
    direct_case(
        "function holder({0: cb}) { cb('control index'); }",
        "holder([verified]);",
        Language::JavaScript,
    )
    .targets("cb('control index')", &["verified"]);
}

#[test]
fn default_literal_cannot_relay_a_formal_across_callers() {
    for default in [
        "{cb:a}",
        "flag ? {cb:a} : {cb:stale}",
        "flag ? {cb:stale} : {cb:a}",
    ] {
        let (fixture, trace) = diagnostic::capture::run(|| {
            direct_case(
                &format!("function holder(a, {{cb}}={default}) {{ cb('subject'); }}"),
                "holder(stale); holder(fallback,{cb:verified});",
                Language::JavaScript,
            )
        });
        fixture.targets("cb('subject')", &["verified"]);
        assert!(fixture.index.refusal_count("default_dependency") > 0);
        let records: Vec<serde_json::Value> = trace
            .iter()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let record = records
            .iter()
            .find(|row| row["reason"] == "default_dependency")
            .unwrap();
        let span = |s: &str| {
            let start = fixture.source.find(s).unwrap();
            serde_json::json!([start, start + s.len()])
        };
        assert_eq!(record["site"]["span"], span("cb('subject')"));
        assert_eq!(record["chain"][0]["caller"]["span"], span("holder(stale)"));
        let binding = fixture.source.find("{cb}=").unwrap() + 1;
        assert_eq!(
            record["chain"][0]["declaration"],
            serde_json::json!([binding, binding + 2])
        );
        let target = records
            .iter()
            .find(|row| {
                row["site"]["span"] == span("cb('subject')")
                    && row["target"]["id"]["name"] == "verified"
            })
            .unwrap();
        assert_eq!(
            target["target"]["span"],
            span("function verified(value) {}")
        );
    }
}

#[test]
fn default_array_body_copy_nested_default_and_tdz() {
    entry_row("a, [cb]=[a]", "", "stale", "path_step");
    for (formals, body, bad, good) in [
        (
            "a, p={cb:a}",
            "const {cb}=p;",
            "stale",
            "fallback,{cb:verified}",
        ),
        (
            "a, p",
            "const {k:{cb}={cb:a}}=p;",
            "stale,{__proto__:null}",
            "fallback,{k:{cb:verified}}",
        ),
        ("{cb}={cb:a}, a", "", "", "{cb:verified},fallback"),
    ] {
        let fixture = direct_case(
            &format!("function holder({formals}) {{ {body} cb('subject'); }}"),
            &format!("holder({bad}); holder({good});"),
            Language::JavaScript,
        );
        fixture.targets("cb('subject')", &["verified"]);
        assert!(fixture.index.refusal_count("default_dependency") > 0);
    }
}

#[test]
fn deferred_snapshot_can_run_again_after_write_or_delete() {
    for (snapshot, mutation, reason) in [
        ("{cb}", "p.cb=fallback", "stale_path"),
        ("{cb=fallback}", "delete p.cb", "stale_path"),
        ("{cb}", "arguments[0].cb=fallback", "stale_path"),
        ("{cb=fallback}", "delete arguments[0].cb", "stale_path"),
    ] {
        direct_case(
            &format!("function holder(p) {{ function g() {{ const {snapshot}=p; cb('subject'); }} g(); {mutation}; g(); }}"),
            "holder({__proto__:null, cb:stale});", Language::JavaScript,
        ).refuses("cb('subject')", reason);
    }
    direct_case(
        "function holder(p) { const {cb}=p; cb('control snapshot'); }",
        "holder({cb:verified});",
        Language::JavaScript,
    )
    .targets("cb('control snapshot')", &["verified"]);
}

#[test]
fn jsx_legacy_context_is_not_omission() {
    let fixture = direct_case(
        "function Context(_, cb=stale) { cb('subject'); }",
        "<Context />; Context({}, verified);",
        Language::Tsx,
    );
    fixture.targets("cb('subject')", &["verified"]);
    assert_eq!(fixture.index.refusal_count("jsx_attribute"), 1);
}

#[test]
fn jsx_default_props_and_reserved_keys_are_renderer_owned() {
    let fixture = direct_case(
        "function Defaults({cb=stale}) { cb('default props'); }
         Defaults.defaultProps={cb:fallback};
         function Reserved({key:cb}) { cb('reserved'); }",
        "<Defaults />; Defaults({cb:verified});
         <Reserved key={stale} />; Reserved({key:verified});",
        Language::Tsx,
    );
    fixture.targets("cb('default props')", &["verified"]);
    fixture.targets("cb('reserved')", &["verified"]);
    assert_eq!(fixture.index.refusal_count("jsx_attribute"), 2);
}

#[test]
fn exported_mutable_prototype_cannot_supply_inherited_callback() {
    // The prototype use shares the const's execution so initialization
    // succeeds before static_object reaches the exported-mutable check.
    let fixture = Fixture::build(
        format!(
            "{CONTROL}
            function stale(value) {{}}
            export const proto={{cb:stale}};
            function holder({{cb}}) {{ cb('subject'); }}
            holder({{__proto__:proto}});
            holder({{cb:verified}});
            control(verified);"
        ),
        Language::JavaScript,
    );
    fixture.targets("good('control')", &["verified"]);
    fixture.targets("cb('subject')", &["verified"]);
    assert_eq!(fixture.index.refusal_count("stale_path"), 1);
}

#[test]
fn self_initializers_refuse_before_recursion() {
    // A self-read in the initializer is a tracked use (O2): `stale_path`
    // precedes `default_dependency` in the counter order.
    entry_row("cb=cb", "", "", "stale_path");
    entry_row("{cb}=cb", "", "", "stale_path");
    direct_case(
        "function holder({cb}={cb:verified}) { cb('control default'); }",
        "holder();",
        Language::JavaScript,
    )
    .targets("cb('control default')", &["verified"]);
}

#[test]
fn this_member_source_refuses_but_bare_source_survives() {
    let fixture = direct_case(
        "class Base { m(cb) { cb('subject'); } run() { this.m(stale); } }
         class Derived extends Base { m(cb) {} }
         function holder(cb) { cb('bare'); }",
        "holder(verified);",
        Language::JavaScript,
    );
    fixture.targets("cb('bare')", &["verified"]);
    fixture.refuses("cb('subject')", "source");
}

#[test]
fn baseline_fanout_includes_refused_spread_after_relay() {
    let declarations: String = (1..=9).map(|i| format!("function f{i}() {{}}\n")).collect();
    let direct: String = (1..=8).map(|i| format!("holder(f{i});")).collect();
    let holder = format!(
        "{declarations}
        function holder(cb) {{ cb('subject'); }}
        function relay(f) {{ holder(f); }}"
    );
    let eight = direct_case(&holder, &direct, Language::JavaScript);
    eight.targets(
        "cb('subject')",
        &["f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8"],
    );
    let nine = direct_case(
        &holder,
        &format!("{direct} relay(...[f9]);"),
        Language::JavaScript,
    );
    assert_eq!(nine.outcome("cb('subject')"), Some(&ArgOutcome::Fanout(9)));
    assert_eq!(nine.index.refusal_count("fanout"), 1);
}

#[test]
fn read_timing_precedes_unproved_unrelated_default() {
    direct_case(
        "function h({cb}, x=effect()){ if (flag) cb() }",
        "h({cb:()=>{}}, make());",
        Language::JavaScript,
    )
    .refuses("cb()", "read_timing");
}

#[test]
fn supplied_i0_placement_families() {
    let fixture = direct_case(
        "function branch(cb) { if (flag) cb('branch'); }
         function looped(cb) { while (flag) cb('loop'); }
         function closure(cb) { return () => cb('closure'); }
         async function task(cb) { await 0; cb('async'); }
         function* generator(cb) { yield 0; cb('generator'); }
         function prefix(cb) { effect(); cb('prefix'); }
         function written(cb) { function change() { cb = stale; } return () => cb('written'); }
         function shadow(cb) { return cb => cb('shadow'); }",
        "branch(verified); looped(verified); closure(verified); task(verified);
         generator(verified); prefix(verified); written(verified); shadow(verified);",
        Language::JavaScript,
    );
    for name in ["branch", "loop", "closure", "async", "generator", "prefix"] {
        fixture.targets(&format!("cb('{name}')"), &["verified"]);
    }
    fixture.refuses("cb('written')", "entry_write");
    assert_eq!(
        fixture.outcome("cb('shadow')"),
        Some(&ArgOutcome::Empty("no_caller".into()))
    );
}

#[test]
fn supplied_i0_refuses_mapped_slot_even_through_alias() {
    let fixture = classic_case(
        "function holder(cb) { const args=arguments; args[0]=fallback; return () => cb('subject'); }",
        "holder(verified);",
    );
    fixture.refuses("cb('subject')", "mapped_arguments");
}

#[test]
fn facts_leave_resolution_and_call_edges_unchanged() {
    use crate::cpg::CodePropertyGraph;
    use petgraph::graph::NodeIndex;

    let fixture = direct_case(
        "function holder(cb) { cb('fact'); } function relay(cb) { holder(cb); }
         function ordinary() { verified(); }",
        "holder(verified); relay(stale); ordinary(); unbound();",
        Language::JavaScript,
    );
    fixture.targets("cb('fact')", &["stale", "verified"]);
    let populated = fixture.graph;
    let mut empty = populated.clone();
    empty.js_anonymous.arg_follow = ArgIndex::default();
    assert_eq!(empty.js_anonymous.arg_follow.stats.eligible_sites, 0);
    assert_eq!(populated.calls, empty.calls);
    assert_eq!(populated.callers, empty.callers);
    for site in populated.calls.values().flatten() {
        assert_eq!(
            populated.resolve_call_site_full(site),
            empty.resolve_call_site_full(site),
            "{site:?}"
        );
        assert_eq!(
            populated.resolve_call_site(site),
            empty.resolve_call_site(site),
            "{site:?}"
        );
    }

    // Give every possible endpoint an index so a leaked parameter edge
    // cannot disappear because the edge collector cannot represent its target.
    let functions = populated
        .functions
        .values()
        .flatten()
        .chain(populated.calls.keys());
    let indices: BTreeMap<_, _> = functions
        .enumerate()
        .map(|(index, id)| {
            (
                (id.file.clone(), id.name.clone(), id.start_line),
                NodeIndex::new(index),
            )
        })
        .collect();
    let without = CodePropertyGraph::collect_step5_edges(&empty, &indices);
    assert!(!without.is_empty(), "ordinary Call and Return edge control");
    assert_eq!(
        CodePropertyGraph::collect_step5_edges(&populated, &indices),
        without
    );
}

#[test]
fn distinguishes_jsx_at_relay_and_invocation_before_later_refusals() {
    let (fixture, lines) = diagnostic::capture::run(|| {
        direct_case(
            "function holder(cb) { cb('subject'); }
         function Relay(f) { if (flag) holder(f); }
         function Render(Cb) { Cb=stale; return <Cb />; }",
            "<Relay f={stale} />; holder(verified); Render(verified);",
            Language::Tsx,
        )
    });
    fixture.targets("cb('subject')", &["verified"]);
    assert_eq!(
        fixture.outcome("<Cb />"),
        Some(&ArgOutcome::Empty("jsx_attribute".into()))
    );
    let rows: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(rows
        .iter()
        .any(|row| row["reason"] == "jsx_attribute" && row["jsx_invocation"] == true));
    // The JSX hop wins even though the transfer is conditionally placed.
    let jsx_start = fixture.source.find("<Relay ").unwrap();
    assert!(rows.iter().any(|row| row["reason"] == "jsx_attribute"
        && row["jsx_hop"] == true
        && row["caller"]["span"][0] == jsx_start));
}

#[test]
fn exact_native_namespace_member_remains_a_source_ground() {
    let files = [
        ("holder.js", "export function holder(cb) { cb('subject'); }"),
        (
            "m.js",
            "import * as api from './holder.js'; function verified() {} api.holder(verified);",
        ),
    ]
    .into_iter()
    .map(|(file, source)| {
        (
            file.into(),
            ParsedFile::parse(file, source, Language::JavaScript).unwrap(),
        )
    })
    .collect();
    let graph = CallGraph::build(&files);
    let subject = graph
        .calls
        .values()
        .flatten()
        .find(|site| site.callee_name == "cb")
        .unwrap();
    let Some(ArgOutcome::Targets(targets)) = graph.js_anonymous.arg_follow.lookup(subject) else {
        panic!("expected the baseline native ESM namespace ground");
    };
    assert_eq!(
        targets
            .iter()
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>(),
        ["verified"]
    );
}

#[test]
fn colliding_holder_spans_cannot_share_an_exact_ground() {
    // Two holders with one FunctionId (same file, name and lines). The call
    // graph already skips call extraction in an ambiguous owner, so neither
    // body site reaches the index; if it did, `unique_owner` would refuse it
    // as `holder_identity`. Either way no caller's value may be credited.
    let fixture = direct_case(
        "function left() { function h(cb) { cb('left'); } h(stale); } function right() { function h(cb) { cb('right'); } h(fallback); }",
        "left(); right();",
        Language::JavaScript,
    );
    for site in ["cb('left')", "cb('right')"] {
        // Unreachable branch evidence: the colliding holder's own body
        // never yields a call site, so `unique_owner` is not consulted.
        assert!(fixture.site(site).is_none(), "{site} has a graph site");
        assert!(fixture.outcome(site).is_none(), "{site}");
    }
    assert_eq!(fixture.index.stats().target_sites, 1, "only the control");
}

#[test]
fn default_closure_can_read_a_supplied_slot_only_after_entry() {
    let fixture = direct_case(
        "function later(cb, f=()=>cb('later')) { return f; }
         function during(cb, f=()=>cb('during'), value=f()) { return value; }",
        "later(verified); during(verified);",
        Language::JavaScript,
    );
    fixture.targets("cb('later')", &["verified"]);
    assert_eq!(
        fixture.outcome("cb('during')"),
        Some(&ArgOutcome::Empty("read_timing".into()))
    );
    // The formal-list call `f()` is a second refused eligible site (the
    // closure is read before the formal list finishes).
    assert_eq!(
        fixture.outcome("f()"),
        Some(&ArgOutcome::Empty("read_timing".into()))
    );
    assert_eq!(fixture.index.refusal_count("read_timing"), 2);
}
