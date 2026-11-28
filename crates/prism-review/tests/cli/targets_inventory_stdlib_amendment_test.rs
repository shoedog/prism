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
        ("    if flag: raise Error", 6),
        ("    if opaque(): raise Error\n    else: pass", 7),
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(line), None);
    }
    for body in [
        "    if flag: raise Error\n    else: marker = 1",
        "    if flag: raise Error\n    elif other: marker = 1\n    else: raise Error",
        "    if False: raise Error\n    marker = 1",
    ] {
        check(&scan(&helper_source(body), &[]), &[], Some(NONE));
    }
}

#[test]
fn loops_are_outside_the_fallthrough_accept_list() {
    check(&scan(&helper_source("    pass"), &[]), &[], Some(NONE));
    for (body, line) in [
        ("    while True:\n        pass", 7),
        ("    while (True):\n        pass\n    marker = 1", 8),
        ("    while True:\n        while True: break", 7),
        ("    while True:\n        if False: break", 7),
        ("    while True:\n        raise Error\n        break", 8),
        ("    while False: pass\n    else: raise Error", 7),
        ("    while True: break", 6),
        ("    while True:\n        if flag: break", 7),
        ("    while True: break\n    else: raise Error", 7),
        ("    while False: raise Error", 6),
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(line), None);
    }
}

#[test]
fn integer_conditions_do_not_create_phantom_completion_paths() {
    check(
        &scan(&helper_source("    if 0: raise Error"), &[]),
        &[],
        Some(NONE),
    );
    check(
        &scan(&helper_source("    if 1: pass"), &[]),
        &[],
        Some(NONE),
    );
    check(
        &scan(&helper_source("    if 1: raise Error"), &[]),
        &path_read(6),
        None,
    );
    check(
        &scan(&helper_source("    if 0: pass\n    else: raise Error"), &[]),
        &path_read(7),
        None,
    );
}

#[test]
fn a_loop_with_only_a_dead_break_keeps_the_declaration() {
    check(&scan(&helper_source("    pass"), &[]), &[], Some(NONE));
    check(
        &scan(&helper_source("    while True:\n        if 0: break"), &[]),
        &path_read(7),
        None,
    );
}

