//! F11-F14 native producer pairs; the original-anchor subrow explicitly controls raw emission coordinates.
use super::*;
use prism::finding_confidence::{EvidenceHop, EvidencePath};
use std::collections::BTreeSet;

fn sink(result: &SliceResult, file: &str) -> usize {
    result
        .findings
        .iter()
        .position(|f| f.file == file && f.category.as_deref() == Some("taint_sink"))
        .expect("native selected context sink")
}
fn coordinates(evidence: &EvidencePath) -> Vec<(String, usize)> {
    let mut result = Vec::new();
    for hop in &evidence.hops {
        match hop {
            EvidenceHop::DataFlow { from, to, .. } => {
                result.extend([(from.file.clone(), from.line), (to.file.clone(), to.line)])
            }
            EvidenceHop::Call { edge, .. } => {
                result.push((edge.caller.file.clone(), edge.call_site_line))
            }
            _ => {}
        }
    }
    result
}

/// CONTROLLED raw anchor/sidecar coordinates over native maps and an actual producer witness.
/// This exercises U's original-anchor branch, not a native return-flow route or a fabricated hop.
pub(super) fn controlled_original_anchor(
    f: &Fixture,
    mut sample: Sample,
    context_file: &str,
    original: &str,
    line: usize,
    retain_former_anchor: bool,
) -> (Sample, crate::algorithms::taint::result_union::UnionStats) {
    use crate::algorithms::taint::result_union::{union_results, UnionInput};
    let native = f.native();
    let analysis = native
        .bundle
        .analysis
        .as_ref()
        .expect("actual driver analysis");
    let mut expanded = analysis.result.clone();
    let mut attribution = analysis.emission.clone();
    let index = sink(&expanded, context_file);
    if retain_former_anchor {
        let actual = expanded.findings[index].file.clone();
        assert!(native.bundle.context_files.contains(&actual));
        if !expanded.findings[index].related_files.contains(&actual) {
            expanded.findings[index].related_files.push(actual);
        }
    }
    expanded.findings[index].file = original.into();
    expanded.findings[index].line = line;
    let emission = attribution
        .sinks
        .iter_mut()
        .find(|s| s.finding_index == index)
        .unwrap();
    emission.file = original.into();
    emission.line = line;
    let originals = f.inputs.files.keys().cloned().collect();
    let baseline = SliceResult::new(SlicingAlgorithm::Taint);
    let mut union = union_results(UnionInput {
        baseline: &baseline,
        expanded: &expanded,
        attribution: &attribution,
        original_files: &originals,
        context_files: &native.bundle.context_files,
    })
    .unwrap();
    crate::algorithms::finalize_diagrams(
        &mut union.result,
        SliceConfig::default().diagram_node_cap,
    );
    let stats = union.stats;
    sample.run.findings = union.result.findings.clone();
    sample.run.evidence = union.result.evidence.clone();
    sample.run.results = vec![union.result];
    (sample, stats)
}

