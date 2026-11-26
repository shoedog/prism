use super::targets_inventory_stdlib_roots_test::{check, scan};

const NONE: &str = "1 (receiver_is_none: 1)";
const REBOUND: &str = "1 (member_rebound: 1)";

fn helper_source(body: &str) -> String {
    format!("from pathlib import Path\ndef make() -> Path:\n{body}\ndef read():\n    p = make()\n    return p.read_text()\n")
}

fn path_read(line: u64) -> [(&'static str, &'static str, &'static str, u64); 1] {
    [("filesystem", "pathlib.Path.read_text", "nameonly", line)]
}

#[test]
fn straight_line_helpers_supply_implicit_none() {
    check(
        &scan(&helper_source("    return opaque()"), &[]),
        &path_read(6),
        None,
    );
    for body in ["    pass", "    return", "    return None"] {
        check(&scan(&helper_source(body), &[]), &[], Some(NONE));
    }
    for body in [
        "    'doc'",
        "    marker = 1",
        "    ...",
        "    opaque()",
        "    'doc'\n    marker = 1\n    opaque()",
    ] {
        check(&scan(&helper_source(body), &[]), &[], Some(NONE));
    }
}

#[test]
fn conditional_helpers_supply_implicit_none_when_an_arm_completes() {
    for (body, line) in [
        (
            "    if flag:\n        raise First\n    else:\n        raise Second",
            9,
        ),
        (
            "    if True:\n        raise Error\n    else:\n        pass",
            9,
        ),
        ("    if True: raise Error\n    marker = 1", 7),
        ("    raise Error\n    marker = 1", 7),
        ("    if flag: return opaque()\n    marker = 1", 7),
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(line), None);
    }
    for body in [
        "    if flag: raise Error",
        "    if flag: raise Error\n    else: marker = 1",
        "    if flag: raise Error\n    elif other: marker = 1\n    else: raise Error",
        "    if False: raise Error\n    marker = 1",
    ] {
        check(&scan(&helper_source(body), &[]), &[], Some(NONE));
    }
}

#[test]
fn while_helpers_supply_implicit_none_only_with_a_completion_witness() {
    for (body, line) in [
        ("    while True:\n        pass", 7),
        ("    while (True):\n        pass\n    marker = 1", 8),
        ("    while True:\n        while True: break", 7),
        ("    while True:\n        if False: break", 7),
        ("    while True:\n        raise Error\n        break", 8),
        ("    while False: pass\n    else: raise Error", 7),
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(line), None);
    }
    for body in [
        "    while True: break",
        "    while True:\n        if flag: break",
        "    while True: break\n    else: raise Error",
        "    while False: raise Error",
    ] {
        check(&scan(&helper_source(body), &[]), &[], Some(NONE));
    }
}

#[test]
fn opaque_compound_statements_do_not_supply_a_completion_witness() {
    for (body, line) in [
        (
            "    try:\n        raise Error\n    finally:\n        pass",
            9,
        ),
        ("    with opaque():\n        pass", 7),
        (
            "    match value:\n        case _:\n            raise Error",
            8,
        ),
        ("    with opaque():\n        pass\n    marker = 1", 8),
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(line), None);
    }
}

fn check_mapping_reexports(module: &str) {
    for class in ["Mapping", "MutableMapping"] {
        for (setup, owner) in [
            (format!("import {module}\n\n"), format!("{module}.{class}")),
            (
                format!("import {module}\nAlias = {module}.{class}\n"),
                "Alias".to_owned(),
            ),
            (
                format!("from {module} import {class} as Imported\n\n"),
                "Imported".to_owned(),
            ),
            (
                format!("from {module} import {class}\nAlias = {class}\n"),
                "Alias".to_owned(),
            ),
        ] {
            let source = |owner: &str, member: &str| {
                format!("import os\nfrom collections.abc import Sequence\n{setup}{owner}.{member} = replacement\ndef read(key):\n    os.environ.get(key)\n    os.getenv(key)\n")
            };
            let getenv = ("config", "os.getenv", "exact", 8);
            let exact = [("config", "os.environ.get", "exact", 7), getenv];
            check(&scan(&source(&owner, "sibling"), &[]), &exact, None);
            check(&scan(&source("Sequence", "get"), &[]), &exact, None);
            check(&scan(&source(&owner, "get"), &[]), &[getenv], Some(REBOUND));
        }
    }
}

#[test]
fn environ_get_follows_os_mapping_reexports() {
    check_mapping_reexports("os");
}

#[test]
fn environ_get_follows_typing_mapping_reexports() {
    check_mapping_reexports("typing");
}

#[test]
fn known_limit_starred_annotation_refusals() {
    let source = |annotation| {
        format!("import json\ndef outer():\n    def inner(*items: {annotation}):\n        pass\n    json.loads('null')\n")
    };
    check(
        &scan(&source("*(json.loads('[]'))"), &[]),
        &[
            ("parse", "json.loads", "exact", 3),
            ("parse", "json.loads", "exact", 5),
        ],
        None,
    );
    for annotation in [
        r#"*(f"{json.loads('[]')}")"#,
        "*((item := json.loads('[]')))",
        "*(yield json.loads('[]'))",
        "*(lambda item=json.loads('[]'): item)",
    ] {
        let doc = scan(&source(annotation), &[]);
        let [row] = doc["targets"].as_array().unwrap().as_slice() else {
            panic!("expected only the following call for {annotation}, got {doc:#}");
        };
        assert_eq!(row["site"]["line"], 5, "{annotation}");
        assert_eq!(row["kind"], "other", "{annotation}");
        assert_eq!(
            row["dependency_hint"]["callee"], "json.loads",
            "{annotation}"
        );
        assert_eq!(
            row["expected"]["detail"], "parse_error_in_function",
            "{annotation}"
        );
    }
}

#[test]
fn known_limit_starred_yield_does_not_disqualify_helper() {
    let source = |annotation| {
        helper_source(&format!(
            "    def inner(*a: {annotation}):\n        pass\n    return Path()"
        ))
    };
    check(
        &scan(&source("(yield None)"), &[]),
        &[],
        Some("1 (helper_not_admissible: 1)"),
    );
    check(&scan(&source("*(yield None)"), &[]), &path_read(8), None);
}

#[test]
fn known_limit_duplicate_parameter_beside_starred_annotation() {
    let source = |second| {
        format!("import json\ndef outer():\n    def inner(*items: *json.loads('[]'), {second}):\n        pass\n")
    };
    for second in ["other", "items"] {
        check(
            &scan(&source(second), &[]),
            &[("parse", "json.loads", "exact", 3)],
            None,
        );
    }
}
