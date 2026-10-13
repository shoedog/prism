#[cfg(test)]
pub use super::framing::InMemoryTransport;
use super::framing::LineTransport;
#[cfg(test)]
use super::framing::MAX_REQUEST_BYTES;
pub use super::framing::{ReadOutcome, Transport};

/// The name the existing transport test corpus uses for the stdio instantiation.
#[cfg(test)]
type StdioTransport<R, W> = LineTransport<R, W>;

#[cfg(test)]
use super::concise_shape::ConciseShapeMode;
use super::freshness::{
    apply_freshness_report, FreshnessProbe, FreshnessReport, FRESHNESS_RESERVE_BYTES,
};
use super::lazy::{LazySessionProvider, Readiness};
#[cfg(test)]
use super::output::StructuredContentMode;
use super::output::{clamp_user_text, McpToolResult, MAX_RESULT_CHARS_FLOOR, SCHEMA_VERSION};
use super::registry::{ToolContext, ToolDescriptor, ToolRegistry, ToolRuntimeBehavior};
use super::settings::ConnectionSettings;
use super::{
    tools_refresh, AutoRefreshSummary, RefreshPolicy, RefreshSummary, RefreshVerification,
    SessionProvider, StartupMode,
};
use crate::navigation::NavigationSession;
use serde_json::{json, Map, Value};
use std::io;

const PROTOCOL_VERSION: &str = "2025-11-25";
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
pub(crate) const AUTO_REFRESH_RESERVE_BYTES: usize = 2048;
pub(crate) const MIN_MUTATING_TOOL_CAP_BYTES: usize = 4096;

const _: () = assert!(
    MAX_RESULT_CHARS_FLOOR
        >= FRESHNESS_RESERVE_BYTES
            + AUTO_REFRESH_RESERVE_BYTES
            + MIN_MUTATING_TOOL_CAP_BYTES
            + ENVELOPE_RESERVE
);

/// Result-payload budget under `cap` once this transport's envelope reserve is removed. The result
/// shaper (`output::shape_result`) sizes results against this so value + envelope <= `cap` on the wire.
pub(crate) fn payload_budget(cap: usize) -> usize {
    cap.saturating_sub(ENVELOPE_RESERVE)
}

/// Lifecycle gate (holistic-review MAJOR): a bare `bool` let `notifications/initialized` mark the
/// session initialized *before* a valid `initialize`. Three states enforce the handshake order:
/// `PreInit` → (valid `initialize`) → `InitializeReceived` → (`notifications/initialized`) → `Initialized`.
#[derive(PartialEq, Eq)]
pub(super) enum Lifecycle {
    PreInit,
    InitializeReceived,
    Initialized,
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
    serve_connection(
        runtime,
        registry,
        transport,
        &ConnectionSettings::from_env(),
    )
}

/// Serve one connection until its transport reaches end of input. `settings` are this
/// connection's output settings, fixed for its whole life.
pub(super) fn serve_connection(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    transport: &mut impl Transport,
    settings: &ConnectionSettings,
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

        let response = match handle_message_with(&message, runtime, registry, &mut state, settings)
        {
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
    serve_stdio_runtime(p, r)
}

pub(super) fn serve_stdio_runtime(
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut transport = LineTransport::new(stdin.lock(), stdout.lock());
    serve_runtime(runtime, registry, &mut transport)
}

pub(super) trait SessionRuntime {
    fn ensure_ready(&mut self) -> Readiness;
    fn admission_diagnostic(&self) -> Option<Value> {
        None
    }
    fn startup_mode(&self) -> StartupMode;
    fn session(&self) -> &NavigationSession;
    fn freshness(&self) -> Option<&FreshnessProbe>;
    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport>;
    fn refresh_policy(&self) -> RefreshPolicy;
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary>;
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary>;
}

struct StaticRuntime<'a> {
    session: &'a NavigationSession,
    freshness: Option<&'a FreshnessProbe>,
}

impl SessionRuntime for StaticRuntime<'_> {
    fn ensure_ready(&mut self) -> Readiness {
        Readiness::Ready
    }

    fn startup_mode(&self) -> StartupMode {
        StartupMode::Eager
    }

    fn session(&self) -> &NavigationSession {
        self.session
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        self.freshness
    }

    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        None
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::WarnOnly
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        anyhow::bail!("refresh_index requires provider-backed prism-mcp transport")
    }

    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        anyhow::bail!("auto refresh requires provider-backed prism-mcp transport")
    }
}

