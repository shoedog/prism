//! One admitted connection of the daemon (spec §4.2 "Per connection", §4.5).
//!
//! A connection is the frame loop the in-process server runs (`serve_loop`), the same control
//! path (`handle_control`: lifecycle, `tools/list`, call validation; no index, no lock), and
//! for a validated tool call the coordinator's readiness followed by the same three dispatch
//! functions, under the provider slot's read or write lock.
//!
//! What this slice leaves to S3b: under an automatic refresh policy the trigger is not
//! re-checked after exclusive access is acquired, so two clients that both find the index stale
//! each refresh it; a query that follows a refresh runs while the slot is still held
//! exclusively; and there is no panic boundary.
use super::budget::ConnectionBudget;
use super::coordinator::DaemonState;
use crate::mcp::framing::{Transport, UnixStreamTransport};
use crate::mcp::registry::ToolRegistry;
use crate::mcp::settings::{ClientSettings, ConnectionSettings};
use crate::mcp::transport::{
    dispatch_tool_auto_refresh, dispatch_tool_read, dispatch_tool_refresh, handle_control,
    readiness_response, serve_loop, Control, ReadDispatch, ToolCall,
};
use serde_json::Value;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// Serve one admitted connection until its peer closes it or it fails.
///
/// `stream` is the connection after the preamble: blocking, with no read timeout (a timeout
/// would end the connection as a read error). `client` is what this client sent in its
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
    let mut transport = UnixStreamTransport::from_stream(stream)?;
    serve_client(state, registry, &mut transport, client)
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
    let readiness = state.ensure_ready(budget)?;
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
    let initial_report = match published(read)? {
        ReadDispatch::Response(response) => return Ok(response),
        ReadDispatch::NeedsAutoRefresh { initial_report } => initial_report,
    };
    // The read lock is released: `read` has returned. Exclusive access is taken afresh.
    let refreshed = state
        .write(|provider| dispatch_tool_auto_refresh(call, provider, initial_report, settings))?;
    published(refreshed)
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
