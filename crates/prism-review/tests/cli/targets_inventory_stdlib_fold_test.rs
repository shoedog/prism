use super::targets_inventory_stdlib_roots_test::{check, scan};

const REBOUND: &str = "1 (member_rebound: 1)";
const NONE: &str = "1 (receiver_is_none: 1)";
const CONFLICT: &str = "1 (annotation_conflict: 1)";

fn path(grade: &'static str, line: u64) -> [(&'static str, &'static str, &'static str, u64); 1] {
    [("filesystem", "pathlib.Path.read_text", grade, line)]
}

#[test]
fn declaration_keeps_the_constructor_instance_slots() {
    for (import, class, member, kind, callee) in [
        (
            "import configparser as lib",
            "ConfigParser",
            "read",
            "parse",
            "configparser.ConfigParser.read",
        ),
        (
            "import configparser as lib",
            "RawConfigParser",
            "read",
            "parse",
            "configparser.RawConfigParser.read",
        ),
        (
            "import pathlib as lib",
            "Path",
            "read_text",
            "filesystem",
            "pathlib.Path.read_text",
        ),
    ] {
        let source = |write: &str| {
            format!("{import}\np: lib.{class} = lib.{class}()\nalias = p\n{write}\ndef read():\n    return p.{member}('x')\n")
        };
        check(
            &scan(&source("alias.sibling = replacement"), &[]),
            &[(kind, callee, "exact", 6)],
            None,
        );
        for write in [
            format!("p.{member} = replacement"),
            format!("del p.{member}"),
            "p.__class__ = Other".to_owned(),
            "p.__dict__ = namespace".to_owned(),
            format!("setattr(p, '{member}', replacement)"),
            format!("p.__dict__['{member}'] = replacement"),
            format!("vars(alias)['{member}'] = replacement"),
            format!("alias.{member} = replacement"),
        ] {
            check(&scan(&source(&write), &[]), &[], Some(REBOUND));
        }
        let local = |write| {
            format!("{import}\ndef read():\n    p: lib.{class} = lib.{class}()\n    {write}\n    return p.{member}('x')\n")
        };
        check(
            &scan(&local("p.sibling = replacement"), &[]),
            &[(kind, callee, "exact", 5)],
            None,
        );
        check(
            &scan(&local(&format!("p.{member} = replacement")), &[]),
            &[],
            Some(REBOUND),
        );
    }
}

#[test]
fn context_protocol_checks_all_tracked_path_origins() {
    for (setup, header, init, grade, line) in [
        ("", "p: Path", "", "nameonly", 6),
        ("", "", "    p: Path = opaque()\n", "nameonly", 7),
        ("p: Path = opaque()\n", "", "", "nameonly", 7),
        (
            "def make():\n    return opaque()\n",
            "",
            "    p: Path = make()\n",
            "nameonly",
            9,
        ),
        (
            "def make() -> Path:\n    return opaque()\n",
            "",
            "    p = make()\n",
            "nameonly",
            9,
        ),
        ("", "", "    p: Path = Path()\n", "exact", 7),
    ] {
        for platform in ["PosixPath", "WindowsPath"] {
            let source = |slot, body| {
                format!("from pathlib import Path, {platform}\n{platform}.{slot} = replacement\n{setup}def read({header}):\n{init}    with p:\n{body}\n")
            };
            let after = "        pass\n    return p.read_text()";
            let inside = "        return p.read_text()";
            check(
                &scan(&source("sibling", after), &[]),
                &path(grade, line),
                None,
            );
            let inside_exit = scan(&source("__exit__", inside), &[]);
            if setup.starts_with("p:") {
                // A previous invocation may already have exited with this module object.
                check(&inside_exit, &[], Some(REBOUND));
            } else {
                check(&inside_exit, &path(grade, line - 1), None);
            }
            check(&scan(&source("__enter__", after), &[]), &[], Some(REBOUND));
            check(&scan(&source("__exit__", after), &[]), &[], Some(REBOUND));
        }
    }
}

