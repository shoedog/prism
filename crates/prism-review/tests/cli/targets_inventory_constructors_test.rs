use super::targets_inventory_test::{document, run};
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

fn inline(imports: &str, body: &str) -> Value {
    source_document(&format!("{imports}\ndef fetch(u):\n{body}\n"))
}

pub(super) fn source_document(source: &str) -> Value {
    source_document_with(source, &[])
}

pub(super) fn source_document_with(source: &str, options: &[&str]) -> Value {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("probe.py"), source).unwrap();
    let mut args = vec!["--files", "probe.py"];
    args.extend_from_slice(options);
    document(&run(repo.path(), &args))
}

pub(super) fn assert_exact(doc: &Value, canonical: &str) {
    let [row] = doc["targets"].as_array().unwrap().as_slice() else {
        panic!("expected one target for {canonical}, got {doc:#}");
    };
    assert_eq!(row["kind"], "external_call");
    assert_eq!(row["confidence"], "exact");
    assert_eq!(row["dependency_hint"]["callee"], canonical);
    assert!(identity_warnings(doc).is_empty());
}

pub(super) fn identity_warnings(doc: &Value) -> Vec<&str> {
    doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning.as_str().unwrap())
        .filter(|warning| warning.starts_with("inventory: identity unproven:"))
        .collect()
}

pub(super) fn assert_refusal(doc: &Value, cause: &str) {
    assert_eq!(doc["targets"], json!([]));
    assert_eq!(
        identity_warnings(doc),
        [format!(
            "inventory: identity unproven: probe.py: 1 ({cause}: 1)"
        )]
    );
}

#[test]
fn inline_constructor_import_refusals() {
    for (cause, imports, body, control_imports, control) in [
        (
            "star_import",
            "from requests import Session as C\nfrom unknown import *",
            "    return C().get(u)",
            "from requests import Session as C\n# from unknown import *",
            "    return C().get(u)",
        ),
        (
            "shadowed_root",
            "",
            "    requests = other\n    return requests.Session().get(u)",
            "import requests",
            "    return requests.Session().get(u)",
        ),
        (
            "not_imported",
            "",
            "    return requests.Session().get(u)",
            "import requests",
            "    return requests.Session().get(u)",
        ),
        (
            "repository_local_module",
            "from .requests import Session as C",
            "    return C().get(u)",
            "from requests import Session as C",
            "    return C().get(u)",
        ),
        (
            "binding_unavailable",
            "",
            "    return C().get(u)\n    from requests import Session as C",
            "",
            "    from requests import Session as C\n    return C().get(u)",
        ),
        (
            "binding_unavailable",
            "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    import requests",
            "    return requests.Session().get(u)",
            "import requests",
            "    return requests.Session().get(u)",
        ),
    ] {
        assert_refusal(&inline(imports, body), cause);
        assert_exact(&inline(control_imports, control), "requests.Session.get");
    }
}

#[test]
fn inline_constructor_slots_and_module_capture() {
    for slot in ["__new__", "__init__", "get", "__getattribute__"] {
        let write = format!("from requests import Session as C\nC.{slot} = replacement");
        assert_refusal(&inline(&write, "    return C().get(u)"), "member_rebound");
        assert_exact(
            &inline(
                "from requests import Session as C\nC.post = replacement",
                "    return C().get(u)",
            ),
            "requests.Session.get",
        );
    }
    for key in ["Session", "__class__"] {
        let write = format!("import requests\nrequests.{key} = replacement");
        assert_refusal(
            &inline(&write, "    return requests.Session().get(u)"),
            "member_rebound",
        );
        let capture = format!("from requests import Session as C\n{write}");
        assert_exact(
            &inline(&capture, "    return C().get(u)"),
            "requests.Session.get",
        );
        let late_capture = format!("{write}\nfrom requests import Session as C");
        assert_refusal(
            &inline(&late_capture, "    return C().get(u)"),
            "member_rebound",
        );
    }
    for (path, slot, selected, sibling) in [
        ("urllib3._request_methods", "request", "request", "urlopen"),
        (
            "urllib3._request_methods",
            "__getattribute__",
            "request",
            "post",
        ),
        ("urllib3.poolmanager", "__bases__", "request", "get"),
    ] {
        let owner = if path == "urllib3.poolmanager" {
            "PoolManager"
        } else {
            "RequestMethods"
        };
        let imports =
            format!("from urllib3 import PoolManager\nfrom {path} import {owner} as WriteOwner");
        let body = format!("    return PoolManager().{selected}(u)");
        assert_refusal(
            &inline(
                &format!("{imports}\nWriteOwner.{slot} = replacement"),
                &body,
            ),
            "member_rebound",
        );
        assert_exact(
            &inline(
                &format!("{imports}\nWriteOwner.{sibling} = replacement"),
                &body,
            ),
            "urllib3.PoolManager.request",
        );
    }
}

