use super::freshness::{apply_freshness_report, FreshnessProbe, FRESHNESS_RESERVE_BYTES};
use super::output::{resolve_cap, McpToolResult, SCHEMA_VERSION};
use super::registry::{ToolContext, ToolRegistry, ToolRuntimeBehavior};
use super::{tools_refresh, RefreshSummary, SessionProvider};
use crate::navigation::NavigationSession;
use serde_json::{json, Map, Value};
#[cfg(test)]
use std::collections::VecDeque;
use std::io::{self, BufRead, Write};

const PROTOCOL_VERSION: &str = "2025-11-25";
/// Max inbound JSON-RPC message size (one line). A nav request is tiny; this caps a hostile/buggy
/// local client's memory (holistic-review MAJOR — `read_line` was unbounded).
const MAX_REQUEST_BYTES: usize = 1 << 20; // 1 MiB
/// Max serialized size of a request `id` (echoed into every response envelope). Bounding it keeps the
/// JSON-RPC envelope within `ENVELOPE_RESERVE`, so the on-the-wire response stays under the cap
/// regardless of a client-controlled `id` (holistic re-review MAJOR). A real id is a small int/string.
const MAX_ID_BYTES: usize = 256;

/// Bytes reserved for the JSON-RPC success envelope this transport wraps around a tool result
/// (`{"jsonrpc","id","result":…}`). The envelope is a transport concern, so the reserve and the
/// `payload_budget` helper live here (not in `output`) and are pinned by
/// `reserve_covers_envelope_and_max_id`. Generous vs the real envelope: base ~35 bytes + a bounded
/// request id <= `MAX_ID_BYTES` (holistic re-review MAJOR — was owned by the result shaper).
pub(crate) const ENVELOPE_RESERVE: usize = 512;

/// Result-payload budget under `cap` once this transport's envelope reserve is removed. The result
/// shaper (`output::shape_result`) sizes results against this so value + envelope <= `cap` on the wire.
pub(crate) fn payload_budget(cap: usize) -> usize {
    cap.saturating_sub(ENVELOPE_RESERVE)
}

/// Lifecycle gate (holistic-review MAJOR): a bare `bool` let `notifications/initialized` mark the
/// session initialized *before* a valid `initialize`. Three states enforce the handshake order:
/// `PreInit` → (valid `initialize`) → `InitializeReceived` → (`notifications/initialized`) → `Initialized`.
#[derive(PartialEq, Eq)]
enum Lifecycle {
    PreInit,
    InitializeReceived,
    Initialized,
}

/// Outcome of reading one inbound frame. `Malformed` is a *recoverable* framing/encoding fault
/// (bad UTF-8, or an oversized line we drained to resync) — the caller replies `-32700` and keeps the
/// session (and its warm index) alive, instead of letting one stray byte from a buggy/hostile local
/// client kill the server (codex MAJOR — these used to propagate as fatal errors).
pub enum ReadOutcome {
    Message(String),
    Malformed,
}

pub trait Transport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>>;
    fn write_message(&mut self, v: Value) -> anyhow::Result<()>;
}

pub fn serve_session(
    session: &NavigationSession,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
) -> anyhow::Result<()> {
    serve_session_with_freshness(session, None, registry, transport)
}

pub fn serve_session_with_freshness(
    session: &NavigationSession,
    freshness: Option<&FreshnessProbe>,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
) -> anyhow::Result<()> {
    let mut runtime = StaticRuntime { session, freshness };
    serve_runtime(&mut runtime, registry, transport)
}

