use super::*;

#[test]
fn duplicate_diagnostics_and_changed_failure_frames_fail_exact_comparison() {
    use super::super::check_roots;
    let source = "é\nregister(p)\n";
    let sources = BTreeMap::from([("app.js".into(), source.into())]);
    let reason = "hapi_registration_unproven";
    let root = json!({
        "file":"app.js","binding":"app","construction_byte":0,"invocations":[
            {"file":"app.js","start_byte":3,"end_byte":14}
        ]
    });
    let row = json!({
        "expect_diagnostics":[{"site":{"file":"app.js","line":2,"text":"register(p)"},"reason":reason}],
        "expect_failed_roots":[{"root":root,"site":{"file":"app.js","line":2,"text":"register(p)"},"reason":reason}]
    });
    let site = json!({"file":"app.js","line":2,"start_byte":3,"end_byte":14});
    let diagnostic = json!({"site":site,"reason":reason});
    let mut summary = json!({
        "diagnostics":[diagnostic],
        "failed_roots":[{"root":root,"site":site,"reason":reason}]
    });
    let mut errors = Vec::new();
    check_events(&row, &summary, &sources, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    summary["diagnostics"]
        .as_array_mut()
        .unwrap()
        .push(diagnostic);
    summary["failed_roots"][0]["root"]["invocations"][0]["start_byte"] = json!(5);
    check_events(&row, &summary, &sources, &mut errors);
    assert_eq!(errors.len(), 2, "{errors:?}");
    let mut exposure = json!({"status":"full","root":root});
    let mut errors = Vec::new();
    check_roots(&[json!({"paths":[exposure]})], &root, &mut errors);
    assert!(errors.is_empty());
    exposure["root"]["binding"] = json!("other");
    check_roots(&[json!({"paths":[exposure]})], &root, &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn wire_requires_nullable_fields_and_failed_root_blocks() {
    let root = json!({"file":"app.js","binding":"app","construction_byte":0,"invocations":[]});
    let site = json!({"file":"app.js","line":1,"start_byte":0,"end_byte":20});
    let mut route = json!({
        "framework":"hapi", "protocol":"http", "site":site,
        "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
        "methods":null, "path":{"raw":"'/ready'","literal":"/ready"},
        "paths":[{"root":root,"status":"full","value":"/ready","path_tokens":[["literal","/ready"]],
            "normalized":"/ready","host_kind":"unconstrained","hosts":[],"version":null,
            "method_guard":"any","binding_blocked":"any","shadowed":null}],
        "callbacks":[{"position":0,"status":"resolved","role":"pre","group":0,"delegate":null,
            "file":"app.js","name":"prepare","start_line":1,"end_line":1,"start_byte":0,"end_byte":5},
            {"position":1,"status":"resolved","role":"handler","delegate":null,
            "file":"app.js","name":"ready","start_line":1,"end_line":1,"start_byte":6,"end_byte":12}]
    });
    let summary = json!({"enabled_adapters":["hapi"],"registrations":1,"proven":1,"unproven":0,"candidates":0,
        "mounts":0,"diagnostics":[],"failed_roots":[{"root":root,"site":site,"reason":"hapi_handler_conflict"}]});
    let mut errors = Vec::new();
    validate(&[route.clone()], &[], &summary, None, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    route["callbacks"][0]
        .as_object_mut()
        .unwrap()
        .remove("delegate");
    route["paths"][0]
        .as_object_mut()
        .unwrap()
        .remove("binding_blocked");
    validate(&[route], &[], &summary, None, &mut errors);
    assert!(
        errors.iter().any(|error| error.contains("noncanonical")),
        "{errors:?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("binding_blocked")),
        "{errors:?}"
    );
}

#[test]
fn rejects_independent_wire_field_corruptions() {
    let root = json!({"file":"app.js","binding":"app","construction_byte":0,"invocations":[]});
    let site = json!({"file":"app.js","line":1,"start_byte":0,"end_byte":20});
    let route = json!({
        "framework":"express", "protocol":"http", "site":site,
        "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
        "methods":null, "path":{"raw":"'/ready'","literal":"/ready"},
        "paths":[{"root":root,"status":"full","value":"/ready","path_tokens":[["literal","/ready"]],
            "normalized":"/ready","host_kind":"unconstrained","hosts":[],"version":null,"method_guard":"any"}],
        "callbacks":[{"position":0,"status":"unresolved","text":"show","reason":"callback_unproven","delegate":null}]
    });
    let summary = json!({"enabled_adapters":["express"],"registrations":1,"proven":1,"unproven":0,"candidates":0,
        "mounts":0,"diagnostics":[],"failed_roots":[]});
    let mut undetected = Vec::new();
    let mut check = |good: &Value, bad: &Value, field: &str| {
        let mut errors = Vec::new();
        validate(&[good.clone()], &[], &summary, None, &mut errors);
        assert!(errors.is_empty(), "{field}: {errors:?}");
        validate(&[bad.clone()], &[], &summary, None, &mut errors);
        if !errors.iter().any(|error| error.contains(field)) {
            undetected.push((field.to_owned(), errors));
        }
    };
    let mut dynamic = route.clone();
    dynamic["paths"][0].as_object_mut().unwrap().remove("value");
    dynamic["paths"][0]["status"] = json!("dynamic");
    dynamic["paths"][0]["reason"] = json!("non_literal_path");
    dynamic["paths"][0]["path_tokens"] = Value::Null;
    dynamic["paths"][0]["normalized"] = Value::Null;
    let mut bad = dynamic.clone();
    bad["paths"][0]["path_tokens"] = json!([["literal", "/ready"]]);
    bad["paths"][0]["normalized"] = json!("/ready");
    check(&dynamic, &bad, "dynamic path");

    let mut opaque = route.clone();
    opaque["paths"] = json!([]);
    opaque["callbacks"] = json!([]);
    opaque["candidate"] = json!(true);
    opaque["reason"] = json!("hapi_route_not_literal");
    opaque["framework"] = json!("hapi");
    opaque.as_object_mut().unwrap().remove("path");
    opaque.as_object_mut().unwrap().remove("methods");
    let mut bad = opaque.clone();
    bad["callbacks"] = route["callbacks"].clone();
    check(&opaque, &bad, "opaque");

    for (key, value) in [
        ("strict_slashes", Value::Null),
        (
            "version",
            json!({"kind":"header","header":"Version","value":"1"}),
        ),
        ("host_kind", json!("unknown")),
    ] {
        let mut bad = route.clone();
        bad["paths"][0][key] = value;
        check(&route, &bad, key);
    }
    let mut bad = route.clone();
    bad["candidate"] = json!(false);
    check(&route, &bad, "noncanonical");
    let mut bad = route.clone();
    bad["endpoint"] = json!("ready");
    check(&route, &bad, "endpoint");
    let mut bad = route.clone();
    bad["superseded_keys"] = json!([{
        "root":root,"method_guard":"any","path":"/ready","site":site,"reason":"superseded"
    }]);
    check(&route, &bad, "superseded_keys");
    let mut bad = route.clone();
    bad["origin_plugin"] = json!({"file":"app.js","object_byte":0,"name":null});
    check(&route, &bad, "origin_plugin");
    let mut bad = route.clone();
    bad["callbacks"][0]["delegate"] = json!({
        "file":"app.js","name":"show","start_line":1,"end_line":1,"start_byte":0,"end_byte":5,
    });
    check(&route, &bad, "delegate");

    let mut websocket = route.clone();
    websocket["framework"] = json!("fastapi");
    websocket["protocol"] = json!("websocket");
    let mut bad = websocket.clone();
    bad["default_methods"] = json!({"known":["GET"]});
    check(&websocket, &bad, "default methods");

    let mut unproven = route.clone();
    unproven["receiver"] = json!({"status":"unproven","reason":"receiver_unproven"});
    unproven["candidate"] = json!(true);
    unproven["paths"] = json!([]);
    unproven["callbacks"][0]["reason"] = json!("receiver_unproven");
    let mut bad = unproven.clone();
    bad["callbacks"][0]["reason"] = json!("imported_unproven");
    check(&unproven, &bad, "callbacks[0].reason");

    let mut weak = route;
    weak["framework"] = json!("hapi");
    weak["origin_plugin"] = json!({"file":"app.js","object_byte":0,"name":null});
    weak["callbacks"][0]["role"] = json!("handler");
    let mut bad = weak.clone();
    bad["callbacks"][0] = json!({
        "position":0,"status":"unresolved","text":"show","reason":"callback_unproven",
        "delegate":null,"role":"pre","group":0,
    });
    check(&weak, &bad, "role");
    let strong = weak.clone();
    let mut bad = strong.clone();
    bad["receiver"]["proof"] = json!("plugin_contract");
    check(&strong, &bad, "plugin_contract");
    weak["receiver"]["proof"] = json!("plugin_contract");
    weak["paths"][0]["mount_refusal"] = json!("no_known_mount");
    weak["paths"][0]["status"] = json!("suffix");
    weak["paths"][0]["reason"] = json!("no_known_mount");
    weak["paths"][0]["root"] = Value::Null;
    weak["paths"][0]["host_kind"] = json!("unknown");
    let mut bad = weak.clone();
    bad["paths"][0]["root"] = root;
    check(&weak, &bad, "plugin_contract");
    let mut bad = weak.clone();
    bad["paths"][0]["reason"] = json!("variable_prefix");
    check(&weak, &bad, "plugin_contract");
    assert!(undetected.is_empty(), "{undetected:?}");
}

#[test]
fn empty_exposures_use_literal_then_raw_in_wire_order() {
    let row = |raw: &str, literal: &str, start| {
        json!({
            "framework":"express","protocol":"http",
            "site":{"file":"app.js","line":1,"start_byte":start,"end_byte":start + 1},
            "receiver":{"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]},
            "methods":null,"path":{"raw":raw,"literal":literal},"paths":[],"callbacks":[]
        })
    };
    let a = row("'/a'", "/a", 2);
    let a_template = row("`/a`", "/a", 1);
    let z = row("\"/z\"", "/z", 0);
    let summary = json!({"enabled_adapters":["express"],"registrations":3,"proven":3,"unproven":0,"candidates":0,
        "mounts":0,"diagnostics":[],"failed_roots":[]});
    let mut errors = Vec::new();
    validate(
        &[a.clone(), a_template.clone(), z.clone()],
        &[],
        &summary,
        None,
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
    validate(&[a_template, a, z], &[], &summary, None, &mut errors);
    assert!(
        errors.iter().any(|error| error.contains("wire order")),
        "{errors:?}"
    );
}

#[test]
fn source_mount_ordinals_reject_swapped_and_equal_pairs() {
    use super::super::{fixture_root, read_rows};
    let rows = read_rows(&fixture_root().join("rows.jsonl"), &mut Vec::new());
    let row = &rows["ex30"];
    let expected = &row["expect_mounts"];
    let source = std::fs::read_to_string(fixture_root().join("ex30/app.js")).unwrap();
    let sources = BTreeMap::from([("app.js".into(), source)]);
    let mut full_pins = expected.clone();
    resolve_pins(&mut full_pins, &sources, "mounts", &mut Vec::new());
    let mut actual = items(&full_pins).to_vec();
    for (index, edge) in actual.iter_mut().enumerate() {
        edge["alternative_ordinal"] = json!(index);
        edge["child_ordinal"] = json!(0);
    }
    let mut errors = Vec::new();
    compare_collection(expected, &actual, &sources, "expect_mounts", &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    validate_mount_pins(&actual, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    for ordinals in [[1, 0], [0, 0]] {
        for (edge, ordinal) in actual.iter_mut().zip(ordinals) {
            edge["alternative_ordinal"] = json!(ordinal);
        }
        compare_collection(expected, &actual, &sources, "expect_mounts", &mut errors);
        assert!(!errors.is_empty(), "{ordinals:?}");
        errors.clear();
    }
    validate_mount_pins(&actual, &mut errors);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("distinct source ordinals")),
        "{errors:?}"
    );
}

#[test]
fn mount_ordinals_can_repeat_across_invocations_but_not_within_one() {
    let first = json!({
        "parent":{"status":"router","file":"app.js","binding":"app","construction_byte":10,
            "invocations":[{"file":"app.js","start_byte":80,"end_byte":89}]},
        "child":{"status":"router","file":"app.js","binding":"child","construction_byte":25,
            "invocations":[{"file":"app.js","start_byte":80,"end_byte":89}]},
        "site":{"file":"app.js","line":4,"start_byte":45,"end_byte":67},
        "prefix":{"status":"literal","value":"/api"},"alternative_ordinal":0,"child_ordinal":0,
    });
    let mut second = first.clone();
    for endpoint in ["parent", "child"] {
        second[endpoint]["invocations"] = json!([{"file":"app.js","start_byte":91,"end_byte":100}]);
    }
    let mut summary = json!({
        "enabled_adapters":["express"],"registrations":0,"proven":0,"unproven":0,"candidates":0,
        "mounts":3,"diagnostics":[],"failed_roots":[],
    });
    let mut duplicate = first.clone();
    duplicate["prefix"]["value"] = json!("/other");
    let mounts = [first.clone(), second.clone(), duplicate];
    let mut wire_errors = Vec::new();
    let mut pin_errors = Vec::new();
    validate(&[], &mounts, &summary, None, &mut wire_errors);
    validate_mount_pins(&mounts, &mut pin_errors);
    assert!(!wire_errors.is_empty(), "listing accepted duplicate");
    assert!(!pin_errors.is_empty(), "pins accepted duplicate");

    summary["mounts"] = json!(2);
    let mounts = [second, first];
    wire_errors.clear();
    pin_errors.clear();
    validate(&[], &mounts, &summary, None, &mut wire_errors);
    validate_mount_pins(&mounts, &mut pin_errors);
    assert!(
        wire_errors.is_empty() && pin_errors.is_empty(),
        "listing: {wire_errors:?}, pins: {pin_errors:?}"
    );
}

#[test]
fn weak_contract_and_dangling_guards_cannot_create_mount_edges() {
    let router = json!({"status":"router","file":"app.js","binding":"app","construction_byte":0,"invocations":[]});
    let mount = json!({
        "parent":router,"child":router,"site":{"file":"app.js","line":2,"start_byte":20,"end_byte":40},
        "prefix":{"status":"absent"},"alternative_ordinal":0,"child_ordinal":0,
    });
    let summary = json!({
        "enabled_adapters":["hapi"],"registrations":0,"proven":0,"unproven":0,"candidates":0,
        "mounts":1,"diagnostics":[],"failed_roots":[],
    });
    let mut errors = Vec::new();
    validate(&[], &[mount.clone()], &summary, None, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    let mut weak_child = mount.clone();
    weak_child["child"]["proof"] = json!("plugin_contract");
    let mut missing_pattern = mount;
    missing_pattern["end_guard"] = json!(true);
    let mut undetected = Vec::new();
    for (bad, field) in [
        (weak_child, "plugin_contract"),
        (missing_pattern, "end_guard"),
    ] {
        validate(&[], &[bad], &summary, None, &mut errors);
        if !errors.iter().any(|error| error.contains(field)) {
            undetected.push((field, std::mem::take(&mut errors)));
        }
        errors.clear();
    }
    assert!(undetected.is_empty(), "{undetected:?}");
}