#[test]
fn inline_constructor_shape_allow_list() {
    for body in [
        "    return (client := C()).get(u)",
        "    return C().get(u).get(u)",
        "    return super().get(u)",
        "    return sessions[0].get(u)",
        "    fn = C().get\n    return fn(u)",
        "    return factory().get(u)",
        "    return C().close()",
    ] {
        let doc = inline("from requests import Session as C", body);
        let expected = if body.contains("C().get(u).get") {
            1
        } else {
            0
        };
        assert_eq!(doc["targets"].as_array().unwrap().len(), expected, "{body}");
        assert!(identity_warnings(&doc).is_empty(), "{body}");
        assert_exact(
            &inline(
                "from requests import Session as C",
                "    return (C()).get(u)",
            ),
            "requests.Session.get",
        );
    }
    for (imports, body) in [
        (
            "import urllib3 as requests",
            "    return requests.Session().get(u)",
        ),
        (
            "",
            "    return requests.Session().get(u)\n    import urllib3 as requests",
        ),
    ] {
        let doc = inline(imports, body);
        assert_eq!(doc["targets"], json!([]));
        assert!(identity_warnings(&doc).is_empty());
    }
    assert_exact(
        &inline(
            "import httpx as requests",
            "    return requests.Client().get(u)",
        ),
        "httpx.Client.get",
    );
}

#[test]
fn named_constructor_binding_refusals() {
    let imports = "from requests import Session";
    let run = |prefix: &str, read: &str| {
        inline(imports, &format!("    s = Session()\n{prefix}    {read}"))
    };
    for competitor in [
        "s = opaque",
        "s: Other = opaque",
        "s += opaque",
        "del s",
        "s, *_ = opaque",
        "for s in opaque: pass",
        "with opaque as s: pass",
        "try: pass\n    except Exception as s: pass",
        "(s := opaque)",
        "match opaque:\n        case {'key': s}: pass",
        "import opaque as s",
        "def s(): pass",
        "def change():\n        nonlocal s\n        s = opaque",
    ] {
        let doc = run(&format!("    {competitor}\n"), "return s.get(u)");
        assert_refusal(&doc, "receiver_not_unique");
        assert_exact(&run("", "return s.get(u)"), "requests.Session.get");
    }
    assert_exact(
        &run("", "result = s.get(u)\n    del s\n    return result"),
        "requests.Session.get",
    );
    assert_refusal(
        &inline(imports, "    s.get(u)\n    s = Session()"),
        "binding_unavailable",
    );
    assert_exact(&run("", "return s.get(u)"), "requests.Session.get");
}

#[test]
fn named_constructor_instance_writes() {
    for (write, control) in [
        ("s.get = replacement", "s.post = replacement"),
        ("s.__class__ = Other", "s.other = Other"),
        ("s.__dict__ = {}", "s.headers = {}"),
        (
            "s.__dict__['get'] = replacement",
            "s.__dict__['post'] = replacement",
        ),
        ("del vars(s)['get']", "del vars(s)['post']"),
        (
            "setattr(s, 'get', replacement)",
            "setattr(s, 'post', replacement)",
        ),
        (
            "alias = s\n    alias.get = replacement",
            "alias = Session()\n    alias.get = replacement",
        ),
    ] {
        let doc = |setup| {
            inline(
                "from requests import Session",
                &format!("    s = Session()\n    {setup}\n    return s.get(u)"),
            )
        };
        assert_refusal(&doc(write), "member_rebound");
        assert_exact(&doc(control), "requests.Session.get");
    }
    assert_exact(
        &inline(
            "from requests import Session",
            "    s = Session()\n    result = s.get(u)\n    s.get = replacement\n    return result",
        ),
        "requests.Session.get",
    );
}