impl SessionRuntime for SessionProvider {
    fn ensure_ready(&mut self) -> Readiness {
        Readiness::Ready
    }

    fn startup_mode(&self) -> StartupMode {
        StartupMode::Eager
    }

    fn session(&self) -> &NavigationSession {
        SessionProvider::session(self)
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        Some(SessionProvider::freshness(self))
    }

    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        SessionProvider::known_stale_after_refresh(self)
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        SessionProvider::refresh_policy(self)
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        self.refresh()
    }

    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        self.auto_refresh()
    }
}

impl SessionRuntime for LazySessionProvider {
    fn ensure_ready(&mut self) -> Readiness {
        LazySessionProvider::ensure_ready(self)
    }

    fn startup_mode(&self) -> StartupMode {
        LazySessionProvider::startup_mode(self)
    }

    fn session(&self) -> &NavigationSession {
        self.ready()
            .expect("lazy runtime methods run only after readiness")
            .session()
    }

    fn freshness(&self) -> Option<&FreshnessProbe> {
        Some(
            self.ready()
                .expect("lazy runtime methods run only after readiness")
                .freshness(),
        )
    }

    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.ready()
            .expect("lazy runtime methods run only after readiness")
            .known_stale_after_refresh()
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        self.ready()
            .expect("lazy runtime methods run only after readiness")
            .refresh_policy()
    }

    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        self.ready_mut()
            .expect("lazy runtime methods run only after readiness")
            .refresh()
    }

    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        self.ready_mut()
            .expect("lazy runtime methods run only after readiness")
            .auto_refresh()
    }
}

enum Dispatch {
    Response(Value),
    NoResponse,
}

/// A `tools/call` that passed every check that needs no index: params shape, a registered tool
/// and, for `refresh_index`, empty arguments. Produced by `handle_control`; executed by the
/// `dispatch_tool_*` functions against a provider.
pub(super) struct ToolCall<'r> {
    id: Value,
    tool: &'r ToolDescriptor,
    arguments: Value,
}

impl ToolCall<'_> {
    /// Whether this call is `refresh_index`, which needs exclusive access to the provider.
    pub(super) fn is_refresh(&self) -> bool {
        self.tool.runtime_behavior == Some(ToolRuntimeBehavior::RefreshIndex)
    }
}

/// What the control path made of one message.
pub(super) enum Control<'r> {
    Response(Value),
    NoResponse,
    /// A validated tool call, to be run once the index is ready.
    ToolCall(ToolCall<'r>),
}

/// The in-process composition of control and dispatch over one `SessionRuntime`.
fn handle_message_with(
    message: &Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    state: &mut Lifecycle,
    settings: &ConnectionSettings,
) -> Dispatch {
    match handle_control(message, state, registry, runtime.startup_mode(), settings) {
        Control::Response(response) => Dispatch::Response(response),
        Control::NoResponse => Dispatch::NoResponse,
        Control::ToolCall(call) => Dispatch::Response(dispatch_tool(&call, runtime, settings)),
    }
}

/// Test entry point with the pre-split signature (used by `owner_transport_tests.rs`).
#[cfg(all(test, feature = "detached-owner-audit"))]
fn handle_message(
    message: &Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    state: &mut Lifecycle,
) -> Dispatch {
    handle_message_with(
        message,
        runtime,
        registry,
        state,
        &ConnectionSettings::from_env(),
    )
}

