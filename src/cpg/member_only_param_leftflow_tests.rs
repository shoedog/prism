//! Follow-up to PR #390: LeftFlow's condition phase (shared with FullFlow through
//! `left_flow_core`) walked backward from `Use(p.x)` to a member-only formal's base Def
//! and then forward through every projection, so a diff on `if (p.x)` pulled in lines
//! that use a local derived from a sibling field (`const html = p.z; ... sink(html)`).
//! Pre-#390 the formal had no Def, so the condition variable took the name fallback
//! (every textual `p` line, nothing derived). The forward leg now seeds only from
//! locations reached before a projection-only Def, and a condition whose walk reaches
//! one keeps that name fallback.

use super::member_only_param_tests::{file_name, render, JS_TS};
use crate::algorithms::run_slicing_compat;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceConfig, SlicingAlgorithm};
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
        crate::ast::ParsedFile::parse(file, source, language).unwrap(),
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

fn both(
    language: Language,
    template: &str,
    diff: &[usize],
) -> [(SlicingAlgorithm, BTreeSet<usize>); 2] {
    let src = render(language, template, "p");
    let file = file_name(language);
    [SlicingAlgorithm::LeftFlow, SlicingAlgorithm::FullFlow]
        .map(|algorithm| (algorithm, lines_of(language, &src, file, diff, algorithm)))
}

fn set(lines: &[usize]) -> BTreeSet<usize> {
    lines.iter().copied().collect()
}

// 1. Leak shapes: each fails on d293009c (line 6 / 7 present) ------------------------

#[test]
fn a_field_condition_never_reaches_a_local_derived_from_a_sibling_field() {
    for template in [
        // `if` on the field, sibling-derived local used after it
        "function h({P}) {\n  const html = p.z;\n  if (p.x) {\n    a();\n  }\n  sinkC(html);\n}\n",
        // `while` condition
        "function h({P}) {\n  let n = p.count;\n  while (p.more) {\n    step();\n  }\n  log(n);\n}\n",
        // arrow function formal
        "const h = ({P}) => {\n  const v = p.y;\n  if (p.x) {\n    a();\n  }\n  sink(v);\n};\n",
    ] {
        for language in JS_TS {
            for (algorithm, lines) in both(language, template, &[3]) {
                assert!(
                    !lines.contains(&6),
                    "{language:?} {algorithm:?}: sibling-derived use leaked: {lines:?}"
                );
                // Pre-#390 name fallback for `p` is kept: the sibling read line stays.
                assert!(
                    lines.contains(&2),
                    "{language:?} {algorithm:?}: name-based `p` line lost: {lines:?}"
                );
            }
            let left = lines_of(
                language,
                &render(language, template, "p"),
                file_name(language),
                &[3],
                SlicingAlgorithm::LeftFlow,
            );
            assert_eq!(left, set(&[1, 2, 3, 7]), "{language:?}: {template}");
        }
    }
}

#[test]
fn a_field_condition_in_a_called_or_nested_callee_never_reaches_a_sibling_local() {
    // Same-file caller passes a whole object; the diff is on the callee's field condition.
    let caller = "function g({P}) {\n  const v = p.y;\n  if (p.x) {\n    a();\n  }\n  sink(v);\n}\nfunction caller(o) {\n  g(o);\n}\n";
    // Callee nested in, and called from, its enclosing function.
    let nested = "function outer(o) {\n  function inner({P}) {\n    const v = p.y;\n    if (p.x) {\n      a();\n    }\n    sink(v);\n  }\n  inner(o);\n  use(o.q);\n}\n";
    for language in JS_TS {
        for (algorithm, lines) in both(language, caller, &[3]) {
            assert!(
                !lines.contains(&6),
                "{language:?} {algorithm:?} caller: {lines:?}"
            );
        }
        for (algorithm, lines) in both(language, nested, &[4]) {
            assert!(
                !lines.contains(&7),
                "{language:?} {algorithm:?} nested: {lines:?}"
            );
            assert!(
                lines.contains(&3),
                "{language:?} {algorithm:?} nested: {lines:?}"
            );
        }
    }
}

