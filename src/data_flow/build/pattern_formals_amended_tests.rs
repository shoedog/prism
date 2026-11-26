use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::data_flow::s1_evidence::{Disposition, ProjectionCause, Refusal};
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::BTreeMap;

#[test]
fn amended_projection_causes_overlap_and_credit_the_base_once() {
    use ProjectionCause::*;
    let source = "function C({o}, q, t = +q, ...r) {\n\
        observe(o, arguments);\n\
        with (q) { k = 1; }\n\
        q.k = 1;\n\
        o.left.k = 1;\n\
        return o.right.k;\n}\n";
    let parsed = ParsedFile::parse("s1.js", source, Language::JavaScript).unwrap();
    let evidence = DataFlowGraph::collect_s1_evidence(
        &BTreeMap::from([(parsed.path.clone(), parsed)]),
        "projection-cause-overlap",
    );
    let at = source.rfind("o.right.k").unwrap();
    let matches: Vec<_> = evidence.files["s1.js"]
        .attempts
        .iter()
        .filter(|attempt| attempt.key.operand == [at, at + 9])
        .collect();
    let [attempt] = matches.as_slice() else {
        panic!("expected one attempt, found {matches:?}");
    };
    assert_eq!(
        attempt.projection_causes,
        [
            BareOccurrence,
            Arguments,
            With,
            OtherRootStore,
            SameRootNonComparableStore,
            SignatureEffect,
            S2
        ]
    );
    assert_eq!(attempt.disposition, Disposition::Emitted);
    let endpoint = attempt.endpoint.as_ref().unwrap();
    assert_eq!(endpoint.bytes, [at, at + 1]);
    assert_eq!(endpoint.path.to_string(), "o");
    assert!(endpoint.label.is_some());
}

