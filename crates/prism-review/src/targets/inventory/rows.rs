use super::{Grade, InventoryOutput, UnsupportedSite};
use crate::targets::{
    target_id, target_order, DependencyHint, Diff, Expected, Producer, Repo, Site, Target,
    TargetsDocument, TargetsMeta,
};
use prism::api::build_info;
use prism::finding_confidence::{FindingConfidence, FindingTier, ParseQuality};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

const ALGORITHM: &str = "prism.inventory/1";

pub fn inventory_document(meta: &TargetsMeta, out: &InventoryOutput) -> TargetsDocument {
    let build = build_info();
    let mut files = out.files.clone();
    files.sort();
    files.dedup();
    let sha256 =
        Sha256::digest(serde_json::to_vec(&files).expect("inventory file list serializes"));
    let doc = TargetsDocument {
        schema_version: "1.1".to_owned(),
        producer: Producer {
            tool: "prism".to_owned(),
            version: build.package_version.to_string(),
            resolution_mode: meta.resolution.as_str().to_owned(),
            build_identity: Some(build.build_identity.to_string()),
            algorithms: Vec::new(),
        },
        repo: Some(Repo {
            root: Some(meta.repo_root.to_string_lossy().into_owned()),
            sha: meta.repo_sha.clone(),
        }),
        diff: Some(Diff {
            sha256: Some(format!("{sha256:x}")),
            files,
        }),
        targets: Vec::new(),
        errors: Vec::new(),
        warnings: meta.run_warnings.clone(),
    };
    merge_inventory(doc, meta, out)
}

pub fn merge_inventory(
    mut doc: TargetsDocument,
    meta: &TargetsMeta,
    out: &InventoryOutput,
) -> TargetsDocument {
    doc.schema_version = "1.1".to_owned();
    if !doc.producer.algorithms.iter().any(|name| name == ALGORITHM) {
        doc.producer.algorithms.push(ALGORITHM.to_owned());
    }
    for (file, causes) in &out.unproven {
        let total: usize = causes.values().sum();
        if total > 0 {
            let detail = causes
                .iter()
                .filter(|(_, n)| **n > 0)
                .map(|(cause, n)| format!("{}: {n}", cause.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            doc.warnings.push(format!(
                "inventory: identity unproven: {file}: {total} ({detail})"
            ));
        }
    }
    let mut skipped = out.skipped.iter().collect::<Vec<_>>();
    skipped.sort();
    skipped.dedup();
    for (file, reason) in skipped {
        doc.warnings
            .push(format!("inventory: skipped: {file}: {reason}"));
    }
    doc.warnings.push("inventory: excluded directories: .git, target, node_modules, vendor, venv, site-packages, dot directories, build and dist at the root".to_owned());

    let targets = rows(out, meta);
    if meta.min_severity_rank > 0 && !targets.is_empty() {
        let floor = match meta.min_severity_rank {
            1 => "suggestion",
            2 => "warning",
            _ => "concern",
        };
        doc.warnings.push(format!(
            "inventory: {} rows removed by --min-severity {floor}",
            targets.len()
        ));
    } else {
        let mut ids: HashSet<_> = doc.targets.iter().map(|row| row.id.clone()).collect();
        doc.targets
            .extend(targets.into_iter().filter(|row| ids.insert(row.id.clone())));
    }
    doc.targets.sort_by(target_order);
    doc
}

type Classification = (FindingConfidence, FindingTier, ParseQuality);

fn classify(
    out: &InventoryOutput,
    meta: &TargetsMeta,
    file: &str,
    grade: Grade,
) -> Option<Classification> {
    let quality = out.parse_quality[file];
    let confidence = match grade {
        Grade::Exact => FindingConfidence::Exact,
        Grade::AnnotationTyped => FindingConfidence::NameOnly,
    };
    let tier = if grade == Grade::Exact && quality == ParseQuality::Clean {
        FindingTier::Asserted
    } else {
        FindingTier::Candidate
    };
    (meta.min_confidence.admits(confidence)
        && (meta.min_tier != FindingTier::Asserted || tier == FindingTier::Asserted))
        .then_some((confidence, tier, quality))
}

fn rows(out: &InventoryOutput, meta: &TargetsMeta) -> Vec<Target> {
    let mut targets = Vec::new();
    for site in &out.sites {
        let Some(class) = classify(out, meta, &site.file, site.grade) else {
            continue;
        };
        let mut row = make_row(
            Site {
                file: site.file.clone(),
                line: site.line,
                symbol: Some(site.symbol.clone()),
                function_start_line: Some(site.function_start_line),
                function_end_line: Some(site.function_end_line),
                language: Some("python".to_owned()),
            },
            site.family.as_str(),
            &site.resolved_callee,
            format!("dependency_call.{}", site.family.as_str()),
            format!("{}: dependency call site", site.resolved_callee),
            class,
        );
        row.kind = "external_call".to_owned();
        row.expected = Expected {
            property: "error_handled".to_owned(),
            detail: Some(format!("source callee: {}", site.source_callee)),
        };
        targets.push(row);
    }

    let mut groups = BTreeMap::new();
    for site in &out.unsupported {
        let Some(class) = classify(out, meta, &site.file, site.grade) else {
            continue;
        };
        let key = (
            &site.file,
            site.line,
            &site.symbol,
            &site.resolved_callee,
            site.reason,
        );
        let group = groups.entry(key).or_insert((site, class, 0));
        group.2 += 1;
        if class.0 == FindingConfidence::Exact {
            group.0 = site;
            group.1 = class;
        }
    }
    targets.extend(
        groups
            .into_values()
            .map(|(site, class, count)| unsupported(site, class, count)),
    );
    targets
}

fn unsupported(site: &UnsupportedSite, class: Classification, count: usize) -> Target {
    let reason = site.reason.as_str();
    let mut row = make_row(
        Site {
            file: site.file.clone(),
            line: site.line,
            symbol: site.symbol.clone(),
            function_start_line: None,
            function_end_line: None,
            language: Some("python".to_owned()),
        },
        site.family.as_str(),
        &site.resolved_callee,
        "dependency_call_unsupported".to_owned(),
        format!(
            "{}: unsupported dependency call: {reason}",
            site.resolved_callee
        ),
        class,
    );
    row.expected.detail = Some(if count == 1 {
        reason.to_owned()
    } else {
        format!("{reason}; {count} calls")
    });
    row
}

fn make_row(
    site: Site,
    family: &str,
    callee: &str,
    category: String,
    description: String,
    (confidence, tier, quality): Classification,
) -> Target {
    let id = target_id(
        &site.file,
        site.line,
        site.symbol.as_deref(),
        ALGORITHM,
        &category,
        &description,
        "info",
        &[],
        &[],
    );
    Target {
        id,
        site,
        kind: "other".to_owned(),
        category,
        expected: Expected {
            property: "unknown".to_owned(),
            detail: None,
        },
        dependency_hint: Some(DependencyHint {
            kind: Some(family.to_owned()),
            callee: Some(callee.to_owned()),
            counterpart: None,
        }),
        source_algorithm: ALGORITHM.to_owned(),
        confidence,
        tier,
        severity: "info".to_owned(),
        description: Some(description),
        related: None,
        parse_quality: Some(quality.as_str().to_owned()),
    }
}
