use super::targets_inventory_constructors_test::{
    assert_refusal, identity_warnings, source_document,
};
use serde_json::{json, Value};

fn helper(definition: &str, assignment: &str, method: &str) -> Value {
    source_document(&format!(
        "from requests import Session\nfrom httpx import Client\nfrom typing import Optional as Maybe, Annotated, Union\n{definition}\nasync def fetch(u):\n    {assignment}\n    return client.{method}(u)\n"
    ))
}

pub(super) fn assert_nameonly(doc: &Value, canonical: &str) {
    let [row] = doc["targets"].as_array().unwrap().as_slice() else {
        panic!("expected one target for {canonical}, got {doc:#}");
    };
    assert_eq!(row["kind"], "external_call");
    assert_eq!(row["confidence"], "nameonly");
    assert_eq!(row["tier"], "candidate");
    assert_eq!(row["dependency_hint"]["callee"], canonical);
    assert!(identity_warnings(doc).is_empty());
}

#[test]
fn helper_value_direct_declarations() {
    let factory = "def make() -> Session:\n    return Session()";
    for annotation in [
        "Session",
        "Maybe[Session]",
        "(Session | None)",
        "None | Session",
        "'Sess' 'ion'",
        "'Maybe[Session]'",
        "Maybe['Session']",
    ] {
        let declaration = format!("def make() -> {annotation}:\n    return Session()");
        assert_nameonly(
            &helper(&declaration, "client = make()", "get"),
            "requests.Session.get",
        );
    }
    for annotation in [
        "Annotated[Session, 'value']",
        "Union[Session, None]",
        "Session | Client",
        "list[Session]",
        "session_alias",
        "'\"Session\"'",
    ] {
        let declaration = format!("def make() -> {annotation}:\n    return Session()");
        assert_refusal(
            &helper(&declaration, "client = make()", "get"),
            "helper_unannotated",
        );
        assert_nameonly(
            &helper(factory, "client = make()", "get"),
            "requests.Session.get",
        );
    }
    let promoted = "def make() -> Client:\n    return Session()";
    assert_nameonly(
        &helper(promoted, "client = make()", "get"),
        "requests.Session.get",
    );
    assert_refusal(
        &helper(promoted, "client = make()", "stream"),
        "annotation_conflict",
    );
    let silent = helper(promoted, "client = make()", "close");
    assert_eq!(silent["targets"], json!([]));
    assert!(identity_warnings(&silent).is_empty());
}

#[test]
fn helper_value_direct_eligibility() {
    let control = "def make() -> Session:\n    return Session()";
    for (definition, assignment, cause) in [
        (
            "@decorate\ndef make():\n    return Session()",
            "client = make()",
            "helper_unannotated",
        ),
        (
            "@decorate\ndef make() -> Session:\n    return Session()",
            "client = make()",
            "helper_not_admissible",
        ),
        (
            "async def make() -> Session:\n    return Session()",
            "client = make()",
            "helper_not_admissible",
        ),
        (control, "client = await make()", "helper_not_admissible"),
        (
            control,
            "make = other\n    client = make()",
            "helper_not_admissible",
        ),
        (
            "def make() -> Session:\n    if False: yield 1\n    return Session()",
            "client = make()",
            "helper_not_admissible",
        ),
        (
            "def make() -> Session:\n    if False: yield from values\n    return Session()",
            "client = make()",
            "helper_not_admissible",
        ),
    ] {
        assert_refusal(&helper(definition, assignment, "get"), cause);
        assert_nameonly(
            &helper(control, "client = make()", "get"),
            "requests.Session.get",
        );
    }
    assert_nameonly(
        &helper(
            "if flag:\n    def make() -> Session:\n        return Session()",
            "client = make()",
            "get",
        ),
        "requests.Session.get",
    );
    assert_nameonly(
        &helper(
            "def make() -> Session:\n    def nested(): yield 1\n    return Session()",
            "client = make()",
            "get",
        ),
        "requests.Session.get",
    );
}

