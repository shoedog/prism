use super::*;
use prism::{
    api::{ReviewHintBasis, ReviewImportHint, ReviewUnresolvedReason},
    call_graph::FunctionId,
    languages::Language,
};
#[path = "display_tests/population.rs"]
mod population;

fn hit(files: &[&str], declaring: &[&str], basis: ReviewHintBasis, spec: &str) -> ReviewImportHint {
    ReviewImportHint::InRepo {
        files: files.iter().map(|s| (*s).into()).collect(),
        declaring: declaring.iter().map(|s| (*s).into()).collect(),
        basis,
        spec: spec.into(),
        member: Some("send".into()),
    }
}
fn record(hint: ReviewImportHint, language: Language) -> BoundaryRecord {
    let kind = match &hint {
        ReviewImportHint::InRepo { declaring, .. }
            if language == Language::Go && declaring.is_empty() =>
        {
            BoundaryKind::GoDeclarationUnknown
        }
        ReviewImportHint::InRepo { .. } => BoundaryKind::InRepo,
        _ => BoundaryKind::Unresolved,
    };
    BoundaryRecord {
        site: PhysicalSite {
            caller: FunctionId {
                file: "a.js".into(),
                name: "h".into(),
                start_line: 1,
                end_line: 6,
            },
            start_byte: 32,
            end_byte: 40,
        },
        line: 3,
        callee: "send".into(),
        language,
        kind,
        hint,
    }
}
fn collection(records: Vec<BoundaryRecord>) -> BoundaryCollection {
    BoundaryCollection {
        availability: Availability::Available,
        records,
    }
}
fn ordinary() -> BoundaryRecord {
    record(
        hit(&["z.js"], &[], ReviewHintBasis::Bound, "./lib"),
        Language::JavaScript,
    )
}

