use super::super::super::binding_table::{rows, DeclarationKind, Predicate, Role};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(super) const SOURCE: &str = r#"from __future__ import annotations
import os as operating_system
from .pkg import item as alias
from pkg import *
top = source()
@decorator
def outer[T](a, typed: int, b: int = 1, *args, c: int = 2, **kwargs):
    global global_name
    def inner():
        nonlocal a
        return a
    class C[T](Base):
        pass
    values = [x for x in args]
    unique = {x for x in args}
    mapping = {x: x for x in args}
    stream = (x for x in args)
    target = lambda p=1, *ps, **ks: p
    for left, *rest in args:
        left += 1
    with context() as (first, second), other() as third:
        first = second
    try:
        risky()
    except Error as error:
        del error
    try:
        grouped()
    except* GroupError as group_error:
        pass
    (tuple_left, tuple_right) = pair
    match value:
        case {"key": [head, *tail], **extra} | C(head, value=tail) as whole:
            result = (named := head)
        case 1+2j:
            result = 0
    return result
"#;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "py-for-target-reuse-inside",
        language: Language::Python,
        kind: "for_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f(it):\n    x = source()\n    for x in it:\n        sink(x)\n    done()\n    sink(x)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-for-one-line-body-same-line",
        language: Language::Python,
        kind: "for_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(it):\n    x = source()\n    for x in it: sink(x)\n",
        expect: &[(
            "2:x",
            "3:x",
            FlowConfidence::NameOnly(FlowDoubt::SameLine),
        )],
        expect_counter: Some(("dfg_label_nameonly_sameline", 1)),
    },
    super::case::Case {
        id: "py-with-as-reuse-inside",
        language: Language::Python,
        kind: "with_item",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f(p):\n    x = source(); y = source()\n    with open(p) as x:\n        sink(x); sink(y)\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:y", "4:y", FlowConfidence::Exact),
            ("2:y", "5:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "py-with-statement-container",
        language: Language::Python,
        kind: "with_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f(p):\n    x = source()\n    with open(p) as y:\n        sink(x)\n    sink(x)\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::Exact),
            ("2:x", "5:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "py-with-bare-name-is-not-binding",
        language: Language::Python,
        kind: "with_item",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f():\n    lock = source()\n    with lock:\n        sink(lock)\n",
        expect: &[("2:lock", "4:lock", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "py-with-multiple-bare-names-are-not-binding",
        language: Language::Python,
        kind: "with_item",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f():\n    a = source(); b = source()\n    with a, b:\n        sink(a); sink(b)\n",
        expect: &[
            ("2:a", "4:a", FlowConfidence::Exact),
            ("2:b", "4:b", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "py-with-mixed-bare-and-alias",
        language: Language::Python,
        kind: "with_item",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f(p):\n    lock = source(); x = source()\n    with lock, open(p) as x:\n        sink(lock); sink(x)\n",
        expect: &[
            ("2:lock", "4:lock", FlowConfidence::Exact),
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-with-call-and-attribute-controls",
        language: Language::Python,
        kind: "with_item",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(self):\n    lock = source()\n    with get_lock(): pass\n    sink(lock)\n    with self.lock: pass\n    sink(lock)\n",
        expect: &[
            ("2:lock", "4:lock", FlowConfidence::Exact),
            ("2:lock", "6:lock", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "py-with-one-line-body-same-line",
        language: Language::Python,
        kind: "with_item",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(p):\n    x = source()\n    with open(p) as x: sink(x)\n",
        expect: &[(
            "2:x",
            "3:x",
            FlowConfidence::NameOnly(FlowDoubt::SameLine),
        )],
        expect_counter: Some(("dfg_label_nameonly_sameline", 1)),
    },
    super::case::Case {
        id: "py-with-sequence-aliases",
        language: Language::Python,
        kind: "with_item",
        variant: Some("binding"),
        row_variant: Some(Predicate::NestedFieldChildKindIs {
            field: "value",
            nested_field: "alias",
            any_of: &["tuple", "list"],
        }),
        class: super::case::CaseClass::Behavioral,
        src: "def f(p):\n    a = source(); b = source(); c = source(); d = source()\n    with p as (a, b), p as [c, d]:\n        sink(a); sink(b); sink(c); sink(d)\n",
        expect: &[
            (
                "2:a",
                "4:a",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            (
                "2:b",
                "4:b",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            (
                "2:c",
                "4:c",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            (
                "2:d",
                "4:d",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 4)),
    },
    super::case::Case {
        id: "e0a-py-assignment",
        language: Language::Python,
        kind: "assignment",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f(c):\n    x = source()\n    if c:\n        x = clean()\n        sink(x)\n    sink(x)\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "e0a-py-augmented_assignment",
        language: Language::Python,
        kind: "augmented_assignment",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    x += clean()\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:y", "4:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-py-named_expression",
        language: Language::Python,
        kind: "named_expression",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    if (x := clean()):\n        sink(x)\n    sink(x); sink(y)\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("2:y", "5:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "py-except-alias",
        language: Language::Python,
        kind: "except_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    try: risky()\n    except Error as x:\n        sink(x)\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
            ("2:y", "6:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-except-without-alias-preserves-type-name",
        language: Language::Python,
        kind: "except_clause",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    Error = source(); x = source()\n    try: risky()\n    except Error:\n        x = clean()\n        sink(Error); sink(x)\n",
        expect: &[
            ("2:Error", "6:Error", FlowConfidence::Exact),
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
            ),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-except-group-alias",
        language: Language::Python,
        kind: "except_group_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source()\n    try: risky()\n    except* Error as x:\n        sink(x)\n    sink(x)\n",
        expect: &[
            (
                "2:x",
                "5:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-del-kills",
        language: Language::Python,
        kind: "delete_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    del x\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:y", "4:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-del-bypass-preserves",
        language: Language::Python,
        kind: "delete_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(c):\n    x = source()\n    if c:\n        del x\n    sink(x)\n",
        expect: &[("2:x", "5:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "py-del-list-kills-each-name",
        language: Language::Python,
        kind: "delete_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f():\n    x = source(); y = source()\n    del [x, y]\n    sink(x); sink(y)\n",
        expect: &[
            ("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("2:y", "4:y", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "py-import-statement-masks",
        language: Language::Python,
        kind: "import_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    a = source(); x = source(); keep = source()\n    import a, x\n    sink(a); sink(x); sink(keep)\n",
        expect: &[
            (
                "2:a",
                "4:a",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("2:keep", "4:keep", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    super::case::Case {
        id: "py-import-from-statement-masks",
        language: Language::Python,
        kind: "import_from_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    y = source(); z = source(); w = source(); q = source(); keep = source()\n    from pkg import y, z\n    from pkg import (w, q)\n    sink(y); sink(z); sink(w); sink(q); sink(keep)\n",
        expect: &[
            ("2:y", "5:y", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            ("2:z", "5:z", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })),
            (
                "2:w",
                "5:w",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
            ),
            ("2:q", "5:q", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })),
            ("2:keep", "5:keep", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 4)),
    },
    super::case::Case {
        id: "py-import-alias-masks",
        language: Language::Python,
        kind: "aliased_import",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    import package as x\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:y", "4:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-import-bypass-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(c):\n    x = source()\n    if c:\n        import pkg as x\n    sink(x)\n",
        expect: &[("2:x", "5:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "py-import-exception-bypass-preserves",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f():\n    json = source()\n    try:\n        import ujson as json\n    except ImportError:\n        pass\n    sink(json)\n",
        expect: &[("2:json", "7:json", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "py-import-both-branches-kill",
        language: Language::Python,
        kind: "import_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def f(c):\n    x = source()\n    if c:\n        import a as x\n    else:\n        import b as x\n    sink(x)\n",
        expect: &[(
            "2:x",
            "7:x",
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }),
        )],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "py-global-uncertain",
        language: Language::Python,
        kind: "global_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "def f():\n    x = source(); y = source()\n    global x\n    sink(x); sink(y)\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
            ),
            ("2:y", "4:y", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    super::case::Case {
        id: "py-nonlocal-in-nested-def-is-capture",
        language: Language::Python,
        kind: "nonlocal_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "def outer():\n    x = source()\n    def inner():\n        nonlocal x\n        sink(x)\n    return inner\n",
        expect: &[(
            "2:x",
            "5:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("function_definition", "e0a-py-function_definition"),
    ("lambda", "e0a-py-lambda"),
    ("class_definition", "e0a-py-class_definition"),
    ("list_comprehension", "e0a-py-list_comprehension"),
    ("set_comprehension", "e0a-py-set_comprehension"),
    (
        "dictionary_comprehension",
        "e0a-py-dictionary_comprehension",
    ),
    ("generator_expression", "e0a-py-generator_expression"),
    ("assignment", "e0a-py-assignment"),
    ("augmented_assignment", "e0a-py-augmented_assignment"),
    ("named_expression", "e0a-py-named_expression"),
    ("parameters", "e0a-x-py-parameters"),
];

#[test]
fn task11_python_reuse_cases() {
    for case in CASES {
        super::case::check_case(case);
    }
}

#[test]
fn python_rows_reproduce_the_old_match_arms() {
    let scope_kinds = [
        "function_definition",
        "lambda",
        "class_definition",
        "list_comprehension",
        "set_comprehension",
        "dictionary_comprehension",
        "generator_expression",
    ];
    for k in scope_kinds {
        assert!(
            rows(Language::Python)
                .iter()
                .any(|r| r.kind == k && r.roles.has(Role::Scope)),
            "missing scope row {k}"
        );
    }
    for k in ["assignment", "augmented_assignment", "named_expression"] {
        let r = rows(Language::Python)
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(DeclarationKind::PythonAssignment));
        assert!(r.roles.has(Role::Binding));
    }
}
