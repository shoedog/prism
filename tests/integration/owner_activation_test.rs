//! Installed CLI process coverage; compiler tests explicitly opt in to the audit feature.
use assert_cmd::Command;

fn admission(error: &str) -> serde_json::Value {
    let (_, json) = error.split_once("; owner_admission=").expect(error);
    serde_json::from_str(json.trim()).unwrap()
}

fn refused(root: &std::path::Path) -> serde_json::Value {
    use prism::api::{nav_session, NavOptions, OwnerOptions};
    let mut options = NavOptions::default();
    options.no_cache = true;
    options.owner = Some(OwnerOptions::new(root.join("absent.js"), "tsconfig.json"));
    let error = nav_session(root, &options).err().unwrap().to_string();
    let output = Command::cargo_bin("prism")
        .unwrap()
        .args([
            "nav",
            "--no-cache",
            "--owner-config",
            "tsconfig.json",
            "--owner-compiler",
        ])
        .arg(root.join("absent.js"))
        .args(["repo-map", "--format", "json", "--repo"])
        .arg(root)
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(output.stdout.is_empty());
    let report = admission(&error);
    assert_eq!(
        report,
        admission(&String::from_utf8(output.stderr).unwrap())
    );
    assert_eq!(report["authorizes_runtime_edge"], false);
    assert_eq!(report["schema"], "prism.owner-admission/1");
    assert_eq!(report["phases"]["compiler_evidence"], "not_reached");
    assert_eq!(report["phases"]["owner_mapping"], "not_reached");
    assert_eq!(report["phases"]["session_build"], "not_reached");
    report
}

#[test]
fn owner_admission_reports_simultaneous_language_and_count_facts_without_bypassing_first_gate() {
    let root = tempfile::tempdir().unwrap();
    for i in 0..512 {
        std::fs::write(root.path().join(format!("f{i}.ts")), "// input\n").unwrap();
    }
    std::fs::write(root.path().join("extra.sh"), "echo hello\n").unwrap();
    let r = refused(root.path());
    assert_eq!(r["reason"], "owner_requires_js_ts_only");
    assert_eq!(r["failed_phase"], "select_inputs");
    assert_eq!(r["phases"]["prepare_inputs"], "not_reached");
    assert_eq!(r["inputs"]["loaded_files"], 513);
    assert_eq!(r["inputs"]["js_ts_files"], 512);
    assert_eq!(r["inputs"]["languages"]["Bash"], 1);
    assert_eq!(r["inputs"]["within_file_limit"], false);
    assert_eq!(r["inputs"]["within_byte_limit"], true);
}

#[test]
fn owner_admission_reports_empty_missing_and_exact_file_boundary() {
    let root = tempfile::tempdir().unwrap();
    let empty = refused(root.path());
    assert_eq!(empty["reason"], "empty_inputs");
    assert_eq!(empty["inputs"]["loaded_files"], 0);
    for i in 0..512 {
        std::fs::write(root.path().join(format!("f{i}.ts")), "// input\n").unwrap();
    }
    let exact = refused(root.path());
    assert_eq!(exact["reason"], "compiler_unavailable");
    assert_eq!(exact["failed_phase"], "locate_inputs");
    assert_eq!(exact["inputs"]["within_file_limit"], true);
    std::fs::write(root.path().join("overflow.ts"), "// input\n").unwrap();
    let over = refused(root.path());
    assert_eq!(over["reason"], "input_budget");
    assert_eq!(over["failed_phase"], "prepare_inputs");
    assert_eq!(over["inputs"]["within_file_limit"], false);
}

#[test]
fn owner_admission_reports_exact_byte_boundary_and_overflow() {
    let root = tempfile::tempdir().unwrap();
    let source = format!("//{}", " ".repeat(2 * 1024 * 1024 - 2));
    for i in 0..4 {
        std::fs::write(root.path().join(format!("f{i}.ts")), &source).unwrap();
    }
    let exact = refused(root.path());
    assert_eq!(exact["reason"], "compiler_unavailable");
    assert_eq!(exact["inputs"]["js_ts_bytes"], 8 * 1024 * 1024);
    assert_eq!(exact["inputs"]["within_byte_limit"], true);
    std::fs::write(root.path().join("overflow.ts"), " ").unwrap();
    let over = refused(root.path());
    assert_eq!(over["reason"], "input_budget");
    assert_eq!(over["inputs"]["within_byte_limit"], false);
    assert_eq!(over["inputs"]["within_file_limit"], true);
}

#[test]
fn owner_admission_reports_loader_failure_as_unknown_inputs() {
    let parent = tempfile::tempdir().unwrap();
    let report = refused(&parent.path().join("absent"));
    assert_eq!(report["reason"], "index_unavailable");
    assert_eq!(report["failed_phase"], "load_inputs");
    assert!(report["inputs"].is_null());
    assert_eq!(report["phases"]["select_inputs"], "not_reached");
}

#[test]
fn owner_admission_reports_parse_failure_before_compiler_location() {
    let root = tempfile::tempdir().unwrap();
    let source = format!(
        "{}\nconst broken = ;",
        "export const okay = 1;\n".repeat(30)
    );
    let parsed =
        prism::ast::ParsedFile::parse("a.ts", &source, prism::languages::Language::TypeScript)
            .unwrap();
    assert!(parsed.parse_error_count > 0 && parsed.error_rate() < 0.3);
    std::fs::write(root.path().join("a.ts"), source).unwrap();
    let report = refused(root.path());
    assert_eq!(report["reason"], "parse_error");
    assert_eq!(report["failed_phase"], "prepare_inputs");
    assert_eq!(report["inputs"]["loaded_files"], 1);
    assert_eq!(report["phases"]["locate_inputs"], "not_reached");
}

#[cfg(feature = "mcp")]
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
    assert_eq!(report, refused(root.path()));
}

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
