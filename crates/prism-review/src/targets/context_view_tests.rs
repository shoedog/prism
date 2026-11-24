//! REAL parses and original input loading; CONTROLLED union maps, findings and witnesses.
//! Shared support does not run taint analysis or the expansion engine.

use super::{dependency_hint, project, project_with_context};
use crate::api::map_view::tests::{bytes, finding, witness, Fixture, CONTEXT};
use prism::ast::ParsedFile;
use prism::finding_confidence::FindingTier;
use prism::languages::Language;
use prism::resolution::ResolutionConfidence;

#[test]
fn v04_targets_site_and_grade_use_the_bundle_for_taint() {
    let fixture = Fixture::new(CONTEXT);
    let findings = [finding("taint", "context.py")];
    let evidence = [Some(witness("context.py", ResolutionConfidence::Exact))];
    let meta = fixture.meta();
    let original = project(&findings, &evidence, &fixture.inputs, &meta);
    assert!(
        original
            .warnings
            .iter()
            .any(|w| w.contains("file not parsed")),
        "inadmissible precondition: original projection must lack context site"
    );
    let parsed = &fixture.files["context.py"];
    let node = parsed
        .function_node_spanning(3)
        .expect("inadmissible precondition: context function must span anchor");
    assert_eq!(
        parsed.node_line_range(&node),
        (1, 3),
        "inadmissible precondition: real context function bounds"
    );
    let document = project_with_context(
        &findings,
        &evidence,
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    let target = &document.targets[0];
    assert_eq!(target.site.symbol.as_deref(), Some("contextual"));
    assert_eq!(target.site.function_start_line, Some(1));
    assert_eq!(target.site.function_end_line, Some(3));
    assert_eq!(target.parse_quality.as_deref(), Some("clean"));
    assert_eq!(target.tier, FindingTier::Asserted);
    assert!(!document
        .warnings
        .iter()
        .any(|w| w.contains("file not parsed")));
}

#[test]
fn v05_non_taint_keeps_original_maps() {
    let mut fixture = Fixture::new(CONTEXT);
    // In-memory only: the dependency resolver also checks disk, so writing this
    // file there would make both views suppress the library kind and be vacuous.
    fixture.files.insert(
        "requests.py".into(),
        ParsedFile::parse(
            "requests.py",
            "def get(value):\n    return value\n",
            Language::Python,
        )
        .unwrap(),
    );
    fixture.sources.insert(
        "requests.py".into(),
        "def get(value):\n    return value\n".into(),
    );
    let parsed = &fixture.inputs.files["main.py"];
    let original_site = dependency_hint::SiteContext {
        file: "main.py",
        repo_root: fixture.repo.path(),
        known_files: &fixture.inputs.files,
        resolved_name: None,
    };
    let context_site = dependency_hint::SiteContext {
        known_files: &fixture.files,
        ..original_site
    };
    let original_hint = dependency_hint::resolve(parsed, 3, &original_site);
    let context_hint = dependency_hint::resolve(parsed, 3, &context_site);
    assert_ne!(
        original_hint, context_hint,
        "inadmissible precondition: bundle known_files must perturb direct dependency hint"
    );
    assert!(
        matches!(original_hint, Some(dependency_hint::Resolution::Hint(ref hint)) if hint.kind == Some("http")),
        "inadmissible precondition: unshadowed requests binding must have http kind"
    );
    assert!(
        matches!(context_hint, Some(dependency_hint::Resolution::Hint(ref hint)) if hint.kind.is_none()),
        "inadmissible precondition: bundle-only requests.py must suppress kind"
    );
    let mut echo = finding("echo", "main.py");
    echo.category = Some("missing_error_handling".into());
    let findings = [
        finding("echo", "context.py"),
        echo,
        finding("taint", "context.py"),
    ];
    let evidence = [
        Some(witness("context.py", ResolutionConfidence::Exact)),
        Some(witness("main.py", ResolutionConfidence::Exact)),
        Some(witness("context.py", ResolutionConfidence::Exact)),
    ];
    let meta = fixture.meta();
    let original = project(&findings, &evidence, &fixture.inputs, &meta);
    let viewed = project_with_context(
        &findings,
        &evidence,
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    let select = |document: &super::TargetsDocument| {
        document
            .targets
            .iter()
            .filter(|t| t.source_algorithm == "echo")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(bytes(&select(&original)), bytes(&select(&viewed)));
    let unknown = viewed
        .targets
        .iter()
        .find(|t| t.source_algorithm == "echo" && t.site.file == "context.py")
        .unwrap();
    assert_eq!(unknown.parse_quality.as_deref(), Some("unknown"));
    assert_eq!(unknown.site.function_start_line, None);
    let external = viewed
        .targets
        .iter()
        .find(|t| t.kind == "external_call")
        .unwrap();
    assert_eq!(
        external
            .dependency_hint
            .as_ref()
            .and_then(|hint| hint.kind.as_deref()),
        Some("http")
    );
}

#[test]
fn v06_diff_metadata_is_original() {
    let fixture = Fixture::new(CONTEXT);
    assert!(
        fixture.files.len() > fixture.inputs.files.len(),
        "inadmissible precondition: union must have extra files"
    );
    let document = project_with_context(
        &[finding("taint", "context.py")],
        &[Some(witness("context.py", ResolutionConfidence::Exact))],
        &fixture.inputs,
        Some(fixture.view()),
        &fixture.meta(),
    );
    let diff = document.diff.unwrap();
    assert_eq!(
        diff.sha256.as_deref(),
        Some(fixture.inputs.diff_text_sha256.as_str())
    );
    assert_eq!(
        diff.files,
        fixture
            .inputs
            .diff
            .files
            .iter()
            .map(|f| f.file_path.clone())
            .collect::<Vec<_>>()
    );
    assert!(!diff.files.iter().any(|file| file == "context.py"));
}

#[test]
fn v07_baseline_taint_finding_grades_identically() {
    let fixture = Fixture::new(CONTEXT);
    let findings = [finding("taint", "main.py")];
    let evidence = [Some(witness("main.py", ResolutionConfidence::Exact))];
    let meta = fixture.meta();
    let original = project(&findings, &evidence, &fixture.inputs, &meta);
    let viewed = project_with_context(
        &findings,
        &evidence,
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    assert_eq!(bytes(&original), bytes(&viewed));
    assert_eq!(viewed.targets[0].tier, FindingTier::Asserted);
}

#[test]
fn v10_non_taint_dependency_site_does_not_use_context_parse() {
    let fixture =
        Fixture::new("import requests\ndef contextual(value):\n    return requests.get(value)\n");
    let mut echo = finding("echo", "context.py");
    echo.category = Some("missing_error_handling".into());
    let direct = dependency_hint::SiteContext {
        file: "context.py",
        repo_root: fixture.repo.path(),
        known_files: &fixture.files,
        resolved_name: None,
    };
    assert!(
        matches!(
            dependency_hint::resolve(&fixture.files["context.py"], 3, &direct),
            Some(dependency_hint::Resolution::Hint(ref hint)) if hint.callee == "requests.get"
        ),
        "inadmissible precondition: context parse must recover a callee absent from original maps"
    );
    let findings = [echo, finding("taint", "context.py")];
    let evidence = vec![Some(witness("context.py", ResolutionConfidence::Exact)); 2];
    let meta = fixture.meta();
    let original = project(&findings, &evidence, &fixture.inputs, &meta);
    let viewed = project_with_context(
        &findings,
        &evidence,
        &fixture.inputs,
        Some(fixture.view()),
        &meta,
    );
    let original_echo = original
        .targets
        .iter()
        .find(|t| t.source_algorithm == "echo")
        .unwrap();
    let viewed_echo = viewed
        .targets
        .iter()
        .find(|t| t.source_algorithm == "echo")
        .unwrap();
    assert_eq!(bytes(original_echo), bytes(viewed_echo));
    assert!(viewed_echo.dependency_hint.is_none());
}
