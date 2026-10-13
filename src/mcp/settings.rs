//! Output settings of one MCP connection. The in-process server resolves them once from its own
//! environment; the shared daemon builds one per connection from the shim's preamble, so a
//! client's output shape never depends on which client started the daemon.
use super::concise_shape::{resolve_concise_shape_mode, ConciseShapeMode};
use super::output::{
    resolve_cap, resolve_structured_content_mode, StructuredContentMode, MAX_RESULT_CHARS_FLOOR,
};
use super::FIRST_CALL_WAIT_MAX;
use std::fmt;
use std::time::Duration;

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

/// A setting a client asked for that a server must not run with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The result cap is below `MAX_RESULT_CHARS_FLOOR`, the same floor
    /// `PRISM_MCP_MAX_RESULT_CHARS` has.
    ResultCapBelowFloor { cap: usize, floor: usize },
    /// The first-call wait is above `FIRST_CALL_WAIT_MAX`, the `--first-call-wait` maximum.
    FirstCallWaitTooLong { secs: u64, max_secs: u64 },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResultCapBelowFloor { cap, floor } => {
                write!(f, "result cap {cap} is below the floor of {floor}")
            }
            Self::FirstCallWaitTooLong { secs, max_secs } => write!(
                f,
                "first-call wait of {secs} s is above the maximum of {max_secs} s"
            ),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Everything one client of a shared daemon chooses for its own connection (spec §4.1 step 3,
/// §4.4): the output settings dispatch reads, plus the two that only the readiness path reads.
/// The shim resolves it from its own environment and flags and sends it in the preamble; it is
/// never inherited from the client that started the daemon.
///
/// [`ClientSettings::new`] is the only way to make one, and the fields are private, so a value
/// of this type always holds a result cap dispatch can subtract its reserves from and a wait
/// that can be added to an `Instant`. A daemon builds each connection's `ConnectionSettings`
/// from [`ClientSettings::output`] and from nowhere else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientSettings {
    output: ConnectionSettings,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
}

impl ClientSettings {
    /// Check the two bounded settings and keep the rest as given. `first_call_wait_secs` is
    /// `--first-call-wait`; `warm_at_startup` is `--warm-at-startup`.
    pub fn new(
        output: ConnectionSettings,
        first_call_wait_secs: u64,
        warm_at_startup: bool,
    ) -> Result<Self, SettingsError> {
        if output.result_cap < MAX_RESULT_CHARS_FLOOR {
            return Err(SettingsError::ResultCapBelowFloor {
                cap: output.result_cap,
                floor: MAX_RESULT_CHARS_FLOOR,
            });
        }
        let max_secs = FIRST_CALL_WAIT_MAX.as_secs();
        if first_call_wait_secs > max_secs {
            return Err(SettingsError::FirstCallWaitTooLong {
                secs: first_call_wait_secs,
                max_secs,
            });
        }
        Ok(Self {
            output,
            first_call_wait_secs,
            warm_at_startup,
        })
    }

    /// The output settings for this connection's dispatch.
    pub fn output(&self) -> &ConnectionSettings {
        &self.output
    }

    /// How long this connection's first readiness-requiring call may wait for the index.
    pub fn first_call_wait(&self) -> Duration {
        Duration::from_secs(self.first_call_wait_secs)
    }

    pub fn first_call_wait_secs(&self) -> u64 {
        self.first_call_wait_secs
    }

    /// Whether this client asked for the index to be built at daemon start. Only the daemon
    /// this client starts honors it (spec §8).
    pub fn warm_at_startup(&self) -> bool {
        self.warm_at_startup
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
