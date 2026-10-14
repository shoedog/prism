//! Spec §4.2, "Reads and writes happen outside every lock", shown without depending on how
//! large a socket's buffers are: one connection's response write is held at a gate while
//! another client refreshes and queries, and two queries are inside dispatch at one moment.
use super::super::coordinator::DaemonState;
use super::super::test_support::{
    client_settings, fixture, scripted, status, wait_until, Gate, HANG,
};
use super::super::wire_support::{
    close, connect, connect_with, CALLEES, CALLERS, INIT, INITED, REFRESH,
};
use super::serve_client;
use crate::mcp::output::McpToolResult;
use crate::mcp::registry::{ToolAnnotations, ToolContext, ToolDescriptor, ToolRegistry};
use crate::mcp::transport::{InMemoryTransport, ReadOutcome, Transport};
use crate::mcp::{RefreshPolicy, StartupMode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// A transport whose second write (the first is the answer to `initialize`) stops at a gate:
/// the connection's thread is then inside `write_message` with a tool call's response in hand,
/// which is where a socket write to a client that reads nothing blocks.
struct HeldWrite {
    inner: InMemoryTransport,
    writes: usize,
    entered: Arc<AtomicBool>,
    release: Arc<Gate>,
}

impl Transport for HeldWrite {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        self.inner.read_message()
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.writes += 1;
        if self.writes == 2 {
            self.entered.store(true, Ordering::SeqCst);
            let _opened = self.release.pass();
        }
        self.inner.write_message(v)
    }
}

/// Spec §9 test 7's "one blocked writer", made deterministic: a connection is held inside the
/// write of a query's response, and meanwhile another client's refresh (which needs the
/// provider's write lock) and its next query complete. Rules out any daemon lock, the provider
/// slot's included, held across a response write, on any host, whatever its socket buffers.
#[test]
fn a_refresh_and_another_client_s_query_complete_while_one_response_write_is_held() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let (mut other, other_worker) = connect(&state, client_settings(600));
    other.handshake();
    let first = other.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    let entered = Arc::new(AtomicBool::new(false));
    let release = Gate::closed();
    let held = {
        let state = Arc::clone(&state);
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        std::thread::spawn(move || {
            let mut transport = HeldWrite {
                inner: InMemoryTransport::new(vec![INIT, INITED, CALLEES]),
                writes: 0,
                entered,
                release,
            };
            let served = serve_client(
                &state,
                &ToolRegistry::all_v1(),
                &mut transport,
                &client_settings(600),
            );
            (served, transport.inner.responses().to_vec())
        })
    };
    wait_until(
        "the held connection to be inside its response write",
        || entered.load(Ordering::SeqCst),
    );

    let refreshed = other.request(REFRESH);
    assert_eq!(
        refreshed["result"]["structuredContent"]["status"], "refreshed",
        "the refresh completes while a response write is held: {refreshed}"
    );
    let after = other.request(CALLERS);
    assert_eq!(after["result"]["isError"], false, "{after}");
    assert_eq!(status(&state).generation, 1);

    release.open();
    let (served, responses) = held.join().expect("the held connection's thread ends");
    served.expect("the held connection ends cleanly at end of input");
    assert_eq!(responses.len(), 2, "{responses:?}");
    assert_eq!(responses[1]["result"]["isError"], false, "{responses:?}");
    close(other, other_worker);
    assert_eq!(builds.load(Ordering::SeqCst), 1);
}

const MEETING_TOOL: &str = "meet_another_reader";
const MEET: &str = r#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"meet_another_reader","arguments":{}}}"#;

/// Counts the calls inside the meeting tool's handler.
#[derive(Default)]
struct Rendezvous {
    inside: Mutex<u64>,
    changed: Condvar,
}

/// Every tool, and one more whose handler returns only once two calls are inside it at the same
/// time (or reports that it was alone, after [`HANG`]). A handler runs under the provider
/// slot's read lock, so two calls inside it at once are two readers holding the lock at once.
fn registry_with_a_meeting_tool(rendezvous: &Arc<Rendezvous>) -> Arc<ToolRegistry> {
    let rendezvous = Arc::clone(rendezvous);
    let mut registry = ToolRegistry::all_v1();
    registry.register(ToolDescriptor {
        name: MEETING_TOOL,
        description: String::new(),
        input_schema: json!({ "type": "object" }),
        annotations: ToolAnnotations::read_only("meets another reader"),
        runtime_behavior: None,
        handler: Box::new(
            move |_context: &ToolContext<'_>, _arguments: &Value| -> McpToolResult {
                let mut inside = rendezvous.inside.lock().expect("rendezvous");
                *inside += 1;
                rendezvous.changed.notify_all();
                let (inside, _timed_out) = rendezvous
                    .changed
                    .wait_timeout_while(inside, HANG, |inside| *inside < 2)
                    .expect("rendezvous");
                let met = *inside >= 2;
                McpToolResult {
                    content_text: if met { "met" } else { "alone" }.to_string(),
                    structured: None,
                    is_error: !met,
                    meta: serde_json::Map::new(),
                }
            },
        ),
    });
    Arc::new(registry)
}

/// Concurrent dispatch (spec §12 row S3b): two clients' queries are inside dispatch at the same
/// moment. Each call of the meeting tool waits for the other inside its handler; both answer
/// `met` only if the two ran at once. Rules out queries served one at a time (a lock that
/// excludes readers from one another), which barrier-released clients alone do not.
#[test]
fn two_clients_queries_are_inside_dispatch_at_the_same_time() {
    let (_dir, cfg) = fixture(RefreshPolicy::WarnOnly);
    let (builder, _builds) = scripted(&cfg, vec![]);
    let state = DaemonState::new(builder, StartupMode::Lazy).expect("a lazy daemon state");
    let rendezvous = Arc::new(Rendezvous::default());
    let registry = registry_with_a_meeting_tool(&rendezvous);
    let (mut a, a_worker) = connect_with(&state, &registry, client_settings(600));
    let (mut b, b_worker) = connect_with(&state, &registry, client_settings(600));
    a.handshake();
    b.handshake();
    let first = a.request(CALLEES);
    assert_eq!(first["result"]["isError"], false, "{first}");

    a.send(MEET);
    b.send(MEET);
    let from_a = a.recv();
    let from_b = b.recv();
    for answer in [&from_a, &from_b] {
        assert_eq!(answer["result"]["isError"], false, "{answer}");
        assert_eq!(answer["result"]["content"][0]["text"], "met", "{answer}");
    }
    close(a, a_worker);
    close(b, b_worker);
}
