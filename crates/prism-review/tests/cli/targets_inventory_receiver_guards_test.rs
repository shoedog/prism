use super::targets_inventory_constructors_test::{
    assert_exact, assert_refusal, identity_warnings, source_document,
};
use super::targets_inventory_helpers_test::assert_nameonly;
use super::targets_inventory_test::{document, run};
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

fn assert_silent(doc: &Value) {
    assert_eq!(doc["targets"], json!([]), "{doc:#}");
    assert!(identity_warnings(doc).is_empty(), "{doc:#}");
}

#[test]
fn untracked_parameter_write_keeps_the_root_member_veto() {
    let scan = |key| {
        source_document(&format!(
            "import requests\ndef alter(obj):\n    alias = obj\n    vars(alias)['{key}'] = replacement\ndef root(u):\n    return requests.get(u)\ndef receiver(u):\n    s = requests.Session()\n    return s.get(u)\n"
        ))
    };

    let control = scan("post");
    let rows = control["targets"].as_array().unwrap();
    assert!(identity_warnings(&control).is_empty());
    assert_eq!(rows.len(), 2, "{control:#}");
    assert!(rows.iter().any(|row| {
        row["site"]["line"] == 6
            && row["confidence"] == "exact"
            && row["dependency_hint"]["callee"] == "requests.get"
    }));

    let refused = scan("get");
    assert_eq!(
        identity_warnings(&refused),
        ["inventory: identity unproven: probe.py: 1 (member_rebound: 1)"]
    );
    let [receiver] = refused["targets"].as_array().unwrap().as_slice() else {
        panic!("expected only the constructed receiver, got {refused:#}");
    };
    assert_eq!(receiver["site"]["line"], 9);
    assert_eq!(receiver["confidence"], "exact");
    assert_eq!(
        receiver["dependency_hint"]["callee"],
        "requests.Session.get"
    );
}

#[test]
fn untracked_parameter_write_vetoes_a_matching_import_capture() {
    let scan = |member| {
        source_document(&format!(
            "def fetch(obj, u):\n    obj.{member} = replacement\n    from requests import get\n    return get(u)\n"
        ))
    };
    assert_exact(&scan("post"), "requests.get");
    assert_refusal(&scan("get"), "member_rebound");
}

#[test]
fn inline_constructor_checks_repository_origin_outside_the_file_filter() {
    let source = "from requests import Session\ndef fetch(u):\n    return Session().get(u)\n";
    assert_exact(&source_document(source), "requests.Session.get");

    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("probe.py"), source).unwrap();
    fs::create_dir_all(repo.path().join("src/requests")).unwrap();
    fs::write(
        repo.path().join("src/requests/__init__.py"),
        "class Session: pass\n",
    )
    .unwrap();
    assert_refusal(
        &document(&run(repo.path(), &["--files", "probe.py"])),
        "repository_local_module",
    );
}

#[test]
fn constructor_slot_refusals_require_the_construction_read() {
    for slot in ["__new__", "__init__"] {
        let header = format!("import requests\nrequests.Session.{slot} = replacement\n");
        assert_exact(
            &source_document(&format!(
                "{header}def fetch(s, u):\n    return requests.Session.get(s, u)\n"
            )),
            "requests.Session.get",
        );
        assert_refusal(
            &source_document(&format!(
                "{header}def fetch(u):\n    return requests.Session().get(u)\n"
            )),
            "member_rebound",
        );
    }
}

