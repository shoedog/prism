//! Tests for `serve_loop`, the frame loop the in-process server and a daemon connection share:
//! what the loop answers itself, what it leaves to its handler, and when it ends.
use super::*;
use crate::mcp::concise_shape::ConciseShapeMode;

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

/// Run `input` (one frame per line) through the loop; return how the loop ended and every
/// response it wrote.
fn run(
    input: &[u8],
    handle: impl FnMut(&Value, &mut Lifecycle) -> anyhow::Result<Option<Value>>,
) -> (anyhow::Result<()>, Vec<Value>) {
    let mut written = Vec::new();
    let outcome = {
        let mut transport = LineTransport::new(std::io::Cursor::new(input.to_vec()), &mut written);
        serve_loop(&mut transport, handle)
    };
    let responses = String::from_utf8(written)
        .expect("responses are UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each response is one JSON line"))
        .collect();
    (outcome, responses)
}

/// Spec §10: invalid JSON gets `-32700` and the connection stays. Rules out handing such a
/// frame to the handler, ending the loop on it, and answering it with nothing.
#[test]
fn a_frame_that_is_not_a_json_value_is_answered_by_the_loop_and_never_reaches_the_handler() {
    let mut seen = Vec::new();
    let (outcome, responses) = run(
        b"this is not json\n\xff\xfe\n{\"ok\":1}\n",
        |message, _state| {
            seen.push(message.clone());
            Ok(None)
        },
    );

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(seen, [json!({"ok": 1})], "only the JSON frame was handled");
    assert_eq!(responses.len(), 2, "one answer per frame that is not JSON");
    for response in &responses {
        assert_eq!(response["error"]["code"], -32700);
        assert_eq!(response["error"]["message"], "Parse error");
        assert!(response["id"].is_null());
    }
}

#[test]
fn the_handler_decides_whether_a_frame_is_answered() {
    let (outcome, responses) = run(b"1\n2\n3\n4\n", |message, _state| {
        let number = message.as_u64().unwrap_or(0);
        Ok((number % 2 == 1).then(|| json!({ "odd": number })))
    });

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(responses, [json!({"odd": 1}), json!({"odd": 3})]);
}

/// Rules out a lifecycle state made per frame: `tools/list` would then be refused for ever.
#[test]
fn one_lifecycle_state_lasts_the_whole_connection_and_starts_before_initialize() {
    let registry = ToolRegistry::all_v1();
    let settings = ConnectionSettings {
        result_cap: 80_000,
        structured_content: StructuredContentMode::Always,
        concise_shape: ConciseShapeMode::Slim,
    };
    let input = format!("{LIST}\n{INIT}\n{INITED}\n{LIST}\n");
    let (outcome, responses) = run(input.as_bytes(), |message, state| {
        let control = handle_control(
            message,
            state,
            &registry,
            StartupMode::Lazy,
            false,
            &settings,
        );
        Ok(match control {
            Control::Response(response) => Some(response),
            Control::NoResponse | Control::ToolCall(_) => None,
        })
    });

    outcome.expect("the loop ends cleanly at end of input");
    assert_eq!(responses.len(), 3, "the notification is not answered");
    assert_eq!(
        responses[0]["error"]["code"], -32600,
        "tools/list before initialize: the state starts before the handshake"
    );
    assert!(responses[1]["result"]["instructions"].is_string());
    assert_eq!(
        responses[2]["result"]["tools"].as_array().map(Vec::len),
        Some(9),
        "tools/list after the handshake: the state carried over"
    );
}

/// A daemon's handler fails when a lock is poisoned; the connection must then end. Rules out
/// logging the error and reading on.
#[test]
fn a_handler_error_ends_the_loop_and_nothing_after_it_is_read_or_answered() {
    let mut handled = 0;
    let (outcome, responses) = run(b"1\n2\n3\n", |message, _state| {
        handled += 1;
        if message == &json!(2) {
            anyhow::bail!("stop here");
        }
        Ok(Some(json!({ "echo": message })))
    });

    let error = outcome.expect_err("the handler's error is the loop's");
    assert!(error.to_string().contains("stop here"), "{error}");
    assert_eq!(handled, 2, "the third frame was never handled");
    assert_eq!(responses, [json!({"echo": 1})]);
}

#[test]
fn end_of_input_ends_the_loop_without_calling_the_handler() {
    let mut called = false;
    let (outcome, responses) = run(b"", |_message, _state| {
        called = true;
        Ok(None)
    });

    outcome.expect("an empty stream is a clean end");
    assert!(!called);
    assert!(responses.is_empty());
}