#[test]
fn amended_projection_member_targets_and_static_reads() {
    use ProjectionCause::*;
    for (formals, body, read, expected_path, cause) in [
        (
            "{o}, key",
            "observe(o[key]);",
            "o.k",
            "o",
            Some(BareOccurrence),
        ),
        ("{o}", "observe(o?.a, o['b']);", "o.k", "o.k", None),
        ("{o}", "observe(o?.a);", "o['b']", "o.b", None),
        ("{o}", "observe(o?.a);", r#"o["\u006b"]"#, "o.k", None),
        (
            "{o}, p = q, q",
            "observe(0);",
            "o.k",
            "o",
            Some(SignatureEffect),
        ),
        (
            "{o}, q",
            "[q.k] = values;",
            "o.k",
            "o",
            Some(OtherRootStore),
        ),
        (
            "{o}, q",
            "({a: q.k} = values);",
            "o.k",
            "o",
            Some(OtherRootStore),
        ),
        (
            "{o}, q",
            "for (q.k of values) {}",
            "o.k",
            "o",
            Some(OtherRootStore),
        ),
        (
            "{o}, q",
            "for (q.k in values) {}",
            "o.k",
            "o",
            Some(OtherRootStore),
        ),
        ("{o}, q", "q.k ||= 1;", "o.k", "o", Some(OtherRootStore)),
        ("{o}, q", "++q.k;", "o.k", "o", Some(OtherRootStore)),
        ("{o}, q", "delete q.k;", "o.k", "o", Some(OtherRootStore)),
        (
            "{o}, key",
            "o[key] = 1;",
            "o.k",
            "o",
            Some(SameRootNonComparableStore),
        ),
        (
            "{o}",
            "o['other'] = 1;",
            "o.k",
            "o",
            Some(SameRootNonComparableStore),
        ),
        (
            "before = +globalName, {o}",
            "observe(0);",
            "o.k",
            "o.k",
            None,
        ),
        (
            "{o}, later = [{get k() { return globalName; }}]",
            "observe(0);",
            "o.k",
            "o.k",
            None,
        ),
        (
            "{o}, later = [{q: globalName}]",
            "observe(0);",
            "o.k",
            "o",
            Some(SignatureEffect),
        ),
    ] {
        let source = format!("function C({formals}) {{\n  {body}\n  return {read};\n}}\n");
        let parsed = ParsedFile::parse("s1.js", &source, Language::JavaScript).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{source}");
        let evidence = DataFlowGraph::collect_s1_evidence(
            &BTreeMap::from([(parsed.path.clone(), parsed)]),
            "projection-targets",
        );
        let at = source.rfind(read).unwrap();
        let attempt = evidence.files["s1.js"]
            .attempts
            .iter()
            .find(|attempt| attempt.key.operand == [at, at + read.len()])
            .unwrap_or_else(|| panic!("missing attempt: {source}"));
        assert_eq!(attempt.projection_causes, cause.as_slice(), "{source}");
        assert_eq!(
            attempt.disposition,
            if cause.is_some() && cause != Some(BareOccurrence) {
                Disposition::Refused(Refusal::WriteEffect)
            } else {
                Disposition::Emitted
            },
            "{source}"
        );
        let endpoint = attempt
            .endpoint
            .as_ref()
            .unwrap_or_else(|| panic!("{source}: {attempt:?}"));
        assert_eq!(endpoint.path.to_string(), expected_path, "{source}");
        let end = if expected_path == "o" {
            at + 1
        } else {
            at + read.len()
        };
        assert_eq!(endpoint.bytes, [at, end], "{source}");
    }
}

fn assert_legacy_replay(live: FileRefs) {
    for source in [57, 64] {
        let mut edges = live.edges.iter().filter(|edge| {
            edge.from.start_byte == source && edge.to.line == 2 && edge.from.path.base == "x"
        });
        let edge = edges.next().expect("replayed edge");
        assert!(edges.next().is_none(), "extra replayed edge at {source}");
        assert_eq!((edge.to.start_byte, edge.to.end_byte), (31, 31));
        assert_eq!(
            live.labels[&(edge.from.clone(), edge.to.clone())],
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
        );
    }
    // Both cap configurations produce this whole-owner snapshot from the
    // 4138d8b9f4af producer compiled with the same runtime and test limits.
    assert_eq!(
        seam_digest(&live),
        "c91f6bca074452b76b0412ef6f0e7f1458b65b1dc7dd3b91512a0b9799cc7f62"
    );
}

const SEAM: &str = "function C({x=0, live}, f=()=>\nx) { if (x) return 1;\nvar x;\nvar a = x;\na.k = 1;\nreturn use(live); }\n";

fn seam_evidence(
    parsed: &ParsedFile,
    pair_cap: usize,
) -> crate::data_flow::s1_evidence::FileEvidence {
    use crate::call_graph::{c::macro_census::CMacroCensus, rust::macro_args::RustMacroCensus};
    Producer::file_s1_evidence(
        &parsed.path,
        parsed,
        &RustMacroCensus::of_file(parsed),
        &CMacroCensus::of_file(parsed),
        pair_cap,
    )
}

fn seam_digest(refs: &FileRefs) -> String {
    use sha2::{Digest, Sha256};
    let bytes = bincode::serialize(&(
        &refs.defs,
        &refs.uses,
        &refs.edges,
        &refs.labels,
        &refs.exact_labels,
        &refs.rd_function_stats,
    ))
    .unwrap();
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn amended_projection_seam_replays_the_def_cap() {
    use crate::cpg::{with_availability_limits, RD_MAX_LINES};
    use crate::data_flow::s1_evidence::CapKind;
    let parsed = ParsedFile::parse("s1.js", SEAM, Language::JavaScript).unwrap();
    let pair_cap = 1_000_000;
    let def_limit = 5;
    with_availability_limits(def_limit, RD_MAX_LINES, || {
        let receipt = seam_evidence(&parsed, pair_cap);
        assert_eq!(receipt.cap_withholdings.len(), 1);
        assert_eq!(receipt.cap_withholdings[0].kind, CapKind::Definitions);
        let cap = Some(pair_cap);
        let live = Producer::file_refs_with_cap(&parsed.path, &parsed, cap);
        assert_legacy_replay(live);
    });
}

#[test]
fn amended_projection_seam_replays_the_pair_cap() {
    use crate::cpg::{with_availability_limits, RD_MAX_DEFS};
    use crate::data_flow::s1_evidence::CapKind;
    let parsed = ParsedFile::parse("s1.js", SEAM, Language::JavaScript).unwrap();
    with_availability_limits(RD_MAX_DEFS, 1, || {
        let uncapped = Producer::file_refs_with_cap(&parsed.path, &parsed, Some(1_000_000));
        assert!(uncapped
            .edges
            .iter()
            .any(|edge| edge.from.start_byte == 17 && edge.to.path.base == "live"));
        assert!(uncapped
            .edges
            .iter()
            .any(|edge| edge.from.start_byte == 64 && edge.to.start_byte == 40));
        let limit = 4;
        let receipt = seam_evidence(&parsed, limit);
        assert_eq!(receipt.cap_withholdings.len(), 1);
        assert_eq!(receipt.cap_withholdings[0].kind, CapKind::Pairs);
        let live = Producer::file_refs_with_cap(&parsed.path, &parsed, Some(limit));
        assert_legacy_replay(live);
    });
}

#[test]
fn amended_projection_seam_requires_a_closure_on_the_mixed_line() {
    use crate::access_path::AccessPath;
    use crate::data_flow::{VarAccessKind, VarLocation};

    for (closure, can_relocate) in [("see(x)", true), ("see(y)", false)] {
        let source =
            format!("function C({{x=0}}, f=()=>\n{closure}) {{ if (x) return 1;\nvar x = 0;\n}}");
        let parsed = ParsedFile::parse("s1.js", &source, Language::JavaScript).unwrap();
        let owner = parsed.all_functions()[0];
        let body = owner.child_by_field_name("body").unwrap();
        let range = Some((body.start_byte(), body.end_byte()));
        let path = AccessPath::simple("x");
        let signature = source.find(closure).unwrap() + "see(".len();
        let use_at = |at| VarLocation {
            file: parsed.path.clone(),
            function: "C".into(),
            function_start_line: 1,
            line: 2,
            path: path.clone(),
            start_byte: at,
            end_byte: at + 1,
            kind: VarAccessKind::Use,
        };
        for preferred in [None, Some(use_at(signature))] {
            assert_eq!(
                super::pattern_seam::relocate(&parsed, owner, &path, 2, preferred.as_ref(), range),
                can_relocate,
                "{source}"
            );
        }
        let in_body = use_at(source.find("if (x)").unwrap() + "if (".len());
        assert!(!super::pattern_seam::relocate(
            &parsed,
            owner,
            &path,
            2,
            Some(&in_body),
            range
        ));
    }
}