#[test]
fn helper_value_direct_none_and_conflicts() {
    let control = "def make() -> Session:\n    return Session()";
    for body in [
        "return None",
        "return",
        "pass",
        "if flag: return None\n    raise ValueError",
    ] {
        let definition = format!("def make() -> Session:\n    {body}");
        assert_refusal(
            &helper(&definition, "client = make()", "get"),
            "receiver_is_none",
        );
        assert_nameonly(
            &helper(control, "client = make()", "get"),
            "requests.Session.get",
        );
    }
    assert_refusal(
        &helper(control, "client: Session = None", "get"),
        "receiver_is_none",
    );
    assert_refusal(
        &source_document("from requests import Session\ndef fetch(client: Session | None = None):\n    return client.get(u)"),
        "receiver_value_unproven",
    );
    for value in ["{}", "[1]", "{1}", "(1,)", "'x'", "b'x'", "1", "False"] {
        let definition = format!("def make() -> Session:\n    return {value}");
        assert_refusal(
            &helper(&definition, "client = make()", "get"),
            "annotation_conflict",
        );
        assert_nameonly(
            &helper(control, "client = make()", "get"),
            "requests.Session.get",
        );
    }
    for body in [
        "return opaque()",
        "if flag: return Session()",
        "if flag: return Session()\n    return Session()",
    ] {
        let definition = format!("def make() -> Session:\n    {body}");
        assert_refusal(
            &helper(&definition, "client = make()", "get"),
            "receiver_value_unproven",
        );
        assert_nameonly(
            &helper(control, "client = make()", "get"),
            "requests.Session.get",
        );
    }
}

#[test]
fn helper_returned_local_escape_allow_list() {
    let result = |setup: &str| {
        helper(
            &format!("def make() -> Session:\n    v = Session()\n    {setup}\n    return v"),
            "client = make()",
            "get",
        )
    };
    for escape in [
        "use(v)",
        "use(value=nested(v))",
        "v.mount(v)",
        "alias = v",
        "v = other",
        "del v",
        "items = [v]",
        "def capture(): return v.get(u)",
    ] {
        let doc = result(escape);
        assert_eq!(
            identity_warnings(&doc),
            ["inventory: identity unproven: probe.py: 1 (receiver_value_unproven: 1)"],
            "{escape}"
        );
        assert!(doc["targets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["site"]["symbol"] != "fetch"));
        assert_nameonly(&result("v.mount(u)"), "requests.Session.get");
    }
    assert_nameonly(
        &result("def unrelated(v): return v.get(u)"),
        "requests.Session.get",
    );
    assert_nameonly(
        &result("for value in values:\n        if value: v.mount(value)"),
        "requests.Session.get",
    );
}

#[test]
fn helper_returned_local_selected_slot_precedes_escape() {
    let result = |setup: &str| {
        helper(
            &format!("def make() -> Session:\n    v = Session()\n    {setup}\n    return v"),
            "client = make()",
            "send",
        )
    };
    for write in [
        "v.send = replacement",
        "alias = v\n    alias.send = replacement",
        "v.__class__ = Other",
        "v.__dict__ = {}",
        "v.__dict__['send'] = replacement",
        "vars(v)['send'] = replacement",
        "setattr(v, 'send', replacement)",
    ] {
        assert_refusal(&result(write), "member_rebound");
        assert_nameonly(&result("v.post = replacement"), "requests.Session.send");
    }
    // KNOWN LIMIT (DESIGN §10): the saved mapping can change dispatch.
    assert_nameonly(
        &result("namespace = v.__dict__\n    namespace['send'] = replacement"),
        "requests.Session.send",
    );
    assert_refusal(
        &result("v.__dict__['send'] = replacement"),
        "member_rebound",
    );
}

#[test]
fn helper_binding_shape_requires_a_simple_assignment() {
    let factory = "def make() -> Session:\n    return Session()";
    for assignment in [
        "other = client = make()",
        "client = other = make()",
        "client, _ = make()",
        "(client := make())",
    ] {
        let doc = helper(factory, assignment, "get");
        assert_eq!(doc["targets"], json!([]), "{assignment}");
        assert!(identity_warnings(&doc).is_empty(), "{assignment}");
    }
    assert_nameonly(
        &helper(factory, "client = make()", "get"),
        "requests.Session.get",
    );
}
