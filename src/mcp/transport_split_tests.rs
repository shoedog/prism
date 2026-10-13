//! Tests for the control/dispatch split: `handle_control` needs no index, `dispatch_tool_read`
//! needs only shared access, and output settings are an explicit per-connection value.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::settings::ConnectionSettings;
use crate::mcp::{CacheMode, RefreshPolicy, ServerConfig, SessionProvider};

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
const CALLEES: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"symbol","name":"run","file":"main.py"}}}}"#;
const REFRESH: &str = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"refresh_index","arguments":{}}}"#;

fn settings(structured_content: StructuredContentMode) -> ConnectionSettings {
    ConnectionSettings {
        result_cap: 80_000,
        structured_content,
        concise_shape: ConciseShapeMode::Slim,
    }
}

fn provider(refresh_policy: RefreshPolicy) -> (tempfile::TempDir, SessionProvider) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("util.py"), "def helper():\n    return 1\n").unwrap();
    std::fs::write(
        dir.path().join("main.py"),
        "from util import helper\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let cfg = ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: CacheMode::NoCache,
        refresh_policy,
        startup: StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    };
    (dir, SessionProvider::bootstrap(&cfg).unwrap())
}

fn make_stale(dir: &tempfile::TempDir) {
    std::fs::write(
        dir.path().join("util.py"),
        "def helper():\n    return 1\n\n\ndef extra():\n    return helper()\n",
    )
    .unwrap();
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn control<'r>(
    registry: &'r ToolRegistry,
    state: &mut Lifecycle,
    request: &str,
    startup_mode: StartupMode,
    settings: &ConnectionSettings,
) -> Control<'r> {
    handle_control(&parse(request), state, registry, startup_mode, settings)
}

fn response_of(control: Control<'_>) -> Value {
    match control {
        Control::Response(response) => response,
        Control::NoResponse => panic!("expected a response, got none"),
        Control::ToolCall(_) => panic!("expected a response, got a tool call"),
    }
}

fn tool_call<'r>(
    registry: &'r ToolRegistry,
    request: &str,
    settings: &ConnectionSettings,
) -> ToolCall<'r> {
    let mut state = Lifecycle::Initialized;
    match control(registry, &mut state, request, StartupMode::Eager, settings) {
        Control::ToolCall(call) => call,
        Control::Response(response) => panic!("expected a tool call, got {response}"),
        Control::NoResponse => panic!("expected a tool call, got no response"),
    }
}

fn read_response(dispatch: ReadDispatch) -> Value {
    match dispatch {
        ReadDispatch::Response(response) => response,
        ReadDispatch::NeedsAutoRefresh { .. } => panic!("expected a response, got a deferral"),
    }
}

#[test]
fn handle_control_serves_the_handshake_and_listing_with_no_index() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::OmitDefaultPath);
    let mut state = Lifecycle::PreInit;

    let early = response_of(control(
        &registry,
        &mut state,
        LIST,
        StartupMode::Lazy,
        &settings,
    ));
    assert_eq!(
        early["error"]["code"], -32600,
        "tools/list before initialize"
    );

    let init = response_of(control(
        &registry,
        &mut state,
        INIT,
        StartupMode::Background,
        &settings,
    ));
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("started at server startup"),
        "initialize advertises the startup mode it was given: {init}"
    );
    assert!(state == Lifecycle::InitializeReceived);

    assert!(matches!(
        control(
            &registry,
            &mut state,
            INITED,
            StartupMode::Background,
            &settings
        ),
        Control::NoResponse
    ));
    assert!(state == Lifecycle::Initialized);

    let listed = response_of(control(
        &registry,
        &mut state,
        LIST,
        StartupMode::Background,
        &settings,
    ));
    assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), 9);

    let ping = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
        StartupMode::Background,
        &settings,
    ));
    assert_eq!(ping["result"], json!({}));
}

