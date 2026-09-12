//! Binding-enumeration census support.
//!
//! Task 6a provides the path-confidence delta mode. Later enumeration tasks add
//! the grammar census and edge-label delta modes.

use prism::cpg::{CodePropertyGraph, FlowConfidence};
use prism::data_flow::VarLocation;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXPECTED_DFG_FIXTURES: usize = 57;

type AnyResult<T> = Result<T, Box<dyn Error>>;

fn location_key(location: &VarLocation) -> String {
    format!(
        "{}:{}:{}:{}:{}:{:?}",
        location.file,
        location.function,
        location.function_start_line,
        location.line,
        location.path,
        location.kind
    )
}

fn path_key(from: &VarLocation, to: &VarLocation) -> String {
    format!("{} -> {}", location_key(from), location_key(to))
}

fn confidence_value(confidence: FlowConfidence) -> Value {
    serde_json::to_value(confidence).expect("FlowConfidence must serialize")
}

fn dfg_fixture_dirs(corpus: &Path) -> AnyResult<Vec<(String, PathBuf)>> {
    let mut fixtures = Vec::new();
    for language_entry in fs::read_dir(corpus)? {
        let language_entry = language_entry?;
        if !language_entry.file_type()?.is_dir() {
            continue;
        }
        let language = language_entry.file_name().to_string_lossy().into_owned();
        for fixture_entry in fs::read_dir(language_entry.path())? {
            let fixture_entry = fixture_entry?;
            let fixture_name = fixture_entry.file_name().to_string_lossy().into_owned();
            if fixture_entry.file_type()?.is_dir() && fixture_name.starts_with("dfg_") {
                fixtures.push((format!("{language}/{fixture_name}"), fixture_entry.path()));
            }
        }
    }
    fixtures.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(fixtures)
}

fn fixture_paths(fixture: &Path) -> AnyResult<BTreeMap<String, Value>> {
    let repo = prism::repo_loader::load_repo(fixture)?;
    let cpg = prism::build_pool::install(|| CodePropertyGraph::build(&repo.files));
    let defs: BTreeSet<VarLocation> = cpg.dfg.defs.values().flatten().cloned().collect();
    let mut paths = BTreeMap::new();

    for from in defs {
        let (reachable, labels) = cpg.dfg_forward_reachable_labeled(&from);
        for to in reachable {
            let target = cpg.var_node_for_location(&to).ok_or_else(|| {
                format!("reachable target has no CPG node: {}", location_key(&to))
            })?;
            let confidence = labels.get(&target).copied().ok_or_else(|| {
                format!("reachable target has no path label: {}", location_key(&to))
            })?;
            paths.insert(path_key(&from, &to), confidence_value(confidence));
        }
    }

    Ok(paths)
}

fn path_snapshot(corpus: &Path) -> AnyResult<Value> {
    let fixture_dirs = dfg_fixture_dirs(corpus)?;
    let mut fixtures = BTreeMap::new();
    let mut errors = Vec::new();

    if fixture_dirs.len() != EXPECTED_DFG_FIXTURES {
        errors.push(format!(
            "expected {EXPECTED_DFG_FIXTURES} dfg fixtures, found {}",
            fixture_dirs.len()
        ));
    }

    for (case, path) in fixture_dirs {
        match fixture_paths(&path) {
            Ok(paths) if paths.is_empty() => {
                errors.push(format!("{case}: zero traversed paths"));
                fixtures.insert(case, json!({"paths": {}}));
            }
            Ok(paths) => {
                fixtures.insert(case, json!({"paths": paths}));
            }
            Err(error) => {
                errors.push(format!("{case}: {error}"));
                fixtures.insert(case, json!({"paths": {}}));
            }
        }
    }

    Ok(json!({
        "fixture_count": fixtures.len(),
        "fixtures": fixtures,
        "errors": errors,
    }))
}

fn command_output(mut command: Command, label: &str) -> AnyResult<Output> {
    command
        .output()
        .map_err(|error| format!("{label}: {error}").into())
}