/// Everything a message needs that does not touch the index: envelope validation, the lifecycle
/// gate, `initialize`, `ping`, `tools/list`, and validation of a `tools/call` up to the point
/// where it must run. Takes no runtime, so a caller can answer the handshake while another
/// client's build or refresh holds the provider.
pub(super) fn handle_control<'r>(
    message: &Value,
    state: &mut Lifecycle,
    registry: &'r ToolRegistry,
    startup_mode: StartupMode,
    settings: &ConnectionSettings,
) -> Control<'r> {
    if message.is_array() {
        return Control::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let Some(obj) = message.as_object() else {
        return Control::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    };
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Control::Response(error_response(
            safe_id_from_message(message),
            -32600,
            "Invalid Request",
        ));
    }
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        return Control::Response(error_response(
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
        return Control::NoResponse;
    }

    let id = obj.get("id").cloned();
    if id.is_none() {
        // A non-notification request with no id is invalid.
        return Control::Response(error_response(Value::Null, -32600, "Invalid Request"));
    }
    let id = id.unwrap_or(Value::Null);

    // Validate the echoed id's shape AND size (re-review). Same rule as `is_safe_id`; reject a bad id
    // with a Null-id error (we won't echo a bad id).
    if !is_safe_id(&id) {
        return Control::Response(error_response(
            Value::Null,
            -32600,
            "Invalid Request: bad id",
        ));
    }

    // Pre-initialized gating: only `initialize` and `ping` are allowed before `Initialized`.
    if *state != Lifecycle::Initialized && method != "initialize" && method != "ping" {
        return Control::Response(error_response(id, -32600, "Invalid Request"));
    }

    match method {
        "initialize" => {
            let response = initialize_response(obj, id, startup_mode);
            // Advance ONLY from PreInit, and only on a *successful* initialize. A repeat initialize once
            // negotiated responds but never downgrades the state (lifecycle is monotonic — re-review MAJOR).
            if *state == Lifecycle::PreInit && response.get("result").is_some() {
                *state = Lifecycle::InitializeReceived;
            }
            Control::Response(response)
        }
        "ping" => Control::Response(success_response(id, json!({}))),
        "tools/list" => Control::Response(success_response(id, list_tools(registry))),
        "tools/call" => parse_tool_call(obj, id, registry, settings),
        _ => Control::Response(error_response(id, -32601, "Method not found")),
    }
}

fn initialize_response(obj: &Map<String, Value>, id: Value, startup_mode: StartupMode) -> Value {
    let Some(params) = obj.get("params").and_then(Value::as_object) else {
        return error_response(id, -32602, "Invalid params");
    };
    if params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .is_none()
        || !matches!(params.get("capabilities"), Some(Value::Object(_)))
        || !matches!(params.get("clientInfo"), Some(Value::Object(_)))
    {
        return error_response(id, -32602, "Invalid params");
    }

    // Compatibility policy (spec §9): the server always offers its single supported version
    // (`PROTOCOL_VERSION`) regardless of what the client requested — the client then proceeds or
    // disconnects. We do not adopt the client's version, so an incompatible client is never
    // silently treated as negotiated on its own version.
    success_response(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "prism-mcp",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": server_instructions(startup_mode)
        }),
    )
}