#[test]
fn named_receiver_requires_an_eligible_binding_and_context_shape() {
    let scan = |body| source_document(&format!("from requests import Session\n{body}\n"));
    for body in [
        "def fetch(u):\n    ignored = s = Session()\n    return s.get(u)",
        "def fetch(u):\n    s: Other = Session()\n    return s.get(u)",
        "def fetch(u):\n    with Session() as s:\n        return s.get(u)",
        "def fetch(u):\n    s = Session()\n    return [s for s in s.get(u)]",
    ] {
        assert_exact(&scan(body), "requests.Session.get");
    }
    for body in [
        "def fetch(u):\n    s, *_ = [Session()]\n    return s.get(u)",
        "def fetch(u):\n    (s := Session())\n    return s.get(u)",
        "def fetch(u):\n    for s in [Session()]:\n        return s.get(u)",
        "def fetch(u):\n    s = Session()\n    return [s.get(u) for s in values]",
        "def fetch(u, s=Session()):\n    return s.get(u)",
        "def fetch(u):\n    s = Session()\n    alias = s\n    return alias.get(u)",
        "def fetch(u):\n    with wrapper(Session()) as s:\n        return s.get(u)",
        "def fetch(u):\n    return Session().__enter__().get(u)",
        "class Example:\n    s = Session()\n    result = s.get(u)",
        "async def fetch(u):\n    async with Session() as s:\n        return s.get(u)",
    ] {
        assert_silent(&scan(body));
    }
}

#[test]
fn named_receiver_freshness_depends_on_escape_and_loop_reentry() {
    let scan = |body: &str| {
        source_document(&format!(
            "from requests import Session\ndef fetch(u):\n{body}\n"
        ))
    };
    for setup in ["s.mount(u)", "save(s)"] {
        let doc = scan(&format!("    s = Session()\n    {setup}\n    result = s.get(u)\n    s.get = replacement\n    return result"));
        if setup == "save(s)" {
            assert_refusal(&doc, "member_rebound");
        } else {
            assert_exact(&doc, "requests.Session.get");
        }
    }
    assert_exact(
        &scan("    for item in u:\n        s = Session()\n        result = s.get(item)\n        s.get = replacement\n    return result"),
        "requests.Session.get",
    );
    assert_refusal(
        &scan("    s = Session()\n    for item in u:\n        result = s.get(item)\n        s.get = replacement\n    return result"),
        "member_rebound",
    );
}

#[test]
fn named_receiver_vetoes_a_reflected_rebind_of_the_deciding_name() {
    let scan = |key| {
        source_document(&format!("from requests import Session\ns = Session()\nglobals()['{key}'] = replacement\ndef fetch(u):\n    return s.get(u)\n"))
    };
    assert_exact(&scan("other"), "requests.Session.get");
    assert_refusal(&scan("s"), "receiver_not_unique");
}

#[test]
fn later_context_entry_can_affect_a_persistent_receiver() {
    let scan = |prefix: &str, initialization: &str| {
        source_document(&format!("from requests import Session\nSession.__exit__ = replacement\n{prefix}def fetch(u):\n{initialization}    result = s.get(u)\n    with s:\n        pass\n    return result\n"))
    };
    assert_exact(&scan("", "    s = Session()\n"), "requests.Session.get");
    assert_refusal(&scan("s = Session()\n", ""), "member_rebound");
}

#[test]
fn field_tie_requires_the_owning_instance_parameter_and_literal_store() {
    let scan = |methods| {
        source_document(&format!(
            "from requests import Session\nclass API:\n{methods}\n"
        ))
    };
    assert_refusal(
        &scan("    def fetch(self, u):\n        self.s = Session()\n        return self.s.get(u)"),
        "receiver_value_unproven",
    );
    // DESIGN 6.12 (relaxed discovery): a constructor tie whose using method fails
    // the shape still supplies the diagnostic, never a row.
    for methods in [
        "    @staticmethod\n    def fetch(self, u):\n        self.s = Session()\n        return self.s.get(u)",
        "    def fetch(other, self, u):\n        self.s = Session()\n        return self.s.get(u)",
    ] {
        assert_refusal(&scan(methods), "receiver_value_unproven");
    }
    for methods in [
        "    def fetch(self, u):\n        self.s: Session\n        return self.s.get(u)",
        "    def fetch(self, u):\n        setattr(self, slot, Session())\n        return self.s.get(u)",
        "    def fetch(self, u):\n        self.other = Session()\n        return self.s.get(u)",
    ] {
        assert_silent(&scan(methods));
    }
}