#[test]
fn handle_control_rejects_what_needs_no_index_to_reject() {
    let registry = ToolRegistry::all_v1();
    let mut state = Lifecycle::Initialized;
    let always = settings(StructuredContentMode::Always);

    let unknown = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(unknown["result"]["isError"], true);
    assert!(unknown["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with("unknown tool 'no_such_tool'"));

    let refresh_with_args = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"refresh_index","arguments":{"x":1}}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(
        refresh_with_args["result"]["content"][0]["text"],
        "refresh_index takes no arguments"
    );

    let array_arguments = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"nav_callees","arguments":[]}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(array_arguments["error"]["code"], -32602);

    let missing_name = response_of(control(
        &registry,
        &mut state,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{}}"#,
        StartupMode::Eager,
        &always,
    ));
    assert_eq!(missing_name["error"]["code"], -32602);
}

#[test]
fn handle_control_returns_a_validated_tool_call() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    assert!(!tool_call(&registry, CALLEES, &settings).is_refresh());
    assert!(tool_call(&registry, REFRESH, &settings).is_refresh());
}

#[test]
fn dispatch_tool_read_answers_from_shared_access() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (_dir, provider) = provider(RefreshPolicy::WarnOnly);
    let call = tool_call(&registry, CALLEES, &settings);

    // `provider` is not `mut`: the read path compiles against shared access only.
    let response = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["query"],
        "callees:run@main.py"
    );
}

#[test]
fn a_stale_warn_only_index_is_answered_with_the_stale_warning() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, provider) = provider(RefreshPolicy::WarnOnly);
    make_stale(&dir);
    let call = tool_call(&registry, CALLEES, &settings);

    let response = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert_eq!(
        response["result"]["_meta"]["prism/index_freshness"],
        "stale"
    );
}

#[test]
fn a_stale_auto_refresh_index_is_deferred_to_the_exclusive_path() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, mut provider) = provider(RefreshPolicy::AutoFull);
    make_stale(&dir);
    let call = tool_call(&registry, CALLEES, &settings);

    let initial_report = match dispatch_tool_read(&call, &provider, &settings) {
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
        ReadDispatch::Response(response) => panic!("expected a deferral, got {response}"),
    };
    assert!(initial_report.as_ref().is_some_and(|report| report.stale));

    let response = dispatch_tool_auto_refresh(&call, &mut provider, initial_report, &settings);
    assert_eq!(
        response["result"]["_meta"]["prism/auto_refresh"],
        "refreshed"
    );
    assert_eq!(response["result"]["_meta"]["prism/refresh_generation"], 1);

    // Fresh again: the next read is answered directly.
    let again = read_response(dispatch_tool_read(&call, &provider, &settings));
    assert!(again["result"]["_meta"]
        .get("prism/index_freshness")
        .is_none());
}

#[test]
fn dispatch_tool_refresh_rebuilds_and_reports_the_new_generation() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (dir, mut provider) = provider(RefreshPolicy::WarnOnly);
    make_stale(&dir);
    let call = tool_call(&registry, REFRESH, &settings);

    let response = dispatch_tool_refresh(&call, &mut provider, &settings);
    assert_eq!(response["id"], 8);
    assert_eq!(response["result"]["structuredContent"]["generation"], 1);
    assert_eq!(
        response["result"]["structuredContent"]["stale_before_refresh"],
        true
    );
}

#[test]
fn output_settings_belong_to_the_call_site_not_the_process() {
    let registry = ToolRegistry::all_v1();
    let (_dir, provider) = provider(RefreshPolicy::WarnOnly);
    let always = settings(StructuredContentMode::Always);
    let omit = settings(StructuredContentMode::OmitDefaultPath);

    let with = read_response(dispatch_tool_read(
        &tool_call(&registry, CALLEES, &always),
        &provider,
        &always,
    ));
    let without = read_response(dispatch_tool_read(
        &tool_call(&registry, CALLEES, &omit),
        &provider,
        &omit,
    ));
    assert!(with["result"].get("structuredContent").is_some());
    assert!(without["result"].get("structuredContent").is_none());
}

