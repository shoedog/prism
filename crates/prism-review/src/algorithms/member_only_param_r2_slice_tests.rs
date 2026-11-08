//! PR #390 round-2 fold: FullFlow through any route into a member-only formal, the write
//! census over a whole receiver chain, `with` refusal, and linear-cost fixtures.
//! (Moved from `src/cpg/member_only_param_r2_tests.rs` by #701 S5: they call the review layer.)

use super::member_only_param_test_support::{file_name, render, JS_TS};
use crate::algorithms::run_slicing_compat;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use prism::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

fn lines_of(
    language: Language,
    source: &str,
    file: &str,
    diff_lines: &[usize],
    algorithm: SlicingAlgorithm,
) -> BTreeSet<usize> {
    let files = BTreeMap::from([(
        file.to_string(),
        prism::ast::ParsedFile::parse(file, source, language).unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: file.into(),
            modify_type: ModifyType::Modified,
            diff_lines: diff_lines.iter().copied().collect(),
        }],
    };
    let result = run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(algorithm),
        None,
    )
    .unwrap();
    result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.get(file).into_iter().flatten())
        .map(|(line, _)| *line)
        .collect()
}
fn full_flow(language: Language, template: &str, diff: &[usize]) -> BTreeSet<usize> {
    let src = render(language, template, "p");
    lines_of(
        language,
        &src,
        file_name(language),
        diff,
        SlicingAlgorithm::FullFlow,
    )
}
#[test]
fn fullflow_through_an_upstream_argument_never_reaches_sibling_fields() {
    // sol W1: `run(q) { h(q) }` feeds the formal; main gives exactly [4,5,6,9].
    let template = "function run(q) {\n h(q);\n}\nfunction h({P}) {\n  const a = p.x;\n  sinkA(a);\n  const b = p.y;\n  sinkB(b);\n}\n";
    for language in JS_TS {
        assert_eq!(
            full_flow(language, template, &[5]),
            set(&[4, 5, 6, 9]),
            "{language:?}"
        );
    }
    // A local actual argument defined in the caller takes the same route.
    let template = "function run() {\n const q = mk();\n h(q);\n}\nfunction h({P}) {\n  const a = p.x;\n  sinkA(a);\n  const b = p.y;\n  sinkB(b);\n}\n";
    for language in JS_TS {
        assert_eq!(
            full_flow(language, template, &[6]),
            set(&[5, 6, 7, 10]),
            "{language:?}"
        );
    }
}
#[test]
fn fullflow_through_a_same_file_caller_never_reaches_sibling_fields() {
    // Opus ff1: `inner(o)` passes a variable; main gives [1,2,4,5].
    let template =
        "function inner({P}) {\n  const a = p.x;\n  sinkB(p.y);\n  return a;\n}\nfunction caller() {\n  const o = mk();\n  inner(o);\n}\n";
    for language in JS_TS {
        assert_eq!(
            full_flow(language, template, &[2]),
            set(&[1, 2, 4, 5]),
            "{language:?}"
        );
    }
    // Declared before the callee (Opus ff5) takes the same route.
    let template =
        "function caller() {\n  const o = mk();\n  inner(o);\n}\nfunction inner({P}) {\n  const a = p.x;\n  sinkB(p.y);\n  return a;\n}\n";
    for language in JS_TS {
        let lines = full_flow(language, template, &[6]);
        assert!(
            !lines.contains(&7),
            "{language:?}: sibling leaked {lines:?}"
        );
    }
}
#[test]
fn fullflow_tsx_component_called_directly_never_reaches_a_sibling_prop() {
    let card = "export function Card(props: any) {\n  const title = props.title.trim();\n  const html = props.rawHtml;\n  return <div dangerouslySetInnerHTML={{ __html: html }}>{title}</div>;\n}\n";
    let with_caller = format!(
        "{card}export function Page(data: any) {{\n  const p = load(data);\n  return Card(p);\n}}\n"
    );
    let alone = lines_of(
        Language::Tsx,
        card,
        "c.tsx",
        &[2],
        SlicingAlgorithm::FullFlow,
    );
    let called = lines_of(
        Language::Tsx,
        &with_caller,
        "c.tsx",
        &[2],
        SlicingAlgorithm::FullFlow,
    );
    assert_eq!(called, alone, "a direct caller must not change the slice");
    assert!(!called.contains(&3), "{called:?}");
}
#[test]
fn fullflow_controls_no_caller_and_whole_parameter_seeds_are_unchanged() {
    // No caller: direct field diff.
    let direct =
        "function h({P}) {\n  const a = p.x;\n  sinkA(a);\n  const b = p.y;\n  sinkB(b);\n}\n";
    for language in JS_TS {
        assert_eq!(
            full_flow(language, direct, &[2]),
            set(&[1, 2, 3, 6]),
            "{language:?}"
        );
    }
    // Whole-parameter seed with a caller: the declaration line still fans out to every
    // field read (accepted cost), and the caller's line is not pulled in.
    let template = "function run(q) {\n h(q);\n}\nfunction h({P}) {\n  a(p.x);\n  b(p.y);\n}\n";
    for language in JS_TS {
        let lines = full_flow(language, template, &[4]);
        assert!(
            lines.contains(&5) && lines.contains(&6),
            "{language:?}: {lines:?}"
        );
    }
    // A bare formal keeps full expansion through the same caller shape.
    let bare = "function run(q) {\n h(q);\n}\nfunction h({P}) {\n  const a = p.x;\n  use(p);\n  sinkB(p.y);\n}\n";
    for language in JS_TS {
        let lines = full_flow(language, bare, &[5]);
        assert!(
            lines.contains(&7),
            "{language:?}: bare formal must still expand {lines:?}"
        );
    }
}

fn set(lines: &[usize]) -> BTreeSet<usize> {
    lines.iter().copied().collect()
}