#[test]
fn eleven_literal_messages_keep_complete_typed_records() {
    use ReviewHintBasis::{Bound, Candidates};
    use ReviewUnresolvedReason::{NoFile, ReplaceUnproven, TooMany};
    let mut rows = vec![
        (record(hit(&["z.js"], &[], Bound, "./lib"), Language::JavaScript), "import \"./lib\" suggests the callee is likely in [\"z.js\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["z.js"], &[], Candidates, "./lib"), Language::JavaScript), "import \"./lib\" suggests the callee is likely in [\"z.js\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["z.js", "a.js"], &[], Candidates, "./lib"), Language::JavaScript), "import \"./lib\" suggests the callee may be in one of [\"z.js\", \"a.js\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["z.go", "a.go"], &["a.go"], Candidates, "./lib"), Language::Go), "Go import \"./lib\"; the declaration scan suggests the callee is likely in [\"a.go\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["z.go", "a.go", "b.go"], &["b.go", "a.go"], Candidates, "./lib"), Language::Go), "Go import \"./lib\"; the declaration scan suggests the callee may be in one of [\"b.go\", \"a.go\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["a.go"], &["a.go"], Bound, ""), Language::Go), "Go same-package hint; the declaration scan suggests the callee is likely in [\"a.go\"] (outside the analysed file set; not analysed)."),
        (record(hit(&["a.go"], &[], Bound, "./lib"), Language::Go), "Go import \"./lib\" has candidate package files [\"a.go\"] outside the analysed file set; the declaring file for this call is unknown."),
        (record(hit(&["z.go", "a.go"], &[], Candidates, ""), Language::Go), "Go same-package hint has candidate package files [\"z.go\", \"a.go\"] outside the analysed file set; the declaring file for this call is unknown."),
    ];
    for (why, expected) in [
        (NoFile, "import \"./lib\" looks in-repo, but no matching source file was found in the captured repository listing."),
        (TooMany, "import \"./lib\" looks in-repo, but its repository attribution is ambiguous or exceeds the candidate limit."),
        (ReplaceUnproven, "import \"./lib\" looks in-repo, but the Go replacement's repository identity could not be proved."),
    ] {
        rows.push((record(ReviewImportHint::Unresolvable { spec: "./lib".into(), why }, Language::Go), expected));
    }
    let observed: Vec<_> = rows
        .iter()
        .map(|(r, _)| project(&collection(vec![r.clone()])).located)
        .collect();
    let expected: Vec<_> = rows
        .into_iter()
        .map(|(record, suffix)| {
            vec![LocatedBoundaryNote {
                record,
                message: format!(
                    "taint-context: tainted value reaches \"send\"(...) at \"a.js\":3; {suffix}"
                ),
            }]
        })
        .collect();
    eprintln!("WORDING observed={observed:?}");
    assert_eq!(observed.len(), 11);
    assert_eq!(observed, expected);
}

#[test]
fn quoted_dynamic_tokens_are_single_line_and_keep_raw_unicode() {
    let raw = "éλ中\"\\\\n\n\r\t\0\u{1b}\u{7f}\u{85}";
    let expected = "\"éλ中\\\"\\\\\\\\n\\n\\r\\t\\u{0}\\u{1b}\\u{7f}\\u{85}\"";
    let mut r = record(
        hit(&[raw], &[], ReviewHintBasis::Bound, raw),
        Language::JavaScript,
    );
    r.site.caller.file = raw.into();
    r.callee = raw.into();
    let input = collection(vec![r.clone()]);
    let before = input.clone();
    let out = project(&input);
    let expected_message = format!("taint-context: tainted value reaches {expected}(...) at {expected}:3; import {expected} suggests the callee is likely in [{expected}] (outside the analysed file set; not analysed).");
    let distinct = quoted_token("\\n") != quoted_token("\n");
    assert_eq!(input, before);
    assert_eq!(
        (quoted_token(raw), out.located, distinct),
        (
            expected.into(),
            vec![LocatedBoundaryNote {
                record: r,
                message: expected_message
            }],
            true
        )
    );
}

#[test]
fn all_controls_and_extra_unicode_marks_are_visible() {
    let controls: Vec<_> = (0u8..=159)
        .filter(|c| char::from(*c).is_control())
        .collect();
    assert_eq!(controls.len(), 65);
    let observed: Vec<_> = controls
        .iter()
        .map(|c| quoted_token(&char::from(*c).to_string()))
        .collect();
    let expected: Vec<_> = controls
        .iter()
        .map(|c| match c {
            9 => "\"\\t\"".into(),
            10 => "\"\\n\"".into(),
            13 => "\"\\r\"".into(),
            c => format!("\"\\u{{{c:x}}}\""),
        })
        .collect();
    let extras = [
        ('\u{2028}', "\"\\u{2028}\""),
        ('\u{2029}', "\"\\u{2029}\""),
        ('\u{61c}', "\"\\u{61c}\""),
        ('\u{200e}', "\"\\u{200e}\""),
        ('\u{200f}', "\"\\u{200f}\""),
        ('\u{202a}', "\"\\u{202a}\""),
        ('\u{202b}', "\"\\u{202b}\""),
        ('\u{202c}', "\"\\u{202c}\""),
        ('\u{202d}', "\"\\u{202d}\""),
        ('\u{202e}', "\"\\u{202e}\""),
        ('\u{2066}', "\"\\u{2066}\""),
        ('\u{2067}', "\"\\u{2067}\""),
        ('\u{2068}', "\"\\u{2068}\""),
        ('\u{2069}', "\"\\u{2069}\""),
    ];
    let extra_observed: Vec<_> = extras
        .iter()
        .map(|(c, _)| quoted_token(&c.to_string()))
        .collect();
    let extra_expected: Vec<_> = extras
        .iter()
        .map(|(_, expected)| expected.to_string())
        .collect();
    eprintln!("ESCAPES controls={observed:?} extras={extra_observed:?}");
    assert_eq!((observed, extra_observed), (expected, extra_expected));
}
