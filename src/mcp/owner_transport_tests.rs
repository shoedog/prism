//! Actual transport dispatch and response decoding; no fake proof producer.
use super::*;
use crate::api::OwnerOptions;
use crate::mcp::owner::OwnerRuntime;

fn fixture() -> (tempfile::TempDir, OwnerRuntime) {
    let root = tempfile::tempdir().unwrap();
    let corpus: Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    for (name, source) in corpus["files"].as_object().unwrap() {
        let file = root.path().join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, source.as_str().unwrap()).unwrap();
    }
    std::fs::write(root.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    std::fs::write(
        root.path().join("tsconfig.json"),
        json!({
            "compilerOptions":corpus["compiler_options"], "include":["src"]
        })
        .to_string(),
    )
    .unwrap();
    let mut cfg = crate::mcp::ServerConfig::new(root.path().to_owned());
    cfg.startup = StartupMode::Eager;
    cfg.cache = crate::mcp::CacheMode::NoCache;
    let owner = OwnerOptions::new(
        std::env::var("PRISM_TYPESCRIPT").expect("explicit compiler"),
        "tsconfig.json",
    );
    let runtime = OwnerRuntime::new(cfg, owner).unwrap();
    (root, runtime)
}

fn request(runtime: &mut OwnerRuntime, tool: &str, name: &str, file: &str) -> Value {
    let args = if tool == "refresh_index" {
        json!({})
    } else {
        json!({"seed":{"kind":"symbol","name":name,"file":file},"depth":1,"verbosity":"detailed"})
    };
    let message = json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":tool,"arguments":args}});
    let mut state = Lifecycle::Initialized;
    let Dispatch::Response(response) =
        handle_message(&message, runtime, &ToolRegistry::all_v1(), &mut state)
    else {
        panic!("response");
    };
    response["result"].clone()
}

fn payload(result: &Value) -> Value {
    result.get("structuredContent").cloned().unwrap_or_else(|| {
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
    })
}

#[test]
fn owner_admission_wire_reports_fresh_inputs_and_compiler_phase_without_stale_edges() {
    let (root, mut runtime) = fixture();
    assert_eq!(
        proof_items(
            &request(&mut runtime, "nav_callees", "run", "src/app.ts"),
            "src/client.ts"
        ),
        1
    );
    let extra = root.path().join("extra.py");
    std::fs::write(&extra, "def extra(): pass\n").unwrap();
    for tool in ["nav_callees", "nav_callers", "refresh_index"] {
        let refused = request(&mut runtime, tool, "run", "src/app.ts");
        assert_eq!(refused["isError"], true);
        let body = payload(&refused);
        assert_eq!(body["status"], "build_failed");
        assert!(body.get("items").is_none());
        let message = body["cause"].as_str().unwrap();
        assert_eq!(
            message,
            "owner acquisition failed: owner_requires_js_ts_only"
        );
        let r = &body["owner_admission"];
        assert_eq!(r["failed_phase"], "select_inputs");
        assert_eq!(r["inputs"]["loaded_files"], 4);
        assert_eq!(r["inputs"]["languages"]["Python"], 1);
        assert_eq!(r["phases"]["compiler_evidence"], "not_reached");
    }
    std::fs::remove_file(extra).unwrap();
    let config = root.path().join("tsconfig.json");
    let original = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, "{broken").unwrap();
    let refused = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    let body = payload(&refused);
    assert_eq!(refused["isError"], true);
    assert!(body.get("items").is_none());
    let r = &body["owner_admission"];
    assert_eq!(r["failed_phase"], "compiler_evidence");
    assert_eq!(r["phases"]["compiler_evidence"], "failed");
    assert_eq!(r["phases"]["owner_mapping"], "not_reached");
    assert_eq!(r["inputs"]["loaded_files"], 3);
    assert!(r["inputs"]["languages"].get("Python").is_none());
    std::fs::write(config, original).unwrap();
    assert_eq!(
        proof_items(
            &request(&mut runtime, "nav_callees", "run", "src/app.ts"),
            "src/client.ts"
        ),
        1
    );
    assert!(runtime.admission_diagnostic().is_none());
}