/// Build this exact source against the base revision without writing into the
/// read-only base clone. All Cargo products stay below target/path-delta.
fn build_base_adapter(workspace: &Path, base_repo: &Path) -> AnyResult<PathBuf> {
    let build_root = workspace.join("target/path-delta/base-adapter-build");
    let adapter_crate = workspace.join("target/path-delta/base-adapter-crate");
    let adapter_src = adapter_crate.join("src");
    fs::create_dir_all(&adapter_src)?;
    fs::write(
        adapter_crate.join("Cargo.toml"),
        format!(
            "[package]\nname = \"binding-kinds-base-adapter\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\nprism = {{ path = {:?} }}\nserde_json = \"1\"\nsha2 = \"0.10\"\n",
            base_repo
        ),
    )?;
    fs::write(
        adapter_src.join("main.rs"),
        fs::read(workspace.join("examples/binding_kinds_census.rs"))?,
    )?;

    let mut cargo = Command::new("cargo");
    cargo.args(["build", "--release", "--offline", "--manifest-path"]);
    cargo
        .arg(adapter_crate.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&build_root);
    let output = command_output(cargo, "build base adapter")?;
    if !output.status.success() {
        return Err(format!(
            "base adapter build failed ({}); stdout: {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(build_root.join("release/binding-kinds-base-adapter"))
}

fn run_base_snapshot(adapter: &Path, corpus: &Path) -> AnyResult<Value> {
    let mut command = Command::new(adapter);
    command.arg("--path-snapshot").arg(corpus);
    let output = command_output(command, "run base adapter")?;
    let snapshot: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "base adapter returned invalid JSON ({}): {error}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    let reported_errors = snapshot
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty());
    if !output.status.success() && !reported_errors {
        return Err(format!(
            "base adapter failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(snapshot)
}

fn fixture_path_map<'a>(
    snapshot: &'a Value,
    case: &str,
) -> Option<&'a serde_json::Map<String, Value>> {
    snapshot
        .get("fixtures")?
        .get(case)?
        .get("paths")?
        .as_object()
}

fn compare_snapshots(base: &Value, branch: &Value) -> Vec<Value> {
    let base_fixtures = base
        .get("fixtures")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let branch_fixtures = branch
        .get("fixtures")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let cases: BTreeSet<_> = base_fixtures
        .keys()
        .chain(branch_fixtures.keys())
        .cloned()
        .collect();
    let mut changes = Vec::new();

    for case in cases {
        let base_paths = fixture_path_map(base, &case).cloned().unwrap_or_default();
        let branch_paths = fixture_path_map(branch, &case).cloned().unwrap_or_default();
        let paths: BTreeSet<_> = base_paths
            .keys()
            .chain(branch_paths.keys())
            .cloned()
            .collect();
        for path in paths {
            if base_paths.get(&path) != branch_paths.get(&path) {
                changes.push(json!({
                    "matrix_case": case,
                    "path": path,
                    "base": base_paths.get(&path),
                    "branch": branch_paths.get(&path),
                }));
            }
        }
    }
    changes
}

fn cli_case_paths(workspace: &Path, corpus: &Path, case: &str) -> (PathBuf, PathBuf) {
    (
        corpus.join(case),
        workspace
            .join("target/path-delta")
            .join(format!("{case}.diff")),
    )
}

fn is_source_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some(
            "c" | "cc"
                | "cpp"
                | "cs"
                | "cxx"
                | "go"
                | "h"
                | "hpp"
                | "java"
                | "js"
                | "jsx"
                | "php"
                | "py"
                | "rb"
                | "rs"
                | "ts"
                | "tsx"
        )
    )
}

fn collect_source_paths(root: &Path, dir: &Path, paths: &mut Vec<PathBuf>) -> AnyResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_source_paths(root, &entry.path(), paths)?;
        } else if file_type.is_file() && is_source_path(&entry.path()) {
            paths.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn generate_fixture_diff(fixture: &Path, output_path: &Path) -> AnyResult<()> {
    let mut paths = Vec::new();
    collect_source_paths(fixture, fixture, &mut paths)?;
    paths.sort();
    if paths.is_empty() {
        return Err(format!(
            "fixture has no recognized source files: {}",
            fixture.display()
        )
        .into());
    }
    let files: Vec<Value> = paths
        .into_iter()
        .map(|path| {
            let line_count = fs::read_to_string(fixture.join(&path))?.lines().count();
            Ok(json!({
                "file_path": path,
                "modify_type": "Modified",
                "diff_lines": (1..=line_count.max(1)).collect::<Vec<_>>(),
            }))
        })
        .collect::<AnyResult<_>>()?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output_path,
        serde_json::to_vec_pretty(&json!({"files": files}))?,
    )?;
    Ok(())
}

