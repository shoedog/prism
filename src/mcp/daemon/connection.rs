//! One admitted connection of the daemon (spec §4.2 "Per connection", §4.5).
//!
//! A connection is the frame loop the in-process server runs (`serve_loop`), the same control
//! path (`handle_control`: lifecycle, `tools/list`, call validation; no index, no lock), and
//! for a validated tool call the coordinator's readiness followed by the same three dispatch
//! functions, under the provider slot's read or write lock.
//!
//! One answer here is the daemon's alone. A call whose own build attempt failed, and which
//! runs again only while a later attempt is building, is told that the repository is being
//! indexed and to retry (`indexing_status`); the in-process server has one client and no such
//! case. If the later attempt has already published the index, the call is served. Everything else a call is told before the index is ready is the in-process
//! server's `warming` or `build_failed`, from the same `readiness_response`.
//!
//! Under an automatic refresh policy a query that finds the index stale asks for exclusive
//! access and checks the trigger again once it has it, so two clients that both found the
//! index stale refresh it once, unless the first refresh leaves it still known stale. The
//! query that follows a refresh runs under the same exclusive access, so its response and the
//! refresh's generation are one snapshot's.
//!
//! Panics: a panic in a query, a refresh, the build or the control plane ends the daemon
//! (`fatal.rs`); one anywhere else on this connection's thread ends this connection only
//! (`serve_bounded`).
use super::budget::ConnectionBudget;
use super::coordinator::{Answer, DaemonState};
use crate::mcp::framing::{Transport, UnixStreamTransport};
use crate::mcp::lazy::Readiness;
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::transport::{
    dispatch_tool_auto_refresh, dispatch_tool_read, dispatch_tool_refresh, handle_control,
    readiness_response, retryable_status_response, serve_loop, Control, ReadDispatch, ToolCall,
};
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

/// What a connection's serve loop returns when a panic on its thread was raised outside the
/// daemon's two domains.
const CONNECTION_PANICKED: &str =
    "a panic on this connection's thread outside the index and the control plane; the \
     connection is closed and the daemon goes on";

/// How many seconds a client that is told `indexing` is asked to wait before it retries. Five
/// is the owner's value (2026-10-13); a change needs measured build-time evidence (how long
/// builds take, by repository size), not an estimate.
const INDEXING_RETRY_AFTER_SECS: u64 = 5;

/// The `prism/index_state` an `indexing` answer carries: the same as a warming answer's, so
/// that a client that keys on the index state treats the two alike.
const INDEXING_INDEX_STATE: &str = "warming";

/// What a call is told when its own build attempt failed and a retry has already superseded
/// that failure (a later attempt is building): retry shortly, and ignore this if a retry has
/// already succeeded. The stale failure's text is not in it.
fn indexing_status() -> Value {
    json!({
        "status": "indexing",
        "retry_after_secs": INDEXING_RETRY_AFTER_SECS,
        "message": format!(
            "prism-mcp is indexing the repository; retry this call in about \
             {INDEXING_RETRY_AFTER_SECS} seconds. If you already retried and that call \
             succeeded, ignore this message."
        ),
    })
}

/// Serve one admitted connection until its peer closes it or it fails.
///
/// `stream` is the connection after the preamble, in whatever mode the handshake left it: it
/// is set blocking and with no read timeout here, because a timeout or a would-block read
/// would end the connection as a read error. `client` is what this client sent in its
/// `HELLO`; its output settings and its first-call wait apply to this connection only.
///
/// Returns `Ok` at end of input. Returns an error when a read or write fails (the peer went
/// away mid-frame or stopped reading) or when a daemon lock is poisoned; the caller closes the
/// socket either way.
pub(super) fn serve_admitted(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    stream: UnixStream,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(None)?;
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_bounded(state, registry, &mut transport, client)
}

