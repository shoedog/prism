//! Deterministic projection of annotated findings into targets contract v1.0.
//!
//! Invocation diagnostics are independent of findings: a run can have boundary
//! notes without producing any target. When using the review facade, pass its
//! diagnostics through [`TargetsMeta::run_warnings`] before calling [`project`].
//! Preserve the ordinary parse, load and build warning prefix in that order:
//!
//! ```text
//! let mut meta = TargetsMeta::default();
//! // Set repository identity, algorithms, errors and admission policy as usual.
//! meta.run_warnings = outcome.inputs.parse_warnings.clone();
//! meta.run_warnings.extend(outcome.inputs.load_warnings.clone());
//! meta.run_warnings.extend(outcome.build_warnings.clone());
//! if let Some(diagnostics) = &outcome.run.diagnostics {
//!     meta.run_warnings.extend(diagnostics.messages());
//! }
//! let document = project(
//!     &outcome.run.findings, &outcome.run.evidence, &outcome.inputs, &meta,
//! );
//! ```
//!
//! Do not also append `outcome.run.warnings`: it already includes parse warnings
//! and rendered diagnostics. Projection preserves the supplied prefix and adds
//! its own finding warnings before severity/tier filtering; it cannot recover
//! boundary diagnostics from findings alone.

mod dependency_hint;
mod dependency_identity;
pub mod mapping;
mod model;

#[cfg(test)]
mod context_shape_tests;
#[cfg(test)]
mod context_view_tests;

pub use model::{
    DependencyHint, Diff, Expected, Producer, Related, Repo, Site, Target, TargetsDocument,
};

use crate::output::{sarif::path_escapes_repo, severity_rank};
use mapping::{language_tag, map_finding};
use prism::api::{build_info, ReviewInputs};
use prism::finding_confidence::{
    admit_finding_for_resolution, parse_quality_for_selected_evidence, EvidencePath, FindingTier,
    MinConfidence, ResolutionMode,
};
use prism::languages::Language;
use prism::slice::{AlgorithmError, SliceFinding};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::PathBuf;

/// Run-level facts `project` cannot derive from the findings. `#[non_exhaustive]`
/// (§2.3.1): re-exported from `prism::api`, so build one with `Default` plus
/// field assignment rather than a struct literal.
#[derive(Default)]
#[non_exhaustive]
pub struct TargetsMeta {
    pub algorithms_run: Vec<String>,
    pub repo_root: PathBuf,
    pub repo_sha: Option<String>,
    pub errors: Vec<AlgorithmError>,
    pub run_warnings: Vec<String>,
    pub min_severity_rank: u8,
    pub min_tier: FindingTier,
    pub min_confidence: MinConfidence,
    pub resolution: ResolutionMode,
}

/// Projects findings and records per-finding warnings against the complete
/// input population before applying severity or tier filters.
pub fn project(
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    inputs: &ReviewInputs,
    meta: &TargetsMeta,
) -> TargetsDocument {
    project_with_context(findings, evidence, inputs, None, meta)
}

/// Project with eligible context for taint findings, preserving the legacy None path.
/// `inputs` and `context` must belong to the same captured review.
pub fn project_in_context(
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    inputs: &ReviewInputs,
    context: Option<&crate::api::ReviewContext>,
    meta: &TargetsMeta,
) -> TargetsDocument {
    project_with_context(
        findings,
        evidence,
        inputs,
        context.and_then(crate::api::ReviewContext::view),
        meta,
    )
}

