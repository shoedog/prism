use super::{
    targets_inventory_matrix_test::check_receiver_pins,
    targets_inventory_stdlib_roots_test::{check, scan},
};

const REBOUND: &str = "1 (member_rebound: 1)";
const PATH_READ: &str = "pathlib.Path.read_text";

#[test]
fn stdlib_construction_pins() {
    check_receiver_pins(&[
        "f01d", "f01e", "o17", "sl1-63", "tr1-25", "tr1-26", "tr1-27", "tr1-28", "tr1-29",
        "tr1-30", "tr1-31", "tr1-32", "tr1-33", "tr1-34", "tr1-35", "tr1-36", "tr1-37", "tr1-38",
        "tr1-39", "tr1-72", "tr1-73", "tr1-74", "tr1-75", "tr1-76", "pin07", "pin08", "wf48",
        "wf49", "wf50", "wf51", "wf53", "esc35", "esc36", "esc38",
    ]);
}

#[test]
fn direct_construction_reads_only_required_class_slots() {
    for (owner, slot, callee, canonical, family) in [
        (
            "Path",
            "__new__",
            "P(p).read_text()",
            PATH_READ,
            "filesystem",
        ),
        (
            "WindowsPath",
            "__init__",
            "P(p).read_text()",
            PATH_READ,
            "filesystem",
        ),
        (
            "PosixPath",
            "read_text",
            "P(p).read_text()",
            PATH_READ,
            "filesystem",
        ),
        (
            "WindowsPath",
            "__getattribute__",
            "P(p).read_text()",
            PATH_READ,
            "filesystem",
        ),
        (
            "PureWindowsPath",
            "__bases__",
            "P(p).read_text()",
            PATH_READ,
            "filesystem",
        ),
        (
            "RawConfigParser",
            "read",
            "ConfigParser().read(p)",
            "configparser.ConfigParser.read",
            "parse",
        ),
        (
            "MutableMapping",
            "__new__",
            "ConfigParser().read(p)",
            "configparser.ConfigParser.read",
            "parse",
        ),
    ] {
        let source = |slot| {
            format!(
            "from pathlib import Path as P, Path, PosixPath, WindowsPath, PureWindowsPath\nfrom configparser import ConfigParser, RawConfigParser, MutableMapping\n{owner}.{slot} = replacement\ndef read(p):\n    return {callee}\n"
        )
        };
        check(
            &scan(&source("unselected"), &[]),
            &[(family, canonical, "exact", 5)],
            None,
        );
        check(&scan(&source(slot), &[]), &[], Some(REBOUND));
    }
}

#[test]
fn constructor_and_context_shapes_are_closed() {
    for (receiver, accepted) in [
        ("Path(p)", true),
        ("((Path(p)))", true),
        ("wrap(Path(p))", false),
        ("(x := Path(p))", false),
        ("Path(p)[0]", false),
        ("Path(p).resolve()", false),
        ("super()", false),
    ] {
        let source =
            format!("from pathlib import Path\ndef read(p):\n    return {receiver}.read_text()\n");
        let rows = [("filesystem", PATH_READ, "exact", 3)];
        check(
            &scan(&source, &[]),
            if accepted { &rows } else { &[] },
            None,
        );
    }
    let source = "from pathlib import Path\nasync def read():\n    with (Path('a') as target, Path('b') as other):\n        target.read_text()\n        other.read_text()\n";
    check(
        &scan(source, &[]),
        &[
            ("filesystem", PATH_READ, "exact", 4),
            ("filesystem", PATH_READ, "exact", 5),
        ],
        None,
    );
    check(
        &scan(&source.replace("    with", "    async with"), &[]),
        &[],
        None,
    );
    check(
        &scan(
            &source
                .replace("Path(", "wrap(Path(")
                .replace(") as", ")) as"),
            &[],
        ),
        &[],
        None,
    );
    let source = "from configparser import ConfigParser\ndef read():\n    with ConfigParser() as p:\n        p.read('settings')\n";
    check(&scan(source, &[]), &[], None);
}