/// S1: the snapshot + view notices, stated ONCE here (the protocol-legal home for state-once
/// server text) instead of being repeated in full on every nav tool description. Each tool
/// description keeps only a short hedge (`crate::mcp::tools::SNAPSHOT_VIEW_HEDGE` and
/// `tools_reasoning`'s equivalent) pointing back here, since client ingestion of `instructions`
/// is unverified (codex MAJOR) — the hedge preserves discoverability even for a client that never
/// surfaces it.
fn server_instructions(startup_mode: StartupMode) -> String {
    let snapshot_notice = match startup_mode {
        StartupMode::Eager => crate::mcp::tools::SNAPSHOT_NOTICE,
        StartupMode::Lazy => "The repository snapshot is loaded by a background build started by the first tool call; until it completes, tool calls return an `index warming` result — retry shortly. Freshness warnings compare the working tree against the most recently completed build or refresh snapshot.",
        StartupMode::Background => "The repository snapshot is loaded by a background build started at server startup; until it completes, tool calls return an `index warming` result — retry shortly. Freshness warnings compare the working tree against the most recently completed build or refresh snapshot.",
    };
    format!("{snapshot_notice} {}", crate::mcp::tools::VIEW_NOTICE)
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

/// Test-only entry point that fixes the cap but not the S2/S3 env-gated modes (always
/// `StructuredContentMode::Always` and `ConciseShapeMode::Legacy`, i.e. today's unconditional
/// behavior) — existing `transport_tests.rs` callers inject a deterministic `cap` this way without
/// touching either gate.
#[cfg(test)]
fn call_tool_response_with_cap(
    obj: &Map<String, Value>,
    id: Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    original_cap: usize,
) -> Dispatch {
    call_tool_response_with_cap_and_mode(
        obj,
        id,
        runtime,
        registry,
        original_cap,
        StructuredContentMode::Always,
        ConciseShapeMode::Legacy,
    )
}

/// Test-only entry point with the pre-split signature: one `tools/call` with every output setting
/// given explicitly.
#[cfg(test)]
fn call_tool_response_with_cap_and_mode(
    obj: &Map<String, Value>,
    id: Value,
    runtime: &mut impl SessionRuntime,
    registry: &ToolRegistry,
    original_cap: usize,
    structured_content_mode: StructuredContentMode,
    concise_shape_mode: ConciseShapeMode,
) -> Dispatch {
    let settings = ConnectionSettings {
        result_cap: original_cap,
        structured_content: structured_content_mode,
        concise_shape: concise_shape_mode,
    };
    match parse_tool_call(obj, id, registry, &settings) {
        Control::Response(response) => Dispatch::Response(response),
        Control::NoResponse => Dispatch::NoResponse,
        Control::ToolCall(call) => Dispatch::Response(dispatch_tool(&call, runtime, &settings)),
    }
}

/// Validate a `tools/call` as far as possible without the index.
fn parse_tool_call<'r>(
    obj: &Map<String, Value>,
    id: Value,
    registry: &'r ToolRegistry,
    settings: &ConnectionSettings,
) -> Control<'r> {
    let Some(params) = obj.get("params").and_then(Value::as_object) else {
        return Control::Response(error_response(id, -32602, "Invalid params"));
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Control::Response(error_response(id, -32602, "Invalid params"));
    };
    // Per MCP, `arguments` must be an object. A wrong-TYPE `arguments` is a protocol error (-32602),
    // distinct from a bad tool INPUT (which correctly returns a result with isError) — round-6 MAJOR.
    let arguments = match params.get("arguments") {
        None => json!({}),
        Some(v) if v.is_object() => v.clone(),
        Some(_) => {
            return Control::Response(error_response(
                id,
                -32602,
                "Invalid params: arguments must be an object",
            ));
        }
    };

    let Some(tool) = registry.get(name) else {
        return Control::Response(success_response(
            id,
            unknown_tool_result(name, registry)
                .to_call_tool_result_value(settings.structured_content),
        ));
    };

    if tool.runtime_behavior == Some(ToolRuntimeBehavior::RefreshIndex)
        && !arguments.as_object().is_some_and(|obj| obj.is_empty())
    {
        return Control::Response(success_response(
            id,
            tools_refresh::invalid_arguments_result()
                .to_call_tool_result_value(settings.structured_content),
        ));
    }

    Control::ToolCall(ToolCall {
        id,
        tool,
        arguments,
    })
}

/// Run one validated call against an in-process runtime: readiness first, then the refresh tool
/// or a read that may hand over to the automatic-refresh path.
fn dispatch_tool(
    call: &ToolCall<'_>,
    runtime: &mut impl SessionRuntime,
    settings: &ConnectionSettings,
) -> Value {
    let readiness = runtime.ensure_ready();
    if let Some(response) =
        readiness_response(call, readiness, || runtime.admission_diagnostic(), settings)
    {
        return response;
    }
    if call.is_refresh() {
        return dispatch_tool_refresh(call, runtime, settings);
    }
    match dispatch_tool_read(call, &*runtime, settings) {
        ReadDispatch::Response(response) => response,
        ReadDispatch::NeedsAutoRefresh { initial_report } => {
            dispatch_tool_auto_refresh(call, runtime, initial_report, settings)
        }
    }
}

/// The response for a call that cannot run yet; `None` when the index is ready. `admission` is
/// asked only when the build failed.
pub(super) fn readiness_response(
    call: &ToolCall<'_>,
    readiness: Readiness,
    admission: impl FnOnce() -> Option<Value>,
    settings: &ConnectionSettings,
) -> Option<Value> {
    let result = match readiness {
        Readiness::Ready => return None,
        Readiness::Warming { elapsed } => warming_result(elapsed),
        Readiness::Failed { error } => build_failure_result(&error, admission()),
    };
    Some(success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    ))
}

/// `refresh_index`: rebuild the snapshot. Needs exclusive access to the provider.
pub(super) fn dispatch_tool_refresh(
    call: &ToolCall<'_>,
    provider: &mut impl SessionRuntime,
    settings: &ConnectionSettings,
) -> Value {
    let result = match provider.refresh_index() {
        Ok(summary) => tools_refresh::refresh_result(&summary),
        Err(error) => tools_refresh::refresh_error_result(&error),
    };
    success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    )
}

/// Outcome of running a query with shared access.
pub(super) enum ReadDispatch {
    Response(Value),
    /// The index is stale and the refresh policy is automatic: the caller must take exclusive
    /// access and finish through `dispatch_tool_auto_refresh`, passing this report on.
    NeedsAutoRefresh {
        initial_report: Option<FreshnessReport>,
    },
}

