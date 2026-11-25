use anyhow::{Context, Result};
use clap::{parser::ValueSource, ArgMatches};
use prism::slice::SliceConfig;
use prism_review::cli::TargetsArgs;
use prism_review::output;
use prism_review::targets::inventory::{
    collect, load_python_files, Family, InventoryOptions, InventoryOutput,
};
use prism_review::targets::{self, TargetsDocument, TargetsMeta};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_output::targets_exit_code;

pub(crate) fn run_targets(args: &TargetsArgs, matches: &ArgMatches) -> Result<()> {
    let inventory = args
        .inventory
        .as_ref()
        .map(|_| inventory_options(args))
        .transpose()?;
    let document = if let Some(diff) = &args.diff {
        run_review(args, diff, inventory.as_ref())?
    } else {
        for option in [
            "algorithm",
            "scoped_cpg",
            "old_repo",
            "compile_commands",
            "cache_dir",
            "no_cache",
        ] {
            if matches.value_source(option) == Some(ValueSource::CommandLine) {
                anyhow::bail!("targets: --{} requires --diff", option.replace('_', "-"));
            }
        }
        let repo = canonical_repo(args)?;
        let meta = targets_meta(args, repo);
        let out = collect_inventory(
            args,
            &meta.repo_root,
            &inventory.expect("clap requires --inventory or --diff"),
        )?;
        targets::inventory_document(&meta, &out)
    };
    write_document(args, &document)
}

