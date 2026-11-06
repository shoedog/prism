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
