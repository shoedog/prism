use super::*;
use crate::algorithms::taint::{
    self,
    context_budget::{ContextLimits, TimeGuard},
    rounds::{expand, ExpansionRequest},
};
use crate::api::{
    self,
    context_options::TaintContextMode,
    context_route::{
        run_algorithm_authorized, run_review_authorized, run_review_legacy, ReviewAuthority,
    },
    AlgorithmParams, ReviewOptions,
};
use prism::{
    api::{ContextUnavailable, ReviewContextLoader},
    cpg::CrossingCeiling,
    finding_confidence::ResolutionMode,
    slice::{SliceConfig, SlicingAlgorithm},
};
use std::{path::Path, process::Command};

const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
const B: &str = "export function g(x) { eval(x); }\n";
const DIFF: &str =
    r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#;
const SELECTOR: &str = "api::context_stats::tests::profile::child";
fn imports() -> AlgorithmParams {
    let mut p = AlgorithmParams::default();
    p.taint_context.mode = TaintContextMode::Imports;
    p
}
fn exec_case(case: &str, profiling: bool) -> (serde_json::Value, BTreeMap<String, u64>) {
    let tmp = tempfile::tempdir().unwrap();
    let result = tmp.path().join("result.json");
    let profile = tmp.path().join("profile.jsonl");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", SELECTOR, "--nocapture"])
        .env("X_STATS_CASE", case)
        .env("X_STATS_RESULT", &result)
        .env_remove("PRISM_PROFILE");
    if profiling {
        command.env("PRISM_PROFILE", &profile);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let data = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
    let mut rows = BTreeMap::new();
    if profiling {
        for line in std::fs::read_to_string(profile).unwrap().lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            if v["record"] == "counter"
                && v["name"]
                    .as_str()
                    .unwrap()
                    .starts_with("review.taint_context.")
            {
                let name = v["name"].as_str().unwrap().to_owned();
                assert!(rows.insert(name, v["value"].as_u64().unwrap()).is_none());
            }
        }
    }
    (data, rows)
}
fn valid_names(rows: &BTreeMap<String, u64>) {
    let expected: BTreeSet<_> = ContextStats::default()
        .rows()
        .into_iter()
        .map(|(n, _)| n.to_owned())
        .collect();
    assert_eq!(rows.keys().cloned().collect::<BTreeSet<_>>(), expected);
}
fn deterministic(rows: &BTreeMap<String, u64>) -> BTreeMap<String, u64> {
    rows.iter()
        .filter(|(k, _)| !k.contains("elapsed_"))
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}
fn timing_bounds(rows: &BTreeMap<String, u64>) {
    let total = rows["review.taint_context.elapsed_total_ns"];
    for part in ["discovery", "read_parse", "build", "analysis"] {
        assert!(total >= rows[&format!("review.taint_context.elapsed_{part}_ns")]);
    }
}
fn unavailable(repo: &Path) {
    let options = ReviewOptions::new(repo.join("missing"));
    let inputs = api::load_review_inputs(&options, r#"{"files":[]}"#).unwrap();
    assert!(
        matches!(
            ReviewContextLoader::new(&inputs),
            Err(ContextUnavailable::RootUnavailable { .. })
        ),
        "inadmissible precondition: missing captured root"
    );
    let built = api::build_context(&inputs, &options).unwrap();
    let run = run_review_authorized(
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &imports(),
        repo,
    );
    let stats = run
        .stats()
        .expect("authorized request owns unavailable stats");
    assert_eq!(
        (
            stats.context_unavailable,
            stats.driver_stats_available,
            stats.union_stats_available
        ),
        (1, 0, 0)
    );
}
#[test]
fn child() {
    let Ok(case) = std::env::var("X_STATS_CASE") else {
        return;
    };
    let result = std::env::var_os("X_STATS_RESULT").unwrap();
    let tmp = tempfile::tempdir().unwrap();
    if case == "unavailable" {
        let session = prism::profile::Session::start();
        unavailable(tmp.path());
        drop(session);
        std::fs::write(result, "{}").unwrap();
        return;
    }
    for (name, src) in [("main.js", MAIN), ("a.js", A), ("b.js", B)] {
        std::fs::write(tmp.path().join(name), src).unwrap();
    }
    let mut options = ReviewOptions::new(tmp.path());
    options.no_cache = true;
    options.resolution = ResolutionMode::Scoped;
    let inputs = api::load_review_inputs(&options, DIFF).unwrap();
    assert!(
        inputs.files.contains_key("main.js"),
        "inadmissible precondition: original parsed"
    );
    let built = api::build_context(&inputs, &options).unwrap();
    if matches!(
        case.as_str(),
        "single" | "duplicate" | "no_profile" | "single_owner"
    ) {
        let loader = ReviewContextLoader::new(&inputs).unwrap();
        let tc = taint::TaintConfig::default();
        let (_, trace) = taint::slice_with_trace(&built.ctx, &inputs.diff, &tc).unwrap();
        let limits = ContextLimits::new(2, 64).unwrap();
        let guard = TimeGuard::new(limits.time);
        let direct = expand(ExpansionRequest {
            loader: &loader,
            inputs: &inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &tc,
            limits,
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        });
        assert!(
            direct.bundle.context_files.contains("b.js")
                && direct.bundle.analysis.as_ref().is_some_and(|a| a
                    .result
                    .findings
                    .iter()
                    .enumerate()
                    .any(|(i, f)| f.file == "b.js"
                        && f.category.as_deref() == Some("taint_sink")
                        && a.result
                            .evidence
                            .get(i)
                            .and_then(Option::as_ref)
                            .is_some_and(|e| !e.hops.is_empty()))),
            "inadmissible precondition: independent native context sink and aligned evidence"
        );
    }
    let session = prism::profile::Session::start();
    if case == "single_owner" {
        let run = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &SliceConfig::default(),
            &imports(),
            tmp.path(),
        )
        .unwrap();
        let stats = run.stats().expect("single requesting run owns stats");
        let context = run.context().expect("single returned completed context");
        assert_eq!(stats.admitted_files, context.published.files as u64);
        assert_eq!(stats.admitted_bytes, context.published.bytes);
        let data = serde_json::json!({"has_stats":true,"rows":stats.rows().into_iter().collect::<BTreeMap<_,_>>()});
        drop(session);
        std::fs::write(result, serde_json::to_vec(&data).unwrap()).unwrap();
        return;
    }
    let algos = if case == "duplicate" {
        vec![SlicingAlgorithm::Taint, SlicingAlgorithm::Taint]
    } else if case == "no_taint" || case == "ignored" {
        vec![SlicingAlgorithm::OriginalDiff]
    } else {
        vec![SlicingAlgorithm::Taint]
    };
    let mut params = if case == "off" {
        AlgorithmParams::default()
    } else {
        imports()
    };
    if case == "invalid" {
        params.taint_context.depth = 0;
    }
    let run = if case == "unauthorized" {
        run_review_legacy(
            &built.ctx,
            &inputs,
            &algos,
            &SliceConfig::default(),
            &params,
            tmp.path(),
        )
    } else {
        run_review_authorized(
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &algos,
            &SliceConfig::default(),
            &params,
            tmp.path(),
        )
    };
    if matches!(case.as_str(), "single" | "duplicate" | "no_profile") {
        let stats = run.stats().expect("requesting run owns stats");
        let context = run.context().expect("returned completed context");
        assert_eq!(stats.admitted_files, context.published.files as u64);
        assert_eq!(stats.admitted_bytes, context.published.bytes);
        assert_eq!(stats.admitted_depth, u64::from(context.round));
    }
    let data = match run.stats() {
        Some(s) => {
            serde_json::json!({"has_stats":true,"rows":s.rows().into_iter().collect::<BTreeMap<_,_>>()})
        }
        None => serde_json::json!({"has_stats":false}),
    };
    drop(session);
    std::fs::write(result, serde_json::to_vec(&data).unwrap()).unwrap();
}
#[test]
fn x09_authorized_profile_has_all_rows_and_actual_admission() {
    let (data, rows) = exec_case("single", true);
    valid_names(&rows);
    assert_eq!(data["has_stats"], true);
    assert!(
        rows["review.taint_context.admitted_files"] >= 2
            && rows["review.taint_context.rounds_completed"] >= 1
    );
    for (name, value) in &rows {
        assert_eq!(data["rows"][name].as_u64(), Some(*value));
    }
    let (single, single_rows) = exec_case("single_owner", true);
    valid_names(&single_rows);
    assert_eq!(single["has_stats"], true);
    for (name, value) in &single_rows {
        assert_eq!(single["rows"][name].as_u64(), Some(*value));
    }
}
#[test]
fn x10_duplicate_selection_exports_once() {
    let (_, one) = exec_case("single", true);
    let (_, two) = exec_case("duplicate", true);
    valid_names(&one);
    valid_names(&two);
    assert!(
        one["review.taint_context.attempted_files"] > 0
            && one["review.taint_context.admitted_files"] > 0
    );
    assert_eq!(deterministic(&one).len(), 34);
    assert_eq!(deterministic(&one), deterministic(&two));
    timing_bounds(&one);
    timing_bounds(&two);
}
#[test]
fn x11_off_and_no_taint_emit_nothing() {
    for case in ["off", "no_taint"] {
        let (data, rows) = exec_case(case, true);
        assert_eq!(data["has_stats"], false);
        assert!(rows.is_empty());
    }
}
#[test]
fn x12_stats_survive_absent_session() {
    let (_, profile) = exec_case("single", true);
    let (data, rows) = exec_case("no_profile", false);
    assert!(rows.is_empty());
    assert_eq!(data["has_stats"], true);
    let without: BTreeMap<String, u64> = serde_json::from_value(data["rows"].clone()).unwrap();
    assert_eq!(deterministic(&without), deterministic(&profile));
    let req = crate::api::context_run::ContextRequest {
        limits: ContextLimits::new(2, 64).unwrap(),
        ceiling: CrossingCeiling::NameOnly,
    };
    let mut pending_run = crate::api::context_run::RunExpansion::new(Some(&req), true);
    let pending = pending_run
        .take_stats()
        .expect("requested preapply state retains pending stats");
    assert!(pending_run.take_stats().is_none());
    assert!(
        crate::api::context_run::RunExpansion::new(Some(&req), false)
            .take_stats()
            .is_none()
    );
    assert!(crate::api::context_run::RunExpansion::new(None, true)
        .take_stats()
        .is_none());
    assert_eq!(
        (
            pending.driver_stats_available,
            pending.context_unavailable,
            pending.union_stats_available
        ),
        (0, 0, 0)
    );
}
#[test]
fn x13_elapsed_values_forward_and_native_parts_bounded() {
    let mut c = context();
    c.stats.elapsed_discovery_ns = 11;
    c.stats.elapsed_read_parse_ns = 22;
    c.stats.elapsed_build_ns = 33;
    c.stats.elapsed_analysis_ns = 44;
    c.stats.elapsed_total_ns = 100;
    let s = ContextStats::from_context(&c);
    assert_eq!(
        (
            s.elapsed_discovery_ns,
            s.elapsed_read_parse_ns,
            s.elapsed_build_ns,
            s.elapsed_analysis_ns,
            s.elapsed_total_ns
        ),
        (11, 22, 33, 44, 100)
    );
    let (_, rows) = exec_case("single", true);
    valid_names(&rows);
    timing_bounds(&rows);
}
#[test]
fn x14_non_authorized_and_unavailable_distinguished() {
    for case in ["unauthorized", "ignored", "invalid"] {
        let (data, rows) = exec_case(case, true);
        assert_eq!(data["has_stats"], false);
        assert!(rows.is_empty());
    }
    let (_, rows) = exec_case("unavailable", true);
    valid_names(&rows);
    assert_eq!(rows["review.taint_context.context_unavailable"], 1);
    assert_eq!(rows["review.taint_context.driver_stats_available"], 0);
}
