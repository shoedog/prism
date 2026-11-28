use super::{
    targets_inventory_matrix_test::check_receiver_pins,
    targets_inventory_stdlib_roots_test::{check, scan},
    targets_inventory_test::{matrix_doc, warning},
};
use serde_json::json;

const READ: &str = "pathlib.Path.read_text";
const CONFLICT: &str = "1 (annotation_conflict: 1)";
const NONE: &str = "1 (receiver_is_none: 1)";
const UNPROVEN: &str = "1 (receiver_value_unproven: 1)";

fn path_at(line: u64) -> [(&'static str, &'static str, &'static str, u64); 1] {
    [("filesystem", READ, "nameonly", line)]
}

#[test]
fn declaration_and_helper_pins() {
    check_receiver_pins(&[
        "tr3-24", "tr3-25", "tr3-26", "sl1-64", "sl1-65", "sl4-14", "as1-27", "as1-28", "as5-10",
        "as5-11", "as5-14", "pin03", "pin04", "pin05", "pin06", "wf50", "wf51", "wf72", "esc37",
        "esc39", "esc40", "esc41", "esc42", "esc43", "esc44",
    ]);
    for (file, canonical, line) in [
        ("stdlib/as1-27.py", "configparser.ConfigParser.read", 6),
        ("stdlib/wf72.py", "configparser.ConfigParser.read_file", 7),
    ] {
        let doc = matrix_doc(file, &["--inventory-kinds", "parse"]);
        let [target] = doc["targets"].as_array().unwrap().as_slice() else {
            panic!("{file}: {doc:#}");
        };
        assert_eq!(
            target["dependency_hint"],
            json!({"kind":"parse","callee":canonical})
        );
        assert_eq!(target["confidence"], "nameonly");
        assert_eq!(target["site"]["line"], line);
        assert_eq!(doc["warnings"].as_array().unwrap().len(), 1);
    }
    for (file, method, grade, line) in [
        ("stdlib/pin04.py", "pathlib.Path.write_text", "nameonly", 6),
        ("stdlib/pin08.py", "pathlib.Path.exists", "exact", 9),
    ] {
        let doc = matrix_doc(file, &["--inventory-kinds", "filesystem"]);
        let rows = doc["targets"].as_array().unwrap();
        assert_eq!(rows.len(), 2, "{doc:#}");
        assert_eq!(rows[1]["dependency_hint"]["callee"], method);
        assert_eq!(rows[1]["site"]["line"], line);
        assert_eq!(rows[1]["confidence"], grade);
    }
}

#[test]
fn declaration_positions_are_closed_and_http_still_requires_construction() {
    for (parameters, admitted) in [
        ("p: Path", true),
        ("p: Path, /", true),
        ("*, p: Path", true),
        ("p: Path | None = None", true),
        ("*p: Path", false),
        ("**p: Path", false),
        ("p=opaque()", false),
    ] {
        let source = format!(
            "from pathlib import Path\ndef read({parameters}):\n    return p.read_text()\n"
        );
        let site = path_at(3);
        check(
            &scan(&source, &[]),
            if admitted { &site } else { &[] },
            None,
        );
    }
    for (value, admitted) in [
        ("class Reader:\n    def read(p: Path):\n        return p.read_text()\n", true),
        ("class Reader:\n    p: Path = opaque()\n    def read(self):\n        return self.p.read_text()\n", false),
        ("def read():\n    p: Path = opaque()\n    return p.read_text()\n", true),
        ("def read():\n    p: Path\n    return p.read_text()\n", false),
        ("def read():\n    (p,) = opaque()\n    return p.read_text()\n", false),
        ("def read():\n    (p := opaque())\n    return p.read_text()\n", false),
        ("p: Path = opaque()\ndef read():\n    return p.read_text()\n", true),
        ("p: Path\ndef read():\n    return p.read_text()\n", false),
        ("def make() -> Path:\n    return opaque()\np = make()\ndef read():\n    return p.read_text()\n", false),
    ] {
        let source = format!("from pathlib import Path\n{value}");
        let site = path_at(source.lines().count() as u64);
        check(&scan(&source, &[]), if admitted { &site } else { &[] }, None);
    }
    let http = "from requests import Session\ndef read(p: Session):\n    return p.get('url')\n";
    check(
        &scan(http, &["--inventory-kinds", "http"]),
        &[],
        Some(UNPROVEN),
    );
    let module_http = "from requests import Session\np: Session = opaque()\ndef read():\n    return p.get('url')\n";
    check(
        &scan(module_http, &["--inventory-kinds", "http"]),
        &[],
        None,
    );
}

#[test]
fn declarations_use_their_defining_environment_and_closed_grammar() {
    for (value, accepted) in [
        ("Path", true),
        ("Maybe[Path]", true),
        ("Maybe[(Path)]", true),
        ("Maybe[Path,]", false),
        ("None | Path", true),
        ("'Pa' 'th'", true),
        ("'Maybe[Path]'", true),
        (r#""'Path'""#, false),
        ("Union[Path, None]", false),
        ("Annotated[Path, 1]", false),
        ("list[Path]", false),
        ("Path | int", false),
        ("Alias", false),
    ] {
        for body in [
            "def read(p: TYPE):\n    return p.read_text()",
            "def read():\n    p: TYPE = opaque()\n    return p.read_text()",
            "def make() -> TYPE:\n    return opaque()\ndef read():\n    p = make()\n    return p.read_text()",
        ] {
            let source = format!(
                "from typing import Optional as Maybe, Union, Annotated\nfrom pathlib import Path\nAlias = Path\n{}\n",
                body.replace("TYPE", value)
            );
            let rows = path_at(source.lines().count() as u64);
            check(&scan(&source, &[]), if accepted { &rows } else { &[] }, None);
        }
    }
    let imports =
        "from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    from pathlib import Path\n";
    let eager = "def read(p: Path):\n    return p.read_text()\n";
    check(
        &scan(&format!("{imports}{eager}"), &[]),
        &[],
        Some("1 (binding_unavailable: 1)"),
    );
    for (source, line) in [
        (format!("{imports}{}", eager.replace("p: Path", "p: 'Path'")), 5),
        (format!("from __future__ import annotations\n{imports}{eager}"), 6),
        (format!("{imports}def read():\n    p: Path = opaque()\n    return p.read_text()\n"), 6),
        ("class Reader:\n    from pathlib import Path\n    def read(self, p: Path):\n        return p.read_text()\n".into(), 4),
    ] {
        check(&scan(&source, &[]), &path_at(line), None);
    }
}

#[test]
fn module_declarations_require_available_sole_binding() {
    let top = "from pathlib import Path\n";
    let read = "def read():\n    return p.read_text()\n";
    let bound = "p: Path = opaque()\n";
    check(
        &scan(&format!("{top}{bound}{read}"), &[]),
        &path_at(4),
        None,
    );
    for mutation in [
        "def change():\n    global p\n    p = opaque()\n",
        "p = opaque()\n",
    ] {
        check(
            &scan(&format!("{top}{bound}{mutation}{read}"), &[]),
            &[],
            Some("1 (receiver_not_unique: 1)"),
        );
    }
    check(
        &scan(&format!("{top}p.read_text()\n{bound}"), &[]),
        &[],
        Some("1 (binding_unavailable: 1)"),
    );
    let guarded =
        format!("{top}from typing import TYPE_CHECKING\nif TYPE_CHECKING:\n    {bound}{read}");
    check(
        &scan(&guarded, &[]),
        &[],
        Some("1 (binding_unavailable: 1)"),
    );
    let available = guarded.replace("if TYPE_CHECKING:", "if enabled:");
    check(&scan(&available, &[]), &path_at(6), None);
}

#[test]
fn known_none_tracks_bound_values_and_eligible_helper_results() {
    for (pattern, line) in [
        (
            "def read():\n    p: Path | None = VALUE\n    return p.read_text()\n",
            4,
        ),
        (
            "p: Path | None = VALUE\ndef read():\n    return p.read_text()\n",
            4,
        ),
    ] {
        let source = |value| {
            format!(
                "from pathlib import Path\n{}",
                pattern.replace("VALUE", value)
            )
        };
        check(&scan(&source("opaque()"), &[]), &path_at(line), None);
        check(&scan(&source("None"), &[]), &[], Some(NONE));
        check(
            &scan(&source("None").replace("Path | None", "Path"), &[]),
            &[],
            Some(NONE),
        );
    }
    for (head, binding, line) in [
        (
            "def",
            "def read():\n    p = make()\n    return p.read_text()",
            6,
        ),
        (
            "def",
            "def read():\n    p: Path = make()\n    return p.read_text()",
            6,
        ),
        (
            "async def",
            "async def read():\n    p: Path = await make()\n    return p.read_text()",
            6,
        ),
        (
            "def",
            "p: Path = make()\ndef read():\n    return p.read_text()",
            6,
        ),
    ] {
        let source = |value| {
            format!("from pathlib import Path\n{head} make() -> Path:\n    {value}\n{binding}\n")
        };
        check(&scan(&source("return opaque()"), &[]), &path_at(line), None);
        for none in ["return None", "return", "pass"] {
            check(&scan(&source(none), &[]), &[], Some(NONE));
        }
    }
    let mixed = "from pathlib import Path\ndef make(flag) -> Path | None:\n    if flag:\n        return None\n    return opaque()\ndef read(flag):\n    p = make(flag)\n    return p.read_text()\n";
    check(&scan(mixed, &[]), &path_at(8), None);
    check(
        &scan(&mixed.replace("return opaque()", "return None"), &[]),
        &[],
        Some(NONE),
    );
    let second = "from pathlib import Path\ndef read(flag):\n    p: Path = None\n    if flag:\n        p = opaque()\n    return p.read_text()\n";
    check(&scan(second, &[]), &[], Some("1 (receiver_not_unique: 1)"));
}

#[test]
fn conflicts_and_opaque_helpers_preserve_route_and_family_precedence() {
    let top = "from pathlib import Path\nfrom configparser import ConfigParser, RawConfigParser\n";
    for value in [
        "None",
        "0",
        "-1",
        "'path'",
        "b'path'",
        "True",
        "{}",
        "[0]",
        "{0}",
        "(0,)",
        "[i for i in items]",
        "ConfigParser()",
    ] {
        let source =
            |value| format!("{top}def read(p: Path = {value}):\n    return p.read_text()\n");
        check(&scan(&source("opaque()"), &[]), &path_at(4), None);
        check(&scan(&source(value), &[]), &[], Some(CONFLICT));
    }
    let source = |result| {
        format!("{top}def make(flag) -> Path:\n    if flag:\n        return {result}\n    return opaque()\ndef read(flag):\n    p = make(flag)\n    return p.read_text()\n")
    };
    check(&scan(&source("opaque()"), &[]), &path_at(9), None);
    check(&scan(&source("ConfigParser()"), &[]), &[], Some(CONFLICT));
    let promote = format!("{top}def make() -> Path:\n    return ConfigParser()\ndef read():\n    p: Path = make()\n    return p.read('config')\n");
    check(
        &scan(&promote, &["--inventory-kinds", "parse"]),
        &[("parse", "configparser.ConfigParser.read", "nameonly", 7)],
        None,
    );
    check(
        &scan(&promote, &["--inventory-kinds", "filesystem"]),
        &[],
        None,
    );
    check(
        &scan(&promote.replace("read('config')", "read_text()"), &[]),
        &[],
        Some(CONFLICT),
    );
    check(
        &scan(&promote.replace("read('config')", "not_listed()"), &[]),
        &[],
        None,
    );
    let home = format!("{top}def read(p: Path):\n    return p.home()\n");
    check(
        &scan(&home, &["--inventory-kinds", "config"]),
        &[("config", "pathlib.Path.home", "nameonly", 4)],
        None,
    );
    check(
        &scan(&home, &["--inventory-kinds", "filesystem"]),
        &[],
        None,
    );
}

#[test]
fn helper_admission_and_returned_local_refusals_have_opaque_or_constructed_controls() {
    let make = "def make() -> Path:\n    return opaque()";
    let read = "def read():\n    p = make()\n    return p.read_text()";
    let run = |defs: &str, caller: &str| {
        scan(
            &format!("from pathlib import Path\n{defs}\n{caller}\n"),
            &[],
        )
    };
    check(&run(make, read), &path_at(6), None);
    for (definition, caller) in [
        (format!("@decorate\n{make}"), read.to_owned()),
        (format!("{make}\n{make}"), read.to_owned()),
        (make.to_owned(), read.replace("read()", "read(make)")),
        (
            make.replace("    return", "    yield 0\n    return"),
            read.to_owned(),
        ),
        (make.replace("def make", "async def make"), read.to_owned()),
        (
            format!("{make}\ndef reset():\n    global make\n    make = opaque()"),
            read.to_owned(),
        ),
        (
            "".to_owned(),
            read.replace(
                "    p = make()",
                "    def make() -> Path:\n        return opaque()\n    p = make()",
            ),
        ),
    ] {
        check(
            &run(&definition, &caller),
            &[],
            Some("1 (helper_not_admissible: 1)"),
        );
    }
    check(
        &run("def make():\n    return Path()", read),
        &[],
        Some("1 (helper_unannotated: 1)"),
    );
    check(
        &run("def make() -> Path:\n    return Path()", read),
        &path_at(6),
        None,
    );
    check(
        &run(
            "async def make() -> Path:\n    return opaque()",
            &read
                .replace("def read", "async def read")
                .replace("= make()", "= await make()"),
        ),
        &path_at(6),
        None,
    );
    let witness = |extra| format!("def make() -> Path:\n    v = Path()\n    {extra}\n    return v");
    for other in [
        "v.exists()",
        "setattr(v, 'write_text', fn)",
        "vars(v)['read_text',] = fn",
        "def unrelated(v):\n        yield v",
    ] {
        let site = path_at(if other.contains('\n') { 9 } else { 8 });
        let mut rows = Vec::new();
        if other == "v.exists()" {
            rows.push(("filesystem", "pathlib.Path.exists", "exact", 4));
        }
        rows.extend(site);
        check(&run(&witness(other), read), &rows, None);
    }
    for escape in [
        "sink(v)",
        "v.exists(v)",
        "alias = v",
        "del v",
        "[v.exists() for _ in xs]",
        "match obj:\n        case v(): pass",
    ] {
        let rows = if escape == "v.exists(v)" || escape.contains("for _") {
            vec![("filesystem", "pathlib.Path.exists", "exact", 4)]
        } else {
            vec![]
        };
        check(&run(&witness(escape), read), &rows, Some(UNPROVEN));
    }
    check(
        &run(&witness("setattr(v, 'read_text', fn); sink(v)"), read),
        &[],
        Some("1 (member_rebound: 1)"),
    );
}

#[test]
fn declaration_lookup_slots_do_not_read_construction_protocols() {
    for owner in ["PosixPath", "WindowsPath"] {
        let source = |slot| {
            format!("from pathlib import Path, PosixPath, WindowsPath\n{owner}.{slot} = replacement\ndef read(p: Path):\n    return p.read_text()\n")
        };
        check(
            &scan(&source("read_text"), &[]),
            &[],
            Some("1 (member_rebound: 1)"),
        );
        for slot in ["write_text", "__init__", "__new__", "__enter__"] {
            check(&scan(&source(slot), &[]), &path_at(4), None);
        }
    }
    for write in [
        "p.read_text = replacement",
        "p.__class__ = replacement",
        "p.__dict__ = replacement",
        "vars(p)['read_text'] = replacement",
        "p.__dict__['read_text'] = replacement",
    ] {
        let source = |write| {
            format!("from pathlib import Path\ndef read(p: Path):\n    {write}\n    return p.read_text()\n")
        };
        check(
            &scan(&source("p.write_text = replacement"), &[]),
            &path_at(4),
            None,
        );
        check(
            &scan(&source(write), &[]),
            &[],
            Some("1 (member_rebound: 1)"),
        );
    }
}

#[test]
fn helper_selected_construction_remains_a_tie_after_returned_local_escape() {
    let control = "from pathlib import Path\nfrom configparser import ConfigParser\ndef make() -> ConfigParser:\n    v = Path()\n    v.write_text('setup')\n    return v\ndef read():\n    p = make()\n    return p.read_text()\n";
    check(
        &scan(control, &[]),
        &[
            ("filesystem", "pathlib.Path.write_text", "exact", 5),
            path_at(9)[0],
        ],
        None,
    );
    let escaped = control.replace("v.write_text('setup')", "store(v)");
    check(&scan(&escaped, &[]), &[], Some(UNPROVEN));
}

#[test]
fn helper_construction_promotions_respect_platform_and_inherited_member_writes() {
    for platform in ["PosixPath", "WindowsPath"] {
        let source = |slot| {
            format!(
            "from pathlib import Path, {platform} as Platform\nPlatform.{slot} = replacement\ndef make() -> Path:\n    return Path()\ndef read():\n    p = make()\n    return p.read_text()\n"
        )
        };
        check(&scan(&source("write_text"), &[]), &path_at(7), None);
        check(
            &scan(&source("read_text"), &[]),
            &[],
            Some("1 (member_rebound: 1)"),
        );
    }
    let source = |slot| {
        format!(
        "from configparser import ConfigParser, RawConfigParser\nRawConfigParser.{slot} = replacement\ndef make() -> RawConfigParser:\n    return ConfigParser()\ndef read():\n    p = make()\n    return p.read('settings')\n"
    )
    };
    let options = ["--inventory-kinds", "parse"];
    check(
        &scan(&source("read_file"), &options),
        &[("parse", "configparser.ConfigParser.read", "nameonly", 7)],
        None,
    );
    check(
        &scan(&source("read"), &options),
        &[],
        Some("1 (member_rebound: 1)"),
    );
}

#[test]
fn stdlib_unsupported_groups_filter_surviving_occurrences() {
    let pinned = matrix_doc("stdlib/sl4-14.py", &["--min-confidence", "exact"]);
    assert_eq!(pinned["targets"], json!([]));
    let source = "from pathlib import Path\ndef read(p: Path):\n    q = Path()\n    return int(p.stat() == q.stat())\n";
    for (options, detail) in [
        (&[][..], "not_line_anchor; 2 calls"),
        (&["--min-confidence", "exact"][..], "not_line_anchor"),
    ] {
        let doc = scan(source, options);
        let [group] = doc["targets"].as_array().unwrap().as_slice() else {
            panic!("{doc:#}");
        };
        assert_eq!(group["kind"], "other");
        assert_eq!(group["confidence"], "exact");
        assert_eq!(group["severity"], "info");
        assert_eq!(group["expected"]["detail"], detail);
        assert_eq!(group["site"]["line"], 4);
        assert!(group["site"].get("function_start_line").is_none());
    }
    let filtered = scan(source, &["--min-severity", "suggestion"]);
    assert_eq!(filtered["targets"], json!([]));
    warning(
        &filtered,
        "inventory: 1 rows removed by --min-severity suggestion",
    );
}
