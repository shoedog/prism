//! #660 S1b-1 round 1: the write census never changes availability, the def cap, cap exemption or any
//! counter. Census-caused unavailability is a label refusal; the owner keeps main's edges and counters.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

fn refs(source: &str, language: Language, cap: Option<usize>) -> FileRefs {
    let file = match language {
        Language::TypeScript => "a.ts",
        _ => "a.js",
    };
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    Producer::file_refs_with_cap(&file.to_string(), &parsed, cap)
}

fn all_cfg_incomplete(r: &FileRefs) -> bool {
    !r.labels.is_empty()
        && r.labels
            .values()
            .all(|l| *l == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))
}

/// One-node owner whose only census write is `delete o.x`: main admits it (one distinct base).
#[test]
fn a_census_only_one_node_flip_moves_no_counter_and_no_edge() {
    let src = "const f = (o) => (delete o.x, use(o));\n";
    let r = refs(src, Language::JavaScript, None);
    assert!(!r.edges.is_empty(), "the edge is still there");
    assert_eq!(
        r.rd_function_stats.functions_without_cfg, 0,
        "main's availability"
    );
    assert_eq!(r.rd_function_stats.fallback_functions_capped, 0);
    assert!(all_cfg_incomplete(&r), "{:?}", r.labels);
}

/// The nodeFactory.ts shape: a one-node setter writing through a `this` member behind `!`.
#[test]
fn a_one_node_setter_with_a_wrapped_this_member_write_stays_available() {
    let src = "class C {\n  set x(this: S, value: number) { this.a!.b = value; }\n}\n";
    let r = refs(src, Language::TypeScript, None);
    assert_eq!(r.rd_function_stats.functions_without_cfg, 0);
    assert_eq!(r.rd_function_stats.fallback_functions_capped, 0);
    assert!(all_cfg_incomplete(&r), "{:?}", r.labels);
}

/// The completed list overflows the 2,048 def cap although the recorded list fits: main runs RD and
/// exempts the owner from the fallback cap, so a tiny cap must not drop its edges.
#[test]
fn a_census_only_def_cap_overflow_keeps_the_edges_and_the_cap_exemption() {
    let mut src = String::from("function f(x) {\n  x = 0;\n");
    for _ in 0..2050 {
        src.push_str("  x++;\n");
    }
    for _ in 0..4 {
        src.push_str("  sink(x);\n");
    }
    src.push_str("}\n");
    let r = refs(&src, Language::JavaScript, Some(5));
    assert!(!r.edges.is_empty(), "edges kept");
    assert_eq!(r.rd_function_stats.fallback_functions_capped, 0);
    assert_eq!(r.rd_function_stats.functions_over_cap, 0);
    assert!(all_cfg_incomplete(&r), "{:?}", r.labels);
}

#[test]
fn pattern_increment_replays_legacy_at_the_completed_def_cap() {
    for (padding, keep_entry, safe_label) in [
        (2046, false, FlowConfidence::Exact),
        (2045, true, FlowConfidence::Exact),
        (0, true, FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
    ] {
        let mut source = String::from("function C({ p }, safe) {\n");
        for index in 0..padding {
            source.push_str(&format!("  const local{index} = 0;\n"));
        }
        source.push_str("  return report(++p, safe, p);\n}\n");
        let cap = (padding != 0).then_some(1_000_000);
        let graph = refs(&source, Language::JavaScript, cap);
        let safe_labels = |graph: &FileRefs| {
            graph
                .labels
                .iter()
                .filter(|((from, to), _)| from.var_name() == "safe" && to.var_name() == "safe")
                .map(|(_, &label)| label)
                .collect::<Vec<_>>()
        };
        if padding != 0 {
            let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
            let legacy = Producer::file_refs_frozen_with_cap(&parsed.path, &parsed, cap);
            assert_eq!(
                safe_labels(&legacy),
                [FlowConfidence::Exact],
                "legacy/{padding}"
            );
        }
        assert_eq!(safe_labels(&graph), [safe_label], "increment/{padding}");
        assert_eq!(
            graph
                .defs
                .values()
                .flatten()
                .any(|def| def.start_byte == 13),
            keep_entry,
            "entry/{padding}",
        );
        let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
        let receipt = crate::data_flow::DataFlowGraph::collect_s1_evidence(
            &std::collections::BTreeMap::from([(parsed.path.clone(), parsed)]),
            "completed-def-cap-fixture",
        );
        let counters = receipt.counters();
        assert_eq!(
            counters["refused_s1_rd_cap"],
            u64::from(!keep_entry),
            "{padding}"
        );
        assert_eq!(counters["cap-withheld"] > 0, !keep_entry, "{padding}");
    }
}