#[test]
fn helper_protocol_write_precedes_returned_local_escape() {
    let source = |slot| {
        format!("from pathlib import Path, PosixPath\nPosixPath.{slot} = replacement\ndef make() -> Path:\n    p = Path()\n    with p:\n        pass\n    return p\ndef read():\n    p = make()\n    return p.read_text()\n")
    };
    check(
        &scan(&source("sibling"), &[]),
        &[],
        Some("1 (receiver_value_unproven: 1)"),
    );
    for slot in ["__enter__", "__exit__"] {
        check(&scan(&source(slot), &[]), &[], Some(REBOUND));
    }
}

#[test]
fn declaration_checks_visible_helper_results_without_an_admitted_type() {
    for (header, caller) in [
        (
            "def",
            "def read():\n    p: Path = make()\n    return p.read_text()",
        ),
        (
            "def",
            "p: Path = make()\ndef read():\n    return p.read_text()",
        ),
        (
            "async def",
            "async def read():\n    p: Path = await make()\n    return p.read_text()",
        ),
    ] {
        for signature in ["", " -> object"] {
            let source = |body| {
                format!("from pathlib import Path\nfrom configparser import ConfigParser\n{header} make(){signature}:\n    {body}\n{caller}\n")
            };
            check(
                &scan(&source("return opaque()"), &[]),
                &path("nameonly", 7),
                None,
            );
            for body in ["return None", "return", "pass"] {
                check(&scan(&source(body), &[]), &[], Some(NONE));
            }
            for body in ["return {}", "return ConfigParser()"] {
                check(&scan(&source(body), &[]), &[], Some(CONFLICT));
            }
            if header == "def" {
                for definition in ["async def make", "@decorate\ndef make"] {
                    let opaque = source("return None").replace("def make", definition);
                    let line = if definition.starts_with('@') { 8 } else { 7 };
                    check(&scan(&opaque, &[]), &path("nameonly", line), None);
                }
            }
        }
    }
}

#[test]
fn helper_defaults_only_contradict_the_parameter_declaration() {
    let source = |body, declaration| {
        format!("from pathlib import Path\nfrom configparser import ConfigParser\ndef make():\n    {body}\ndef read(p: {declaration} = make()):\n    return p.read_text()\n")
    };
    check(
        &scan(&source("return None", "Path | None"), &[]),
        &path("nameonly", 6),
        None,
    );
    check(
        &scan(&source("return opaque()", "Path"), &[]),
        &path("nameonly", 6),
        None,
    );
    for body in ["return {}", "return ConfigParser()", "return None"] {
        check(&scan(&source(body, "Path"), &[]), &[], Some(CONFLICT));
    }
}

#[test]
fn helpers_without_a_completion_witness_stay_opaque() {
    let source = |body: &str| {
        format!("from pathlib import Path\ndef make() -> Path:\n{body}\ndef read():\n    p = make()\n    return p.read_text()\n")
    };
    for body in ["    pass", "    return", "    return None"] {
        check(&scan(&source(body), &[]), &[], Some(NONE));
    }
    for (body, line) in [
        ("    raise Error", 6),
        ("    while True:\n        pass", 7),
        (
            "    if condition:\n        raise First\n    else:\n        raise Second",
            9,
        ),
    ] {
        check(&scan(&source(body), &[]), &path("nameonly", line), None);
    }
}