/// Run a query against a ready provider through shared access only. `view` is the read-only
/// facade: the whole provider, including the sticky stale evidence a refresh recorded, with the
/// precedence `effective_stale_report` gives it.
pub(super) fn dispatch_tool_read(
    call: &ToolCall<'_>,
    view: &impl SessionRuntime,
    settings: &ConnectionSettings,
) -> ReadDispatch {
    let report = effective_stale_report(view);
    let stale = report.as_ref().is_some_and(|report| report.stale);
    if stale
        && matches!(
            view.refresh_policy(),
            RefreshPolicy::AutoFull | RefreshPolicy::AutoIncremental
        )
    {
        return ReadDispatch::NeedsAutoRefresh {
            initial_report: report,
        };
    }
    let cap = if stale {
        cap_after_reserve(settings.result_cap, FRESHNESS_RESERVE_BYTES)
    } else {
        settings.result_cap
    };
    let ctx = ToolContext::new(
        view.session(),
        cap,
        settings.concise_shape,
        settings.structured_content,
    );
    let mut result = (call.tool.handler)(&ctx, &call.arguments);
    if stale && !result.is_error && result.structured.is_some() {
        if let Some(report) = &report {
            apply_freshness_report(&mut result, report, settings.result_cap);
        }
    }

    ReadDispatch::Response(success_response(
        call.id.clone(),
        result.to_call_tool_result_value(settings.structured_content),
    ))
}

/// Refresh a stale index under an automatic policy, then run the query on the result. Needs
/// exclusive access to the provider.
pub(super) fn dispatch_tool_auto_refresh(
    call: &ToolCall<'_>,
    provider: &mut impl SessionRuntime,
    initial_report: Option<FreshnessReport>,
    settings: &ConnectionSettings,
) -> Value {
    let handler = call.tool.handler.as_ref();
    let original_cap = settings.result_cap;
    let mode = settings.structured_content;
    let respond = |result: McpToolResult| {
        success_response(call.id.clone(), result.to_call_tool_result_value(mode))
    };
    match provider.auto_refresh_index() {
        Ok(summary) => {
            let stale_after_refresh = post_refresh_stale_report(provider, &summary);
            let may_apply_stale = stale_after_refresh
                .as_ref()
                .is_some_and(|report| report.stale);
            let reserve = AUTO_REFRESH_RESERVE_BYTES
                + if may_apply_stale {
                    FRESHNESS_RESERVE_BYTES
                } else {
                    0
                };
            let ctx = ToolContext::new(
                provider.session(),
                cap_after_reserve(original_cap, reserve),
                settings.concise_shape,
                mode,
            );
            let mut result = handler(&ctx, &call.arguments);
            if result.is_error {
                return respond(result);
            }
            let status = match &summary.verification {
                RefreshVerification::Clean => "refreshed",
                RefreshVerification::Diverged(_) => "raced_stale",
            };
            apply_auto_refresh_metadata(&mut result, status, &summary, original_cap);
            if let Some(report) = stale_after_refresh.as_ref().filter(|report| report.stale) {
                if result.structured.is_some() {
                    apply_freshness_report(&mut result, report, original_cap);
                }
            }
            respond(result)
        }
        Err(error) => {
            let reserve = AUTO_REFRESH_RESERVE_BYTES + FRESHNESS_RESERVE_BYTES;
            let ctx = ToolContext::new(
                provider.session(),
                cap_after_reserve(original_cap, reserve),
                settings.concise_shape,
                mode,
            );
            let mut result = handler(&ctx, &call.arguments);
            if result.is_error {
                return respond(result);
            }
            if let Some(report) = initial_report.as_ref().filter(|report| report.stale) {
                if result.structured.is_some() {
                    apply_freshness_report(&mut result, report, original_cap);
                }
            }
            apply_auto_refresh_failure_metadata(&mut result, &error, original_cap);
            respond(result)
        }
    }
}

fn effective_stale_report(runtime: &impl SessionRuntime) -> Option<FreshnessReport> {
    runtime
        .known_stale_after_refresh()
        .cloned()
        .or_else(|| runtime.freshness().map(FreshnessProbe::check))
}

