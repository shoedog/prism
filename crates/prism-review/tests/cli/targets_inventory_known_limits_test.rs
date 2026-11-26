use super::targets_inventory_constructors_test::{
    assert_exact, assert_refusal, identity_warnings, source_document,
};
use super::targets_inventory_helpers_test::assert_nameonly;

const SESSION_GET: &str = "requests.Session.get";

fn with_returned_local(body: &str, receiver: &str) -> String {
    format!(
        "from requests import Session\ndef make(obj=None) -> Session:\n    v = Session()\n    {body}\n    return v\ndef use():\n    {receiver} = make()\n    s.get('u')\n"
    )
}

#[test]
fn grouped_starred_unpacking() {
    // KNOWN-LIMIT (Sol 2, DESIGN 10.2). The grouped star can hide the selected write.
    for (constructor, receiver, confidence) in [
        ("Session()", "v", "exact"),
        ("make()", "v: Session", "nameonly"),
        ("make()", "v", "nameonly"),
    ] {
        let source = |write: &str| {
            source_document(&format!(
                "from requests import Session\ndef make() -> Session:\n    return Session()\ndef use():\n    {receiver} = {constructor}\n    {write}\n    v.get('u')\n"
            ))
        };
        for (selected, sibling) in [
            (r#"vars(v)["get"]"#, r#"vars(v)["post"]"#),
            (r#"v.__dict__["get"]"#, r#"v.__dict__["post"]"#),
            ("v.get", "v.post"),
        ] {
            let limit = source(&format!("[*({selected})] = [len]"));
            let [row] = limit["targets"].as_array().unwrap().as_slice() else {
                panic!("expected one target for {selected}, got {limit:#}");
            };
            assert_eq!(row["dependency_hint"]["callee"], SESSION_GET);
            assert_eq!(row["confidence"], confidence);
            assert_eq!(row["kind"], "other");
            assert_eq!(row["expected"]["detail"], "parse_error_in_function");
            assert!(identity_warnings(&limit).is_empty());

            let control = source(&format!("[{sibling}] = [len]"));
            if confidence == "exact" {
                assert_exact(&control, SESSION_GET);
            } else {
                assert_nameonly(&control, SESSION_GET);
            }
            assert_refusal(&source(&format!("[{selected}] = [len]")), "member_rebound");
        }
    }

    // The grouped star can also make a returned local's write look like an escape.
    for key in ["get", "post"] {
        let body = format!(r#"[*(vars(v)["{key}"])] = [len]"#);
        assert_refusal(
            &source_document(&with_returned_local(&body, "s")),
            "receiver_value_unproven",
        );
        assert_nameonly(
            &source_document(&with_returned_local(r#"vars(v)["post"] = len"#, "s")),
            SESSION_GET,
        );
    }
}

#[test]
fn starred_reflection_calls() {
    // KNOWN-LIMIT (Opus, DESIGN 10.2). The star can attach to the reflection callee.
    let globals = |binding: &str, write: &str, callee: &str| {
        source_document(&format!(
            "import requests\n{binding}\ndef use(cm=None):\n    {write}\n    {callee}('u')\n"
        ))
    };
    for (target, plain) in [
        ("*TARGET, = [None]", "TARGET = None"),
        ("[*TARGET] = [None]", "[TARGET] = [None]"),
        ("(*TARGET,) = [None]", "(TARGET,) = [None]"),
        (
            "with cm as [*TARGET]:\n        pass",
            "with cm as [TARGET]:\n        pass",
        ),
    ] {
        let source = |shape: &str, key: &str| {
            globals(
                "s = requests.Session()",
                &shape.replace("TARGET", &format!(r#"globals()["{key}"]"#)),
                "s.get",
            )
        };
        assert_exact(&source(target, "s"), SESSION_GET);
        assert_exact(&source(plain, "other"), SESSION_GET);
        assert_refusal(&source(plain, "s"), "receiver_not_unique");
    }
    for (target, plain) in [
        ("*TARGET, = [None]", "TARGET = None"),
        (
            "with cm as [*TARGET]:\n        pass",
            "with cm as [TARGET]:\n        pass",
        ),
    ] {
        let source = |shape: &str, key: &str| {
            globals(
                "",
                &shape.replace("TARGET", &format!(r#"globals()["{key}"]"#)),
                "requests.get",
            )
        };
        assert_exact(&source(target, "requests"), "requests.get");
        assert_exact(&source(plain, "other"), "requests.get");
        assert_refusal(&source(plain, "requests"), "shadowed_root");
    }

    let vars = |target: &str| {
        source_document(&format!(
            "from requests import Session\ndef use(cm):\n    s = Session()\n    with cm as {target}:\n        pass\n    s.get('u')\n"
        ))
    };
    assert_exact(&vars(r#"[*vars(s)["get"]]"#), SESSION_GET);
    assert_exact(&vars(r#"(*vars(s)["post"],)"#), SESSION_GET);
    assert_refusal(&vars(r#"(*vars(s)["get"],)"#), "member_rebound");
}

#[test]
fn fstring_format_spec_lookalike() {
    // KNOWN-LIMIT (Sol 3, DESIGN 10.2). Recovery drops the bare read before the `=` format spec.
    for receiver in ["s: Session", "s"] {
        let run = |body| source_document(&with_returned_local(body, receiver));
        for interpolation in [
            r#"f"{v:=12}""#,
            r#"f"{0:{v:=12}}""#,
            r#"f"{ v := 12}""#,
            r#"f"{v:=}""#,
        ] {
            assert_nameonly(&run(interpolation), SESSION_GET);
        }
        assert_nameonly(&run(r#"f"{v.cookies}""#), SESSION_GET);
        assert_refusal(&run(r#"f"{v:12}""#), "receiver_value_unproven");
    }
}

#[test]
fn nested_dotted_pattern_captures() {
    // KNOWN-LIMIT (Sol 4, DESIGN 10.2). A dotted pattern base can hide a nested capture.
    for receiver in ["s: Session", "s"] {
        for (scope, pattern) in [
            ("def inner():", "v.cookies"),
            ("def inner():", "list(v=v.cookies)"),
            ("class Inner:", "v.cookies()"),
            ("class Inner:", "{v.cookies: _}"),
        ] {
            let body = format!("{scope}\n        match obj:\n            case {pattern}: pass");
            assert_nameonly(
                &source_document(&with_returned_local(&body, receiver)),
                SESSION_GET,
            );
            let same_scope = format!("match obj:\n        case {pattern}: pass");
            assert_nameonly(
                &source_document(&with_returned_local(&same_scope, receiver)),
                SESSION_GET,
            );
        }
        let shadowed = "def inner(v):\n        match obj:\n            case v.cookies: pass";
        assert_nameonly(
            &source_document(&with_returned_local(shadowed, receiver)),
            SESSION_GET,
        );
    }
}

#[test]
fn r4_helper_eligibility_reason_precedence() {
    // KNOWN-LIMIT (Sol 5, DESIGN 10.2). The R4 row is refused with a different cause.
    let run = |factory: &str, caller: &str| {
        source_document(&format!(
            "from requests import Session\nfrom httpx import Client\n{factory}\n{caller}\n"
        ))
    };
    let local = "def use():\n    v: Session = make()\n    v.get('u')";
    let default = "def use(v: Session = make()):\n    v.get('u')";
    let hidden_default = "def outer(make):\n    def use(v: Session = make()):\n        v.get('u')";
    let conflict = "def make() -> Client:\n    return Client()";
    for caller in [
        "def use(make):\n    v: Session = make()\n    v.get('u')",
        hidden_default,
    ] {
        assert_refusal(&run(conflict, caller), "annotation_conflict");
    }
    for (caller, reason) in [
        (local, "helper_unannotated"),
        (default, "receiver_value_unproven"),
    ] {
        assert_refusal(&run("def make():\n    return Client()", caller), reason);
        assert_refusal(
            &run("def make() -> Client:\n    return opaque()", caller),
            "annotation_conflict",
        );
    }
    for result in ["return None", "return", "pass"] {
        assert_refusal(
            &run(&format!("def make():\n    {result}"), local),
            "receiver_value_unproven",
        );
        assert_refusal(
            &run(&format!("def make() -> Session:\n    {result}"), local),
            "receiver_is_none",
        );
    }
    let good = "def make() -> Session:\n    return Session()";
    assert_nameonly(&run(good, local), SESSION_GET);
    assert_exact(
        &run(
            good,
            "def use(v: Session = make()):\n    s = Session()\n    s.get('u')",
        ),
        SESSION_GET,
    );
}

#[test]
fn special_dictionary_keys_count_as_member_rebound() {
    // KNOWN-LIMIT (Sol 6, DESIGN 10.2). A dictionary entry is counted like attribute replacement.
    for namespace in ["vars(v)", "v.__dict__"] {
        let write = |key| format!(r#"{namespace}["{key}"] = object"#);
        let run = |key| {
            source_document(&format!(
                "from requests import Session\ndef use():\n    v = Session()\n    {}\n    v.get('u')\n",
                write(key)
            ))
        };
        for key in ["__class__", "__dict__"] {
            assert_refusal(&run(key), "member_rebound");
            assert_refusal(
                &source_document(&with_returned_local(&write(key), "s")),
                "member_rebound",
            );
        }
        assert_exact(&run("post"), SESSION_GET);
        assert_nameonly(
            &source_document(&with_returned_local(&write("post"), "s")),
            SESSION_GET,
        );
        assert_refusal(&run("get"), "member_rebound");
    }
    let module_entry = |key| {
        source_document(&format!(
            "import requests\ndef use():\n    vars(requests)[{key:?}] = object\n    requests.Session().get('u')\n"
        ))
    };
    assert_refusal(&module_entry("__class__"), "member_rebound");
    assert_exact(&module_entry("post"), SESSION_GET);
}