fn run_review(binary: &Path, fixture: &Path, diff: &Path) -> AnyResult<Output> {
    let mut command = Command::new(binary);
    command
        .arg("--repo")
        .arg(fixture)
        .arg("--diff")
        .arg(diff)
        .args([
            "--format",
            "json",
            "--algorithm",
            "taint",
            "--resolution",
            "scoped",
            "--no-cache",
        ]);
    command_output(command, "run review")
}

fn cli_delta(
    workspace: &Path,
    corpus: &Path,
    base_binary: &Path,
    current_binary: &Path,
    cases: impl Iterator<Item = String>,
) -> Value {
    let mut changes = Vec::new();
    let mut errors = Vec::new();
    let mut selected_case_count = 0;
    let mut base_case_count = 0;
    let mut branch_case_count = 0;
    let mut compared_case_count = 0;

    for case in cases {
        selected_case_count += 1;
        let (fixture, diff) = cli_case_paths(workspace, corpus, &case);
        if let Err(error) = generate_fixture_diff(&fixture, &diff) {
            errors.push(format!("{case}: {error}"));
            continue;
        }
        let base = run_review(base_binary, &fixture, &diff);
        let branch = run_review(current_binary, &fixture, &diff);
        base_case_count += usize::from(base.is_ok());
        branch_case_count += usize::from(branch.is_ok());
        match (base, branch) {
            (Ok(base), Ok(branch)) => {
                compared_case_count += 1;
                let base_status = base.status.code();
                let branch_status = branch.status.code();
                if !base.status.success() || !branch.status.success() {
                    errors.push(format!(
                        "{case}: base status {base_status:?}, branch status {branch_status:?}; base stderr: {}; branch stderr: {}",
                        String::from_utf8_lossy(&base.stderr),
                        String::from_utf8_lossy(&branch.stderr)
                    ));
                }
                if base_status != branch_status
                    || base.stdout != branch.stdout
                    || base.stderr != branch.stderr
                {
                    changes.push(json!({
                        "matrix_case": case,
                        "base_status": base_status,
                        "branch_status": branch_status,
                        "base_stdout": String::from_utf8_lossy(&base.stdout),
                        "branch_stdout": String::from_utf8_lossy(&branch.stdout),
                        "base_stderr": String::from_utf8_lossy(&base.stderr),
                        "branch_stderr": String::from_utf8_lossy(&branch.stderr),
                    }));
                }
            }
            (base, branch) => errors.push(format!(
                "{case}: base={}, branch={}",
                base.err()
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "ok".to_string()),
                branch
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "ok".to_string())
            )),
        }
    }

    json!({
        "selected_case_count": selected_case_count,
        "base_case_count": base_case_count,
        "branch_case_count": branch_case_count,
        "compared_case_count": compared_case_count,
        "changes": changes,
        "errors": errors,
    })
}

fn snapshot_errors(snapshot: &Value, side: &str) -> Vec<String> {
    snapshot
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|error| format!("{side}: {error}"))
        .collect()
}

fn snapshot_cases(snapshot: &Value) -> BTreeSet<String> {
    snapshot
        .get("fixtures")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|fixtures| fixtures.keys().cloned())
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct PathDeltaOptions {
    base_binary: PathBuf,
    corpus: PathBuf,
}

fn parse_path_delta_options(
    base_binary: OsString,
    mut args: impl Iterator<Item = OsString>,
    workspace: &Path,
) -> AnyResult<PathDeltaOptions> {
    let mut corpus = workspace.join("eval/fixtures");
    if let Some(flag) = args.next() {
        if flag != OsStr::new("--corpus") {
            return Err(usage().into());
        }
        corpus = args.next().ok_or_else(|| usage().to_string())?.into();
    }
    if args.next().is_some() {
        return Err(usage().into());
    }
    Ok(PathDeltaOptions {
        base_binary: base_binary.into(),
        corpus,
    })
}

#[derive(Debug)]
struct BaseIdentity {
    binary: PathBuf,
    binary_sha256: String,
    version: String,
    version_git_sha: String,
    source_repo: PathBuf,
    source_git_sha: String,
}

fn git_output(repo: &Path, args: &[&str]) -> AnyResult<Output> {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo).args(args);
    command_output(command, "inspect base source checkout")
}