fn post_refresh_stale_report(
    runtime: &impl SessionRuntime,
    summary: &AutoRefreshSummary,
) -> Option<FreshnessReport> {
    match &summary.verification {
        RefreshVerification::Clean => effective_stale_report(runtime).filter(|report| report.stale),
        RefreshVerification::Diverged(report) => Some(report.clone()),
    }
}

fn cap_after_reserve(original_cap: usize, reserve: usize) -> usize {
    debug_assert!(
        original_cap >= reserve + MIN_MUTATING_TOOL_CAP_BYTES + ENVELOPE_RESERVE,
        "accepted MCP cap must leave room for reserve and minimum shaped result"
    );
    original_cap.saturating_sub(reserve)
}

fn apply_auto_refresh_metadata(
    result: &mut McpToolResult,
    status: &str,
    summary: &AutoRefreshSummary,
    original_cap: usize,
) {
    result.meta.insert(
        "prism/auto_refresh".into(),
        Value::String(status.to_string()),
    );
    result.meta.insert(
        "prism/refresh_generation".into(),
        Value::Number(serde_json::Number::from(summary.generation)),
    );
    result.meta.insert(
        "prism/refresh_strategy".into(),
        Value::String(summary.strategy.into()),
    );
    if let Some(fallback_reason) = summary.fallback_reason {
        result.meta.insert(
            "prism/refresh_fallback_reason".into(),
            Value::String(fallback_reason.into()),
        );
    }
    result.meta.insert(
        "prism/indexed_files".into(),
        Value::Number(serde_json::Number::from(summary.indexed_files)),
    );
    result.meta.insert(
        "prism/tracked_paths".into(),
        Value::Number(serde_json::Number::from(summary.tracked_paths)),
    );
    result.meta.insert(
        "prism/stale_index_total_before_refresh".into(),
        Value::Number(serde_json::Number::from(
            summary.stale_before_refresh.total_changed,
        )),
    );
    result.meta.insert(
        "prism/stale_index_paths_before_refresh".into(),
        Value::Array(
            summary
                .stale_before_refresh
                .changed_paths
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    result.meta.insert(
        "anthropic/maxResultSizeChars".into(),
        Value::Number(serde_json::Number::from(original_cap)),
    );
}

fn apply_auto_refresh_failure_metadata(
    result: &mut McpToolResult,
    error: &anyhow::Error,
    original_cap: usize,
) {
    result
        .meta
        .insert("prism/auto_refresh".into(), Value::String("failed".into()));
    result.meta.insert(
        "prism/auto_refresh_error".into(),
        Value::String(clamp_user_text(&error.to_string())),
    );
    result.meta.insert(
        "anthropic/maxResultSizeChars".into(),
        Value::Number(serde_json::Number::from(original_cap)),
    );
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

fn warming_result(elapsed: std::time::Duration) -> McpToolResult {
    retryable_status_result(
        json!({
            "status": "warming",
            "elapsed_secs": elapsed.as_secs(),
            "message": "prism-mcp is still building the repository index; retry this call shortly — no other action is needed; later calls are fast."
        }),
        "warming",
    )
}

fn build_failure_result(error: &str, admission: Option<Value>) -> McpToolResult {
    let mut status = json!({
        "status": "build_failed",
        "cause": clamp_user_text(error),
        "message": "the server keeps running; the next tool call retries the build"
    });
    // Error paths bypass normal shaping. Preserve the untrusted-text clamp and
    // independently bound the optional internal diagnostic, including both wire views.
    if let Some(report) = admission {
        if serde_json::to_vec(&report).is_ok_and(|bytes| bytes.len() <= 2048) {
            status["owner_admission"] = report;
        } else {
            status["owner_admission_omitted"] = Value::Bool(true);
        }
    }
    retryable_status_result(status, "failed")
}

fn retryable_status_result(status: Value, index_state: &'static str) -> McpToolResult {
    let content_text = serde_json::to_string(&status).expect("status result serializes");
    let mut meta = Map::new();
    meta.insert(
        "prism/schema_version".into(),
        Value::String(SCHEMA_VERSION.into()),
    );
    meta.insert(
        "prism/index_state".into(),
        Value::String(index_state.into()),
    );
    meta.insert("prism/retryable".into(), Value::Bool(true));
    McpToolResult {
        content_text,
        structured: Some(status),
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

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;

#[cfg(all(test, feature = "detached-owner-audit"))]
#[path = "owner_transport_tests.rs"]
mod owner_tests;

#[cfg(test)]
#[path = "transport_split_tests.rs"]
mod split_tests;
