//! The operator's control exchange with a running daemon (spec §4.7): `daemon-status` asks a
//! daemon what it is doing; `daemon-shutdown` asks one instance of it to drain once it has no
//! client. One line each way on a fresh connection, in place of the preamble's `HELLO`; then the
//! daemon closes the connection. The connection is never admitted, and the daemon's idle time
//! does not count it (S4's interface for S5b).
//!
//! ```text
//! operator -> daemon:  STATUS {"control":2}
//!                      SHUTDOWN {"control":2,"identity":"<64 hex>","pid":N,"start_time":N}
//! daemon -> operator:  STATUS {"control":2,"identity":"<64 hex>","pid":N,"start_time":N,...}
//!                  or  RETIRING {}  |  BUSY {"max_clients":N}  |  nothing, for a line it refuses
//! ```
//!
//! The exchange has a version of its own, `control`, apart from the preamble's `protocol`
//! (D97). The preamble's version is part of every identity key, and `daemon-status` lists the
//! daemons of every build in the runtime directory (spec §4.6): an operator tool must be able to
//! ask a daemon of another build. A daemon that does not know these verbs, or this version,
//! closes the connection without a reply. Decoding is as strict as the preamble's: one spelling
//! per message, the one `encode` writes.
use super::identity::IdentityKey;
use super::preamble::{
    clip, key_field, parse_object, require_canonical, verb_and_payload, BuildState, PreambleError,
    MAX_VERB_BYTES,
};
use serde::Deserialize;
use std::path::Path;

/// Version of the control exchange. Not part of any identity key.
pub const CONTROL_VERSION: u32 = 2;

/// Most characters of the repository root a status reply carries. A JSON string can take six
/// bytes per character, and the reply must fit one preamble line (4 KiB) with its other fields.
pub const MAX_ROOT_CHARS: usize = 512;

/// The verbs of the control exchange. Both requests and the status reply use them.
const CONTROL_VERBS: [&str; 2] = ["STATUS", "SHUTDOWN"];

/// Whether `line` (without its newline) is a control request rather than a `HELLO`: its verb is
/// `STATUS` or `SHUTDOWN`, followed by a space. Whether it is a valid one is
/// [`ControlRequest::decode`]'s to say.
pub fn is_control_line(line: &[u8]) -> bool {
    CONTROL_VERBS.iter().any(|verb| {
        line.strip_prefix(verb.as_bytes())
            .is_some_and(|rest| rest.first() == Some(&b' '))
    })
}

/// One daemon instance: the identity it serves, its pid and its start time (seconds since the
/// Unix epoch), as `READY` and the pid file name it (spec §4.7: "the instance verified through
/// the live control exchange (pid + start time echoed in `READY`)").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instance {
    pub identity: IdentityKey,
    pub pid: u32,
    pub start_time: u64,
}

/// What an operator asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlRequest {
    /// Say what you are doing.
    Status,
    /// If you are this instance, drain once nothing is admitted or pending (spec §11 item 3:
    /// graceful and instance-bound). A daemon that is another instance records nothing.
    Shutdown(Instance),
}

/// Read `control` alone and refuse any version but this build's, before the rest of the
/// payload is interpreted.
fn check_control(payload: &str) -> Result<(), PreambleError> {
    #[derive(Deserialize)]
    struct Versioned {
        control: u64,
    }
    let Versioned { control } = parse_object(payload)?;
    if control != u64::from(CONTROL_VERSION) {
        return Err(PreambleError::UnsupportedControl { got: control });
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShutdownWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
    identity: String,
    pid: u32,
    start_time: u64,
}

impl ControlRequest {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        match self {
            Self::Status => format!("STATUS {{\"control\":{CONTROL_VERSION}}}\n"),
            Self::Shutdown(instance) => format!(
                "SHUTDOWN {{\"control\":{CONTROL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{}}}\n",
                instance.identity, instance.pid, instance.start_time
            ),
        }
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (verb, payload) = verb_and_payload(line, &CONTROL_VERBS)?;
        check_control(payload)?;
        let request = match verb {
            "STATUS" => {
                let StatusWire { .. } = parse_object(payload)?;
                Self::Status
            }
            "SHUTDOWN" => {
                let wire: ShutdownWire = parse_object(payload)?;
                Self::Shutdown(Instance {
                    identity: key_field("identity", &wire.identity)?,
                    pid: wire.pid,
                    start_time: wire.start_time,
                })
            }
            other => {
                return Err(PreambleError::UnknownVerb {
                    verb: clip(other, MAX_VERB_BYTES),
                })
            }
        };
        require_canonical(line, &request.encode())?;
        Ok(request)
    }
}