fn inventory_options(args: &TargetsArgs) -> Result<InventoryOptions> {
    let mut options = InventoryOptions::default();
    if let Some(kinds) = &args.inventory_kinds {
        for name in kinds.split(',').map(str::trim) {
            let Some(family) =
                Family::parse(name).filter(|kind| Family::supported().contains(kind))
            else {
                let supported = Family::supported()
                    .iter()
                    .map(|kind| kind.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                anyhow::bail!(
                    "targets: unsupported inventory family {name:?}; supported: {supported}"
                );
            };
            options.families.insert(family);
        }
    }
    Ok(options)
}

fn collect_inventory(
    args: &TargetsArgs,
    repo: &Path,
    options: &InventoryOptions,
) -> Result<InventoryOutput> {
    let only = args.files.as_deref().map(|paths| {
        paths
            .split(',')
            .map(|file| file.trim().to_owned())
            .collect()
    });
    let (files, skipped) = load_python_files(repo, only.as_ref())?;
    let mut out = collect(&files, repo, options);
    out.skipped = skipped;
    Ok(out)
}

fn canonical_repo(args: &TargetsArgs) -> Result<PathBuf> {
    fs::canonicalize(&args.repo)
        .with_context(|| format!("Failed to canonicalize repo: {:?}", args.repo))
}

fn targets_meta(args: &TargetsArgs, repo: PathBuf) -> TargetsMeta {
    let mut meta = TargetsMeta::default();
    meta.repo_sha = repo_sha(&repo);
    meta.repo_root = repo;
    meta.min_severity_rank = output::severity_rank(&args.min_severity);
    meta.min_tier = if args.min_tier == "asserted" {
        prism::api::FindingTier::Asserted
    } else {
        prism::api::FindingTier::Candidate
    };
    meta.min_confidence = args.min_confidence;
    meta.resolution = args.resolution;
    meta
}

fn run_review(
    args: &TargetsArgs,
    diff: &Path,
    inventory: Option<&InventoryOptions>,
) -> Result<TargetsDocument> {
    const FINDING_ALGORITHMS: &[&str] = &[
        "echo",
        "absence",
        "contract",
        "provenance",
        "membrane",
        "taint",
        "symmetry",
        "peer_consistency",
        "callback_dispatcher",
        "primitive",
    ];
    const EMPTY_ALGORITHMS: &[&str] = &["angle", "delta"];
    const ACCEPTED: &str = "echo, absence, contract, provenance, membrane, taint, symmetry, peer_consistency, callback_dispatcher, primitive, angle, delta";

    // The acceptance table is deliberately evaluated before canonicalizing the
    // repo, reading the diff, or building a CPG.
    let requested: Vec<String> = args
        .algorithm
        .split(',')
        .map(|name| name.trim().to_lowercase())
        .collect();
    for name in &requested {
        if name == "chop" || name == "conditioned" {
            anyhow::bail!(
                "targets: algorithm {name} requires --chop-source/--chop-sink (or --condition); use the top-level command"
            );
        }
        if !FINDING_ALGORITHMS.contains(&name.as_str())
            && !EMPTY_ALGORITHMS.contains(&name.as_str())
        {
            anyhow::bail!(
                "targets: algorithm {name} produces slice blocks, not findings; accepted: {ACCEPTED}"
            );
        }
        if name == "delta" && args.old_repo.is_none() {
            anyhow::bail!("targets: delta requires --old-repo");
        }
    }
    for name in &requested {
        if EMPTY_ALGORITHMS.contains(&name.as_str()) {
            eprintln!("targets: {name} produces no findings at this version");
        }
    }

    let algorithms = prism_review::api::parse_algorithms(&requested.join(","))?;
    let repo = canonical_repo(args)?;
    let diff_text =
        fs::read_to_string(diff).with_context(|| format!("Failed to read diff: {:?}", diff))?;

    let mut review_options = prism::api::ReviewOptions::new(&repo);
    review_options.files_filter = args.files.as_ref().map(|files| {
        files
            .split(',')
            .map(|file| file.trim().to_string())
            .collect()
    });
    review_options.compile_commands = args.compile_commands.clone();
    review_options.scoped_cpg = args.scoped_cpg;
    review_options.cache_dir = args.cache_dir.clone();
    review_options.no_cache = args.no_cache;
    review_options.min_confidence = args.min_confidence;
    review_options.resolution = args.resolution;

    let mut algorithm_params = prism_review::api::AlgorithmParams::default();
    algorithm_params.old_repo = args.old_repo.clone();
    algorithm_params.taint_context = args.taint_context.options();
    let config = SliceConfig {
        algorithm: algorithms[0],
        scoped_cpg: args.scoped_cpg,
        ..SliceConfig::default()
    };
    let outcome = prism_review::api::review(
        &review_options,
        &diff_text,
        &algorithms,
        &config,
        &algorithm_params,
    )?;

    let mut run_warnings = outcome.inputs.parse_warnings.clone();
    run_warnings.extend(outcome.inputs.load_warnings.clone());
    run_warnings.extend(outcome.build_warnings.clone());
    if let Some(diagnostics) = &outcome.run.diagnostics {
        run_warnings.extend(diagnostics.messages());
    }
    let mut meta = targets_meta(args, repo);
    meta.algorithms_run = outcome.run.algorithms_run.clone();
    meta.errors = outcome.run.errors.clone();
    meta.run_warnings = run_warnings;
    meta.min_confidence = outcome.min_confidence;
    meta.resolution = outcome.resolution;
    let document = prism_review::targets::project_in_context(
        &outcome.run.findings,
        &outcome.run.evidence,
        &outcome.inputs,
        outcome.run.context(),
        &meta,
    );

    if let Some(options) = inventory {
        let out = collect_inventory(args, &meta.repo_root, options)?;
        Ok(targets::merge_inventory(document, &meta, &out))
    } else {
        Ok(document)
    }
}

fn write_document(args: &TargetsArgs, document: &TargetsDocument) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(document)?;
    bytes.push(b'\n');
    if let Some(path) = &args.out {
        fs::write(path, &bytes).with_context(|| format!("Failed to write targets: {path:?}"))?;
    } else {
        use std::io::Write;
        std::io::stdout().lock().write_all(&bytes)?;
    }

    let exit_code = targets_exit_code(args.strict, &document.errors);
    if exit_code != 0 {
        prism::profile::exit(exit_code);
    }
    Ok(())
}

fn repo_sha(repo: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim();
    (!sha.is_empty()).then(|| sha.to_string())
}
