use super::*;

#[test]
fn population_cap_uses_complete_unique_count_without_mutation() {
    let mut observed = Vec::new();
    for count in [0, 10, 11, 12] {
        let records: Vec<_> = (0..count)
            .map(|i| {
                let mut r = ordinary();
                r.site.start_byte += i;
                r.site.end_byte += i;
                r
            })
            .collect();
        let input = collection(records.clone());
        let before = input.clone();
        let out = project(&input);
        assert_eq!(input, before);
        let mut duplicated = records.clone();
        duplicated.extend(records);
        duplicated.reverse();
        assert_eq!(out, project(&collection(duplicated)));
        observed.push((out.located.len(), out.omitted_count, out.summary()));
    }
    assert_eq!(
        observed,
        vec![
            (0, 0, None),
            (10, 0, None),
            (
                10,
                1,
                Some("taint-context: 1 more boundary calls not listed".into())
            ),
            (
                10,
                2,
                Some("taint-context: 2 more boundary calls not listed".into())
            )
        ]
    );
}

#[test]
fn identity_preserves_full_owner_bytes_and_ordered_raw_targets() {
    let mut records = vec![ordinary(); 7];
    records[1].site.start_byte += 1;
    records[2].site.end_byte += 1;
    records[3].site.caller.name = "other".into();
    records[4].site.caller.start_line += 1;
    records[5].site.caller.end_line += 1;
    records[6].hint = hit(&["a.js", "z.js"], &[], ReviewHintBasis::Candidates, "./lib");
    let mut reversed_targets = records[6].clone();
    reversed_targets.hint = hit(&["z.js", "a.js"], &[], ReviewHintBasis::Candidates, "./lib");
    records.push(reversed_targets);
    let input = collection(records.clone());
    let before = input.clone();
    let out = project(&input);
    records.extend(records.clone());
    records.reverse();
    assert_eq!(out, project(&collection(records)));
    assert_eq!(input, before);
    assert_eq!((out.located.len(), out.omitted_count), (8, 0));
    assert!(input
        .records
        .iter()
        .all(|r| out.located.iter().any(|n| &n.record == r)));
}

#[test]
fn display_order_uses_raw_components_and_go_package_candidates() {
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for key in [
        "file",
        "line",
        "callee",
        "targets",
        "owner",
        "owner_start",
        "owner_end",
        "start",
        "end",
    ] {
        let (mut first, mut second) = (ordinary(), ordinary());
        match key {
            "file" => {
                first.site.caller.file = "\n.js".into();
                second.site.caller.file = "\\n.js".into();
            }
            "line" => {
                first.line -= 1;
                first.site.caller.name = "z".into();
            }
            "callee" => {
                first.callee = "aaa".into();
                first.site.caller.name = "z".into();
            }
            "targets" => {
                first.hint = hit(
                    &["a.go", "z.go"],
                    &["z.go"],
                    ReviewHintBasis::Candidates,
                    "./lib",
                );
                second.hint = hit(
                    &["z.go", "a.go"],
                    &["a.go"],
                    ReviewHintBasis::Candidates,
                    "./lib",
                );
                first.language = Language::Go;
                second.language = Language::Go;
            }
            "owner" => first.site.caller.name = "a".into(),
            "owner_start" => second.site.caller.start_line += 1,
            "owner_end" => second.site.caller.end_line += 1,
            "start" => first.site.start_byte -= 1,
            "end" => first.site.end_byte -= 1,
            _ => unreachable!(),
        }
        let out = project(&collection(vec![second.clone(), first.clone()]));
        observed.push(
            out.located
                .into_iter()
                .map(|n| n.record)
                .collect::<Vec<_>>(),
        );
        expected.push(vec![first, second]);
    }
    eprintln!("ORDER observed={observed:?}");
    assert_eq!(observed, expected);
}

#[test]
fn availability_is_typed_separate_and_preserves_original_error() {
    let mut rows = Vec::new();
    for availability in [
        Availability::Available,
        Availability::ListingUnavailable("denied\n\u{202e}é".into()),
    ] {
        let input = BoundaryCollection {
            availability: availability.clone(),
            records: vec![],
        };
        let before = input.clone();
        let out = project(&input);
        assert_eq!(input, before);
        let summary = out.summary();
        rows.push((
            out.availability,
            out.located.len(),
            out.omitted_count,
            summary,
            availability,
        ));
    }
    assert!(
        rows.iter()
            .all(|(a, located, omitted, summary, original)| a == original
                && *located == 0
                && *omitted == 0
                && summary.is_none()),
        "{rows:?}"
    );
}