#[test]
fn named_constructor_context_protocols() {
    for (import, class, prefix, manager, read, entry, exit) in [
        (
            "from requests import Session",
            "Session",
            "",
            "with",
            "s.get(u)",
            "__enter__",
            "__exit__",
        ),
        (
            "from httpx import AsyncClient",
            "AsyncClient",
            "async ",
            "async with",
            "await s.get(u)",
            "__aenter__",
            "__aexit__",
        ),
        (
            "from aiohttp import ClientSession",
            "ClientSession",
            "async ",
            "async with",
            "await s.get(u)",
            "__aenter__",
            "__aexit__",
        ),
    ] {
        let doc = |write: &str, suite: &str| {
            let repo = TempDir::new().unwrap();
            fs::write(
                repo.path().join("probe.py"),
                format!("{import}\n{write}\n{prefix}def fetch(u):\n    {suite}\n"),
            )
            .unwrap();
            document(&run(repo.path(), &["--files", "probe.py"]))
        };
        let in_suite = format!("{manager} {class}() as s:\n        return {read}");
        let post_exit = format!("{manager} {class}() as s:\n        pass\n    return {read}");
        let canonical = format!("{}.{class}.get", import.split_whitespace().nth(1).unwrap());
        assert_refusal(
            &doc(&format!("{class}.{entry} = replace"), &in_suite),
            "member_rebound",
        );
        assert_exact(&doc("", &in_suite), &canonical);
        let install = format!(
            "{prefix}def replace(obj, *error):\n    obj.get = replacement\n{class}.{exit} = replace"
        );
        assert_refusal(&doc(&install, &post_exit), "member_rebound");
        assert_exact(&doc(&install, &in_suite), &canonical);
        assert_exact(&doc("", &post_exit), &canonical);
    }
    let imports = "from requests import Session";
    assert_refusal(
        &inline(
            imports,
            "    with (s.get(u), Session() as s):\n        pass",
        ),
        "binding_unavailable",
    );
    assert_exact(
        &inline(
            imports,
            "    with (Session() as s,\n          s.get(u)):\n        pass",
        ),
        "requests.Session.get",
    );
    let wrong_protocol = inline(
        "from httpx import AsyncClient",
        "    with AsyncClient() as s:\n        return s.get(u)",
    );
    assert_eq!(wrong_protocol["targets"], json!([]));
    assert!(identity_warnings(&wrong_protocol).is_empty());
}

#[test]
fn context_exit_is_checked_at_capture_and_can_reach_a_reused_instance() {
    let before_exit_write = source_document(
        "from requests import Session\nwith Session() as s:\n    Session.__exit__ = replace\ns.get(u)\n",
    );
    assert!(identity_warnings(&before_exit_write).is_empty());
    let [row] = before_exit_write["targets"].as_array().unwrap().as_slice() else {
        panic!("expected the module-level call after the unchanged exit capture");
    };
    assert_eq!(row["confidence"], "exact");
    assert_eq!(row["expected"]["detail"], "module_level");
    assert_eq!(row["dependency_hint"]["callee"], "requests.Session.get");
    assert_refusal(
        &source_document(
            "from requests import Session\nSession.__exit__ = replace\nwith Session() as s:\n    pass\ns.get(u)\n",
        ),
        "member_rebound",
    );
    assert_refusal(
        &inline(
            "from requests import Session",
            "    with Session() as s:\n        Session.__exit__ = replace\n    return s.get(u)",
        ),
        "member_rebound",
    );

    let imports = "from requests import Session\nSession.__exit__ = replace";
    assert_refusal(
        &inline(
            imports,
            "    s = Session()\n    alias = s\n    with alias:\n        pass\n    return s.get(u)",
        ),
        "member_rebound",
    );
    assert_exact(
        &inline(imports, "    s = Session()\n    alias = Session()\n    with alias:\n        pass\n    return s.get(u)"),
        "requests.Session.get",
    );
    assert_refusal(
        &inline(imports, "    s = Session()\n    for u in values:\n        with s:\n            result = s.get(u)\n    return result"),
        "member_rebound",
    );
    assert_exact(
        &inline(imports, "    for u in values:\n        s = Session()\n        with s:\n            result = s.get(u)\n    return result"),
        "requests.Session.get",
    );
}

#[test]
fn constructor_computed_reflection_known_limits_have_literal_controls() {
    // DESIGN §10.1 keeps computed write targets outside the explicit index.
    assert_exact(
        &inline(
            "from requests import Session as C\nkey = 'get'\nsetattr(C, key, replacement)",
            "    return C().get(u)",
        ),
        "requests.Session.get",
    );
    assert_refusal(
        &inline(
            "from requests import Session as C\nsetattr(C, 'get', replacement)",
            "    return C().get(u)",
        ),
        "member_rebound",
    );
    for namespace in ["s.__dict__", "vars(s)"] {
        let source =
            |key| format!("    s = C()\n    {namespace}[{key}] = replacement\n    return s.get(u)");
        let imports = "from requests import Session as C\nkey = 'get'";
        assert_exact(&inline(imports, &source("key")), "requests.Session.get");
        assert_refusal(&inline(imports, &source("'get'")), "member_rebound");
    }
}