#[test]
fn named_instance_writes_share_slot_and_freshness_checks() {
    for write in [
        "p.read_text = replacement",
        "del p.read_text",
        "setattr(p, 'read_text', replacement)",
        "delattr(p, 'read_text')",
        "p.__dict__['read_text'] = replacement",
        "vars(p)['read_text'] = replacement",
        "alias.read_text = replacement",
        "p.__class__ = replacement",
        "p.__dict__ = replacement",
        "p.read_text += replacement",
        "((p.read_text), other) = pair",
    ] {
        let source = |write: &str| {
            format!(
            "from pathlib import Path\ndef read():\n    p = Path()\n    alias = p\n    {write}\n    return p.read_text()\n"
        )
        };
        let sibling = write
            .replace("read_text", "write_text")
            .replace("__class__", "other")
            .replace(".__dict__ =", ".other =");
        check(
            &scan(&source(&sibling), &[]),
            &[("filesystem", PATH_READ, "exact", 6)],
            None,
        );
        check(&scan(&source(write), &[]), &[], Some(REBOUND));
    }
    let source = "from pathlib import Path\ndef read():\n    p = Path()\n    p.read_text()\n    p.read_text = replacement\n";
    check(
        &scan(source, &[]),
        &[("filesystem", PATH_READ, "exact", 4)],
        None,
    );
    check(
        &scan(
            &source.replace(
                "    p.read_text()\n    p.read_text = replacement",
                "    p.read_text = replacement\n    p.read_text()",
            ),
            &[],
        ),
        &[],
        Some(REBOUND),
    );
}

#[test]
fn context_slots_follow_entry_capture_and_exit_reach() {
    let source = |slot, body| {
        format!(
        "from pathlib import Path, PosixPath, WindowsPath\nWindowsPath.{slot} = replacement\ndef read():\n    with Path() as p:\n{body}\n"
    )
    };
    let inside = "        return p.read_text()";
    let after = "        pass\n    return p.read_text()";
    check(
        &scan(&source("unrelated", inside), &[]),
        &[("filesystem", PATH_READ, "exact", 5)],
        None,
    );
    check(&scan(&source("__enter__", inside), &[]), &[], Some(REBOUND));
    check(
        &scan(&source("__exit__", inside), &[]),
        &[("filesystem", PATH_READ, "exact", 5)],
        None,
    );
    check(&scan(&source("__exit__", after), &[]), &[], Some(REBOUND));
    check(
        &scan(&source("unrelated", after), &[]),
        &[("filesystem", PATH_READ, "exact", 6)],
        None,
    );
    for platform in ["PosixPath", "WindowsPath"] {
        let edited = source("read_text", inside)
            .replace("WindowsPath.read_text", &format!("{platform}.read_text"));
        check(&scan(&edited, &[]), &[], Some(REBOUND));
    }
}

#[test]
fn field_ties_count_exact_owner_writes_and_never_prove() {
    let source = |setup, write| {
        format!(
        "from pathlib import Path\nclass Owner:\n    def setup(self):\n        {setup}\n    def other(self):\n        {write}\n    def read(self):\n        return self.p.read_text()\n"
    )
    };
    const UNPROVEN: &str = "1 (receiver_value_unproven: 1)";
    let run = |write| scan(&source("self.p = Path()", write), &[]);
    check(&run("self.p.write_text = replacement"), &[], Some(UNPROVEN));
    check(&run("self.p.read_text = replacement"), &[], Some(REBOUND));
    for write in [
        "del self.p",
        "setattr(self, 'p', replacement)",
        "alias = self\n        vars(alias)['p'] = replacement",
    ] {
        check(&run(write), &[], Some("1 (receiver_not_unique: 1)"));
    }
    check(
        &run("class Inner:\n            def change(self):\n                self.p = None"),
        &[],
        Some(UNPROVEN),
    );
    check(&scan(&source("self.p: Path", "pass"), &[]), &[], None);
    let local =
        source("self.p = Path()", "pass").replace("return self.p", "p = Path()\n        return p");
    check(
        &scan(&local, &[]),
        &[("filesystem", PATH_READ, "exact", 9)],
        None,
    );
}

#[test]
fn constructor_ties_follow_visibility_and_selected_family() {
    let late = "def read():\n    return Path().home()\n    from pathlib import Path\n";
    check(
        &scan(late, &["--inventory-kinds", "config"]),
        &[],
        Some("1 (binding_unavailable: 1)"),
    );
    check(&scan(late, &["--inventory-kinds", "filesystem"]), &[], None);
    let source = "from pathlib import Path\ndef read():\n    p = Path()\n    return p.home()\n";
    check(
        &scan(source, &["--inventory-kinds", "config"]),
        &[("config", "pathlib.Path.home", "exact", 4)],
        None,
    );
    check(
        &scan(source, &["--inventory-kinds", "filesystem"]),
        &[],
        None,
    );
    let remap = source.replace("from pathlib import Path", "from json import loads as Path");
    check(&scan(&remap, &["--inventory-kinds", "config"]), &[], None);
}
