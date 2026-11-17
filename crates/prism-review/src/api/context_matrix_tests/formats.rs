//! F01-F10 REAL captured engine results through library formatters, single and multi.
use super::*;

fn taint() -> [SlicingAlgorithm; 1] {
    [SlicingAlgorithm::Taint]
}
fn documents(f: &Fixture, sample: &Sample) -> Documents {
    Documents::multi(sample, &f.inputs, RenderSettings::default())
}
fn results(document: &Document) -> &[Value] {
    document.value["runs"][0]["results"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn context_sarif<'a>(doc: &'a Document, file: &str) -> &'a Value {
    results(doc)
        .iter()
        .find(|r| {
            r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == file
                && r["properties"]["category"] == "taint_sink"
        })
        .expect("serialized context sink")
}
fn context_target<'a>(doc: &'a Document, file: &str) -> &'a Value {
    doc.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["site"]["file"] == file && t["category"] == "taint_sink")
        .expect("projected context sink target")
}
fn assert_quality(value: &Value, expected: &BTreeMap<String, prism::slice::FileParseQuality>) {
    if expected.is_empty() {
        assert!(
            value.get("parse_quality").is_none(),
            "empty sparse map remains omitted"
        );
    } else {
        assert_eq!(
            value["parse_quality"],
            serde_json::to_value(expected).unwrap()
        );
    }
}

#[test]
fn f01_off_default_and_prepared_none_all_formats() {
    let f = Fixture::js(B);
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    let baseline = f.off(&algorithms);
    assert!(
        !baseline.run.findings.is_empty(),
        "native original finding is a behavioral nonvacuity oracle"
    );
    let default = f.run(&algorithms, &AlgorithmParams::default());
    let mut off = AlgorithmParams::default();
    off.taint_context.depth = 4;
    off.taint_context.max_files = 1;
    let explicit = f.run(&algorithms, &off);
    assert_eq!(
        documents(&f, &baseline).bytes(),
        documents(&f, &default).bytes()
    );
    assert_eq!(
        documents(&f, &baseline).bytes(),
        documents(&f, &explicit).bytes()
    );
    for ((a, b), c) in baseline
        .run
        .results
        .iter()
        .zip(&default.run.results)
        .zip(&explicit.run.results)
    {
        same_result(a, b);
        same_result(a, c);
    }
    let default = f.single(&AlgorithmParams::default());
    let explicit = f.single(&off);
    assert_eq!(
        Documents::single(&default, &f.inputs, RenderSettings::default()).bytes(),
        Documents::single(&explicit, &f.inputs, RenderSettings::default()).bytes()
    );
    same_result(&default.run.outcome.result, &explicit.run.outcome.result);
}

#[test]
fn f02_baseline_pairs_and_blocks_remain_prefixes() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let single_off = f.single(&AlgorithmParams::default());
    let single_on = f.single(&imports());
    for (old, new) in [
        (&off.run.results[0], &on.run.results[0]),
        (
            &single_off.run.outcome.result,
            &single_on.run.outcome.result,
        ),
    ] {
        assert!(!old.findings.is_empty(), "baseline pair oracle is nonempty");
        assert_eq!(
            bytes(&new.findings[..old.findings.len()]),
            bytes(&old.findings)
        );
        assert_eq!(
            format!("{:?}", &new.evidence[..old.evidence.len()]),
            format!("{:?}", old.evidence)
        );
        assert_eq!(bytes(&new.blocks[..old.blocks.len()]), bytes(&old.blocks));
        assert_eq!(new.algorithm, old.algorithm);
        assert_eq!(new.warnings, old.warnings);
    }
}