#[test]
fn f11_context_and_controlled_original_anchor_wording() {
    let f = Fixture::js(B);
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let finding = &on.run.results[0].findings[sink(&on.run.results[0], "b.js")];
    assert!(finding
        .description
        .contains("(sink file outside the analysed file set; loaded as taint context)"));
    assert!(!finding.description.contains("outside the diff"));
    let doc = documents(&f, &on);
    assert_eq!(
        context_sarif(&doc.sarif, "b.js")["message"]["text"],
        finding.description
    );
    let (controlled, _) = controlled_original_anchor(&f, on, "b.js", "main.js", 3, false);
    let finding = controlled
        .run
        .findings
        .iter()
        .find(|f| f.file == "main.js" && f.line == 3 && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    assert!(finding
        .description
        .contains("(witness passes through taint context:"));
    assert!(!finding.description.contains("sink file outside"));
    assert!(!finding.description.contains("outside the diff"));
    let doc = documents(&f, &controlled);
    assert!(doc.json.value["all_findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["description"] == finding.description));
    assert!(results(&doc.sarif)
        .iter()
        .any(|v| v["message"]["text"] == finding.description));
}

#[test]
fn f12_related_files_follow_selected_actual_coordinates() {
    let f = Fixture::js(B);
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let native = f.native();
    let analysis = native.bundle.analysis.as_ref().unwrap();
    let index = sink(&on.run.results[0], "b.js");
    let finding = &on.run.results[0].findings[index];
    let evidence = on.run.results[0].evidence[index]
        .as_ref()
        .expect("selected sink witness");
    assert!(!evidence.hops.is_empty());
    let emission = analysis
        .emission
        .sinks
        .iter()
        .find(|s| {
            s.file == finding.file
                && s.line == finding.line
                && Some(s.category.as_str()) == finding.category.as_deref()
        })
        .unwrap();
    let mut seen = BTreeSet::new();
    let expected = emission
        .selected_source
        .iter()
        .map(|s| s.0.clone())
        .chain(coordinates(evidence).into_iter().map(|p| p.0))
        .filter(|file| {
            file != &finding.file
                && (f.inputs.files.contains_key(file) || native.bundle.context_files.contains(file))
                && seen.insert(file.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(finding.related_files, expected);
    assert!(!expected.is_empty());
    let doc = documents(&f, &on);
    assert_eq!(
        context_sarif(&doc.sarif, "b.js")["properties"]["related_files"],
        serde_json::to_value(&expected).unwrap()
    );
    assert!(doc.json.value["all_findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["file"] == "b.js"
            && v["related_files"] == serde_json::to_value(&expected).unwrap()));
}

#[test]
fn f13_new_source_backlink_and_evidence_are_licensed_pairs() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let native = f.native();
    let analysis = native.bundle.analysis.as_ref().unwrap();
    let baseline: BTreeSet<_> = off.run.results[0]
        .findings
        .iter()
        .map(|f| (f.category.clone(), f.file.clone(), f.line))
        .collect();
    let included = analysis
        .emission
        .sinks
        .iter()
        .filter(|s| {
            !baseline.contains(&(Some(s.category.clone()), s.file.clone(), s.line))
                && on.run.findings.iter().any(|f| {
                    f.file == s.file
                        && f.line == s.line
                        && f.category.as_deref() == Some(s.category.as_str())
                })
        })
        .collect::<Vec<_>>();
    let sources = on.run.results[0]
        .findings
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.category.as_deref() == Some("taint_source")
                && !baseline.contains(&(f.category.clone(), f.file.clone(), f.line))
        })
        .collect::<Vec<_>>();
    assert!(
        !sources.is_empty(),
        "native union must expose a newly licensed source backlink"
    );
    for (index, source) in sources {
        let selected = on.run.results[0].evidence[index].as_ref().unwrap();
        let licenses = included
            .iter()
            .flat_map(|sink| sink.licensed.iter().map(move |license| (*sink, license)))
            .filter(|(_, l)| l.file == source.file && l.line == source.line)
            .collect::<Vec<_>>();
        assert!(!licenses.is_empty());
        assert!(
            licenses
                .iter()
                .any(|(_, l)| format!("{:?}", l.evidence) == format!("{selected:?}")),
            "selected source evidence belongs to an included sink's producer pair"
        );
        let mut expected = Vec::new();
        for (sink, _) in licenses {
            if sink.file != source.file && !expected.contains(&sink.file) {
                expected.push(sink.file.clone());
            }
        }
        assert_eq!(source.related_files, expected);
        assert!(expected.contains(&"b.js".to_string()));
    }
}

fn check_blocks(result: &SliceResult, sources: &BTreeMap<String, String>, slices: &Value) {
    let slices = slices.as_array().unwrap();
    assert_eq!(slices.len(), result.blocks.len());
    for block in &result.blocks {
        let output = slices
            .iter()
            .find(|s| s["block_id"] == block.block_id)
            .unwrap();
        let mut lines = Vec::new();
        for (file, map) in &block.file_line_map {
            let source = sources
                .get(file)
                .expect("all selected block files have source");
            for (line, diff) in map {
                lines.push(*line);
                let text = source.lines().nth(line.saturating_sub(1)).unwrap_or("");
                assert!(
                    output["slice_text"].as_str().unwrap().contains(&format!(
                        "{}{:>4}|{}",
                        if *diff { "+" } else { " " },
                        line,
                        text
                    )),
                    "block source identity for {file}:{line}"
                );
            }
        }
        assert_eq!(output["slice_lines"], serde_json::to_value(lines).unwrap());
    }
}

#[test]
fn f14_all_block_text_and_unused_ids_preserve_baseline() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let actual = match &on.run.context {
        crate::api::context_run::ContextState::Expanded(c) => c,
        _ => unreachable!(),
    };
    let old = documents(&f, &off);
    let new = documents(&f, &on);
    check_blocks(
        &on.run.results[0],
        &actual.sources,
        &new.json.value["results"][0]["slices"],
    );
    let baseline = old.json.value["results"][0]["slices"].as_array().unwrap();
    let rendered = new.json.value["results"][0]["slices"].as_array().unwrap();
    assert_eq!(
        bytes(&on.run.results[0].blocks[..off.run.results[0].blocks.len()]),
        bytes(&off.run.results[0].blocks)
    );
    for (before, after) in baseline.iter().zip(rendered) {
        let mut original = before.clone();
        let mut current = after.clone();
        for value in [&mut original, &mut current] {
            value.as_object_mut().unwrap().remove("source_lines");
            value.as_object_mut().unwrap().remove("sink_lines");
        }
        assert_eq!(
            original, current,
            "all persistent rendered baseline fields stay identical"
        );
        assert_eq!(before["source_lines"], serde_json::json!([2]));
        assert_eq!(after["source_lines"], serde_json::json!([2, 3]));
        assert_eq!(before["sink_lines"], serde_json::json!([2]));
        assert_eq!(after["sink_lines"], before["sink_lines"]);
    }
    assert!(on.run.findings.iter().any(|f| f.file == "main.js"
        && f.line == 3
        && f.category.as_deref() == Some("taint_source")
        && f.related_files.contains(&"b.js".into())));
    let producer = f.native();
    assert!(
        producer
            .bundle
            .analysis
            .as_ref()
            .unwrap()
            .emission
            .sinks
            .iter()
            .any(|s| s.file == "b.js"
                && s.licensed
                    .iter()
                    .any(|l| l.file == "main.js" && l.line == 3)),
        "the added original-file annotation has an actual included sink license"
    );
    let old_ids = off.run.results[0]
        .blocks
        .iter()
        .map(|b| b.block_id)
        .collect::<BTreeSet<_>>();
    let maximum = *old_ids.iter().max().unwrap();
    let mut added = BTreeSet::new();
    for block in &on.run.results[0].blocks[off.run.results[0].blocks.len()..] {
        assert!(block.block_id > maximum);
        assert!(!old_ids.contains(&block.block_id));
        assert!(added.insert(block.block_id));
    }
    assert!(!added.is_empty());
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, RenderSettings::default());
    check_blocks(
        &one.run.outcome.result,
        &one.run.context().unwrap().sources,
        &single.json.value["slices"],
    );
}
