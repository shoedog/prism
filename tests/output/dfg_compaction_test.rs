#[path = "../common/mod.rs"]
mod common;
use common::dfg_roundtrip::{cpg_bytes, lookup_indexes, observe_evidence};
use common::*;
#[path = "dfg_compaction/controls.rs"]
mod controls;

use prism::api::{
    load_review_inputs, run_review, AlgorithmParams, ReviewInputs, ReviewOptions, ReviewRun,
};
use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
use std::process::Command;

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env_remove("GIT_AUTHOR_DATE")
        .env_remove("GIT_COMMITTER_DATE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
pub(super) fn write_sources(repo: &Path, files: &BTreeMap<String, ParsedFile>) {
    for (path, parsed) in files {
        let p = repo.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, &parsed.source).unwrap();
    }
}
fn committed_repo(files: &BTreeMap<String, ParsedFile>) -> TempDir {
    let repo = TempDir::new().unwrap();
    write_sources(repo.path(), files);
    git(repo.path(), &["init", "--quiet"]);
    for (k, v) in [
        ("user.email", "test@test.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
    ] {
        git(repo.path(), &["config", k, v]);
    }
    git(repo.path(), &["add", "--", "."]);
    git(
        repo.path(),
        &["commit", "--quiet", "-m", "DFG cache fixture"],
    );
    let committed: BTreeSet<_> = git(repo.path(), &["ls-files"])
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(committed, files.keys().cloned().collect());
    repo
}
pub(super) fn loaded(
    repo: &Path,
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> ReviewInputs {
    // Loader must parse every case file, including unchanged companions.
    let mut complete = diff.clone();
    for file in files.keys() {
        if !complete.files.iter().any(|d| &d.file_path == file) {
            complete.files.push(DiffInfo {
                file_path: file.clone(),
                modify_type: ModifyType::Modified,
                diff_lines: BTreeSet::new(),
            });
        }
    }
    let inputs =
        load_review_inputs(&ReviewOptions::new(repo), &complete.to_json().unwrap()).unwrap();
    assert_eq!(
        inputs.files.keys().collect::<Vec<_>>(),
        files.keys().collect::<Vec<_>>()
    );
    inputs
}
pub(super) fn params(files: &BTreeMap<String, ParsedFile>, old: &Path) -> AlgorithmParams {
    let (path, parsed) = files.first_key_value().unwrap();
    let function = parsed
        .all_functions()
        .first()
        .map(|f| parsed.node_line_range(f));
    let (start, end) = function.unwrap_or((1, parsed.source.lines().count().max(1)));
    let g = DataFlowGraph::build(files);
    let endpoints = g.edges().iter().find(|e| &e.from.file == path);
    let (start, end) = endpoints
        .map(|e| (e.from.line, e.to.line))
        .unwrap_or((start, end));
    let variable = g
        .defs()
        .map(|(k, _)| k)
        .next()
        .map(|k| k.3.base.clone())
        .unwrap_or_else(|| "x".into());
    let mut p = AlgorithmParams::default();
    p.chop_source = Some(format!("{path}:{start}"));
    p.chop_sink = Some(format!("{path}:{end}"));
    p.condition = Some("x==5".into());
    p.old_repo = Some(old.into());
    p.taint_sources = vec![format!("{path}:{start}")];
    p.quantum_var = Some(variable);
    p.peer_pattern = Some("name:f".into());
    p.layers = Some("api,service,storage".into());
    p.concern = Some("error".into());
    p
}
pub(super) fn output_pair(old: &ReviewRun, new: &ReviewRun, inputs: &ReviewInputs, repo: &Path) {
    use prism::output::{to_review_output, to_sarif, SarifInputs};
    assert!(old.errors.is_empty(), "baseline errors: {:?}", old.errors);
    assert!(new.errors.is_empty(), "candidate errors: {:?}", new.errors);
    assert_eq!(old.algorithms_run, new.algorithms_run);
    assert_eq!(old.results.len(), 1);
    assert_eq!(new.results.len(), 1);
    for (a, b) in old.results.iter().zip(&new.results) {
        assert_eq!(
            serde_json::to_vec(a).unwrap(),
            serde_json::to_vec(b).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&to_review_output(a, &inputs.sources)).unwrap(),
            serde_json::to_vec(&to_review_output(b, &inputs.sources)).unwrap()
        );
    }
    let sarif = |v: &ReviewRun| {
        to_sarif(
            &SarifInputs::new(&v.findings)
                .evidence(&v.evidence)
                .files(&inputs.files)
                .sources(&inputs.sources)
                .algorithms_run(&v.algorithms_run)
                .errors(&v.errors)
                .parse_quality(&inputs.parse_quality),
        )
    };
    assert_eq!(
        serde_json::to_vec(&sarif(old)).unwrap(),
        serde_json::to_vec(&sarif(new)).unwrap()
    );
    let mut meta = prism::targets::TargetsMeta::default();
    meta.repo_root = repo.into();
    meta.algorithms_run = new.algorithms_run.clone();
    let targets = |v: &ReviewRun| {
        serde_json::to_vec(&prism::targets::project(
            &v.findings,
            &v.evidence,
            inputs,
            &meta,
        ))
        .unwrap()
    };
    assert_eq!(targets(old), targets(new));
    assert_eq!(
        observe_evidence(&old.evidence),
        observe_evidence(&new.evidence)
    );
}
pub(super) fn roundtrip(g: &CodePropertyGraph, inputs: &ReviewInputs) -> CodePropertyGraph {
    let dir = TempDir::new().unwrap();
    let hashes = compute_file_hashes(&inputs.sources);
    assert_eq!(
        save_cache(g, &hashes, false, dir.path()).unwrap(),
        prism::cpg_cache::SaveOutcome::Saved
    );
    match load_cache(&hashes, false, dir.path()) {
        CacheResult::Hit(decoded) => {
            // Check every decode before refresh can rebuild away a codec defect.
            // Hit dispatch additionally binds both sides to the cold production graph.
            assert_cache_matches(g, &decoded, &decoded);
            decoded
        }
        _ => panic!("expected actual cache hit"),
    }
}
pub(super) fn refresh(
    g: CodePropertyGraph,
    files: &BTreeMap<String, ParsedFile>,
) -> CodePropertyGraph {
    let all = files.keys().cloned().collect();
    CodePropertyGraph::build_incremental(g.call_graph, g.dfg, &all, files, None)
}
fn dispatch_case(name: &str, files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) {
    let repo = committed_repo(files);
    let inputs = loaded(repo.path(), files, diff);
    let old_repo = TempDir::new().unwrap();
    let identical = TempDir::new().unwrap();
    write_sources(identical.path(), files);
    let mut old_files = BTreeMap::new();
    for (path, parsed) in files {
        let changed = diff
            .files
            .iter()
            .find(|d| &d.file_path == path)
            .map(|d| &d.diff_lines);
        let source = parsed
            .source
            .lines()
            .enumerate()
            .map(|(i, l)| {
                if changed.is_some_and(|c| c.contains(&(i + 1))) {
                    ""
                } else {
                    l
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        old_files.insert(
            path.clone(),
            ParsedFile::parse(path, &source, parsed.language).unwrap(),
        );
    }
    write_sources(old_repo.path(), &old_files);
    assert_eq!(old_files.len(), files.len());
    let p = params(files, old_repo.path());
    let cold = CodePropertyGraph::build(files);
    let cached = roundtrip(&cold, &inputs);
    let refreshed = refresh(roundtrip(&cold, &inputs), files);
    assert_cache_matches(&cold, &cached, &refreshed);
    let cold_ctx = CpgContext::build_with_cached_cpg(files, cold, None);
    // References depend on the case and params, not the candidate cache mode.
    // output_pair only borrows them, so every mode can share the same runs.
    let references: Vec<_> = SlicingAlgorithm::all()
        .into_iter()
        .map(|algorithm| {
            let config = SliceConfig::default().with_algorithm(algorithm);
            let reference = run_review(&cold_ctx, &inputs, &[algorithm], &config, &p, repo.path());
            (algorithm, reference)
        })
        .collect();
    let mut control = params(files, identical.path());
    control.taint_return_flow = true;
    let control_references: Vec<_> = [
        SlicingAlgorithm::DeltaSlice,
        SlicingAlgorithm::ContractSlice,
        SlicingAlgorithm::Taint,
    ]
    .into_iter()
    .map(|algorithm| {
        let config = SliceConfig::default().with_algorithm(algorithm);
        let reference = run_review(
            &cold_ctx,
            &inputs,
            &[algorithm],
            &config,
            &control,
            repo.path(),
        );
        (algorithm, reference)
    })
    .collect();
    for (mode, candidate) in [
        ("cold", CodePropertyGraph::build(files)),
        ("hit", cached),
        ("refresh-readd", refreshed),
    ] {
        let ctx = CpgContext::build_with_cached_cpg(files, candidate, None);
        let mut seen = BTreeSet::new();
        for (algorithm, reference) in &references {
            let algorithm = *algorithm;
            let config = SliceConfig::default().with_algorithm(algorithm);
            let b = run_review(&ctx, &inputs, &[algorithm], &config, &p, repo.path());
            assert_eq!(b.algorithms_run, [algorithm.name()]);
            output_pair(reference, &b, &inputs, repo.path());
            seen.insert(algorithm.name().to_string());
            eprintln!(
                "DFG_DISPATCH case={name} algorithm={} mode={mode} params={} success=true",
                algorithm.name(),
                serde_json::json!({"barrier_depth":p.barrier_depth,"barrier_symbols":p.barrier_symbols,"chop_source":p.chop_source,"chop_sink":p.chop_sink,"condition":p.condition,"old_repo":p.old_repo,"taint_sources":p.taint_sources,"taint_return_flow":p.taint_return_flow,"spiral_max_ring":p.spiral_max_ring,"quantum_var":p.quantum_var,"peer_pattern":p.peer_pattern,"layers":p.layers,"concern":p.concern,"temporal_days":p.temporal_days})
            );
        }
        assert_eq!(
            seen,
            SlicingAlgorithm::all()
                .iter()
                .map(|a| a.name().to_string())
                .collect()
        );
        for (algorithm, reference) in &control_references {
            let algorithm = *algorithm;
            let config = SliceConfig::default().with_algorithm(algorithm);
            let b = run_review(&ctx, &inputs, &[algorithm], &config, &control, repo.path());
            output_pair(reference, &b, &inputs, repo.path());
            if algorithm == SlicingAlgorithm::DeltaSlice {
                assert!(b.results[0].blocks.is_empty());
            }
        }
    }
}
macro_rules! dispatch {
    ($test:ident,$name:literal,$make:ident,normal) => {
        #[test]
        fn $test() {
            prism::build_pool::install(|| {
                let (f, _, d) = $make();
                dispatch_case($name, &f, &d);
            });
        }
    };
    ($test:ident,$name:literal,$make:ident,security) => {
        #[test]
        fn $test() {
            prism::build_pool::install(|| {
                let (f, d) = $make();
                dispatch_case($name, &f, &d);
            });
        }
    };
}
dispatch!(dispatch_python, "python", make_python_test, normal);
dispatch!(
    dispatch_javascript,
    "javascript",
    make_javascript_test,
    normal
);
dispatch!(dispatch_go, "go", make_go_test, normal);
dispatch!(dispatch_java, "java", make_java_test, normal);
dispatch!(
    dispatch_typescript,
    "typescript",
    make_typescript_test,
    normal
);
dispatch!(dispatch_c, "c", make_c_test, normal);
dispatch!(
    dispatch_c_multifile,
    "c_multifile",
    make_c_multifile_test,
    normal
);
dispatch!(dispatch_cpp, "cpp", make_cpp_test, normal);
dispatch!(dispatch_tsx, "tsx", make_tsx_test, normal);
dispatch!(dispatch_terraform, "terraform", make_terraform_test, normal);
dispatch!(dispatch_bash, "bash", make_bash_test, normal);
dispatch!(dispatch_snmp, "snmp", make_snmp_overflow_test, security);
dispatch!(
    dispatch_double_free,
    "double_free",
    make_double_free_test,
    security
);
dispatch!(dispatch_ring, "ring", make_ring_overflow_test, security);
dispatch!(dispatch_timer, "timer", make_timer_uaf_test, security);
dispatch!(dispatch_large, "large", make_large_function_test, security);
dispatch!(
    dispatch_deep_switch,
    "deep_switch",
    make_deep_switch_test,
    security
);
#[test]
fn dispatch_lua() {
    prism::build_pool::install(|| {
        explicit_dispatch(
            "compact.lua",
            "local function f(x)\n local y = x\n return y\nend\n",
            Language::Lua,
        );
    });
}
#[test]
fn dispatch_rust() {
    prism::build_pool::install(|| {
        explicit_dispatch(
            "compact.rs",
            "fn f(x: i32) -> i32 { let y = x; y }",
            Language::Rust,
        );
    });
}
fn explicit_dispatch(path: &str, source: &str, language: Language) {
    let files = common::dfg_cases::explicit(path, source, language);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([1]),
        }],
    };
    dispatch_case(path, &files, &diff);
}
#[test]
fn expected_dispatch_errors_are_not_successes() {
    prism::build_pool::install(|| {
        let repo = TempDir::new().unwrap();
        let (files, _, diff) = make_python_test();
        write_sources(repo.path(), &files);
        let inputs = loaded(repo.path(), &files, &diff);
        let ctx = CpgContext::build(&files, None);
        for (algorithm, fragment) in [
            (SlicingAlgorithm::Chop, "--chop-source required"),
            (SlicingAlgorithm::ConditionedSlice, "--condition required"),
            (SlicingAlgorithm::DeltaSlice, "--old-repo required"),
        ] {
            let run = run_review(
                &ctx,
                &inputs,
                &[algorithm],
                &SliceConfig::default(),
                &AlgorithmParams::default(),
                repo.path(),
            );
            assert!(run.results.is_empty());
            assert_eq!(run.errors.len(), 1);
            assert!(run.errors[0].error.contains(fragment), "{:?}", run.errors);
        }
        for (algorithm, p, fragment) in [
            (
                SlicingAlgorithm::Chop,
                {
                    let mut p = AlgorithmParams::default();
                    p.chop_source = Some("src/calc.py:1".into());
                    p
                },
                "--chop-sink required",
            ),
            (
                SlicingAlgorithm::Chop,
                {
                    let mut p = AlgorithmParams::default();
                    p.chop_source = Some("bad".into());
                    p.chop_sink = Some("a.py:1".into());
                    p
                },
                "file:line",
            ),
            (
                SlicingAlgorithm::ConditionedSlice,
                {
                    let mut p = AlgorithmParams::default();
                    p.condition = Some("???".into());
                    p
                },
                "Failed to parse condition",
            ),
        ] {
            let run = run_review(
                &ctx,
                &inputs,
                &[algorithm],
                &SliceConfig::default(),
                &p,
                repo.path(),
            );
            assert!(run.results.is_empty());
            assert_eq!(run.errors.len(), 1);
            assert!(run.errors[0].error.contains(fragment), "{:?}", run.errors);
        }
    });
}

#[test]
fn cache_comparator_rejects_identical_decoded_counter_corruption() {
    prism::build_pool::install(|| {
        let (files, _, diff) = make_python_test();
        let repo = committed_repo(&files);
        let inputs = loaded(repo.path(), &files, &diff);
        let cold = CodePropertyGraph::build(&files);
        let mut decoded = roundtrip(&cold, &inputs);
        assert_cache_matches(&cold, &decoded, &decoded);
        decoded.dfg_label_stats.dfg_label_exact += 1;
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                assert_cache_matches(&cold, &decoded, &decoded);
            }))
            .is_err(),
            "identical decoded counter corruption escaped the cold reference"
        );
    });
}

pub(super) fn assert_cache_matches(
    cold: &CodePropertyGraph,
    a: &CodePropertyGraph,
    b: &CodePropertyGraph,
) {
    let expected = cpg_bytes(cold);
    for decoded in [a, b] {
        assert!(
            expected == cpg_bytes(decoded),
            "cache payload differs from cold production"
        );
        assert_eq!(
            lookup_indexes(cold),
            lookup_indexes(decoded),
            "cache lookups differ from cold production"
        );
    }
}