#[test]
fn successful_receiver_proof_can_resolve_a_shadowed_root_spelling() {
    let scan = |value| {
        source_document(&format!("from requests import Session\ndef fetch(u):\n    requests = {value}\n    return requests.get(u)\n"))
    };
    assert_exact(&scan("Session()"), "requests.Session.get");
    assert_refusal(&scan("object()"), "shadowed_root");
    assert_refusal(
        &source_document("from requests import Session\ndef fetch(u):\n    requests = Session()\n    requests = object()\n    return requests.get(u)\n"),
        "shadowed_root",
    );
}

#[test]
fn inherited_allocation_and_lookup_slots_are_checked_through_their_owner() {
    for (import, slot) in [
        (
            "from requests.sessions import SessionRedirectMixin as Owner",
            "__new__",
        ),
        ("from builtins import object as Owner", "__getattribute__"),
    ] {
        let scan = |key| {
            source_document(&format!("from requests import Session\n{import}\nOwner.{key} = replacement\ndef fetch(u):\n    return Session().get(u)\n"))
        };
        assert_exact(&scan("post"), "requests.Session.get");
        assert_refusal(&scan(slot), "member_rebound");
    }
}

fn declared(body: &str) -> Value {
    source_document(&format!("from requests import Session\nfrom typing import TYPE_CHECKING\ndef make() -> Session:\n    return Session()\ndef fetch(u):\n{body}\n"))
}

#[test]
fn helper_and_annotated_locals_require_an_available_unique_binding() {
    for assignment in ["client = make()", "client: Session = make()"] {
        assert_nameonly(
            &declared(&format!("    {assignment}\n    return client.get(u)")),
            "requests.Session.get",
        );
        for body in [
            format!("    client.get(u)\n    {assignment}"),
            format!("    if TYPE_CHECKING:\n        {assignment}\n    return client.get(u)"),
        ] {
            assert_refusal(&declared(&body), "binding_unavailable");
        }
        assert_refusal(
            &declared(&format!(
                "    {assignment}\n    client = other\n    return client.get(u)"
            )),
            "receiver_not_unique",
        );
    }
    assert_refusal(
        &declared("    client: Session = None\n    client = other\n    return client.get(u)"),
        "receiver_not_unique",
    );
}

#[test]
fn helper_and_annotated_locals_check_the_caller_instance_slots() {
    for assignment in ["client = make()", "client: Session = make()"] {
        for write in [
            "client.{key} = replacement",
            "alias = client\n    vars(alias)['{key}'] = replacement",
        ] {
            let scan = |key| {
                declared(&format!(
                    "    {assignment}\n    {}\n    return client.get(u)",
                    write.replace("{key}", key)
                ))
            };
            assert_nameonly(&scan("post"), "requests.Session.get");
            assert_refusal(&scan("get"), "member_rebound");
        }
    }

    let scan = |key| {
        source_document(&format!("from requests import Session\ndef make() -> Session:\n    return Session()\nSession.{key} = replacement\ndef fetch(u):\n    client = make()\n    with client:\n        pass\n    return client.get(u)\n"))
    };
    assert_nameonly(&scan("post"), "requests.Session.get");
    assert_refusal(&scan("__exit__"), "member_rebound");
}

#[test]
fn helper_candidate_must_be_a_unique_visible_module_definition() {
    let factory = "def make() -> Session:\n    return Session()";
    let scan = |definition: &str, setup: &str| {
        source_document(&format!("from requests import Session\n{definition}\ndef fetch(u):\n{setup}    client = make()\n    return client.get(u)\n"))
    };
    assert_nameonly(&scan(factory, ""), "requests.Session.get");
    for (definition, setup) in [
        (format!("{factory}\n{factory}"), ""),
        (format!("{factory}\nglobals()['make'] = replacement"), ""),
        (
            String::new(),
            "    def make() -> Session:\n        return Session()\n",
        ),
    ] {
        assert_refusal(&scan(&definition, setup), "helper_not_admissible");
    }
    assert_silent(&scan("", "    make = factory\n"));
    assert_silent(&scan("def make():\n    return {}", ""));
    assert_refusal(
        &scan("def make():\n    return Session()", ""),
        "helper_unannotated",
    );
}