#[test]
fn readiness_response_maps_each_state_and_asks_for_admission_only_on_failure() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let call = tool_call(&registry, CALLEES, &settings);
    let asked = std::cell::Cell::new(0);
    let admission = || {
        asked.set(asked.get() + 1);
        None
    };

    assert!(readiness_response(&call, Readiness::Ready, admission, &settings).is_none());
    let warming = readiness_response(
        &call,
        Readiness::Warming {
            elapsed: std::time::Duration::from_secs(3),
        },
        admission,
        &settings,
    )
    .expect("warming is answered");
    assert_eq!(warming["id"], 7);
    assert_eq!(warming["result"]["structuredContent"]["status"], "warming");
    assert_eq!(warming["result"]["structuredContent"]["elapsed_secs"], 3);
    assert_eq!(
        asked.get(),
        0,
        "no admission lookup unless the build failed"
    );

    let failed = readiness_response(
        &call,
        Readiness::Failed {
            error: "boom".into(),
        },
        admission,
        &settings,
    )
    .expect("failure is answered");
    assert_eq!(
        failed["result"]["structuredContent"]["status"],
        "build_failed"
    );
    assert_eq!(failed["result"]["structuredContent"]["cause"], "boom");
    assert_eq!(asked.get(), 1);
    assert!(
        failed["result"]["structuredContent"]
            .get("owner_admission")
            .is_none(),
        "no diagnostic from the closure means no owner_admission key: {failed}"
    );
}

#[test]
fn a_build_failure_carries_the_admission_diagnostic_to_the_wire() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let call = tool_call(&registry, CALLEES, &settings);

    let failed = readiness_response(
        &call,
        Readiness::Failed {
            error: "boom".into(),
        },
        || Some(json!({"failed_phase": "x"})),
        &settings,
    )
    .expect("failure is answered");
    assert_eq!(
        failed["result"]["structuredContent"]["owner_admission"]["failed_phase"], "x",
        "the admission diagnostic must reach the wire: {failed}"
    );
    assert!(failed["result"]["structuredContent"]
        .get("owner_admission_omitted")
        .is_none());
    let text: Value = parse(failed["result"]["content"][0]["text"].as_str().unwrap());
    assert_eq!(
        text["owner_admission"]["failed_phase"], "x",
        "and the text view: {failed}"
    );
}

#[test]
fn an_auto_refresh_failure_followed_by_a_tool_error_answers_with_the_bare_tool_error() {
    let registry = ToolRegistry::all_v1();
    let settings = settings(StructuredContentMode::Always);
    let (_fresh_dir, fresh) = provider(RefreshPolicy::WarnOnly);
    let (dir, mut provider) = provider(RefreshPolicy::AutoFull);
    make_stale(&dir);
    let bad_seed = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"nav_callees","arguments":{"seed":{"kind":"bogus"}}}}"#;
    let call = tool_call(&registry, bad_seed, &settings);

    let initial_report = match dispatch_tool_read(&call, &provider, &settings) {
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
        ReadDispatch::Response(response) => panic!("expected a deferral, got {response}"),
    };
    provider.force_next_refresh_error_for_tests("x");

    let response = dispatch_tool_auto_refresh(&call, &mut provider, initial_report, &settings);
    assert_eq!(response["id"], 9);
    assert_eq!(response["result"]["isError"], true, "{response}");
    let meta = &response["result"]["_meta"];
    assert!(
        meta.get("prism/auto_refresh").is_none(),
        "a tool error carries no auto-refresh metadata: {meta}"
    );
    assert!(meta.get("prism/auto_refresh_error").is_none(), "{meta}");

    // The error is the tool's own, not the refresh failure: the same call against an index that
    // was never stale produces the same content.
    let bare = match dispatch_tool_read(&call, &fresh, &settings) {
        ReadDispatch::Response(response) => response,
        ReadDispatch::NeedsAutoRefresh { .. } => panic!("a fresh index defers nothing"),
    };
    assert_eq!(bare["result"]["isError"], true, "{bare}");
    assert_eq!(
        response["result"]["content"], bare["result"]["content"],
        "the tool error's content is unchanged by the failed refresh"
    );
}

