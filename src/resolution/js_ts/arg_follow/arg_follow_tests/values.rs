use super::*;

#[test]
fn refuses_unproved_path_step() {
    for (formal, actual) in [
        ("[cb = fallback]", "unknownIterator"),
        ("{cb = fallback}", "opaqueProperties"),
        ("{ [Symbol.iterator]: cb }", "{cb: stale}"),
        ("{ ...cb }", "{cb: stale}"),
    ] {
        let fixture = direct_case(
            &format!("function holder({formal}) {{ cb('refusal'); }}"),
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", "path_step");
    }
    // One holder per binding: a second invocation would sit behind the first
    // statement and fail I1 placement (`read_timing` precedes `path_step`).
    for invocation in ["one", "two", "three"] {
        let fixture = direct_case(
            &format!("function holder({{a: {{b: one}}, 'a.b': two, 0: [, three]}}) {{ {invocation}('{invocation}'); }}"),
            "holder({a: {b: stale}, 'a.b': fallback, '0': [, verified]});",
            Language::JavaScript,
        );
        fixture.refuses(&format!("{invocation}('{invocation}')"), "path_step");
    }
    direct_case(
        "function holder(key, { [key]: cb }) { cb('paired'); }",
        "holder('left', {left: stale, right: fallback});",
        Language::JavaScript,
    )
    .refuses("cb('paired')", "path_step");
}

#[test]
fn later_argument_selected_path_must_survive_until_entry() {
    for actuals in [
        "{__proto__: proto}, proto.layer.cb = fallback",
        "{__proto__: proto}, delete proto.layer.cb",
        "{__proto__: proto}, proto.layer = {cb: fallback}",
        "{__proto__: proto}, alias.layer[key] = fallback",
        "{__proto__: proto}, external(alias.layer)",
        "...[{__proto__: proto}, proto.layer.cb = fallback]",
    ] {
        let fixture = direct_case(
            "function holder({layer: {cb}}, later) { cb('refusal'); }",
            &format!("const proto = {{layer: {{cb: stale}}}}; const alias = proto; holder({actuals}); holder({{layer: {{cb: verified}}}}, 0);"),
            Language::JavaScript,
        );
        fixture.targets("cb('refusal')", &["verified"]);
        assert_eq!(fixture.index.refusal_count("stale_path"), 1, "{actuals}");
    }
    let fixture = direct_case(
        "function holder({layer: {cb}}, later) { cb('kept'); }",
        "const proto = {layer: {cb: stale}}; holder({__proto__: proto}, proto.layer.other = fallback); proto.layer.cb = fallback;",
        Language::JavaScript,
    );
    fixture.refuses("cb('kept')", "undefined_lookup");
}

#[test]
fn mutable_argument_reference_and_member_remain_opaque() {
    let fixture = direct_case(
        "function byValue(cb, ignored) { cb('value'); } function byObject({cb}, ignored) { cb('object'); }",
        "const a = {cb: stale}, b = {cb: stale}; byValue(a.cb, a.cb = fallback); byObject(b, b.cb = fallback);",
        Language::JavaScript,
    );
    fixture.refuses("cb('value')", "opaque_terminal");
    fixture.refuses("cb('object')", "path_step");
}

#[test]
fn inherited_literal_fact_does_not_credit_s2_25() {
    let rows = include_str!("../../../../../tests/fixtures/param_binding_matrix/rows.jsonl");
    let row = rows
        .lines()
        .find(|line| line.starts_with(r#"{"id":"s2-25","#))
        .unwrap();
    let row: serde_json::Value = serde_json::from_str(row).unwrap();
    let fixture = Fixture::build(
        row["files"]["m.js"].as_str().unwrap().into(),
        Language::JavaScript,
    );
    assert!(
        matches!(fixture.outcome("cb(input)"), Some(ArgOutcome::Empty(reason)) if reason == "undefined_lookup")
    );
    let resolved = fixture
        .graph
        .resolve_call_site_full(fixture.site("cb(input)").unwrap());
    assert!(resolved.resolved.is_empty());
    assert!(matches!(
        resolved.drop,
        Some(
            crate::resolution::DropReason::UnknownName
                | crate::resolution::DropReason::LocalBindingUnproven
        )
    ));
}

#[test]
fn refuses_unproved_jsx_attribute() {
    for (key, jsx) in [
        ("cb", "<Holder {...opaque} />"),
        ("cb", "<Holder cb={stale} {...opaque} />"),
        ("children", "<Holder>{stale}</Holder>"),
        ("cb", "<Holder cb />"),
    ] {
        let fixture = direct_case(
            &format!("function Holder({{{key}: cb = fallback}}) {{ cb('mixed'); }}"),
            &format!(
                "{jsx}; <Holder {{...opaque}} {key}={{verified}} />; Holder({{{key}: verified}});"
            ),
            Language::Tsx,
        );
        fixture.targets("cb('mixed')", &["verified"]);
        assert_eq!(fixture.index.refusal_count("jsx_attribute"), 2, "{jsx}");
    }
}

#[test]
fn refuses_unproved_undefined_lookup() {
    for (actual, reason) in [
        ("{layer: {get cb() { return stale; }}}", "undefined_lookup"),
        ("{layer: {__proto__: external()}}", "undefined_lookup"),
        ("{layer: []}", "path_step"),
    ] {
        let fixture = direct_case(
            "function holder({layer: {cb = fallback}}) { cb('refusal'); }",
            &format!("holder({actual});"),
            Language::JavaScript,
        );
        fixture.refuses("cb('refusal')", reason);
    }
    for entry in ["cb: undefined", "__proto__: null"] {
        let fixture = direct_case(
            "function holder({layer: {cb = fallback}}) { cb('kept'); }",
            &format!("holder({{layer: {{{entry}}}}});"),
            Language::JavaScript,
        );
        fixture.targets("cb('kept')", &["fallback"]);
    }
}

#[test]
fn refuses_unwitnessed_default_alternatives() {
    for actual in ["null", "false", "undefined"] {
        let shadow = if actual == "undefined" {
            "const undefined = 1;"
        } else {
            ""
        };
        let fixture = direct_case(
            "function holder(cb = fallback) { cb('mixed'); }",
            &format!("{shadow} holder({actual}); holder(stale);"),
            Language::JavaScript,
        );
        fixture.targets("cb('mixed')", &["stale"]);
    }
    let fixture = direct_case(
        "function holder(cb = later, later = fallback) { cb('refusal'); }",
        "holder();",
        Language::JavaScript,
    );
    fixture.refuses("cb('refusal')", "default_dependency");
    let fixture = direct_case(
        "function holder(first, {cb = flag ? first : fallback} = {}) { cb('kept'); }",
        "holder(stale, {cb: flag ? undefined : fallback}); holder(stale, {cb:verified});",
        Language::JavaScript,
    );
    fixture.targets("cb('kept')", &["verified"]);
    assert!(fixture.index.refusal_count("hidden_literal_key") > 0);
}

#[test]
fn refuses_colliding_function_ids() {
    let fixture = direct_case(
        "function holder({handler}) { handler('collision'); } function inline(cb) { cb('anonymous'); }",
        "holder({handler() {}}); holder({handler() {}}); inline(() => {}); inline(() => {});",
        Language::JavaScript,
    );
    assert_eq!(
        fixture.outcome("handler('collision')"),
        Some(&ArgOutcome::Empty("callable_collision".into()))
    );
    assert_eq!(fixture.index.refusal_count("callable_collision"), 2);
    assert!(
        matches!(fixture.outcome("cb('anonymous')"), Some(ArgOutcome::Targets(targets)) if targets.len() == 2 && targets.iter().all(|target| target.name.starts_with("<anon@")))
    );
}

#[test]
fn stored_react_use_callback_is_not_a_terminal_witness() {
    let source = format!("import {{ useCallback }} from 'react';\n{CONTROL}\nfunction holder(cb) {{ cb('wrapped'); }}\nfunction run() {{ control(verified); const chosen = useCallback(value => value, []); holder(chosen); }}\nfunction shadow(useCallback) {{ const decoy = useCallback(value => 9, []); holder(decoy); }}");
    let fixture = Fixture::build(source, Language::JavaScript);
    fixture.targets("good('control')", &["verified"]);
    assert!(matches!(
        fixture.outcome("cb('wrapped')"),
        Some(ArgOutcome::Empty(_))
    ));
    assert!(fixture.index.refusal_count("opaque_terminal") > 0);
}

#[test]
fn duplicate_records_count_fanout_once() {
    let declarations: String = (0..9).map(|i| format!("function t{i}(v) {{}}\n")).collect();
    let calls: String = (0..9)
        .map(|i| format!("holder(t{i}, verified); "))
        .collect();
    let mut fixture = direct_case(
        &format!("{declarations}function holder(cb, narrow) {{ cb('broad'); narrow('narrow'); }}"),
        &calls,
        Language::JavaScript,
    );
    fixture.targets("narrow('narrow')", &["verified"]);
    assert_eq!(fixture.outcome("cb('broad')"), Some(&ArgOutcome::Fanout(9)));
    let start = fixture.source.find("cb('broad')").unwrap();
    let mut duplicate = fixture
        .graph
        .calls
        .values()
        .flatten()
        .find(|site| site.start_byte == start)
        .unwrap()
        .clone();
    duplicate.caller.name = "alternate_record_owner".into();
    fixture
        .graph
        .calls
        .entry(duplicate.caller.clone())
        .or_default()
        .insert(duplicate);
    let parsed = ParsedFile::parse("m.js", &fixture.source, Language::JavaScript).unwrap();
    fixture.index = compute(&fixture.graph, &BTreeMap::from([("m.js".into(), parsed)]));
    fixture.targets("narrow('narrow')", &["verified"]);
    assert_eq!(fixture.outcome("cb('broad')"), Some(&ArgOutcome::Fanout(9)));
    assert_eq!(fixture.index.refusal_count("fanout"), 1);
}