/// The connection's panic boundary (spec §4.2 "Panic policy", §10). A panic that reaches it
/// was raised outside both of the daemon's domains, in framing, in a readiness answer or in
/// writing a response: one inside them ends the daemon before it unwinds this far. It ends
/// this connection only, with an error; the caller drops the socket and the client sees its
/// connection end. Nothing the closure borrowed is used after a panic but `state`, whose shared
/// state changes only inside its own domains.
fn serve_bounded(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let served = std::panic::catch_unwind(AssertUnwindSafe(|| {
        serve_client(state, registry, transport, client)
    }));
    match served {
        Ok(result) => result,
        Err(_panic) => Err(anyhow::anyhow!(CONNECTION_PANICKED)),
    }
}

fn serve_client(
    state: &Arc<DaemonState>,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    client: &ClientSettings,
) -> anyhow::Result<()> {
    let settings: ConnectionSettings = *client.output();
    let mut budget = ConnectionBudget::new(client);
    let startup = state.startup_mode();
    serve_loop(transport, |message, lifecycle| {
        match handle_control(message, lifecycle, registry, startup, &settings) {
            Control::Response(response) => Ok(Some(response)),
            Control::NoResponse => Ok(None),
            Control::ToolCall(call) => {
                run_tool_call(state, &mut budget, &call, &settings).map(Some)
            }
        }
    })
}

/// Run one validated tool call: wait for the index within this connection's budget, then
/// query it under the read lock, or refresh it under the write lock. The value returned is
/// owned, so every lock taken here is released before the response is written.
fn run_tool_call(
    state: &Arc<DaemonState>,
    budget: &mut ConnectionBudget,
    call: &ToolCall<'_>,
    settings: &ConnectionSettings,
) -> anyhow::Result<Value> {
    let readiness = match state.ensure_ready(budget)? {
        Answer::Ready => Readiness::Ready,
        Answer::Warming { elapsed } => Readiness::Warming { elapsed },
        Answer::Failed { error } => Readiness::Failed { error },
        Answer::Superseded => {
            return Ok(retryable_status_response(
                call,
                indexing_status(),
                INDEXING_INDEX_STATE,
                settings,
            ));
        }
    };
    if let Some(response) = readiness_response(call, readiness, || None, settings) {
        return Ok(response);
    }

    if call.is_refresh() {
        let refreshed = state.write(|provider| dispatch_tool_refresh(call, provider, settings))?;
        return published(refreshed);
    }

    // The query sees the whole provider, so the stale evidence a refresh recorded takes
    // precedence over a fresh stamp probe exactly as it does in process.
    let read = state.read(|provider| dispatch_tool_read(call, provider, settings))?;
    if let ReadDispatch::Response(response) = published(read)? {
        return Ok(response);
    }
    // The index was stale under an automatic policy. The read lock is released (`read` has
    // returned) and exclusive access is taken afresh, so another client may have refreshed in
    // between. The trigger is checked again under exclusive access (spec §4.2) by asking the
    // read's question again: a query that now finds the index current is answered as a read,
    // and one that still finds it stale refreshes from the report found under this access,
    // not the one found before it. Either way the response comes from the one generation this
    // access saw.
    #[cfg(test)]
    state.before_exclusive();
    let answered =
        state.write(
            |provider| match dispatch_tool_read(call, &*provider, settings) {
                ReadDispatch::Response(response) => response,
                ReadDispatch::NeedsAutoRefresh { initial_report } => {
                    dispatch_tool_auto_refresh(call, provider, initial_report, settings)
                }
            },
        )?;
    published(answered)
}

/// `Ready` is announced only after publication and is never left, so a ready call always
/// finds a provider. If it does not, that is a defect of the coordinator; the connection ends
/// with this error instead of a panic.
fn published<T>(outcome: Option<T>) -> anyhow::Result<T> {
    match outcome {
        Some(value) => Ok(value),
        None => anyhow::bail!("the index is announced ready and no index is published"),
    }
}

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "connection_parity_tests.rs"]
mod parity_tests;

#[cfg(test)]
#[path = "connection_ready_tests.rs"]
mod ready_tests;

#[cfg(test)]
#[path = "connection_refresh_tests.rs"]
mod refresh_tests;

#[cfg(test)]
#[path = "connection_concurrency_tests.rs"]
mod concurrency_tests;

#[cfg(test)]
#[path = "connection_isolation_tests.rs"]
mod isolation_tests;

#[cfg(test)]
#[path = "connection_panic_tests.rs"]
mod panic_tests;