#[test]
fn f03_native_membrane_counterfactual_and_non_taint_parity() {
    let f =
        Fixture::js("import { main } from './main';\nexport function g(x) { eval(x); main(x); }\n");
    let native = f.native();
    let built = build_context(&f.inputs, &f.options).unwrap();
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = prism::api::ReviewContextRound::new(&loader, &native.bundle.files);
    let full = round.build_context(f.inputs.type_db.as_ref(), &built.ctx.types);
    let original = crate::algorithms::membrane_slice::slice(&built.ctx, &f.inputs.diff).unwrap();
    let leaked = crate::algorithms::membrane_slice::slice(&full, &f.inputs.diff).unwrap();
    assert_ne!(
        bytes(&original),
        bytes(&leaked),
        "native context must alter Membrane if leaked; behavioral counterfactual"
    );
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    let off = f.off(&algorithms);
    let on = f.on(&algorithms);
    assert_expanded(&on, "b.js");
    let old = documents(&f, &off);
    let new = documents(&f, &on);
    for i in [1, 2] {
        same_result(&off.run.results[i], &on.run.results[i]);
        assert_eq!(old.json.value["results"][i], new.json.value["results"][i]);
        assert_eq!(
            old.review.value["results"][i],
            new.review.value["results"][i]
        );
    }
    // Echo is a parity row, not a claim of native Echo sensitivity.
}