#[test]
fn async_helpers_need_the_same_completion_witness() {
    let source = |body| {
        format!("from pathlib import Path\nasync def make() -> Path:\n    {body}\nasync def read():\n    p = await make()\n    return p.read_text()\n")
    };
    check(&scan(&source("marker = 1"), &[]), &[], Some(NONE));
    check(&scan(&source("return None"), &[]), &[], Some(NONE));
    check(
        &scan(&source("if 1: raise Error"), &[]),
        &path_read(6),
        None,
    );
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

#[test]
fn recovered_empty_suites_do_not_supply_implicit_none() {
    check(&scan(&helper_source("    pass"), &[]), &[], Some(NONE));
    for body in ["", "    # comment only"] {
        check(&scan(&helper_source(body), &[]), &path_read(6), None);
    }
}

#[test]
fn unsupported_straight_line_statements_do_not_supply_implicit_none() {
    check(&scan(&helper_source("    opaque()"), &[]), &[], Some(NONE));
    check(
        &scan(
            &helper_source("    def inner(): return None\n    return Path()"),
            &[],
        ),
        &path_read(7),
        None,
    );
    for body in [
        "    import os",
        "    from os import path",
        "    del value",
        "    global value",
        "    def inner(): return Path()",
    ] {
        check(&scan(&helper_source(body), &[]), &path_read(6), None);
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

fn check_environ_write(setup: &str, write: &str, rebound: bool) {
    let source = format!(
        "import os\n{setup}{write}\ndef read(key):\n    os.environ.get(key)\n    os.getenv(key)\n"
    );
    let line = 4 + setup.lines().count() as u64;
    let getenv = ("config", "os.getenv", "exact", line + 1);
    if rebound {
        check(&scan(&source, &[]), &[getenv], Some(REBOUND));
    } else {
        check(
            &scan(&source, &[]),
            &[("config", "os.environ.get", "exact", line), getenv],
            None,
        );
    }
}

#[test]
fn environ_get_ignores_vars_writes_to_typing_alias_wrappers() {
    for (setup, owner) in [
        ("import typing\n", "typing.Mapping"),
        (
            "from typing import MutableMapping as M\nAlias = M\n",
            "Alias",
        ),
    ] {
        check_environ_write(setup, &format!("{owner}.get = replacement"), true);
        check_environ_write(setup, &format!("vars({owner})['get'] = replacement"), false);
    }
}

#[test]
fn environ_get_ignores_dict_writes_to_typing_alias_wrappers() {
    let setup = "import typing\nAlias = typing.Mapping\n";
    check_environ_write(setup, "setattr(Alias, 'get', replacement)", true);
    check_environ_write(setup, "Alias.__dict__['get'] = replacement", false);
}

#[test]
fn environ_get_ignores_dunder_writes_to_typing_alias_wrappers() {
    let setup = "import typing\nfrom collections.abc import Mapping\n";
    check_environ_write(setup, "Mapping.__getattribute__ = replacement", true);
    check_environ_write(
        setup,
        "typing.Mapping.__getattribute__ = replacement",
        false,
    );
    check_environ_write(
        setup,
        "setattr(typing.Mapping, '__getattribute__', replacement)",
        false,
    );
}

#[test]
fn environ_get_ignores_attribute_deletes_on_typing_alias_wrappers() {
    let setup = "import typing\nfrom collections.abc import Mapping\n";
    check_environ_write(setup, "delattr(Mapping, 'get')", true);
    check_environ_write(setup, "del typing.Mapping.get", false);
    check_environ_write(setup, "delattr(typing.Mapping, 'get')", false);
}

#[test]
fn environ_get_follows_selectors_mapping_reexport() {
    let setup = "import selectors\n";
    check_environ_write(setup, "selectors.Mapping.sibling = replacement", false);
    check_environ_write(setup, "selectors.Mapping.get = replacement", true);
}

#[test]
fn environ_get_follows_cgi_mapping_reexport() {
    let setup = "from cgi import Mapping as Imported\nAlias = Imported\n";
    check_environ_write(setup, "Alias.sibling = replacement", false);
    check_environ_write(setup, "Alias.get = replacement", true);
}

#[test]
fn environ_lookup_distinguishes_class_stores_from_namespace_entries() {
    let setup = "import builtins\nfrom collections.abc import Mapping\n";
    for write in [
        "setattr(os._Environ, '__getattribute__', replacement)",
        "setattr(os.environ, '__class__', replacement_type)",
        "vars(os.environ)['get'] = replacement",
    ] {
        check_environ_write(setup, write, true);
    }
    for write in [
        "vars(os._Environ)['__getattribute__'] = replacement",
        "os._Environ.__dict__['__getattribute__'] = replacement",
        "vars(os._Environ)['__class__'] = replacement_type",
        "os._Environ.__dict__['__class__'] = replacement_type",
        "vars(os.environ)['__class__'] = replacement_type",
        "os.environ.__dict__['__class__'] = replacement_type",
        "vars(Mapping)['get'] = replacement",
        "Mapping.__dict__['get'] = replacement",
        "setattr(builtins.object, '__getattribute__', replacement)",
    ] {
        check_environ_write(setup, write, false);
    }
}

#[test]
fn platform_module_namespace_preserves_class_but_can_rebind_members() {
    for module in ["posixpath", "ntpath"] {
        let source = |write| {
            format!("import os\nimport {module} as platform\nAlias = platform\n{write}\ndef read(p):\n    return os.path.getsize(p)\n")
        };
        for write in [
            "setattr(Alias, '__class__', replacement_type)",
            "vars(Alias)['getsize'] = replacement",
            "Alias.__dict__['getsize'] = replacement",
        ] {
            check(&scan(&source(write), &[]), &[], Some(REBOUND));
        }
        for write in [
            "vars(Alias)['__class__'] = replacement_type",
            "Alias.__dict__['__class__'] = replacement_type",
        ] {
            check(
                &scan(&source(write), &[]),
                &[("filesystem", "os.path.getsize", "exact", 6)],
                None,
            );
        }
    }
}

#[test]
fn declarations_ignore_read_only_class_namespaces() {
    for (member, context, line) in [
        ("read_text", "", 4),
        ("__enter__", "    with p: pass\n", 5),
        ("__exit__", "    with p: pass\n", 5),
    ] {
        let source = |write: &str| {
            format!("from pathlib import Path, PosixPath as Concrete\n{write}\ndef read(p: Path):\n{context}    return p.read_text()\n")
        };
        check(
            &scan(
                &source(&format!("setattr(Concrete, '{member}', replacement)")),
                &[],
            ),
            &[],
            Some(REBOUND),
        );
        for base in ["vars(Concrete)", "Concrete.__dict__"] {
            check(
                &scan(&source(&format!("{base}['{member}'] = replacement")), &[]),
                &path_read(line),
                None,
            );
        }
    }
}

#[test]
fn declared_config_instances_keep_their_member_vetoes() {
    let source = |write: &str| {
        format!("from configparser import ConfigParser\ndef read(p: ConfigParser):\n    {write}\n    return p.read('config')\n")
    };
    check(
        &scan(&source("vars(p)['sibling'] = replacement"), &[]),
        &[("parse", "configparser.ConfigParser.read", "nameonly", 4)],
        None,
    );
    for write in [
        "setattr(p, 'read', replacement)",
        "vars(p)['read'] = replacement",
        "p.__dict__['read'] = replacement",
    ] {
        check(&scan(&source(write), &[]), &[], Some(REBOUND));
    }
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

#[test]
fn known_limit_wrapper_local_dunder_store_alias_chain() {
    // KNOWN-LIMIT (sol fold-2 finding 1, DESIGN 10.2): a dunder store on a wrapper
    // supplies neither a veto nor identity evidence, so an alias read back out of it
    // is not followed. CPython dispatches `os.environ.get` to the replacement here.
    let setup = "from typing import Mapping as W\nfrom collections.abc import Mapping\nW.__target__ = Mapping\nA = W.__target__\n";
    check_environ_write(setup, "A.get = replacement", false);
    // Control: the alias bound directly to the origin class is a counted write.
    check_environ_write(
        "import typing\nfrom collections.abc import Mapping\nA = Mapping\n",
        "A.get = replacement",
        true,
    );
}

#[test]
fn known_limit_correlated_flag_helper_completion() {
    // KNOWN-LIMIT (sol fold-2 finding 2, DESIGN 6.4/10.2): conditions are not
    // correlated, so the `elif flag` arm counts as a possible normal completion
    // although a Boolean flag always raises. CPython: the helper never returns.
    check(
        &scan(
            &helper_source(
                "    if flag:\n        raise Error\n    elif flag:\n        pass\n    else:\n        raise Error",
            ),
            &[],
        ),
        &[],
        Some(NONE),
    );
    // Control: every arm raising has no completion and keeps the nameonly row.
    check(
        &scan(
            &helper_source(
                "    if flag:\n        raise Error\n    elif other:\n        raise Error\n    else:\n        raise Error",
            ),
            &[],
        ),
        &path_read(11),
        None,
    );
}
