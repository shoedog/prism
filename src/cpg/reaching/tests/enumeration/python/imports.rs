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
];