fn proof_items(result: &Value, file: &str) -> usize {
    assert_ne!(result["isError"], true, "{result}");
    payload(result)["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["location"]["file"] == file
                && item["score"] == 1.0
                && item["why"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|why| why["Resolution"]["kind"] == "contextual_owner")
        })
        .count()
}

#[test]
fn owner_wire_reacquires_a_b_unproven_and_config_failure_without_stale_edges() {
    let (root, mut runtime) = fixture();
    let a = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(proof_items(&a, "src/client.ts"), 1);
    let callers = request(&mut runtime, "nav_callers", "m", "src/client.ts");
    assert_eq!(proof_items(&callers, "src/app.ts"), 1);
    let app = std::fs::read_to_string(root.path().join("src/app.ts")).unwrap();
    std::fs::write(
        root.path().join("src/other.ts"),
        "export class Other {\n m(): number { return 2; }\n}\n",
    )
    .unwrap();
    let b_app = app
        .replace("Client", "Other")
        .replace("'./client'", "'./other'");
    std::fs::write(root.path().join("src/app.ts"), &b_app).unwrap();
    let changed_refresh = request(&mut runtime, "refresh_index", "", "");
    assert_eq!(payload(&changed_refresh)["stale_before_refresh"], true);
    assert!(
        payload(&changed_refresh)["stale_index_total_before_refresh"]
            .as_u64()
            .unwrap()
            > 0
    );
    let b = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(proof_items(&b, "src/other.ts"), 1);
    assert_eq!(proof_items(&b, "src/client.ts"), 0);
    std::fs::write(
        root.path().join("src/app.ts"),
        b_app.replace("client.m();", "client.m(); client.m();"),
    )
    .unwrap();
    let unproven = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(proof_items(&unproven, "src/other.ts"), 0);
    // A config-only change is invisible to ordinary source freshness; each-call
    // acquisition must nevertheless reject it, including repeated calls.
    let config = std::fs::read_to_string(root.path().join("tsconfig.json")).unwrap();
    std::fs::write(root.path().join("tsconfig.json"), "{broken").unwrap();
    for tool in ["nav_callees", "refresh_index", "nav_callers"] {
        let refused = request(&mut runtime, tool, "run", "src/app.ts");
        assert_eq!(refused["isError"], true, "{refused}");
        assert_eq!(payload(&refused)["status"], "build_failed");
        assert!(payload(&refused).get("items").is_none());
    }
    std::fs::write(root.path().join("tsconfig.json"), config).unwrap();
    std::fs::write(root.path().join("src/app.ts"), app).unwrap();
    let restored = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(proof_items(&restored, "src/client.ts"), 1);
    let refresh = request(&mut runtime, "refresh_index", "", "");
    assert_eq!(payload(&refresh)["status"], "refreshed");
    assert!(!root.path().join(".prism").exists());
}

#[test]
fn owner_wire_closure_refusal_and_dependency_restore_are_fresh() {
    let (root, mut runtime) = fixture();
    let dep = root.path().join("src/contract.ts");
    let original = std::fs::read_to_string(&dep).unwrap();
    std::fs::remove_file(&dep).unwrap();
    let refused = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(refused["isError"], true);
    assert_eq!(payload(&refused)["status"], "build_failed");
    std::fs::write(&dep, original).unwrap();
    assert_eq!(
        proof_items(
            &request(&mut runtime, "nav_callees", "run", "src/app.ts"),
            "src/client.ts"
        ),
        1
    );
    let unrelated = root.path().join("src/unrelated.d.ts");
    std::fs::write(&unrelated, "/// <reference types=\"react-scripts\" />\n").unwrap();
    let refused = request(&mut runtime, "nav_callees", "run", "src/app.ts");
    assert_eq!(refused["isError"], true);
    assert_eq!(payload(&refused)["status"], "build_failed");
    std::fs::remove_file(unrelated).unwrap();
    assert_eq!(
        proof_items(
            &request(&mut runtime, "nav_callees", "run", "src/app.ts"),
            "src/client.ts"
        ),
        1
    );
}