#[test]
fn a_tsx_component_prop_condition_never_reaches_a_local_derived_from_a_sibling_prop() {
    let src = "export function Card(props: CardProps) {\n  const html = props.rawHtml;\n  if (props.title) {\n    track();\n  }\n  sink(html);\n  return <div />;\n}\n";
    for algorithm in [SlicingAlgorithm::LeftFlow, SlicingAlgorithm::FullFlow] {
        let lines = lines_of(Language::Tsx, src, "c.tsx", &[3], algorithm);
        assert!(!lines.contains(&6), "{algorithm:?}: {lines:?}");
    }
    assert_eq!(
        lines_of(
            Language::Tsx,
            src,
            "c.tsx",
            &[3],
            SlicingAlgorithm::LeftFlow
        ),
        set(&[1, 2, 3, 7, 8])
    );
}

// 2. Controls: main's output is unchanged ----------------------------------------------

#[test]
fn controls_whole_parameter_bare_formal_and_same_field_reads_are_unchanged() {
    let derived =
        "function h({P}) {\n  const html = p.z;\n  if (p.x) {\n    a();\n  }\n  sinkC(html);\n}\n";
    for language in JS_TS {
        let src = render(language, derived, "p");
        let file = file_name(language);
        // Whole-parameter seed (signature line) still fans out (#390 accepted cost).
        assert_eq!(
            lines_of(language, &src, file, &[1], SlicingAlgorithm::LeftFlow),
            set(&[1, 2, 3, 4, 5, 6, 7]),
            "{language:?}"
        );
        // A bare formal (no projection-only Def) keeps the full DFG walk.
        let bare = render(language, &derived.replace("if (p.x)", "if (p)"), "p");
        assert_eq!(
            lines_of(language, &bare, file, &[3], SlicingAlgorithm::LeftFlow),
            set(&[1, 2, 3, 6, 7]),
            "{language:?}"
        );
        // A formal that is also read bare is not member-only: unchanged.
        let also_bare = render(
            language,
            "function h({P}) {\n  const html = p.z;\n  if (p.x) {\n    a();\n  }\n  sinkC(html);\n  keep(p);\n}\n",
            "p",
        );
        assert_eq!(
            lines_of(language, &also_bare, file, &[3], SlicingAlgorithm::LeftFlow),
            set(&[1, 2, 3, 6, 7, 8]),
            "{language:?}"
        );
        // Another read of the same field (and the sibling read) stay included.
        let same_field = render(
            language,
            "function h({P}) {\n  if (p.x) {\n    a();\n  }\n  log(p.x);\n  b(p.y);\n}\n",
            "p",
        );
        assert_eq!(
            lines_of(
                language,
                &same_field,
                file,
                &[2],
                SlicingAlgorithm::LeftFlow
            ),
            set(&[1, 2, 5, 6, 7]),
            "{language:?}"
        );
        // A plain local in the condition: full DFG walk, unchanged.
        let local = "function h() {\n  let n = 1;\n  if (n) {\n    a();\n  }\n  use(n);\n}\n";
        assert_eq!(
            lines_of(language, local, file, &[3], SlicingAlgorithm::LeftFlow),
            set(&[1, 2, 3, 6, 7]),
            "{language:?}"
        );
        // A condition whose walk reaches no projection-only Def keeps the DFG answer and
        // does not take the name fallback (which would add the shadowing arrow, line 6).
        let shadowed =
            "function h() {\n  let n = 1;\n  if (n) {\n    a();\n  }\n  const g = (n) => n + 1;\n}\n";
        assert_eq!(
            lines_of(language, shadowed, file, &[3], SlicingAlgorithm::LeftFlow),
            set(&[1, 2, 3, 7]),
            "{language:?}"
        );
    }
}

#[test]
fn controls_other_languages_field_conditions_are_unchanged() {
    let c = "void h(struct s *p) {\n  int v = p->z;\n  if (p->x) {\n    a();\n  }\n  sink(v);\n}\n";
    assert_eq!(
        lines_of(Language::C, c, "m.c", &[3], SlicingAlgorithm::LeftFlow),
        set(&[1, 2, 3, 7])
    );
    let py = "def h(p):\n    v = p.z\n    if p.x:\n        a()\n    sink(v)\n";
    assert_eq!(
        lines_of(
            Language::Python,
            py,
            "m.py",
            &[3],
            SlicingAlgorithm::LeftFlow
        ),
        set(&[1, 2, 3, 5])
    );
}