#[test]
fn serve_connection_applies_the_settings_it_is_given() {
    let registry = ToolRegistry::all_v1();
    for (mode, expect_structured) in [
        (StructuredContentMode::Always, true),
        (StructuredContentMode::OmitDefaultPath, false),
    ] {
        let (_dir, mut provider) = provider(RefreshPolicy::WarnOnly);
        let mut transport = InMemoryTransport::new(vec![INIT, INITED, CALLEES]);
        serve_connection(&mut provider, &registry, &mut transport, &settings(mode)).unwrap();
        let responses = transport.responses();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            responses[1]["result"].get("structuredContent").is_some(),
            expect_structured
        );
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use crate::mcp::framing::{LineTransport, UnixStreamTransport};
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;

    #[test]
    fn a_pipelined_session_over_a_unix_socket_matches_stdio_framing_byte_for_byte() {
        let settings = settings(StructuredContentMode::OmitDefaultPath);
        let mut script = format!("{INIT}\n{INITED}\n{LIST}\n");
        for _ in 0..200 {
            script.push_str(CALLEES);
            script.push('\n');
        }

        let registry = ToolRegistry::all_v1();
        let (_dir_a, mut over_stdio) = provider(RefreshPolicy::WarnOnly);
        let mut stdio_out = Vec::new();
        {
            let mut transport = LineTransport::new(
                std::io::Cursor::new(script.clone().into_bytes()),
                &mut stdio_out,
            );
            serve_connection(&mut over_stdio, &registry, &mut transport, &settings).unwrap();
        }

        let (_dir_b, mut over_socket) = provider(RefreshPolicy::WarnOnly);
        let (client, server) = UnixStream::pair().expect("socket pair");
        let server_thread = std::thread::spawn(move || {
            let registry = ToolRegistry::all_v1();
            let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
            serve_connection(&mut over_socket, &registry, &mut transport, &settings)
        });
        // The script is sent by a separate thread in one `write_all` and then half-closed, while
        // this thread reads responses at the same time: the server may start answering before
        // the script is fully written, and neither side's socket buffer can block the other.
        // The requests are still back to back on the wire, with no waiting for a response.
        let mut write_half = client.try_clone().expect("clone client");
        let writer = std::thread::spawn(move || {
            write_half
                .write_all(script.as_bytes())
                .expect("client write");
            write_half
                .shutdown(Shutdown::Write)
                .expect("client half-close");
        });
        let mut socket_out = Vec::new();
        let mut read_half = client;
        read_half.read_to_end(&mut socket_out).expect("client read");
        writer.join().expect("client writer");
        server_thread
            .join()
            .expect("serve loop must not panic")
            .expect("serve loop ends cleanly at end of input");

        assert_eq!(
            socket_out.iter().filter(|byte| **byte == b'\n').count(),
            202,
            "initialize + tools/list + 200 calls"
        );
        assert!(socket_out == stdio_out, "socket and stdio responses differ");
    }

    #[test]
    fn a_peer_that_disconnects_mid_frame_ends_the_connection_without_a_panic() {
        let settings = settings(StructuredContentMode::OmitDefaultPath);
        let (_dir, mut provider) = provider(RefreshPolicy::WarnOnly);
        let (mut client, server) = UnixStream::pair().expect("socket pair");
        let server_thread = std::thread::spawn(move || {
            let registry = ToolRegistry::all_v1();
            let mut transport = UnixStreamTransport::from_stream(server).expect("transport");
            serve_connection(&mut provider, &registry, &mut transport, &settings)
        });
        client
            .write_all(br#"{"jsonrpc":"2.0","id":1,"me"#)
            .expect("partial frame");
        drop(client);
        // The loop returns (the join does not hang or panic). End of input arrives after the
        // truncated frame, which is answered with a parse error; the peer is gone, so that write
        // fails and the loop returns it as an ordinary `Err` value for the daemon to log and move
        // on. This is deterministic: the server can only see the end of input after the client
        // has closed, so the reply is always written to a closed peer. The error text is an OS
        // string (EPIPE, ECONNRESET) that differs between hosts, so it is not asserted.
        let outcome = server_thread.join().expect("serve loop must not panic");
        assert!(
            outcome.is_err(),
            "the reply to a closed peer is a write error, not a clean end: {outcome:?}"
        );
    }
}
