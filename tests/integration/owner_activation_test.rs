//! Installed CLI process coverage; compiler tests explicitly opt in to the audit feature.
use assert_cmd::Command;

#[test]
fn owner_cli_missing_compiler_is_an_error_not_an_ordinary_result() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.ts"), "export function run() {}\n").unwrap();
    let output = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "nav",
            "--no-cache",
            "--owner-config",
            "tsconfig.json",
            "--owner-compiler",
        ])
        .arg(root.path().join("absent.js"))
        .args(["callees", "--symbol", "run", "--repo"])
        .arg(root.path())
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("owner acquisition failed: compiler_unavailable"));
    assert!(output.stdout.is_empty());
}

#[cfg(feature = "detached-owner-audit")]
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/eval/receiver-closure/executable-owner-fixtures.json"
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

#[cfg(feature = "detached-owner-audit")]
#[test]
fn owner_cli_and_api_agree_and_default_has_no_new_exact_edge() {
    use prism::api::{callees, nav_session, NavOptions, OwnerOptions, Seed};
    let root = fixture();
    let compiler = std::env::var("PRISM_TYPESCRIPT").expect("explicit compiler");
    let mut options = NavOptions::default();
    options.no_cache = true;
    let ordinary = nav_session(root.path(), &options).unwrap();
    assert!(callees(
        &ordinary,
        Seed::SymbolInFile {
            symbol: "run",
            file: "src/app.ts"
        },
        1,
        true
    )
    .unwrap()
    .items
    .is_empty());
    options.owner = Some(OwnerOptions::new(&compiler, "tsconfig.json"));
    let session = nav_session(root.path(), &options).unwrap();
    let expected = serde_json::to_value(
        callees(
            &session,
            Seed::SymbolInFile {
                symbol: "run",
                file: "src/app.ts",
            },
            1,
            true,
        )
        .unwrap(),
    )
    .unwrap();
    let output = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "nav",
            "--no-cache",
            "--owner-config",
            "tsconfig.json",
            "--owner-compiler",
            &compiler,
            "callees",
            "--symbol",
            "run",
            "--file",
            "src/app.ts",
            "--confidence",
            "exact",
            "--format",
            "json",
            "--repo",
        ])
        .arg(root.path())
        .assert()
        .success()
        .get_output()
        .clone();
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual["items"].as_array().unwrap().len(), 1);
    assert_eq!(actual["items"][0]["location"]["file"], "src/client.ts");
}

#[cfg(all(feature = "detached-owner-audit", feature = "mcp"))]
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
