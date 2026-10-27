use super::super::case::{Case, CaseClass};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(in crate::cpg::reaching::tests::enumeration) const CASES: &[Case] = &[
    Case {
        id: "py-import-if-else-both-branches-kills",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(c):\n    x = source()\n    if c:\n        import left as x\n    else:\n        import right as x\n    sink(x)\n",
        expect: &[(
            "2:x",
            "7:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-loop-body",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(items):\n    x = source()\n    for item in items:\n        import package as x\n    sink(x)\n",
        expect: &[("2:x", "5:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-module-level-preserved",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "import package as x\ndef f():\n    x = source()\n    sink(x)\n",
        expect: &[("3:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-future-import-not-binding",
        language: Language::Python,
        kind: "future_import_statement",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "from __future__ import annotations\ndef f():\n    annotations = source()\n    sink(annotations)\n",
        expect: &[("3:annotations", "4:annotations", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-star-uncertain",
        language: Language::Python,
        kind: "wildcard_import",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "def f():\n    x = source()\n    from package import *\n    sink(x)\n",
        expect: &[(
            "2:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "py-import-line-use-r01",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    import pkg as x\n    sink(x)\n",
        expect: &[(
            "2:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-dotted-path-r08",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(path, b):\n    import os.path\n    import a.b\n    sink(path); sink(b)\n",
        expect: &[("1:path", "4:path", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-alias-original-r09",
        language: Language::Python,
        kind: "import_from_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(a, pkg):\n    from pkg import a as b\n    import pkg2 as pkg\n    sink(a); sink(pkg)\n",
        expect: &[
            ("1:a", "4:a", FlowConfidence::Exact),
            (
                "1:pkg",
                "4:pkg",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-dotted-from-a06",
        language: Language::Python,
        kind: "import_from_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(project, incident):\n    project = clean()\n    from dispatch.project import flows as project_flows\n    from dispatch.plugins.dispatch_slack.incident import messages\n    sink(project); sink(incident)\n",
        expect: &[
            ("2:project", "5:project", FlowConfidence::Exact),
            ("1:incident", "5:incident", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    Case {
        id: "py-import-exception-finally-kills-r10",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        import ujson as y\n    except ImportError:\n        pass\n    finally:\n        x = clean()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "9:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 8 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-exception-finally-alias-kills-r19",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        import ujson as x\n    except ImportError:\n        pass\n    finally:\n        x = clean()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "9:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 8 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-exception-if-sibling-r16",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(c):\n    if c:\n        x = source()\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    else:\n        sink(x)\n",
        expect: &[(
            "3:x",
            "9:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "py-import-exception-elif-sibling-r21",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(c, d):\n    if c:\n        x = source()\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    elif d:\n        sink(x)\n",
        expect: &[(
            "3:x",
            "9:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "py-import-exception-match-sibling-r23",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(c):\n    match c:\n        case 1:\n            x = source()\n            try:\n                import ujson as y\n            except ImportError:\n                pass\n        case 2:\n            sink(x)\n",
        expect: &[(
            "4:x",
            "10:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "py-import-exception-loop-end-r14",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(items):\n    x = source()\n    for i in items:\n        try:\n            import ujson as x\n        except ImportError:\n            pass\n    sink(x)\n",
        expect: &[("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-exception-outer-handler-r24",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    except Exception:\n        x = clean()\n    sink(x)\n",
        expect: &[("2:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-try-except-handler-use",
        language: Language::Python,
        kind: "import_from_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(status, reason, body, headers):\n    try:\n        from requests.models import Response\n    except ImportError:\n        return g(status, reason, body, dict(headers))\n    response = Response()\n    return response\n",
        expect: &[(
            "1:status",
            "5:status",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "py-import-exception-enclosing-finally-kills",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    finally:\n        x = clean()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "10:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 9 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-exception-enclosing-else-kills",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    except ValueError:\n        return\n    else:\n        x = clean()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "12:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 11 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "py-import-exception-enclosing-handler-finally-kills",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        risky()\n    except ValueError:\n        try:\n            import ujson as y\n        except ImportError:\n            pass\n    finally:\n        x = clean()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "12:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    Case {
        id: "py-import-exception-inside-enclosing-finally-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    try:\n        risky()\n    finally:\n        try:\n            import ujson as x\n        except ImportError:\n            pass\n    sink(x)\n",
        expect: &[("2:x", "10:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-exception-with-last-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(m):\n    x = source()\n    with m:\n        try:\n            import ujson as x\n        except ImportError:\n            pass\n    sink(x)\n",
        expect: &[("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-exception-with-following-statement-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(m):\n    x = source()\n    with m:\n        try:\n            import ujson as x\n        except ImportError:\n            pass\n        k()\n    sink(x)\n",
        expect: &[("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-exception-if-inside-with-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f(m, c):\n    x = source()\n    with m:\n        if c:\n            try:\n                import ujson as x\n            except ImportError:\n                pass\n    sink(x)\n",
        expect: &[("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    Case {
        id: "py-import-exception-no-successor-fails-closed",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "def f():\n    x = source()\n    def g():\n        try:\n            import ujson as x\n        except ImportError:\n            pass\n    g()\n    sink(x)\n",
        expect: &[(
            "2:x",
            "9:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 5 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
];

fn check(id: &str) {
    let case = CASES
        .iter()
        .find(|case| case.id == id)
        .unwrap_or_else(|| panic!("missing Task 20p case {id}"));
    super::super::case::check_case(case);
}

#[test]
fn task20p_fix2_enclosing_try_successors() {
    for id in [
        "py-import-exception-enclosing-finally-kills",
        "py-import-exception-enclosing-else-kills",
        "py-import-exception-enclosing-handler-finally-kills",
        "py-import-exception-inside-enclosing-finally-preserves",
    ] {
        check(id);
    }
}

#[test]
fn task20p_fix2_with_successors() {
    for id in [
        "py-import-exception-with-last-preserves",
        "py-import-exception-with-following-statement-preserves",
        "py-import-exception-if-inside-with-preserves",
    ] {
        check(id);
    }
}

#[test]
fn task20p_fix2_no_successor_fails_closed() {
    check("py-import-exception-no-successor-fails-closed");
}
