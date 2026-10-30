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

// 3. PR #404 round-1 fold ------------------------------------------------------------

/// LeftFlow, FullFlow and ConditionedSlice (`condition`) line sets for one fixture.
fn three(
    language: Language,
    source: &str,
    diff: usize,
    condition: &str,
) -> Vec<(String, BTreeSet<usize>)> {
    let file = file_name(language);
    let mut out: Vec<(String, BTreeSet<usize>)> =
        [SlicingAlgorithm::LeftFlow, SlicingAlgorithm::FullFlow]
            .map(|algorithm| {
                (
                    format!("{algorithm:?}"),
                    lines_of(language, source, file, &[diff], algorithm),
                )
            })
            .into();
    let files = BTreeMap::from([(
        file.to_string(),
        crate::ast::ParsedFile::parse(file, source, language).unwrap(),
    )]);
    let ctx = crate::cpg::CpgContext::build(&files, None);
    let diff_input = DiffInput {
        files: vec![DiffInfo {
            file_path: file.into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([diff]),
        }],
    };
    let result = crate::algorithms::conditioned_slice::slice(
        &ctx,
        &diff_input,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        &crate::algorithms::conditioned_slice::Condition::parse(condition).unwrap(),
    )
    .unwrap();
    let conditioned = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.get(file).into_iter().flatten())
        .map(|(line, _)| *line)
        .collect();
    out.push(("ConditionedSlice".into(), conditioned));
    out
}

#[test]
fn a_condition_on_a_formal_fed_by_a_field_argument_keeps_its_dfg_answer() {
    // sol r1 (WRONG): `q` receives `p.x` from a member-only caller. Its backward walk
    // reaches the caller's projection-only `Def(p)` through the argument edge; that must
    // not switch `q` to the unscoped name fallback (which adds the shadowing arrow at 6
    // and its sibling-derived sink at 8). Pre-#390 output for every combination.
    let sol = "function outer({P}) {\n  function g(q) {\n    if (q) {\n      a();\n    }\n    const f = (q) => {\n      const html = p.y;\n      sink(q, html);\n    };\n    use(q);\n  }\n  g(p.x);\n}\n";
    // Opus r1 W1: same shape across two top-level functions.
    let opus = "function g(q) {\n  if (q) {\n    a();\n  }\n  const f = (q) => q;\n  sink(q);\n}\nfunction h({P}) {\n  g(p.x);\n}\n";
    for language in JS_TS {
        for (algorithm, lines) in three(language, &render(language, sol, "p"), 3, "q != null") {
            assert_eq!(lines, set(&[2, 3, 10, 11]), "{language:?} {algorithm} sol");
        }
        for (algorithm, lines) in three(language, &render(language, opus, "p"), 2, "q != null") {
            assert_eq!(lines, set(&[1, 2, 6, 7]), "{language:?} {algorithm} opus");
        }
    }
}

#[test]
fn a_field_condition_of_the_own_formal_still_takes_the_textual_fallback() {
    // Positive side of the narrowed cut: `if (p.x)` on the callable's own member-only
    // formal keeps every textual `p` line (2 and 6), while a plain `q` condition in the
    // same file keeps its ordinary DFG references.
    let src = "function h({P}) {\n  const html = p.z;\n  if (p.x) {\n    a();\n  }\n  b(p.y);\n}\nfunction g(q) {\n  if (q) {\n    c();\n  }\n  use(q);\n}\n";
    for language in JS_TS {
        let src = render(language, src, "p");
        for (algorithm, lines) in three(language, &src, 3, "p.x == null") {
            assert_eq!(lines, set(&[1, 2, 3, 6, 7]), "{language:?} {algorithm}");
        }
        for (algorithm, lines) in three(language, &src, 9, "q != null") {
            assert!(lines.contains(&12), "{language:?} {algorithm}: {lines:?}");
        }
    }
}

#[test]
fn disclosed_residual_a_local_derived_from_the_conditions_own_field_is_not_reached() {
    // Opus r1 S1: pinned at the pre-#390 output. #390's unbounded walk reached `sink(k)`
    // (line 6); this slice deliberately reverts that (known-gaps data-flow row).
    let src = "function h({P}) {\n  const k = p.x;\n  if (p.x) {\n    a();\n  }\n  sink(k);\n}\n";
    for language in JS_TS {
        for (algorithm, lines) in three(language, &render(language, src, "p"), 3, "p.x == null") {
            assert_eq!(lines, set(&[1, 2, 3, 7]), "{language:?} {algorithm}");
        }
    }
}

