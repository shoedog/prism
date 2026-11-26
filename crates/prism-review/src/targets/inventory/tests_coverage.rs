use super::{assert_http_cases, UnprovenCause};

fn assert_requests_get_after_writes(cases: &[(&str, &str, bool)]) {
    let sources: Vec<_> = cases
        .iter()
        .map(|&(label, write, veto)| {
            let source =
                format!("import requests\n{write}\ndef fetch(u): return requests.get(u)\n");
            (label, source, veto.then_some(UnprovenCause::MemberRebound))
        })
        .collect();
    let cases: Vec<_> = sources
        .iter()
        .map(|(label, source, cause)| (*label, source.as_str(), *cause))
        .collect();
    assert_http_cases(&cases, "requests.get");
}

#[test]
fn escaped_reflection_keys_veto_only_the_decoded_member() {
    for (label, key, neighbor) in [
        ("hex", r"'\x67et'", r"'\x67ex'"),
        ("short Unicode", r"'\u0067et'", r"'\u0067ex'"),
        ("long Unicode", r"'\U00000067et'", r"'\U00000067ex'"),
        ("octal", r"'\147et'", r"'\147ex'"),
        ("short octal followed by text", r"'\147et'", r"'\14get'"),
        ("escaped newline", "'g\\\net'", "'g\\\nex'"),
    ] {
        let matching = format!("setattr(requests, {key}, fake)");
        let other = format!("setattr(requests, {neighbor}, fake)");
        assert_requests_get_after_writes(&[(label, &matching, true), (label, &other, false)]);
    }
}

#[test]
fn recognized_string_escapes_do_not_make_an_unrelated_key_unknown() {
    assert_requests_get_after_writes(&[
        (
            "unrecognized Unicode name retains a veto",
            r"setattr(requests, 'get\N{LATIN SMALL LETTER A WITH ACUTE}', fake)",
            true,
        ),
        (
            "recognized Unicode digit keeps a specific key",
            r"setattr(requests, 'get\N{DIGIT FIVE}', fake)",
            false,
        ),
        (
            "ordinary escaped control characters keep a specific key",
            r#"setattr(requests, 'get\a\b\f\n\r\t\v\'\"\\', fake)"#,
            false,
        ),
        (
            "unrecognized escape preserves the slash and character",
            r"setattr(requests, 'get\q', fake)",
            false,
        ),
    ]);
}

#[test]
fn reflected_subscript_objects_require_a_proven_lookup() {
    assert_requests_get_after_writes(&[
        (
            "computed key",
            "setattr(__builtins__[key], 'get', fake)",
            true,
        ),
        (
            "format string key",
            "setattr(__builtins__[f'{key}'], 'get', fake)",
            true,
        ),
        (
            "ordinary mapping",
            "setattr(mapping['requests'], 'get', fake)",
            true,
        ),
        (
            "proven builtins member",
            "setattr(__builtins__['requests'], 'get', fake)",
            false,
        ),
    ]);
}

#[test]
fn relative_import_writes_do_not_veto_the_external_import() {
    assert_requests_get_after_writes(&[
        (
            "external alias can mutate the external import",
            "import requests as other\nsetattr(other, 'get', fake)",
            true,
        ),
        (
            "relative alias denotes a local object",
            "from . import requests as other\nsetattr(other, 'get', fake)",
            false,
        ),
    ]);
}

#[test]
fn reflection_writes_require_the_builtin_arity() {
    assert_requests_get_after_writes(&[
        (
            "setattr with a value",
            "setattr(requests, 'get', fake)",
            true,
        ),
        ("setattr lacks a value", "setattr(requests, 'get')", false),
        (
            "setattr has an extra argument",
            "setattr(requests, 'get', fake, extra)",
            false,
        ),
        (
            "delattr with two arguments",
            "delattr(requests, 'get')",
            true,
        ),
        (
            "delattr has a value argument",
            "delattr(requests, 'get', fake)",
            false,
        ),
    ]);
}