fn serve_runtime(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
) -> anyhow::Result<()> {
    let mut state = Lifecycle::PreInit;

    while let Some(outcome) = transport.read_message()? {
        let line = match outcome {
            ReadOutcome::Message(line) => line,
            ReadOutcome::Malformed => {
                // Recoverable framing/encoding fault: reply and keep the session alive.
                transport.write_message(error_response(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };
        let message = match serde_json::from_str::<Value>(&line) {
            Ok(message) => message,
            Err(_) => {
                transport.write_message(error_response(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };

        let response = match handle_message(&message, runtime, registry, &mut state) {
            Dispatch::Response(response) => Some(response),
            Dispatch::NoResponse => None,
        };
        if let Some(response) = response {
            transport.write_message(response)?;
        }
    }

    Ok(())
}

pub fn serve_stdio(p: &mut SessionProvider, r: &ToolRegistry) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut transport = StdioTransport::new(stdin.lock(), stdout.lock());
    serve_runtime(p, r, &mut transport)
}

trait SessionRuntime {
    fn session(&self) -> &NavigationSession;
    fn freshness(&self) -> Option<&FreshnessProbe>;
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary>;
}

struct StaticRuntime<'a> {
    session: &'a NavigationSession,
    freshness: Option<&'a FreshnessProbe>,
}

impl SessionRuntime for StaticRuntime<'_> {
    fn session(&self) -> &NavigationSession {
        self.session
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        self.freshness
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        anyhow::bail!("refresh_index requires provider-backed prism-mcp transport")
    }
}

impl SessionRuntime for SessionProvider {
    fn session(&self) -> &NavigationSession {
        SessionProvider::session(self)
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        Some(SessionProvider::freshness(self))
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        self.refresh()
    }
}

enum Dispatch {
    Response(Value),
    NoResponse,
}

fn handle_message(
    message: &Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    state: &mut Lifecycle,
) -> Dispatch {
    if message.is_array() {
        return Dispatch::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let Some(obj) = message.as_object() else {
        return Dispatch::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Dispatch::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    }
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        return Dispatch::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    };

    // Notifications are fire-and-forget. MCP §9: receipt of `notifications/initialized` transitions the
    // session "regardless of its body", so handle any `notifications/*` BEFORE the id/gating checks — a
    // non-conformant client that attaches an `id` must not hit the pre-init gate and permanently deadlock
    // the session (round-8 MINOR). A stray `notifications/initialized` before a valid `initialize` is
    // still ignored (no transition).
    if method.starts_with("notifications/") {
        if method == "notifications/initialized" && *state == Lifecycle::InitializeReceived {
            *state = Lifecycle::Initialized;
        }
        return Dispatch::NoResponse;
    }

    let id = obj.get("id").cloned();
    if id.is_none() {
        // A non-notification request with no id is invalid.
        return Dispatch::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let id = id.unwrap_or(Value::Null);

    // Validate the echoed id's shape AND size (re-review). Same rule as `is_safe_id`; reject a bad id
    // with a Null-id error (we won't echo a bad id).
    if !is_safe_id(&id) {
        return Dispatch::Response(error_response(
            Value::Null,
            -32600,
            "Invalid Request: bad id",
        ));
    }

    // Pre-initialized gating: only `initialize` and `ping` are allowed before `Initialized`.
    if *state != Lifecycle::Initialized && method != "initialize" && method != "ping" {
        return Dispatch::Response(error_response(id, -32600, "Invalid Request"));
    }

    match method {
        "initialize" => {
            let dispatch = initialize_response(obj, id);
            // Advance ONLY from PreInit, and only on a *successful* initialize. A repeat initialize once
            // negotiated responds but never downgrades the state (lifecycle is monotonic — re-review MAJOR).
            if *state == Lifecycle::PreInit
                && matches!(&dispatch, Dispatch::Response(v) if v.get("result").is_some())
            {
                *state = Lifecycle::InitializeReceived;
            }
            dispatch
        }
        "ping" => Dispatch::Response(success_response(id, json!({}))),
        "tools/list" => Dispatch::Response(success_response(id, list_tools(registry))),
        "tools/call" => call_tool_response(obj, id, runtime, registry),
        _ => Dispatch::Response(error_response(id, -32601, "Method not found")),
    }
}

fn initialize_response(obj: &Map<String, Value>, id: Value) -> Dispatch {
    let Some(params) = obj.get("params").and_then(Value::as_object) else {
        return Dispatch::Response(error_response(id, -32602, "Invalid params"));
    };
    if params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .is_none()
        || !matches!(params.get("capabilities"), Some(Value::Object(_)))
        || !matches!(params.get("clientInfo"), Some(Value::Object(_)))
    {
        return Dispatch::Response(error_response(id, -32602, "Invalid params"));
    }

    // Compatibility policy (spec §9): the server always offers its single supported version
    // (`PROTOCOL_VERSION`) regardless of what the client requested — the client then proceeds or
    // disconnects. We do not adopt the client's version, so an incompatible client is never
    // silently treated as negotiated on its own version.
    Dispatch::Response(success_response(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "prism-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    ))
}

fn list_tools(registry: &ToolRegistry) -> Value {
    Value::Object(
        [(
            "tools".into(),
            Value::Array(
                registry
                    .list()
                    .iter()
                    .map(|tool| tool.to_listed())
                    .collect(),
            ),
        )]
        .into_iter()
        .collect(),
    )
}

fn call_tool_response(
    obj: &Map<String, Value>,
    id: Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
) -> Dispatch {
    let Some(params) = obj.get("params").and_then(Value::as_object) else {
        return Dispatch::Response(error_response(id, -32602, "Invalid params"));
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Dispatch::Response(error_response(id, -32602, "Invalid params"));
    };
    // Per MCP, `arguments` must be an object. A wrong-TYPE `arguments` is a protocol error (-32602),
    // distinct from a bad tool INPUT (which correctly returns a result with isError) — round-6 MAJOR.
    let arguments = match params.get("arguments") {
        None => json!({}),
        Some(v) if v.is_object() => v.clone(),
        Some(_) => {
            return Dispatch::Response(error_response(
                id,
                -32602,
                "Invalid params: arguments must be an object",
            ));
        }
    };

    let Some(tool) = registry.get(name) else {
        return Dispatch::Response(success_response(
            id,
            unknown_tool_result(name, registry).to_call_tool_result_value(),
        ));
    };

    if tool.runtime_behavior == Some(ToolRuntimeBehavior::RefreshIndex) {
        let result = if arguments.as_object().is_some_and(|obj| obj.is_empty()) {
            match runtime.refresh_index() {
                Ok(summary) => tools_refresh::refresh_result(&summary),
                Err(error) => tools_refresh::refresh_error_result(&error),
            }
        } else {
            tools_refresh::invalid_arguments_result()
        };
        return Dispatch::Response(success_response(id, result.to_call_tool_result_value()));
    }

    let original_cap = resolve_cap();
    let report = runtime.freshness().map(FreshnessProbe::check);
    let stale = report.as_ref().is_some_and(|report| report.stale);
    let cap = if stale {
        original_cap.saturating_sub(FRESHNESS_RESERVE_BYTES)
    } else {
        original_cap
    };
    let ctx = ToolContext::new(runtime.session(), cap);
    let mut result = (tool.handler)(&ctx, &arguments);
    if stale && !result.is_error && result.structured.is_some() {
        if let Some(report) = &report {
            apply_freshness_report(&mut result, report, original_cap);
        }
    }

    Dispatch::Response(success_response(id, result.to_call_tool_result_value()))
}

fn unknown_tool_result(name: &str, registry: &ToolRegistry) -> McpToolResult {
    let available = registry
        .list()
        .iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>()
        .join(", ");
    let mut meta = Map::new();
    meta.insert(
        "prism/schema_version".into(),
        Value::String(SCHEMA_VERSION.into()),
    );
    let name = crate::mcp::output::clamp_user_text(name);
    McpToolResult {
        content_text: format!("unknown tool '{name}'; available [{available}]"),
        structured: None,
        is_error: true,
        meta,
    }
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

/// A request `id` we will echo unchanged: JSON-RPC restricts ids to string|number|null, and an
/// absurdly large id would bloat every response envelope past the size cap. One rule, used both by
/// the post-method `id_ok` gate and by `safe_id_from_message` (re-review MAJOR).
fn is_safe_id(id: &Value) -> bool {
    (id.is_string() || id.is_number() || id.is_null())
        && serde_json::to_string(id)
            .map(|s| s.len())
            .unwrap_or(usize::MAX)
            <= MAX_ID_BYTES
}

/// Extract a malformed envelope's id to echo, but ONLY if it is safe to echo (`is_safe_id`);
/// otherwise return `Null`. The malformed-envelope error paths run before the post-method `id_ok`
/// gate, so without this they could echo an unvalidated oversized/non-scalar id (re-review MAJOR).
fn safe_id_from_message(message: &Value) -> Value {
    let id = message
        .as_object()
        .and_then(|obj| obj.get("id"))
        .cloned()
        .unwrap_or(Value::Null);
    if is_safe_id(&id) {
        id
    } else {
        Value::Null
    }
}

struct StdioTransport<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> StdioTransport<R, W> {
    fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

/// Strip a trailing `\r`, then UTF-8 validate the whole accumulated line. Returns `None` on a UTF-8
/// failure so the caller can map it to a recoverable `ReadOutcome::Malformed` instead of a fatal
/// error (codex MAJOR — one stray non-UTF-8 byte must not kill the server).
fn finish_line(mut bytes: Vec<u8>) -> Option<String> {
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8(bytes).ok()
}

impl<R: BufRead, W: Write> StdioTransport<R, W> {
    /// Consume bytes from the reader up to and including the next `\n` (or until EOF) so the stream
    /// resyncs to a line boundary after an oversized frame. Bounded only by where the next newline
    /// is; the oversized payload is dropped, not buffered.
    fn drain_to_newline(&mut self) -> anyhow::Result<()> {
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return Ok(()); // EOF
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(pos) => {
                    self.reader.consume(pos + 1);
                    return Ok(());
                }
                None => {
                    let len = available.len();
                    self.reader.consume(len);
                }
            }
        }
    }
}

impl<R: BufRead, W: Write> Transport for StdioTransport<R, W> {
    /// Read one newline-delimited message, **bounded to `MAX_REQUEST_BYTES`**. A genuine IO error
    /// (from `fill_buf`) still propagates (fatal); clean EOF returns `Ok(None)`. A bad-UTF-8 line or
    /// an oversized line is a *recoverable* fault: it returns `ReadOutcome::Malformed` (after draining
    /// the rest of the oversized line so the stream resyncs) instead of killing the server (codex
    /// MAJOR). Accumulates bytes and UTF-8-validates once at the end (correct across multi-byte
    /// boundaries).
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let available = self.reader.fill_buf()?;
            if available.is_empty() {
                return if bytes.is_empty() {
                    Ok(None) // EOF, clean
                } else {
                    // final line without trailing newline
                    Ok(Some(match finish_line(bytes) {
                        Some(line) => ReadOutcome::Message(line),
                        None => ReadOutcome::Malformed,
                    }))
                };
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(pos) => {
                    if bytes.len() + pos > MAX_REQUEST_BYTES {
                        self.drain_to_newline()?;
                        return Ok(Some(ReadOutcome::Malformed));
                    }
                    bytes.extend_from_slice(&available[..pos]);
                    self.reader.consume(pos + 1);
                    return Ok(Some(match finish_line(bytes) {
                        Some(line) => ReadOutcome::Message(line),
                        None => ReadOutcome::Malformed,
                    }));
                }
                None => {
                    if bytes.len() + available.len() > MAX_REQUEST_BYTES {
                        let len = available.len();
                        self.reader.consume(len);
                        self.drain_to_newline()?;
                        return Ok(Some(ReadOutcome::Malformed));
                    }
                    let len = available.len();
                    bytes.extend_from_slice(available);
                    self.reader.consume(len);
                }
            }
        }
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        serde_json::to_writer(&mut self.writer, &v)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
pub struct InMemoryTransport {
    inputs: VecDeque<String>,
    responses: Vec<Value>,
}

#[cfg(test)]
impl InMemoryTransport {
    pub fn new(inputs: Vec<&str>) -> Self {
        Self {
            inputs: inputs.into_iter().map(str::to_owned).collect(),
            responses: Vec::new(),
        }
    }

    pub fn responses(&self) -> &[Value] {
        &self.responses
    }
}

#[cfg(test)]
impl Transport for InMemoryTransport {
    fn read_message(&mut self) -> anyhow::Result<Option<ReadOutcome>> {
        Ok(self.inputs.pop_front().map(ReadOutcome::Message))
    }

    fn write_message(&mut self, v: Value) -> anyhow::Result<()> {
        self.responses.push(v);
        Ok(())
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
