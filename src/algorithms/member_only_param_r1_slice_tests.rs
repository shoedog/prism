//! PR #390 round-1 fold: write census, escaped identifiers, exact-span Uses, `with`
//! bodies, erased type positions, declaration-line reads, Bare classification of reads
//! inside a member object, deeper-write prefix kills, and the FullFlow composition leak.
//! (Moved from `src/cpg/member_only_param_r1_tests.rs` by #701 S5: they call the review layer.)

use super::member_only_param_test_support::{file_name, render, JS_TS};
use crate::algorithms::run_slicing_compat;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceConfig, SlicingAlgorithm};
use std::collections::{BTreeMap, BTreeSet};

fn slice_lines(
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
const CARD: &str = "export function Card(props: CardProps) {\n  const title = props.title.trim().toUpperCase();\n  const html = props.rawHtml;\n  return (\n    <div dangerouslySetInnerHTML={{ __html: html }}>\n      {title}\n    </div>\n  );\n}\n";
#[test]
fn fullflow_diff_on_a_field_read_never_pulls_in_a_sibling_field() {
    // Main (no parameter Def at all) gives exactly [1,2,4,6,9] for a diff on line 2.
    let lines = slice_lines(
        Language::Tsx,
        CARD,
        "c.tsx",
        &[2],
        SlicingAlgorithm::FullFlow,
    );
    assert_eq!(lines, BTreeSet::from([1, 2, 4, 6, 9]), "{lines:?}");

    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  const a = p.x;\n  sinkB(p.y);\n  const html = p.z;\n  sinkC(html);\n}\n",
            "p",
        );
        let file = file_name(language);
        let lines = slice_lines(language, &src, file, &[2], SlicingAlgorithm::FullFlow);
        assert!(
            !lines.contains(&3) && !lines.contains(&4) && !lines.contains(&5),
            "{language:?}: sibling fields leaked into {lines:?}"
        );
    }
}
#[test]
fn fullflow_diff_on_the_parameter_line_still_fans_out() {
    // Accepted cost: a seed on the whole parameter reaches every field read.
    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  a(p.x);\n  b(p.y);\n}\n",
            "p",
        );
        let file = file_name(language);
        let lines = slice_lines(language, &src, file, &[1], SlicingAlgorithm::FullFlow);
        let left = slice_lines(language, &src, file, &[1], SlicingAlgorithm::LeftFlow);
        assert!(
            (lines.contains(&2) && lines.contains(&3)) || (left.contains(&2) && left.contains(&3)),
            "{language:?}: full {lines:?} left {left:?}"
        );
    }
}
#[test]
fn leftflow_condition_variable_slice_is_pinned() {
    // LeftFlow's condition walk stops at the projection-only base Def and keeps the
    // pre-#390 name fallback (every `p` line); pin that it does not shrink or grow.
    // Sibling-derived locals: member_only_param_leftflow_tests.
    for language in JS_TS {
        let src = render(
            language,
            "function h({P}) {\n  if (p.x) {\n    a();\n  }\n  b(p.y);\n}\n",
            "p",
        );
        let file = file_name(language);
        let lines = slice_lines(language, &src, file, &[2], SlicingAlgorithm::LeftFlow);
        assert!(
            lines.contains(&2) && lines.contains(&5),
            "{language:?}: {lines:?}"
        );
    }
}