#[test]
fn platform_module_slots_follow_os_path_and_import_capture() {
    for module in ["posixpath", "ntpath"] {
        for (member, sibling) in [("getsize", "getmtime"), ("getmtime", "getsize")] {
            let setup = format!("import os\nimport {module} as platform\nfrom os import path\n");
            let canonical = format!("os.path.{member}");
            for read in [format!("os.path.{member}(p)"), format!("path.{member}(p)")] {
                let source = |write: &str| {
                    format!("{setup}alias = platform\n{write}\ndef read(p):\n    return {read}\n")
                };
                check(
                    &scan(&source(&format!("platform.{sibling} = replacement")), &[]),
                    &[("filesystem", &canonical, "exact", 7)],
                    None,
                );
                for write in [
                    format!("platform.{member} = replacement"),
                    format!("setattr(alias, '{member}', replacement)"),
                    format!("vars(alias)['{member}'] = replacement"),
                ] {
                    check(&scan(&source(&write), &[]), &[], Some(REBOUND));
                }
            }
            let write = format!("platform.{member} = replacement");
            let capture = format!("from os.path import {member} as call");
            let read = "def read(p):\n    return call(p)\n";
            check(
                &scan(&format!("{setup}{capture}\n{write}\n{read}"), &[]),
                &[("filesystem", &canonical, "exact", 7)],
                None,
            );
            check(
                &scan(&format!("{setup}{write}\n{capture}\n{read}"), &[]),
                &[],
                Some(REBOUND),
            );
        }
    }
}

#[test]
fn environ_get_reads_the_pinned_instance_lookup() {
    for (owner, slot) in [
        ("Mapping", "get"),
        ("MutableMapping", "get"),
        ("os._Environ", "get"),
        ("os._Environ", "__getattribute__"),
    ] {
        for receiver in ["os.environ", "environ"] {
            let source = |slot| {
                format!("import os\nfrom os import environ\nfrom collections.abc import Mapping, MutableMapping\n{owner}.{slot} = replacement\ndef read(p):\n    {receiver}.get(p)\n    os.getenv(p)\n")
            };
            let getenv = ("config", "os.getenv", "exact", 7);
            check(
                &scan(&source("sibling"), &[]),
                &[("config", "os.environ.get", "exact", 6), getenv],
                None,
            );
            check(&scan(&source(slot), &[]), &[getenv], Some(REBOUND));
        }
    }
}

#[test]
fn starred_await_is_evaluated_in_the_enclosing_async_scope() {
    let source = |annotation| {
        format!(
            "import json\nasync def outer():\n    def inner(*items: {annotation}):\n        pass\n"
        )
    };
    for annotation in [
        "await json.loads('[]')",
        "*(await json.loads('[]'))",
        "*await json.loads('[]')",
        "* await json.loads('[]')",
        "*await json.loads('[]') | other",
    ] {
        check(
            &scan(&source(annotation), &[]),
            &[("parse", "json.loads", "exact", 3)],
            None,
        );
    }
    let postponed = format!(
        "from __future__ import annotations\n{}",
        source("*await json.loads('[]')")
    );
    check(&scan(&postponed, &[]), &[], None);
}

#[test]
fn invalid_starred_annotations_do_not_leak_calls() {
    let source = |parameter| {
        format!("import json\ndef outer():\n    def inner({parameter}):\n        pass\ndef read():\n    return json.loads('[]')\n")
    };
    let unsupported = scan(&source("*items: *(lambda: json.loads('[]'))"), &[]);
    assert_eq!(
        unsupported["targets"][0]["expected"]["detail"],
        "lambda_frame"
    );
    for parameter in [
        "*items: *lambda: json.loads('[]')",
        "**items: *lambda: json.loads('[]')",
        "items: *lambda: json.loads('[]')",
        "items: *(json.loads('[]'))",
        "**items: *(json.loads('[]'))",
        "*items: *json.loads('[]') < limit",
        "*items: *json.loads('[]') or default",
        "*items: *json.loads('[]') if condition else default",
        "*items: *json.loads(a=1, a=2)",
        "*items: *json.loads(**kw, *args)",
        "*items: *json.loads(a=1, 2)",
        "*items: *'x' b'y'.join(json.loads('[]'))",
        "*items: *f'{json.loads('[]')}'",
        "*items: *await json.loads('[]')",
    ] {
        check(
            &scan(&source(parameter), &["--inventory-kinds", "parse"]),
            &[("parse", "json.loads", "exact", 6)],
            None,
        );
    }

    let mixed = "import json\ndef outer():\n    def inner(*items: *json.loads('[]'),\n              invalid: *lambda: json.loads('[]')):\n        pass\n";
    check(
        &scan(mixed, &[]),
        &[("parse", "json.loads", "exact", 3)],
        None,
    );
}
