//! CONTROLLED union maps, findings and evidence; underlying source files are genuinely parsed.
use super::*;
use crate::api::map_view::tests::{bytes, finding, witness, Fixture, CONTEXT};
use prism::finding_confidence::{FindingConfidence, FindingTier, ResolutionMode};
use prism::resolution::ResolutionConfidence;
use prism::slice::{AlgorithmError, SliceFinding};

fn exact(file: &str) -> Option<EvidencePath> {
    Some(witness(file, ResolutionConfidence::Exact))
}
fn context(
    f: &Fixture,
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    meta: &TargetsMeta,
) -> TargetsDocument {
    project_with_context(findings, evidence, &f.inputs, Some(f.view()), meta)
}
fn guard(f: &Fixture) {
    assert!(
        f.inputs.files.contains_key("main.py") && !f.inputs.files.contains_key("context.py"),
        "inadmissible precondition: original map"
    );
    assert!(
        f.files.contains_key("context.py") && f.sources.contains_key("context.py"),
        "inadmissible precondition: union map"
    );
}
#[test]
fn t01_related_files_sorted_deduplicated_normalized() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let mut sink = finding("taint", "context.py");
    sink.related_files = vec!["z\\witness.py".into(), "main.py".into(), "main.py".into()];
    let d = context(&f, &[sink], &[exact("context.py")], &f.meta());
    assert_eq!(d.targets.len(), 1);
    assert_eq!(
        d.targets[0].related.as_ref().unwrap().files,
        ["main.py", "z/witness.py"]
    );
    assert!(d
        .warnings
        .iter()
        .any(|w| w.contains("normalised path separators")));
}
#[test]
fn t02_source_backlink_and_original_site() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let mut source = finding("taint", "main.py");
    source.category = Some("taint_source".into());
    source.related_files = vec!["context.py".into()];
    let d = context(&f, &[source], &[exact("main.py")], &f.meta());
    assert_eq!(d.targets.len(), 1);
    let target = &d.targets[0];
    assert_eq!(target.related.as_ref().unwrap().files, ["context.py"]);
    assert_eq!(target.site.file, "main.py");
    assert_eq!(target.site.symbol.as_deref(), Some("baseline"));
    assert_eq!(target.site.function_start_line, Some(2));
    assert_eq!(target.parse_quality.as_deref(), Some("clean"));
}
#[test]
fn t03_baseline_ids_stable_and_context_id_new() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let base = finding("taint", "main.py");
    let extra = finding("taint", "context.py");
    let off = project(&[base.clone()], &[exact("main.py")], &f.inputs, &f.meta());
    let on = context(
        &f,
        &[base, extra],
        &[exact("main.py"), exact("context.py")],
        &f.meta(),
    );
    assert_eq!(off.targets.len(), 1);
    assert_eq!(on.targets.len(), 2);
    let main = on
        .targets
        .iter()
        .find(|x| x.site.file == "main.py")
        .unwrap();
    let added = on
        .targets
        .iter()
        .find(|x| x.site.file == "context.py")
        .unwrap();
    assert_eq!(main.id, off.targets[0].id);
    assert_ne!(main.id, added.id);
    let mut a = finding("taint", "context.py");
    a.related_files = vec!["main.py".into()];
    let mut b = a.clone();
    b.related_files = vec!["context.py".into()];
    let d = context(
        &f,
        &[a, b],
        &[exact("context.py"), exact("context.py")],
        &f.meta(),
    );
    assert_eq!(d.targets.len(), 2);
    assert_ne!(d.targets[0].id, d.targets[1].id);
}
#[test]
fn t04_asserted_tier_uses_clean_context_not_confidence_floor() {
    let clean = Fixture::new(CONTEXT);
    guard(&clean);
    assert!(clean.files["context.py"].error_rate() <= 0.01);
    assert!(!clean.quality.contains_key("context.py"));
    let findings = [finding("taint", "context.py")];
    let evidence = [exact("context.py")];
    let mut meta = clean.meta();
    meta.min_tier = FindingTier::Asserted;
    assert!(project(&findings, &evidence, &clean.inputs, &meta)
        .targets
        .is_empty());
    let on = context(&clean, &findings, &evidence, &meta);
    assert_eq!(on.targets.len(), 1);
    assert_eq!(on.targets[0].confidence, FindingConfidence::Exact);
    assert_eq!(on.targets[0].tier, FindingTier::Asserted);
    let poor =
        Fixture::new("def contextual(value):\n    first = value\n    broken =\n    return first\n");
    guard(&poor);
    let rate = poor.files["context.py"].error_rate();
    assert!(
        rate > 0.01 && rate <= 0.3,
        "inadmissible precondition: poor/degraded parse {rate}"
    );
    let mut poor_meta = poor.meta();
    poor_meta.min_tier = FindingTier::Asserted;
    assert!(context(&poor, &findings, &evidence, &poor_meta)
        .targets
        .is_empty());
}
#[test]
fn t05_duplicate_targets_warn_and_keep_one() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    for file in ["main.py", "context.py"] {
        let x = finding("taint", file);
        let d = context(&f, &[x.clone(), x], &[exact(file), exact(file)], &f.meta());
        assert_eq!(d.targets.len(), 1);
        let warnings: Vec<_> = d
            .warnings
            .iter()
            .filter(|w| w.contains("targets: duplicate id"))
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(file));
    }
}
#[test]
fn t05_filtered_first_duplicate_reserves_identity() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let x = finding("taint", "context.py");
    let mut meta = f.meta();
    meta.min_tier = FindingTier::Asserted;
    let evidence = [
        Some(witness("context.py", ResolutionConfidence::NameOnly)),
        exact("context.py"),
    ];
    let d = context(&f, &[x.clone(), x], &evidence, &meta);
    assert!(d.targets.is_empty());
    assert!(d
        .warnings
        .iter()
        .any(|w| w.contains("targets: duplicate id")));
}
#[test]
fn t06_run_warnings_keep_prefix() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let mut meta = f.meta();
    meta.run_warnings = vec!["parse warning".into(), "taint-context: bounded".into()];
    let d = context(
        &f,
        &[finding("taint", "context.py")],
        &[exact("context.py")],
        &meta,
    );
    assert_eq!(&d.warnings[..2], &meta.run_warnings);
}
#[test]
fn t07_errors_are_unchanged() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let mut meta = f.meta();
    meta.errors.push(AlgorithmError {
        algorithm: "taint".into(),
        error: "controlled failure".into(),
    });
    let findings = [finding("taint", "context.py")];
    let evidence = [exact("context.py")];
    let off = project(&findings, &evidence, &f.inputs, &meta);
    let on = context(&f, &findings, &evidence, &meta);
    assert_eq!(bytes(&off.errors), bytes(&on.errors));
    assert_eq!(bytes(&on.errors), bytes(&meta.errors));
}
#[test]
fn t08_line_zero_and_path_escape_are_dropped() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let mut zero = finding("taint", "context.py");
    zero.line = 0;
    let escape = finding("taint", "../context.py");
    let d = context(&f, &[zero, escape], &[None, None], &f.meta());
    assert!(d.targets.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("line 0")));
    assert!(d.warnings.iter().any(|w| w.contains("path escaping")));
}
#[test]
fn t09_evidence_mismatch_warning_is_unchanged() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let findings = [finding("taint", "context.py"), finding("taint", "main.py")];
    let evidence = [exact("context.py")];
    let off = project(&findings, &evidence, &f.inputs, &f.meta());
    let on = context(&f, &findings, &evidence, &f.meta());
    let mismatch = |d: &TargetsDocument| {
        d.warnings
            .iter()
            .filter(|w| w.contains("evidence alignment mismatch"))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(mismatch(&off).len(), 1);
    assert_eq!(mismatch(&off), mismatch(&on));
    assert_eq!(on.targets.len(), 2);
}
#[test]
fn t10_nominal_underclaims_scoped_evidence() {
    let f = Fixture::new(CONTEXT);
    guard(&f);
    let findings = [finding("taint", "context.py")];
    let evidence = [exact("context.py")];
    let scoped = context(&f, &findings, &evidence, &f.meta());
    let mut meta = f.meta();
    meta.resolution = ResolutionMode::Nominal;
    let nominal = context(&f, &findings, &evidence, &meta);
    assert_eq!(scoped.targets.len(), 1);
    assert_eq!(nominal.targets.len(), 1);
    assert_eq!(scoped.targets[0].confidence, FindingConfidence::Exact);
    assert_eq!(scoped.targets[0].tier, FindingTier::Asserted);
    assert_eq!(nominal.targets[0].confidence, FindingConfidence::Unlabeled);
    assert_eq!(nominal.targets[0].tier, FindingTier::Candidate);
    assert_eq!(nominal.targets[0].id, scoped.targets[0].id);
}