// 4. PR #404 round-2 fold: the cut reproduces pre-#390's fallback decision -------------

#[test]
fn a_field_read_inside_a_call_argument_or_assignment_keeps_the_dfg_answer() {
    // Opus r2 W1 (WRONG): pre-#390 already had Use nodes for a field read inside a call
    // argument or an assignment RHS, so the condition skipped the name fallback and
    // excluded the sibling lines 2 (`const v = p.y`) and 7 (`b(p.z)`). The cut fires only
    // when every condition Use of the variable is a projection read. Pre-#390 output.
    for (shape, condition, is_async) in [
        ("call", "isOk(p.x, 2)", false),
        ("assign", "w = p.x", false),
        ("await", "await g(p.x)", true),
        ("nested-call", "f(g(p.x))", false),
        ("call-one-arg", "g(p.x)", false),
    ] {
        let template = format!(
            "{}function h({{P}}) {{\n  const v = p.y;\n  if ({condition}) {{\n    a();\n  }}\n  sinkC(v);\n  b(p.z);\n}}\n",
            if is_async { "async " } else { "" }
        );
        for language in JS_TS {
            for (algorithm, lines) in three(
                language,
                &render(language, &template, "p"),
                3,
                "p.x == null",
            ) {
                assert_eq!(lines, set(&[1, 3, 8]), "{language:?} {algorithm} {shape}");
            }
        }
    }
    // A later read of the condition's own field is not added either (pre-#390 output).
    let own_read = "function h({P}) {\n  const v = p.y;\n  if (ok(p.x)) {\n    a();\n  }\n  use(p.x);\n  sinkC(v);\n}\n";
    for language in JS_TS {
        for (algorithm, lines) in
            three(language, &render(language, own_read, "p"), 3, "p.x == null")
        {
            assert_eq!(lines, set(&[1, 3, 8]), "{language:?} {algorithm} own-read");
        }
    }
}

#[test]
fn positive_call_shaped_conditions_keep_their_dfg_answer() {
    // Opus r2 S1: a plain local inside a call condition keeps its DFG use (line 6), and
    // a condition mixing a bare projection read with a call-argument read keeps the
    // DFG answer (one Use is not a projection read), as on every build since pre-#390.
    let local = "function h() {\n  let n = mk();\n  if (ok(n)) {\n    a();\n  }\n  use(n);\n}\n";
    let mixed = "function h({P}) {\n  const v = p.y;\n  if (p.x && ok(p.x)) {\n    a();\n  }\n  sinkC(v);\n  b(p.z);\n}\n";
    for language in JS_TS {
        for (algorithm, lines) in three(language, local, 3, "n != null") {
            assert_eq!(
                lines,
                set(&[1, 2, 3, 6, 7]),
                "{language:?} {algorithm} local"
            );
        }
        for (algorithm, lines) in three(language, &render(language, mixed, "p"), 3, "p.x == null") {
            assert_eq!(lines, set(&[1, 3, 8]), "{language:?} {algorithm} mixed");
        }
    }
}

#[test]
fn projection_read_query_is_true_only_for_a_use_fed_by_a_projection_only_def() {
    // Opus r2 S2: LeftFlow only ever passes a Use, so no slice input reaches the
    // `kind != Use` guard; exercise it through the query directly.
    let src = "function h(p) {\n  if (p.x) {\n    a();\n  }\n  let n = 1;\n  use(n);\n}\n";
    let files = BTreeMap::from([(
        "m.js".to_string(),
        crate::ast::ParsedFile::parse("m.js", src, Language::JavaScript).unwrap(),
    )]);
    let ctx = crate::cpg::CpgContext::build(&files, None);
    let loc_at = |line: usize, base: &str, use_kind: bool| {
        ctx.cpg
            .nodes_at("m.js", line)
            .into_iter()
            .filter_map(|idx| ctx.cpg.to_var_location(idx))
            .find(|l| {
                l.path.base == base && (l.kind == crate::data_flow::VarAccessKind::Use) == use_kind
            })
            .unwrap_or_else(|| panic!("no {base} at line {line}"))
    };
    let base_def = loc_at(1, "p", false);
    assert!(ctx.cpg.dfg_is_projection_only_def(&base_def));
    assert!(
        !ctx.cpg.dfg_use_reads_projection_only_def(&base_def),
        "a Def is never a projection read"
    );
    assert!(ctx
        .cpg
        .dfg_use_reads_projection_only_def(&loc_at(2, "p", true)));
    assert!(!ctx
        .cpg
        .dfg_use_reads_projection_only_def(&loc_at(6, "n", true)));
}
