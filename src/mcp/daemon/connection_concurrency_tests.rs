//! Spec §9 tests 7 and 9 as far as they can run before the accept loop and the shim exist:
//! many clients of one daemon at once, each with its own output settings, while one client
//! stops reading its answers and another refreshes the index.
use super::super::coordinator::DaemonState;
use super::super::test_support::{client_settings, fixture, scripted, status};
use super::super::wire_support::{close, connect, CALLEES, CALLERS, INIT, INITED, LIST, REFRESH};
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::transport::{serve_connection, InMemoryTransport};
use crate::mcp::{RefreshPolicy, ServerConfig, SessionProvider, StartupMode};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Barrier};

/// How many clients query at once. The bridge's fan-out gate (spec §6.2) uses eight.
const CLIENTS: usize = 8;

/// Four combinations of the output settings a client chooses for itself (spec §5
/// "Connection-local"), one of them with a smaller result cap.
fn variants() -> [ConnectionSettings; 4] {
    [
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::Always,
            concise_shape: ConciseShapeMode::Slim,
        },
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::OmitDefaultPath,
            concise_shape: ConciseShapeMode::Slim,
        },
        ConnectionSettings {
            result_cap: 80_000,
            structured_content: StructuredContentMode::Always,
            concise_shape: ConciseShapeMode::Legacy,
        },
        ConnectionSettings {
            result_cap: 20_000,
            structured_content: StructuredContentMode::OmitDefaultPath,
            concise_shape: ConciseShapeMode::Legacy,
        },
    ]
}

fn with_id(line: &str, id: u64) -> String {
    let mut message: Value = serde_json::from_str(line).expect("a request constant parses");
    message["id"] = json!(id);
    message.to_string()
}

/// Client `client`'s requests: two queries and a listing, each with an id no other client
/// uses, so that a response delivered to the wrong client or out of order is seen.
fn requests(client: usize) -> Vec<String> {
    let base = 1_000 * (client as u64 + 1);
    vec![
        with_id(CALLEES, base + 1),
        with_id(CALLERS, base + 2),
        with_id(LIST, base + 3),
    ]
}

/// What the in-process server answers `requests` with, for a client with `settings`.
fn in_process_answers(
    cfg: &ServerConfig,
    settings: &ConnectionSettings,
    requests: &[String],
) -> Vec<Value> {
    let mut provider = SessionProvider::bootstrap(cfg).expect("an in-process index");
    let mut inputs = vec![INIT, INITED];
    inputs.extend(requests.iter().map(String::as_str));
    let mut transport = InMemoryTransport::new(inputs);
    serve_connection(
        &mut provider,
        &ToolRegistry::all_v1(),
        &mut transport,
        settings,
    )
    .expect("the in-process server ends at end of input");
    transport.responses()[1..].to_vec()
}

/// Spec §9 test 7, with test 9's settings folded in: eight clients, two of each of four output
/// settings, are released together by a barrier and pipeline three requests each with ids of
/// their own. Meanwhile one client has stopped reading and another client refreshes the index.
/// Every client gets every answer, whole, in order, and byte for byte what the in-process server
/// answers a client with its settings (spec §5); the refresh completes. A smoke test of
/// completion, correlation and per-connection output settings: whether the stalled
/// connection's thread is blocked in a write at the time depends on socket buffer sizes, so the
/// claim that no lock is held across a response write rests on
/// `a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held`.
#[test]
fn many_clients_get_their_own_answers_while_one_stops_reading_and_another_refreshes() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let all = variants();
    let expected: Vec<Vec<Value>> = (0..CLIENTS)
        .map(|client| in_process_answers(&cfg, &all[client % all.len()], &requests(client)))
        .collect();

    let (builder, builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut refresher, refresher_worker) = connect(&state, client_settings(600));
    refresher.handshake();
    let first = refresher.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    // A client that sends and never reads, until its connection's thread can write no more.
    let (mut stalled, stalled_worker) = connect(&state, client_settings(600));
    stalled.handshake();
    let (whole, cut) = stalled.flood_until_full(CALLEES);

    let barrier = Arc::new(Barrier::new(CLIENTS + 1));
    let clients: Vec<_> = (0..CLIENTS)
        .map(|client| {
            let settings = ClientSettings::new(all[client % all.len()], 600, false)
                .expect("settings within the accepted bounds");
            let (mut conn, worker) = connect(&state, settings);
            conn.handshake();
            let barrier = Arc::clone(&barrier);
            let requests = requests(client);
            std::thread::spawn(move || {
                barrier.wait();
                for request in &requests {
                    conn.send(request);
                }
                let answers: Vec<Value> = requests.iter().map(|_| conn.recv()).collect();
                close(conn, worker);
                answers
            })
        })
        .collect();

    barrier.wait();
    let refreshed = refresher.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "the refresh completes while a client is not reading: {refreshed}"
    );

    for (client, handle) in clients.into_iter().enumerate() {
        let answers = handle.join().expect("a client thread ends without a panic");
        assert_eq!(answers.len(), expected[client].len(), "client {client}");
        for (index, (got, want)) in answers.iter().zip(&expected[client]).enumerate() {
            assert_eq!(got, want, "client {client}, answer {}", index + 1);
        }
    }

    // The stalled client reads at last: every request it sent was answered, whole. A request
    // the full socket cut off is finished while the answers are read.
    let sent = whole + usize::from(!cut.is_empty());
    let finishing = stalled.send_raw_in_background(cut);
    for index in 0..sent {
        let answer = stalled.recv();
        assert_eq!(answer["id"], 7, "answer {}: {answer}", index + 1);
        assert_eq!(answer["result"]["isError"], false, "answer {}", index + 1);
    }
    finishing.join().expect("the cut-off request is sent");
    close(stalled, stalled_worker);
    close(refresher, refresher_worker);

    assert_eq!(
        builds.load(Ordering::SeqCst),
        1,
        "one build for every client"
    );
    assert_eq!(status(&state).generation, 1);
}