fn resolve_base_source(workspace: &Path, version_git_sha: &str) -> AnyResult<(PathBuf, String)> {
    let parent = workspace
        .parent()
        .ok_or_else(|| format!("workspace has no parent: {}", workspace.display()))?;
    let mut matches = Vec::new();
    for entry in fs::read_dir(parent)? {
        let path = entry?.path();
        if !path.is_dir() || !path.join(".git").exists() {
            continue;
        }
        let head = git_output(&path, &["rev-parse", "HEAD"])?;
        if !head.status.success() {
            continue;
        }
        let head = String::from_utf8(head.stdout)?.trim().to_string();
        if !head.starts_with(version_git_sha) {
            continue;
        }
        let status = git_output(&path, &["status", "--porcelain"])?;
        if !status.status.success() || !status.stdout.is_empty() {
            continue;
        }
        matches.push((path.canonicalize()?, head));
    }
    matches.sort_by(|left, right| left.0.cmp(&right.0));
    match matches.first() {
        Some((path, head)) => Ok((path.clone(), head.clone())),
        None => Err(format!(
            "no clean sibling checkout matches base binary git SHA {version_git_sha}"
        )
        .into()),
    }
}

fn base_identity(workspace: &Path, base_binary: &Path) -> AnyResult<BaseIdentity> {
    let binary = base_binary.canonicalize()?;
    let output = command_output(
        {
            let mut command = Command::new(&binary);
            command.arg("--version");
            command
        },
        "read base binary version",
    )?;
    if !output.status.success() {
        return Err(format!(
            "base binary --version failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let version = String::from_utf8(output.stdout)?.trim().to_string();
    let version_git_sha = version
        .rsplit_once('(')
        .and_then(|(_, suffix)| suffix.strip_suffix(')'))
        .filter(|sha| sha.len() >= 7 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| format!("base binary version has no git SHA: {version:?}"))?
        .to_string();
    let mut hasher = Sha256::new();
    hasher.update(fs::read(&binary)?);
    let binary_sha256 = format!("{:x}", hasher.finalize());
    let (source_repo, source_git_sha) = resolve_base_source(workspace, &version_git_sha)?;
    Ok(BaseIdentity {
        binary,
        binary_sha256,
        version,
        version_git_sha,
        source_repo,
        source_git_sha,
    })
}

fn path_delta_with_current_binary(
    base_binary: &Path,
    corpus: &Path,
    current_binary: &Path,
) -> AnyResult<Value> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let identity = base_identity(workspace, base_binary)?;
    let corpus = corpus.canonicalize()?;
    let branch = path_snapshot(&corpus)?;
    let base_adapter = build_base_adapter(workspace, &identity.source_repo)?;
    let base = run_base_snapshot(&base_adapter, &corpus)?;
    let adapter_changes = compare_snapshots(&base, &branch);
    let mut errors = snapshot_errors(&base, "base");
    errors.extend(snapshot_errors(&branch, "branch"));
    let base_cases = snapshot_cases(&base);
    let branch_cases = snapshot_cases(&branch);
    let adapter_compared_case_count = base_cases.intersection(&branch_cases).count();
    if adapter_compared_case_count != branch_cases.len() {
        errors.push(format!(
            "adapter compared {adapter_compared_case_count} of {} selected cases",
            branch_cases.len()
        ));
    }
    let cli = cli_delta(
        workspace,
        &corpus,
        &identity.binary,
        current_binary,
        branch_cases.iter().cloned(),
    );
    errors.extend(
        cli.get("errors")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|error| format!("cli: {error}")),
    );
    let cli_compared_case_count = cli
        .get("compared_case_count")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    if cli_compared_case_count != branch_cases.len() {
        errors.push(format!(
            "cli compared {cli_compared_case_count} of {} selected cases",
            branch_cases.len()
        ));
    }

    Ok(json!({
        "mode": "path-delta",
        "base": {
            "binary": identity.binary,
            "binary_sha256": identity.binary_sha256,
            "version": identity.version,
            "version_git_sha": identity.version_git_sha,
            "source_repo": identity.source_repo,
            "source_git_sha": identity.source_git_sha,
        },
        "corpus": corpus,
        "adapter": {
            "base_fixture_count": base.get("fixture_count"),
            "branch_fixture_count": branch.get("fixture_count"),
            "compared_case_count": adapter_compared_case_count,
            "changes": adapter_changes,
        },
        "cli": {
            "route": "prism --repo <fixture> --diff <generated> --format json --algorithm taint --resolution scoped --no-cache",
            "selected_case_count": cli.get("selected_case_count"),
            "base_case_count": cli.get("base_case_count"),
            "branch_case_count": cli.get("branch_case_count"),
            "compared_case_count": cli.get("compared_case_count"),
            "changes": cli.get("changes"),
            "errors": cli.get("errors"),
        },
        "errors": errors,
    }))
}

fn path_delta(base_binary: &Path, corpus: &Path) -> AnyResult<Value> {
    let current_binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/prism");
    path_delta_with_current_binary(base_binary, corpus, &current_binary)
}

fn usage() -> &'static str {
    "usage: binding_kinds_census --path-delta <base-prism-binary> [--corpus <eval/fixtures>]\n       binding_kinds_census --path-snapshot <eval/fixtures>"
}