#[test]
fn f04_json_source_sparse_quality_and_warning_channels() {
    let f = Fixture::js(DEGRADED);
    assert!(
        !f.inputs.files.contains_key("b.js"),
        "inadmissible precondition: context is not original"
    );
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let context = match &on.run.context {
        super::super::context_run::ContextState::Expanded(c) => c,
        _ => unreachable!(),
    };
    let expected = BTreeMap::from([("b.js".into(), context.parse_quality["b.js"].clone())]);
    let multi = documents(&f, &on);
    assert_quality(&multi.json.value, &f.inputs.parse_quality);
    assert_quality(&multi.json.value["results"][0], &expected);
    let slice = multi.json.value["results"][0]["slices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["file"] == "b.js")
        .expect("json context block");
    assert!(slice["slice_text"]
        .as_str()
        .unwrap()
        .contains("export function g(x) { eval(x); }"));
    let notes = on.run.diagnostics.as_ref().unwrap().messages();
    for note in &notes {
        assert_eq!(
            multi.json.value["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v.as_str() == Some(note))
                .count(),
            1
        );
    }
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, RenderSettings::default());
    assert_quality(&single.json.value, &expected);
    for note in one.run.outcome.diagnostics.as_ref().unwrap().messages() {
        assert_eq!(
            single.json.value["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v.as_str() == Some(note.as_str()))
                .count(),
            1
        );
    }
    assert!(!f.inputs.parse_quality.contains_key("b.js"));
}

#[test]
fn f05_compact_review_floor_counts_and_diagram_stripping() {
    let f = Fixture::js(DEGRADED);
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let normal = documents(&f, &on);
    assert!(normal.review.value["results"][0]["slices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["file"] == "b.js" && s["slice_text"].as_str().unwrap().contains("eval(x)")));
    let settings = RenderSettings {
        min_rank: output::severity_rank("error"),
        full_slices: false,
        no_diagrams: true,
    };
    let hidden = Documents::multi(&on, &f.inputs, settings);
    assert!(
        hidden.review.value["findings_below_floor"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(hidden.review.value["results"][0]["slices"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(hidden.review.value["all_findings"]
        .as_array()
        .unwrap()
        .is_empty());
    let full = Documents::multi(
        &on,
        &f.inputs,
        RenderSettings {
            full_slices: true,
            ..settings
        },
    );
    let all = full.review.value["results"][0]["slices"]
        .as_array()
        .unwrap();
    let retained = normal.review.value["results"][0]["slices"]
        .as_array()
        .unwrap();
    assert_eq!(all.len(), on.run.results[0].blocks.len());
    for (rendered, raw) in all.iter().zip(&on.run.results[0].blocks) {
        assert_eq!(rendered["block_id"], raw.block_id);
        assert_eq!(rendered["file"], raw.file);
        let json = normal.json.value["results"][0]["slices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["block_id"] == raw.block_id)
            .unwrap();
        assert_eq!(rendered["slice_text"], json["slice_text"]);
    }
    assert!(retained.iter().all(|slice| all.contains(slice)));
    assert!(
        all.iter().any(|s| s["file"] == "a.js"),
        "full slices retain the actual witness-only a.js block"
    );
    assert!(!retained.iter().any(|s| s["file"] == "a.js"));
    assert!(retained.len() < all.len());
    assert!(full.review.value["results"][0].get("diagrams").is_none());
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, settings);
    assert!(
        single.review.value["findings_below_floor"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(single.review.value.get("diagrams").is_none());
    assert_eq!(
        single.review.value["parse_quality"],
        normal.review.value["results"][0]["parse_quality"]
    );
}

#[test]
fn f06_sarif_context_line_grade_related_and_notes() {
    let f = Fixture::js(B);
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let d = documents(&f, &on);
    let r = context_sarif(&d.sarif, "b.js");
    assert_eq!(r["properties"]["parse_quality"], "clean");
    assert_eq!(r["properties"]["resolution_mode"], "scoped");
    let finding = on
        .run
        .findings
        .iter()
        .find(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    assert_eq!(
        r["partialFingerprints"]["prism/finding/v1"],
        output::sarif::fingerprint(finding, B.lines().nth(finding.line - 1).unwrap())
    );
    assert_ne!(
        r["partialFingerprints"]["prism/finding/v1"],
        output::sarif::fingerprint(finding, "")
    );
    assert!(!r["relatedLocations"].as_array().unwrap().is_empty());
    for note in on.run.diagnostics.as_ref().unwrap().located() {
        assert_eq!(
            notices(&d.sarif.value)
                .iter()
                .filter(|n| n["message"]["text"] == note.message())
                .count(),
            1
        );
    }
    let dirty = Fixture::js(DEGRADED);
    let sample = dirty.on(&taint());
    assert_expanded(&sample, "b.js");
    let doc = documents(&dirty, &sample);
    let run_notes = sample.run.diagnostics.as_ref().unwrap().run_notes();
    let crate::api::context_run::ContextState::Expanded(context) = &sample.run.context else {
        unreachable!()
    };
    let quality = &context.parse_quality["b.js"];
    assert_eq!(quality.quality, "degraded");
    assert!(quality.error_rate > 0.01 && quality.error_rate <= 0.1);
    assert_eq!(context.parse_warnings, dirty.inputs.parse_warnings);
    assert!(run_notes.iter().all(|n| n.kind()
        != crate::algorithms::taint::boundary::records::ContextRunNoteKind::ContextParse));
    let raw = &sample.run.results[0];
    let index = raw
        .findings
        .iter()
        .position(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
        .unwrap();
    let evidence = raw.evidence[index].as_ref().unwrap();
    use prism::finding_confidence::{
        classify_with_evidence, FindingConfidence, FindingTier, ParseQuality,
    };
    assert_eq!(
        classify_with_evidence(
            "taint",
            ParseQuality::from_quality_str(&quality.quality),
            evidence
        ),
        (FindingConfidence::NameOnly, FindingTier::Candidate)
    );
    let graded = context_sarif(&doc.sarif, "b.js");
    assert_eq!(graded["properties"]["parse_quality"], "degraded");
    assert_eq!(graded["properties"]["confidence"], "nameonly");
    assert_eq!(graded["properties"]["tier"], "candidate");
    // REAL positive run-note case is the separately observed accepted depth-one bound.
    let mut bound = imports();
    bound.taint_context.depth = 1;
    let bounded = dirty.run(&taint(), &bound);
    assert_depth_bound_once(&bounded, &documents(&dirty, &bounded));
    assert_eq!(
        doc.sarif.value["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, RenderSettings::default());
    assert_eq!(
        context_sarif(&single.sarif, "b.js")["partialFingerprints"],
        r["partialFingerprints"]
    );
}

#[test]
fn f07_text_context_lines_and_original_prefix() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let old = documents(&f, &off);
    let new = documents(&f, &on);
    assert!(
        !old.text.is_empty(),
        "nonempty baseline rendered text is behavioral"
    );
    assert!(new.text.starts_with(&old.text));
    assert!(new.text.contains(B.trim()));
    let one = f.single(&imports());
    assert!(
        Documents::single(&one, &f.inputs, RenderSettings::default())
            .text
            .contains(B.trim())
    );
}

#[test]
fn f08_paper_is_block_data_and_keeps_prefix() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let old = documents(&f, &off);
    let new = documents(&f, &on);
    let prefix = old.paper.value.as_array().unwrap();
    let all = new.paper.value.as_array().unwrap();
    assert!(!prefix.is_empty());
    assert_eq!(&all[..prefix.len()], prefix);
    assert!(all.iter().any(|b| b["file"] == "b.js"));
    let one = f.single(&imports());
    assert!(
        Documents::single(&one, &f.inputs, RenderSettings::default())
            .paper
            .value
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["file"] == "b.js")
    );
}

#[test]
fn f09_mermaid_baseline_graphs_and_warning_counts() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let old = &off.run.results[0];
    let new = &on.run.results[0];
    assert!(
        !old.diagram_warnings.is_empty(),
        "long native baseline diagram label must exercise warnings"
    );
    assert_eq!(
        bytes(&new.diagram_warnings[..old.diagram_warnings.len()]),
        bytes(&old.diagram_warnings)
    );
    let d = documents(&f, &on);
    let graphs = old
        .diagrams
        .iter()
        .chain(old.findings.iter().flat_map(|f| f.diagrams.iter()))
        .collect::<Vec<_>>();
    assert!(!graphs.is_empty());
    for graph in graphs {
        assert!(!graph.mermaid.is_empty());
        assert!(d.mermaid.contains(&graph.mermaid));
    }
    let mut expected = BTreeMap::new();
    for warning in &new.diagram_warnings {
        let line = format!(
            "- {}/{}: {:?}: {}",
            warning.algorithm,
            warning.graph_title.as_deref().unwrap_or("(no title)"),
            warning.kind,
            warning.detail
        );
        *expected.entry(line).or_insert(0usize) += 1;
    }
    for (line, count) in expected {
        assert_eq!(
            d.mermaid.lines().filter(|actual| *actual == line).count(),
            count,
            "one rendered warning per actual warning"
        );
    }
    assert_eq!(
        d.json.value["diagram_warnings"].as_array().unwrap().len(),
        new.diagram_warnings.len()
    );
    assert_eq!(
        d.review.value["diagram_warnings"].as_array().unwrap().len(),
        new.diagram_warnings.len()
    );
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, RenderSettings::default());
    assert_eq!(
        single.json.value["diagram_warnings"]
            .as_array()
            .unwrap()
            .len(),
        one.run.outcome.result.diagram_warnings.len()
    );
}

#[test]
fn f10_targets_site_backlinks_and_original_metadata() {
    let f = Fixture::js(B);
    let off = f.off(&taint());
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let old = documents(&f, &off);
    let new = documents(&f, &on);
    let target = context_target(&new.targets, "b.js");
    assert_eq!(target["site"]["symbol"], "g");
    assert_eq!(target["site"]["function_start_line"], 1);
    assert_eq!(target["site"]["function_end_line"], 1);
    assert_eq!(new.targets.value["diff"], old.targets.value["diff"]);
    assert_eq!(
        new.targets.value["diff"]["sha256"],
        f.inputs.diff_text_sha256
    );
    assert_eq!(
        new.targets.value["diff"]["files"],
        serde_json::json!(["main.js"])
    );
    for target in old.targets.value["targets"].as_array().unwrap() {
        assert!(new.targets.value["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == target["id"]));
    }
    assert!(
        new.targets.value["targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["category"] == "taint_source"
                && t["related"]["files"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|f| f == "b.js"))),
        "licensed source backlink target"
    );
    let one = f.single(&imports());
    let single = Documents::single(&one, &f.inputs, RenderSettings::default());
    assert_eq!(
        context_target(&single.targets, "b.js")["site"],
        target["site"]
    );
}

mod evidence;
mod notes;
mod policy;
use notes::assert_depth_bound_once;
