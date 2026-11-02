//! U/D decisions and telemetry across R's counting, replay and capped exits.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::RdFileStats;
use crate::languages::Language;

fn prune_counts(s: &RdFileStats) -> [usize; 4] {
    [
        s.dfg_pruned_write_only,
        s.dfg_pruned_scope,
        s.dfg_pruned_dominated,
        s.dfg_pruned_order,
    ]
}

fn bytes(parts: FileRefs) -> Vec<u8> {
    bincode::serialize(
        &crate::data_flow::compact::CompactDfg::from_parts(parts.into_parts()).unwrap(),
    )
    .unwrap()
}

#[test]
fn prune_cap_count_and_replay_preserve_uses_and_count_prunes_once() {
    // Parameter/raw-def and alias-resolved-def paths, including real-span and
    // line-anchor uses. Padding forces RD refusal without adding edge candidates.
    for body in [
        "func f(x int) {\nx = 1\nconsume(x)\nx = 2\nconsume(x)\n",
        "type S struct { f int }\nfunc f(s *S, x int) {\nptr := s\ns.f = 1\nptr.f = 2\nuse(s.f)\nx = 1\nx = 2\nconsume(x)\n",
    ] {
        let source = format!("package p\n{body}{}}}\n", "g()\n".repeat(4100));
        let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
        let file = parsed.path.clone();
        let before = Producer::file_refs_frozen_with_cap(&file, &parsed, None);
        let single = Producer::file_refs_with_cap(&file, &parsed, None);
        let expected_prunes = prune_counts(&single.rd_function_stats);
        assert_eq!(single.rd_function_stats.functions_over_cap, 1);
        assert!(expected_prunes[0] > 0 && expected_prunes[2] > 0, "{body}: {expected_prunes:?}");
        let count = single.edges.len();
        assert!(count > 0 && count < before.edges.len());
        assert_eq!(
            before.edges.len() - count,
            expected_prunes.iter().sum::<usize>()
        );
        println!("prune-cap: raw={} surviving={count} pruned={expected_prunes:?}", before.edges.len());
        // Pruned candidates must not push an owner over the cap. Exactly at
        // the surviving raw count and above it, replay is byte-identical.
        for cap in [count, count + 1] {
            let replay = Producer::file_refs_with_cap(&file, &parsed, Some(cap));
            assert_eq!(replay.defs, single.defs);
            assert_eq!(replay.uses, single.uses);
            assert_eq!(replay.edges, single.edges);
            assert_eq!(replay.labels, single.labels);
            assert_eq!(replay.exact_labels, single.exact_labels);
            assert_eq!(replay.rd_function_stats, single.rd_function_stats);
            assert_eq!(
                bytes(replay),
                bytes(Producer::file_refs_with_cap(&file, &parsed, None))
            );
        }
        // Capped owners retain the counting pass's U/D telemetry once; the cap
        // counter contains only surviving candidates, including duplicates.
        let capped = Producer::file_refs_with_cap(&file, &parsed, Some(count - 1));
        assert!(capped.edges.is_empty());
        assert!(capped.labels.is_empty() && capped.exact_labels.is_empty());
        assert_eq!(capped.defs, single.defs);
        assert_eq!(capped.uses, single.uses);
        assert_eq!(prune_counts(&capped.rd_function_stats), expected_prunes);
        assert_eq!(capped.rd_function_stats.fallback_functions_capped, 1);
        assert_eq!(capped.rd_function_stats.fallback_edges_capped, count);
        let decoded: crate::data_flow::compact::CompactDfg =
            bincode::deserialize(&bytes(capped)).unwrap();
        let stats = &decoded.rd_function_stats()[&file];
        assert_eq!(prune_counts(stats), expected_prunes);
        assert_eq!(stats.fallback_edges_capped, count);
        let json = serde_json::to_value(stats).unwrap();
        let restored: RdFileStats = serde_json::from_value(json).unwrap();
        assert_eq!(&restored, stats);
    }
}

#[test]
fn prune_cap_available_owner_counts_once_and_ignores_zero_cap() {
    let parsed = ParsedFile::parse(
        "a.go",
        "package p\nfunc f(x int) int {\nx = 1\nx = 2\nreturn x\n}\n",
        Language::Go,
    )
    .unwrap();
    let single = Producer::file_refs_with_cap(&parsed.path, &parsed, None);
    assert_eq!(single.rd_function_stats.functions_over_cap, 0);
    assert_eq!(single.rd_function_stats.functions_without_cfg, 0);
    assert!(single.rd_function_stats.dfg_pruned_write_only > 0);
    assert!(single.rd_function_stats.dfg_pruned_dominated > 0);
    let actual = Producer::file_refs_with_cap(&parsed.path, &parsed, Some(0));
    assert_eq!(bytes(actual), bytes(single));
}

#[test]
fn prune_cap_alias_twin_write_only_and_dominated_pairs_are_pruned_in_both_passes() {
    let source = format!(
        "package p\ntype S struct {{ f int }}\nfunc f(s *S) {{\nptr := s\nptr.f = 1\ns.f = 2\nuse(s.f)\n{}}}\n",
        "g()\n".repeat(4100)
    );
    let parsed = ParsedFile::parse("alias.go", &source, Language::Go).unwrap();
    let before = Producer::file_refs_frozen_with_cap(&parsed.path, &parsed, None);
    let has_twin = |parts: &FileRefs, use_line| {
        parts
            .edges
            .iter()
            .any(|e| e.from.path.to_string() == "s.f" && e.from.line == 5 && e.to.line == use_line)
    };
    assert!(has_twin(&before, 6), "baseline alias twin to plain write");
    assert!(has_twin(&before, 7), "baseline alias twin to later read");
    let single = Producer::file_refs_with_cap(&parsed.path, &parsed, None);
    assert_eq!(single.rd_function_stats.functions_over_cap, 1);
    let count = single.edges.len();
    assert!(count > 0);
    for cap in [None, Some(count)] {
        let actual = Producer::file_refs_with_cap(&parsed.path, &parsed, cap);
        assert!(
            !has_twin(&actual, 6),
            "U must prune the resolved-path write anchor"
        );
        assert!(
            !has_twin(&actual, 7),
            "D must prune the overwritten alias twin"
        );
        assert_eq!(actual.defs, single.defs);
        assert_eq!(actual.uses, single.uses);
        assert_eq!(actual.rd_function_stats, single.rd_function_stats);
        assert_eq!(
            bytes(actual),
            bytes(Producer::file_refs_with_cap(&parsed.path, &parsed, None))
        );
    }
}
