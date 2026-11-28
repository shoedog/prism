use super::targets_inventory_matrix_test::check_receiver_pins;
use super::targets_inventory_test::{document, run};
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

const EXCLUSIONS: &str = "inventory: excluded directories: .git, target, node_modules, vendor, venv, site-packages, dot directories, build and dist at the root";

pub(super) fn scan(source: &str, options: &[&str]) -> Value {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("client.py"), source).unwrap();
    document(&run(repo.path(), options))
}

pub(super) fn check(doc: &Value, rows: &[(&str, &str, &str, u64)], cause: Option<&str>) {
    let targets = doc["targets"].as_array().unwrap();
    let actual: Vec<_> = targets
        .iter()
        .map(|target| {
            assert_eq!(target["kind"], "external_call", "{target:#}");
            (
                target["dependency_hint"]["kind"].as_str().unwrap(),
                target["dependency_hint"]["callee"].as_str().unwrap(),
                target["confidence"].as_str().unwrap(),
                target["site"]["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(actual, rows, "rows in {doc:#}");
    let warnings = match cause {
        Some(cause) => json!([
            format!("inventory: identity unproven: client.py: {cause}"),
            EXCLUSIONS
        ]),
        None => json!([EXCLUSIONS]),
    };
    assert_eq!(doc["warnings"], warnings);
}

#[test]
fn starred_vararg_annotation_walks_only_eager_expression() {
    let control =
        "import json\ndef configure():\n    def inner(*items: json.loads('[]')):\n        pass\n";
    let starred = control.replace(": json.loads", ": *json.loads");
    let grouped = starred.replace("*json.loads('[]')", "*(json.loads('[]'))");
    let subscript = starred.replace("*json.loads('[]')", "*json.loads('[]')[0]");
    let bare = starred
        .replace("import json", "from json import loads")
        .replace("json.loads(", "loads(");
    for source in [control, &starred, &grouped, &subscript, &bare] {
        let doc = scan(source, &["--inventory-kinds", "parse"]);
        check(&doc, &[("parse", "json.loads", "exact", 3)], None);
        assert_eq!(
            doc["targets"][0]["site"],
            json!({"file":"client.py", "line":3, "symbol":"configure",
                   "function_start_line":2, "function_end_line":4, "language":"python"})
        );
    }
    for source in [
        format!("from __future__ import annotations\n{starred}"),
        format!("from __future__ import annotations\n{grouped}"),
        starred.replace("*items:", "items:"),
        starred.replace("*items:", "**items:"),
    ] {
        check(&scan(&source, &["--inventory-kinds", "parse"]), &[], None);
    }
    let unrelated = "def malformed(*items: *opaque() if y else z):\n    pass\n";
    let mixed = format!("{starred}{unrelated}");
    check(
        &scan(&mixed, &["--inventory-kinds", "parse"]),
        &[("parse", "json.loads", "exact", 3)],
        None,
    );
    check_receiver_pins(&["as3-32"]);
}

#[test]
fn stdlib_root_pins() {
    check_receiver_pins(&[
        "w06b", "f01a", "f01b", "f01c", "f02", "f03", "f04a", "f04b", "e02", "o15", "o16", "o18",
        "o19", "o20", "o21", "o22", "tr1-40", "tr1-55", "tr1-66", "tr1-77", "tr1-83", "tr2-72",
        "tr2-73", "tr2-74", "tr2-75", "tr2-76", "tr2-77", "tr3-27", "tr4-09", "tr4-11", "sl1-60",
        "sl1-61", "sl1-62", "sl1-66", "sl2-26", "sl3-19", "sl3-20", "as1-24", "as1-25", "as1-26",
        "as1-32", "as2-28", "as2-31", "as3-25", "as3-29", "as3-30", "as5-12", "as5-13", "as5-15",
        "wf03", "wf04", "wf48", "wf96", "wf98", "esc18", "esc19",
    ]);
}

#[test]
fn off_table_paths_and_remaps_are_silent() {
    let source =
        "import os, tempfile\nfrom pathlib import Path\nfrom xml.etree import ElementTree\n\
                  def f(p):\n    os.open(p)\n    os.path.exists(p)\n    tempfile.gettempdir()\n\
                  \x20   ElementTree.XML(p)\n    Path(p)\n    os.stat(p)\n    os.path.getsize(p)\n\
                  \x20   tempfile.mkdtemp()\n    ElementTree.fromstring(p)\n";
    check(
        &scan(source, &[]),
        &[
            ("filesystem", "os.stat", "exact", 10),
            ("filesystem", "os.path.getsize", "exact", 11),
            ("filesystem", "tempfile.mkdtemp", "exact", 12),
            ("parse", "xml.etree.ElementTree.fromstring", "exact", 13),
        ],
        None,
    );
    let source = "def f(p):\n    open(p)\n    from tempfile import gettempdir as open\n";
    check(&scan(source, &[]), &[], None);
}

#[test]
fn regular_package_sibling_is_not_an_absolute_origin() {
    let repo = TempDir::new().unwrap();
    let mut path = repo.path().to_path_buf();
    for package in ["httpie", "output", "formatters"] {
        path.push(package);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("__init__.py"), "").unwrap();
    }
    fs::write(path.join("json.py"), "").unwrap();
    fs::write(
        path.join("colors.py"),
        format!(
            "import json\n{}def color(text):\n    return json.loads(text)\n",
            "\n".repeat(180)
        ),
    )
    .unwrap();
    let file = "httpie/output/formatters/colors.py";
    let read = || document(&run(repo.path(), &["--files", file]));
    let doc = read();
    assert_eq!(doc["warnings"], json!([EXCLUSIONS]));
    let [target] = doc["targets"].as_array().unwrap().as_slice() else {
        panic!("expected the package site: {doc:#}");
    };
    assert_eq!(
        target["dependency_hint"],
        json!({"kind":"parse","callee":"json.loads"})
    );
    assert_eq!(target["confidence"], "exact");
    assert_eq!(target["site"]["line"], 183);
    for directory in ["", "src"] {
        let origin = repo.path().join(directory);
        fs::create_dir_all(&origin).unwrap();
        fs::write(origin.join("json.py"), "").unwrap();
        let doc = read();
        assert_eq!(doc["targets"], json!([]));
        assert_eq!(
            doc["warnings"],
            json!([
                format!("inventory: identity unproven: {file}: 1 (repository_local_module: 1)"),
                EXCLUSIONS
            ])
        );
        fs::remove_file(origin.join("json.py")).unwrap();
    }
}

#[test]
fn stdlib_root_binding_refusals() {
    let baseline = "import json\ndef read(p):\n    return json.loads(p)\n";
    check(
        &scan(baseline, &[]),
        &[("parse", "json.loads", "exact", 3)],
        None,
    );
    for (source, cause) in [
        (
            "from elsewhere import *\nimport json\ndef read(p):\n    return json.loads(p)\n",
            "star_import",
        ),
        (
            "import json\ndef read(json, p):\n    return json.loads(p)\n",
            "shadowed_root",
        ),
        ("def read(p):\n    return json.loads(p)\n", "not_imported"),
        (
            "from . import json\ndef read(p):\n    return json.loads(p)\n",
            "repository_local_module",
        ),
        (
            "def read(p):\n    return json.loads(p)\n    import json\n",
            "binding_unavailable",
        ),
    ] {
        let warning = format!("1 ({cause}: 1)");
        check(&scan(source, &[]), &[], Some(&warning));
    }
}

#[test]
fn builtin_slot_writes_and_captures() {
    for write in [
        "b.open = v",
        "del b.open",
        "setattr(b, 'open', v)",
        "delattr(b, 'open')",
        "b.__dict__['open'] = v",
        "vars(b)['open'] = v",
        "__builtins__['open'] = v",
        "__builtins__.open = v",
        "globals()['open'] = v",
        "globals()['__builtins__'] = v",
        "del open",
    ] {
        let cause = if write.starts_with("globals") || write == "del open" {
            "1 (shadowed_root: 1)"
        } else {
            "1 (member_rebound: 1)"
        };
        let source =
            format!("import builtins as b\ndef read(p, v):\n    {write}\n    return open(p)\n");
        check(&scan(&source, &[]), &[], Some(cause));
        let source = source.replacen(
            write,
            &write
                .replace("open", "print")
                .replace("'__builtins__'", "'other'"),
            1,
        );
        check(
            &scan(&source, &[]),
            &[("filesystem", "builtins.open", "exact", 4)],
            None,
        );
    }
    for (export, import, kind, canonical) in [
        (
            "builtins.open",
            "from builtins import open as call",
            "filesystem",
            "builtins.open",
        ),
        (
            "json.loads",
            "from json import loads as call",
            "parse",
            "json.loads",
        ),
    ] {
        let (module, _) = export.split_once('.').unwrap();
        let before =
            format!("import {module}\n{export} = v\n{import}\ndef read(p):\n    return call(p)\n");
        check(&scan(&before, &[]), &[], Some("1 (member_rebound: 1)"));
        let after =
            format!("import {module}\n{import}\n{export} = v\ndef read(p):\n    return call(p)\n");
        check(&scan(&after, &[]), &[(kind, canonical, "exact", 5)], None);
    }
}

#[test]
fn class_qualified_roots_read_only_the_selected_member_chain() {
    let setup = "from pathlib import Path, PosixPath\nfrom configparser import ConfigParser, RawConfigParser\n";
    for (write, call, kind, callee, veto) in [
        (
            "Path.home",
            "Path.home()",
            "config",
            "pathlib.Path.home",
            true,
        ),
        (
            "Path.__new__",
            "Path.home()",
            "config",
            "pathlib.Path.home",
            false,
        ),
        (
            "PosixPath.home",
            "Path.home()",
            "config",
            "pathlib.Path.home",
            false,
        ),
        (
            "RawConfigParser.read",
            "ConfigParser.read(p, f)",
            "parse",
            "configparser.ConfigParser.read",
            true,
        ),
        (
            "RawConfigParser.read_file",
            "ConfigParser.read(p, f)",
            "parse",
            "configparser.ConfigParser.read",
            false,
        ),
    ] {
        let source = format!("{setup}{write} = replacement\ndef read(p,f):\n    return {call}\n");
        if veto {
            check(&scan(&source, &[]), &[], Some("1 (member_rebound: 1)"));
        } else {
            check(&scan(&source, &[]), &[(kind, callee, "exact", 5)], None);
        }
    }
}

#[test]
fn family_selection_preserves_cross_family_anchor_competition() {
    let source = "import json, os, requests\ndef read(p):\n    json.load(open(p))\n    os.getenv(p)\n    requests.get(p)\n";
    for (kinds, absence, wanted) in [
        ("filesystem", true, vec![]),
        ("parse", false, vec![("parse", "json.load", "exact", 3)]),
        ("config", false, vec![("config", "os.getenv", "exact", 4)]),
        ("http", false, vec![("http", "requests.get", "exact", 5)]),
        (
            "filesystem,parse,config",
            true,
            vec![
                ("parse", "json.load", "exact", 3),
                ("config", "os.getenv", "exact", 4),
            ],
        ),
    ] {
        let mut doc = scan(source, &["--inventory-kinds", kinds]);
        let targets = doc["targets"].as_array_mut().unwrap();
        if absence {
            let position = targets
                .iter()
                .position(|t| t["kind"] == "other")
                .expect("cross-family anchor must win");
            let row = targets.remove(position);
            assert_eq!(row["expected"]["detail"], "not_line_anchor");
            assert_eq!(
                row["dependency_hint"],
                json!({"kind":"filesystem","callee":"builtins.open"})
            );
            assert_eq!(row["site"]["line"], 3);
        }
        check(&doc, &wanted, None);
    }
    let shadow = source.replace("def read(p):", "def read(p, json):");
    check(
        &scan(&shadow, &["--inventory-kinds", "parse"]),
        &[],
        Some("1 (shadowed_root: 1)"),
    );
    check(
        &scan(&shadow, &["--inventory-kinds", "http"]),
        &[("http", "requests.get", "exact", 5)],
        None,
    );
}