/// What a daemon says about itself (spec §4.7: "key, repo root, build identity, pid, start
/// time, admitted and pending clients, build state and attempt, generation, idle time").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonStatus {
    pub instance: Instance,
    /// `PRISM_CACHE_BUILD_IDENTITY` of the daemon's build (the pid file's third line).
    pub build_identity: String,
    /// The canonical repository root, for a person to read ([`display_root`]).
    pub repo_root: String,
    /// Connections served now, and connections in the handshake stage besides this one.
    pub admitted: u32,
    pub pending: u32,
    pub max_clients: u32,
    pub build: BuildState,
    /// The number of the latest build attempt; 0 before the first.
    pub attempt: u64,
    pub generation: u64,
    /// Seconds since the daemon last had a client; `None` while it has one.
    pub idle_secs: Option<u64>,
    pub idle_ttl_secs: u64,
    pub index_idle_timeout_secs: u64,
    pub index_idle_pressure_relief: bool,
    /// Whether an operator has asked it to drain once it has no client.
    pub shutdown: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DaemonStatusWire {
    #[allow(dead_code)] // read by `check_control`
    control: u64,
    identity: String,
    pid: u32,
    start_time: u64,
    build_identity: String,
    repo_root: String,
    admitted: u32,
    pending: u32,
    max_clients: u32,
    build: String,
    attempt: u64,
    generation: u64,
    idle_secs: Option<u64>,
    idle_ttl_secs: u64,
    index_idle_timeout_secs: u64,
    index_idle_pressure_relief: bool,
    shutdown: bool,
}

/// `text` as a JSON string, quotes included.
fn json_string(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

impl DaemonStatus {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        let idle = self
            .idle_secs
            .map_or_else(|| "null".to_string(), |secs| secs.to_string());
        format!(
            "STATUS {{\"control\":{CONTROL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{},\"build_identity\":{},\"repo_root\":{},\"admitted\":{},\"pending\":{},\"max_clients\":{},\"build\":\"{}\",\"attempt\":{},\"generation\":{},\"idle_secs\":{idle},\"idle_ttl_secs\":{},\"index_idle_timeout_secs\":{},\"index_idle_pressure_relief\":{},\"shutdown\":{}}}\n",
            self.instance.identity,
            self.instance.pid,
            self.instance.start_time,
            json_string(&self.build_identity),
            json_string(&self.repo_root),
            self.admitted,
            self.pending,
            self.max_clients,
            self.build.word(),
            self.attempt,
            self.generation,
            self.idle_ttl_secs,
            self.index_idle_timeout_secs,
            self.index_idle_pressure_relief,
            self.shutdown,
        )
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (_, payload) = verb_and_payload(line, &["STATUS"])?;
        check_control(payload)?;
        let wire: DaemonStatusWire = parse_object(payload)?;
        let status = Self {
            instance: Instance {
                identity: key_field("identity", &wire.identity)?,
                pid: wire.pid,
                start_time: wire.start_time,
            },
            build_identity: wire.build_identity,
            repo_root: wire.repo_root,
            admitted: wire.admitted,
            pending: wire.pending,
            max_clients: wire.max_clients,
            build: BuildState::from_word(&wire.build)?,
            attempt: wire.attempt,
            generation: wire.generation,
            idle_secs: wire.idle_secs,
            idle_ttl_secs: wire.idle_ttl_secs,
            index_idle_timeout_secs: wire.index_idle_timeout_secs,
            index_idle_pressure_relief: wire.index_idle_pressure_relief,
            shutdown: wire.shutdown,
        };
        require_canonical(line, &status.encode())?;
        Ok(status)
    }
}

/// The repository root as a status reply shows it: lossy where it is not UTF-8, and cut to
/// [`MAX_ROOT_CHARS`] characters with `…` after it when longer.
pub fn display_root(root: &Path) -> String {
    let text = root.to_string_lossy();
    if text.chars().count() <= MAX_ROOT_CHARS {
        return text.into_owned();
    }
    let mut shown: String = text.chars().take(MAX_ROOT_CHARS).collect();
    shown.push('\u{2026}');
    shown
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