#[test]
fn helper_generator_check_uses_its_compiler_scope() {
    let scan = |setup| {
        source_document(&format!("from requests import Session\ndef make() -> Session:\n    {setup}\n    return Session()\ndef fetch(u):\n    client = make()\n    return client.get(u)\n"))
    };
    for (negative, control) in [
        (
            "def nested(value=(yield 1)): pass",
            "def nested(value=1): pass",
        ),
        ("class Nested((yield 1)): pass", "class Nested(Base): pass"),
        ("note: (yield 1)", "note: int"),
    ] {
        assert_nameonly(&scan(control), "requests.Session.get");
        assert_refusal(&scan(negative), "helper_not_admissible");
    }
    let async_scan = |assignment| {
        source_document(&format!("from requests import Session\nasync def make(items) -> Session:\n    [item async for item in items]\n    return Session()\nasync def fetch(items, u):\n    {assignment}\n    return client.get(u)\n"))
    };
    assert_nameonly(
        &async_scan("client = await make(items)"),
        "requests.Session.get",
    );
    assert_refusal(&async_scan("client = make(items)"), "helper_not_admissible");
    assert_silent(&async_scan(
        "pending = make(items)\n    client = await pending",
    ));
}

#[test]
fn declaration_import_availability_depends_on_annotation_evaluation() {
    for (guarded_import, imported) in [
        ("from requests import Session as C", "C"),
        ("from typing import Optional as Maybe", "Maybe[Session]"),
    ] {
        let scan = |annotation: &str| {
            source_document(&format!("from requests import Session\nfrom typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    {guarded_import}\ndef make() -> {annotation}:\n    return Session()\ndef fetch(u):\n    client = make()\n    return client.get(u)\n"))
        };
        assert_nameonly(&scan(&format!("'{imported}'")), "requests.Session.get");
        assert_refusal(&scan(imported), "binding_unavailable");
    }

    let scan = |annotation| {
        source_document(&format!("from requests import Session\nfrom typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    from requests import Session as C\ndef fetch(u):\n    client: {annotation} = opaque()\n    return client.get(u)\n"))
    };
    assert_refusal(&scan("C"), "receiver_value_unproven");
    assert_silent(&scan("list[C]"));
}

#[test]
fn declaration_and_helper_must_bind_the_receiver_in_an_admitted_position() {
    let scan = |body| {
        source_document(&format!(
            "from requests import Session\ndef make() -> Session:\n    return Session()\n{body}\n"
        ))
    };
    for body in [
        "def fetch(u):\n    client: Session\n    return client.get(u)",
        "def fetch(u, *client: Session):\n    return client.get(u)",
        "def fetch(u, **client: Session):\n    return client.get(u)",
        "def fetch(u, client=make()):\n    return client.get(u)",
        "def fetch(u):\n    return make().get(u)",
        "def fetch(u):\n    client = helpers.make()\n    return client.get(u)",
        "def fetch(u):\n    with make() as client:\n        return client.get(u)",
        "class Example:\n    client = make()\n    result = client.get(u)",
        "client = make()\ndef fetch(u):\n    return client.get(u)",
    ] {
        assert_silent(&scan(body));
    }
    assert_refusal(
        &scan("def fetch(u, *, client: Session):\n    return client.get(u)"),
        "receiver_value_unproven",
    );
    assert_nameonly(
        &scan("def fetch(u):\n    client: Session = make()\n    return client.get(u)"),
        "requests.Session.get",
    );
    assert_nameonly(
        &scan("def fetch(u):\n    client: list[Session] = make()\n    return client.get(u)"),
        "requests.Session.get",
    );
}

#[test]
fn returned_local_requires_a_direct_constructor_binding_in_the_helper() {
    let scan = |body| {
        source_document(&format!("from requests import Session\nshared = Session()\ndef make() -> Session:\n{body}\ndef fetch(u):\n    client = make()\n    return client.get(u)\n"))
    };
    assert_nameonly(
        &scan("    v = Session()\n    return v"),
        "requests.Session.get",
    );
    for body in [
        "    if flag:\n        v = Session()\n    return v",
        "    return shared",
        "    v = opaque()\n    return v",
        "    return opaque()",
    ] {
        assert_refusal(&scan(body), "receiver_value_unproven");
    }
}