fn run_with_current_binary(
    mut args: impl Iterator<Item = OsString>,
    current_binary: Option<&Path>,
) -> AnyResult<Value> {
    let mode = args.next().ok_or_else(|| usage().to_string())?;
    match mode.to_str() {
        Some("--path-snapshot") => {
            let corpus = args.next().ok_or_else(|| usage().to_string())?;
            if args.next().is_some() {
                return Err(usage().into());
            }
            path_snapshot(Path::new(&corpus))
        }
        Some("--path-delta") => {
            let base_binary = args.next().ok_or_else(|| usage().to_string())?;
            let options =
                parse_path_delta_options(base_binary, args, Path::new(env!("CARGO_MANIFEST_DIR")))?;
            match current_binary {
                Some(current_binary) => path_delta_with_current_binary(
                    &options.base_binary,
                    &options.corpus,
                    current_binary,
                ),
                None => path_delta(&options.base_binary, &options.corpus),
            }
        }
        _ => Err(usage().into()),
    }
}

fn main() -> AnyResult<()> {
    let value = run_with_current_binary(std::env::args_os().skip(1), None)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    if value
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty())
    {
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy_fixture(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let destination = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_fixture(&entry.path(), &destination);
            } else {
                fs::copy(entry.path(), destination).unwrap();
            }
        }
    }

    #[test]
    fn path_delta_reports_the_exact_changed_result_and_matrix_case() {
        let base = json!({"fixtures": {"rust/dfg_case": {"paths": {
            "a -> b": "AliasUnstable", "same": "Exact"
        }}}});
        let branch = json!({"fixtures": {"rust/dfg_case": {"paths": {
            "a -> b": "SameLine", "same": "Exact"
        }}}});
        assert_eq!(
            compare_snapshots(&base, &branch),
            vec![json!({
                "matrix_case": "rust/dfg_case",
                "path": "a -> b",
                "base": "AliasUnstable",
                "branch": "SameLine",
            })]
        );
    }

    #[test]
    fn alternate_path_delta_arguments_bind_both_legs_exactly() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
        let scratch = tempfile::Builder::new()
            .prefix("task6a-external-corpus-")
            .tempdir_in(workspace.join("target"))
            .unwrap();
        let corpus = scratch.path().join("fixtures");
        for (language, fixture) in [
            ("rust", "dfg_reaching_shadowed_inner"),
            ("go", "dfg_reaching_killed_def"),
        ] {
            copy_fixture(
                &workspace.join("eval/fixtures").join(language).join(fixture),
                &corpus.join(language).join(fixture),
            );
        }
        let base_binary = Path::new("/Users/wesleyjinks/code/tools/bin/prism-base-afc7814");
        let value = run_with_current_binary(
            [
                OsString::from("--path-delta"),
                base_binary.as_os_str().to_owned(),
                OsString::from("--corpus"),
                corpus.as_os_str().to_owned(),
            ]
            .into_iter(),
            Some(base_binary),
        )
        .unwrap();

        assert_eq!(
            value["base"]["binary"],
            json!(base_binary.canonicalize().unwrap())
        );
        assert_eq!(value["base"]["version_git_sha"], "afc78147fd88");
        assert_eq!(value["corpus"], json!(corpus.canonicalize().unwrap()));
        assert_eq!(value["adapter"]["base_fixture_count"], 2);
        assert_eq!(value["adapter"]["branch_fixture_count"], 2);
        assert_eq!(value["adapter"]["compared_case_count"], 2);
        assert_eq!(value["cli"]["selected_case_count"], 2);
        assert_eq!(value["cli"]["base_case_count"], 2);
        assert_eq!(value["cli"]["branch_case_count"], 2);
        assert_eq!(value["cli"]["compared_case_count"], 2);
        assert_eq!(value["cli"]["errors"], json!([]));
    }
}
