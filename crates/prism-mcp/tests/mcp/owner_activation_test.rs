//! The prism-mcp binary half of the owner-admission coverage (moved from
//! tests/integration/owner_activation_test.rs; the prism CLI half lives in
//! `crates/prism-review/tests/cli/owner_activation_test.rs`).
use assert_cmd::Command;

fn admission(error: &str) -> serde_json::Value {
    let (_, json) = error.split_once("; owner_admission=").expect(error);
    serde_json::from_str(json.trim()).unwrap()
}

/// The refusal report the library API gives (the CLI agreement is asserted by the prism tests).
fn refused_report(root: &std::path::Path) -> serde_json::Value {
    use prism::api::{nav_session, NavOptions, OwnerOptions};
    let mut options = NavOptions::default();
    options.no_cache = true;
    options.owner = Some(OwnerOptions::new(root.join("absent.js"), "tsconfig.json"));
    let report = admission(&nav_session(root, &options).err().unwrap().to_string());
    assert_eq!(report["authorizes_runtime_edge"], false);
    assert_eq!(report["schema"], "prism.owner-admission/1");
    assert_eq!(report["phases"]["compiler_evidence"], "not_reached");
    assert_eq!(report["phases"]["owner_mapping"], "not_reached");
    assert_eq!(report["phases"]["session_build"], "not_reached");
    report
}

#[cfg(feature = "detached-owner-audit")]
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    for (name, source) in corpus["files"].as_object().unwrap() {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source.as_str().unwrap()).unwrap();
    }
    std::fs::write(root.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    std::fs::write(
        root.path().join("tsconfig.json"),
        serde_json::json!({
            "compilerOptions":corpus["compiler_options"],"include":["src"]
        })
        .to_string(),
    )
    .unwrap();
    root
}

#[test]
fn owner_admission_mcp_startup_preserves_refusal_report_without_initializing() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.py"), "def f(): pass\n").unwrap();
    let output = Command::cargo_bin("prism-mcp").unwrap()
        .args(["--eager", "--no-cache", "--owner-config", "tsconfig.json", "--owner-compiler"])
        .arg(root.path().join("absent.js")).arg("--repo").arg(root.path())
        .write_stdin("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"}}}\n")
        .assert().failure().get_output().clone();
    assert!(output.stdout.is_empty());
    let report = admission(&String::from_utf8(output.stderr).unwrap());
    assert_eq!(report, refused_report(root.path()));
}

#[cfg(feature = "detached-owner-audit")]
#[test]
fn owner_mcp_binary_handshake_and_default_concise_wire_gain() {
    let root = fixture();
    let compiler = std::env::var("PRISM_TYPESCRIPT").expect("explicit compiler");
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"owner-test\",\"version\":\"1\"}}}\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"nav_callees\",\"arguments\":{\"seed\":{\"kind\":\"symbol\",\"name\":\"run\",\"file\":\"src/app.ts\"}}}}\n"
    );
    let output = Command::cargo_bin("prism-mcp")
        .unwrap()
        .args([
            "--eager",
            "--no-cache",
            "--owner-config",
            "tsconfig.json",
            "--owner-compiler",
            &compiler,
            "--repo",
        ])
        .arg(root.path())
        .env_remove("PRISM_MCP_STRUCTURED_CONTENT")
        .env_remove("PRISM_MCP_CONCISE_SHAPE")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();
    let responses: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 2);
    assert!(
        responses[0]["result"]["protocolVersion"].is_string(),
        "{responses:?}"
    );
    let result = &responses[1]["result"];
    assert_ne!(result["isError"], true);
    let payload: serde_json::Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(payload["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        payload["items"][0]["symbol"]["Function"]["file"],
        "src/client.ts"
    );
    assert_eq!(payload["items"][0]["score"], 1.0);
}
