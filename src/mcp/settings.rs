//! Output settings of one MCP connection. The in-process server resolves them once from its own
//! environment; the shared daemon builds one per connection from the shim's preamble, so a
//! client's output shape never depends on which client started the daemon.
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{resolve_cap, resolve_structured_content_mode, StructuredContentMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionSettings {
    /// Wire byte cap per tool result (`PRISM_MCP_MAX_RESULT_CHARS`).
    pub result_cap: usize,
    /// Whether the default path carries `structuredContent` (`PRISM_MCP_STRUCTURED_CONTENT`).
    pub structured_content: StructuredContentMode,
    /// Item shape of concise results (`PRISM_MCP_CONCISE_SHAPE`).
    pub concise_shape: ConciseShapeMode,
}

impl ConnectionSettings {
    /// Resolve from this process's environment. Each invalid value warns on stderr when it is
    /// resolved, so call this once per connection, not once per request.
    pub fn from_env() -> Self {
        Self {
            result_cap: resolve_cap(),
            structured_content: resolve_structured_content_mode(),
            concise_shape: resolve_concise_shape_mode(),
        }
    }
}