pub(crate) fn project_with_context(
    findings: &[SliceFinding],
    evidence: &[Option<EvidencePath>],
    inputs: &ReviewInputs,
    context: Option<crate::api::map_view::MapView<'_>>,
    meta: &TargetsMeta,
) -> TargetsDocument {
    let mut warnings = meta.run_warnings.clone();
    let mut targets = Vec::new();
    let mut ids = HashSet::new();

    if findings.len() != evidence.len() {
        warnings.push(format!(
            "targets: evidence alignment mismatch: {} findings, {} artifacts; unmatched findings are unlabeled",
            findings.len(),
            evidence.len()
        ));
    }

    for (index, finding) in findings.iter().enumerate() {
        let original = crate::api::map_view::MapView::original(inputs);
        let view = if finding.algorithm == "taint" {
            context.unwrap_or(original)
        } else {
            original
        };
        if finding.line == 0 {
            warnings.push(format!(
                "targets: dropped finding with line 0: {}/{} in {}",
                finding.algorithm,
                finding.category.as_deref().unwrap_or("uncategorized"),
                finding.file
            ));
            continue;
        }

        // `file` already has `/` separators, which `path_escapes_repo` expects.
        let file = normalise_path(&finding.file, &mut warnings);
        if path_escapes_repo(&file) {
            warnings.push(format!(
                "targets: dropped finding with path escaping repo root: {}",
                finding.file
            ));
            continue;
        }

        let category = finding
            .category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        let severity = normalize_severity(finding, &mut warnings);

        let mut normalized_finding = finding.clone();
        normalized_finding.file = file.clone();
        normalized_finding.related_files = finding
            .related_files
            .iter()
            .map(|path| normalise_path(path, &mut warnings))
            .collect();

        let selected_evidence = evidence.get(index).and_then(Option::as_ref);
        let parse_quality = parse_quality_for_selected_evidence(
            &normalized_finding,
            selected_evidence,
            view.parse_quality,
            view.files,
        );
        let admission = admit_finding_for_resolution(
            &finding.algorithm,
            parse_quality,
            selected_evidence,
            meta.min_confidence,
            meta.resolution,
        );
        let mut mapped = map_finding(finding);
        if mapped.kind == "external_call" {
            let site_context = dependency_hint::SiteContext {
                file: &file,
                repo_root: &meta.repo_root,
                known_files: view.files,
                resolved_name: mapped.hint.as_ref().and_then(|hint| hint.callee.as_deref()),
            };
            match view
                .files
                .get(&file)
                .and_then(|parsed| dependency_hint::resolve(parsed, finding.line, &site_context))
            {
                // AST-recovered callee text (verbatim, dotted chain as
                // written) supersedes the description-derived fallback in
                // `mapped.hint`: it is the syntax the harness must glob
                // against (roadmap `03-tooling-plan-roadmap.md` §3 Phase 1).
                Some(dependency_hint::Resolution::Hint(hint)) => {
                    mapped.hint = Some(DependencyHint {
                        kind: hint.kind.map(str::to_string),
                        callee: Some(hint.callee),
                        counterpart: None,
                    });
                }
                // Several calls span the site line and the finding's own
                // recorded callee identity does not single one out. Keeping
                // the description-derived hint is the honest answer: a
                // deterministic pick would attribute an unrelated call.
                Some(dependency_hint::Resolution::Ambiguous(count)) => {
                    warnings.push(format!(
                        "targets: ambiguous-callee: {count} calls on line {} in {file}",
                        finding.line
                    ));
                }
                // No call node spans the site (e.g. file not parsed): leave
                // whatever hint the mapping already produced, never lose one.
                None => {}
            }
        }
        let Some((confidence, tier)) = admission else {
            continue;
        };
        let (symbol, function_start_line, function_end_line) =
            enclosing_site(&file, finding, view.files, &mut warnings);
        let language = Language::from_path(&file)
            .map(language_tag)
            .map(str::to_string);

        let mut related_lines: Vec<usize> = finding
            .related_lines
            .iter()
            .copied()
            .filter(|line| *line > 0)
            .collect();
        related_lines.sort_unstable();
        related_lines.dedup();
        let mut related_files = normalized_finding.related_files;
        related_files.sort();
        related_files.dedup();

        let id = target_id(
            &file,
            finding.line,
            symbol.as_deref(),
            &finding.algorithm,
            &category,
            &finding.description,
            &severity,
            &related_lines,
            &related_files,
        );
        if !ids.insert(id.clone()) {
            warnings.push(format!(
                "targets: duplicate id {id} dropped ({}/{} {}:{})",
                finding.algorithm, category, file, finding.line
            ));
            continue;
        }

        if severity_rank(&severity) < meta.min_severity_rank
            || (matches!(meta.min_tier, FindingTier::Asserted)
                && !matches!(tier, FindingTier::Asserted))
        {
            continue;
        }

        let related = if related_lines.is_empty() && related_files.is_empty() {
            None
        } else {
            Some(Related {
                lines: related_lines,
                files: related_files,
            })
        };
        targets.push(Target {
            id,
            site: Site {
                file,
                line: finding.line,
                symbol,
                function_start_line,
                function_end_line,
                language,
            },
            kind: mapped.kind.to_string(),
            category,
            expected: Expected {
                property: mapped.property.to_string(),
                detail: mapped.detail,
            },
            dependency_hint: mapped.hint,
            source_algorithm: finding.algorithm.clone(),
            confidence,
            tier,
            severity,
            description: Some(finding.description.clone()),
            related,
            parse_quality: Some(parse_quality.as_str().to_string()),
        });
    }

    targets.sort_by(|left, right| {
        (
            &left.site.file,
            left.site.line,
            &left.source_algorithm,
            &left.category,
            &left.id,
        )
            .cmp(&(
                &right.site.file,
                right.site.line,
                &right.source_algorithm,
                &right.category,
                &right.id,
            ))
    });

    let build = build_info();
    TargetsDocument {
        schema_version: "1.0".to_string(),
        producer: Producer {
            tool: "prism".to_string(),
            version: build.package_version.to_string(),
            resolution_mode: meta.resolution.as_str().to_string(),
            build_identity: Some(build.build_identity.to_string()),
            algorithms: meta.algorithms_run.clone(),
        },
        repo: Some(Repo {
            root: Some(meta.repo_root.to_string_lossy().into_owned()),
            sha: meta.repo_sha.clone(),
        }),
        diff: Some(Diff {
            sha256: Some(inputs.diff_text_sha256.clone()),
            files: inputs
                .diff
                .files
                .iter()
                .map(|file| file.file_path.clone())
                .collect(),
        }),
        targets,
        errors: meta.errors.clone(),
        warnings,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn target_id(
    file: &str,
    line: usize,
    symbol: Option<&str>,
    algorithm: &str,
    category: &str,
    description: &str,
    severity: &str,
    related_lines_sorted: &[usize],
    related_files_sorted: &[String],
) -> String {
    let canonical = serde_json::to_vec(&serde_json::json!([
        file,
        line,
        symbol.unwrap_or(""),
        algorithm,
        category,
        description,
        severity,
        related_lines_sorted,
        related_files_sorted,
    ]))
    .unwrap_or_default();
    format!("{:x}", Sha256::digest(canonical))
}

fn normalise_path(path: &str, warnings: &mut Vec<String>) -> String {
    let normalized = path.replace('\\', "/");
    if normalized != path {
        warnings.push(format!(
            "targets: normalised path separators: {path} -> {normalized}"
        ));
    }
    normalized
}

fn normalize_severity(finding: &SliceFinding, warnings: &mut Vec<String>) -> String {
    match finding.severity.as_str() {
        "info" | "suggestion" | "warning" | "concern" => finding.severity.clone(),
        severity => {
            warnings.push(format!(
                "targets: unknown severity '{severity}' mapped to concern for {}:{}",
                finding.file, finding.line
            ));
            "concern".to_string()
        }
    }
}

fn enclosing_site(
    file: &str,
    finding: &SliceFinding,
    files: &std::collections::BTreeMap<String, prism::ast::ParsedFile>,
    warnings: &mut Vec<String>,
) -> (Option<String>, Option<usize>, Option<usize>) {
    let Some(parsed) = files.get(file) else {
        warnings.push(format!(
            "targets: function bounds omitted for {file}:{}: file not parsed",
            finding.line
        ));
        return (finding.function_name.clone(), None, None);
    };
    let Some(node) = parsed.function_node_spanning(finding.line) else {
        if let Some(name) = &finding.function_name {
            warnings.push(format!(
                "targets: function bounds unavailable for {name} at {file}:{}",
                finding.line
            ));
        }
        return (finding.function_name.clone(), None, None);
    };

    let (start, end) = parsed.node_line_range(&node);
    let enclosing = parsed
        .language
        .function_name(&node)
        .map(|name| parsed.node_text(&name).to_string());
    if let (Some(enclosing), Some(named)) = (&enclosing, &finding.function_name) {
        if enclosing != named {
            warnings.push(format!(
                "targets: symbol {enclosing} differs from finding's function {named} at {file}:{}",
                finding.line
            ));
        }
    }
    (
        enclosing.or_else(|| finding.function_name.clone()),
        Some(start),
        Some(end),
    )
}
