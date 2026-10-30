//! The private preamble: one line each way on a fresh connection, before any MCP byte (spec
//! §4.4). The shim says which identity it wants and how its own connection should behave; the
//! daemon admits it (`READY`) or says why not.
//!
//! ```text
//! shim  -> daemon:  HELLO {"protocol":2,"identity":"<64 hex>","settings":{...}}
//! daemon -> shim:   READY {"protocol":2,"identity":"<64 hex>","pid":N,"start_time":N,"generation":N,"build":"..."}
//!              or   RETIRING {}  |  BUSY {"max_clients":N}  |  MISMATCH {"expected":"<64 hex>"}
//! ```
//!
//! A line arrives from another process, so decoding refuses everything it does not expect: a
//! message has exactly one spelling, the one its `encode` writes, and every value is checked
//! before it becomes a typed setting. Both ends are the same build (the build identity is part
//! of the key), so there is nothing to be lenient towards.
use super::identity::IdentityKey;
use super::SHARED_PROTOCOL_VERSION;
use crate::mcp::concise_shape::ConciseShapeMode;
use crate::mcp::output::StructuredContentMode;
use crate::mcp::settings::{ClientSettings, ConnectionSettings, SettingsError};
use serde::Deserialize;
use std::fmt;

/// Longest preamble line, in bytes, not counting its newline (spec §4.2, §7: "4 KiB").
pub const PREAMBLE_MAX_BYTES: usize = 4096;

/// Longest piece of the other side's text an error message repeats.
const MAX_DETAIL_BYTES: usize = 200;
pub(super) const MAX_VERB_BYTES: usize = 16;

/// Why a preamble line was refused. Nothing here is retried; what a daemon or a shim does
/// about each is theirs to decide (spec §10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreambleError {
    /// The line is longer than [`PREAMBLE_MAX_BYTES`].
    TooLong {
        len: usize,
    },
    NotUtf8,
    /// The line is not `VERB`, one space, payload.
    Malformed,
    /// A verb this side does not accept. `verb` is at most 16 bytes of it.
    UnknownVerb {
        verb: String,
    },
    /// The payload starts like a JSON object and is not valid JSON.
    NotJson {
        detail: String,
    },
    /// A well-formed message of another protocol version. Its identity cannot be the one
    /// served here, because the version is part of the identity key.
    UnsupportedProtocol {
        got: u64,
    },
    /// A control line (`STATUS`/`SHUTDOWN`) of another control-exchange version (D97). Kept apart
    /// from `UnsupportedProtocol` so a daemon's log names the right version.
    UnsupportedControl {
        got: u64,
    },
    /// Settings a server must not run with (result cap below the floor, wait above the
    /// maximum).
    Settings(SettingsError),
    /// Anything else: not a JSON object, a missing, unknown or repeated field, a value of the
    /// wrong kind, an unknown word, a malformed key, or a spelling `encode` would not write.
    Invalid {
        detail: String,
    },
}

impl fmt::Display for PreambleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { len } => write!(
                f,
                "preamble line is {len} bytes; the limit is {PREAMBLE_MAX_BYTES}"
            ),
            Self::NotUtf8 => write!(f, "preamble line is not UTF-8"),
            Self::Malformed => write!(f, "preamble line is not a verb followed by a payload"),
            Self::UnknownVerb { verb } => write!(f, "unexpected preamble verb {verb:?}"),
            Self::NotJson { detail } => write!(f, "preamble payload is not JSON: {detail}"),
            Self::UnsupportedProtocol { got } => write!(
                f,
                "preamble protocol version {got} is not {SHARED_PROTOCOL_VERSION}"
            ),
            Self::UnsupportedControl { got } => write!(
                f,
                "control exchange version {got} is not {}",
                super::control::CONTROL_VERSION
            ),
            Self::Settings(error) => write!(f, "preamble settings refused: {error}"),
            Self::Invalid { detail } => write!(f, "invalid preamble: {detail}"),
        }
    }
}

impl std::error::Error for PreambleError {}

/// `text` with every control character replaced. Text that repeats what the peer sent (a
/// field name inside a parser's message) must not be able to start a new line or drive a
/// terminal where the error is logged or shown.
fn printable(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

/// At most `max` bytes of `text`, cut on a character boundary.
pub(super) fn clip(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// An `Invalid` whose detail is this module's own wording (short by construction).
fn invalid(detail: impl Into<String>) -> PreambleError {
    PreambleError::Invalid {
        detail: detail.into(),
    }
}

/// The word for a structured-content mode: the one `PRISM_MCP_STRUCTURED_CONTENT` uses.
pub(crate) fn structured_content_word(mode: StructuredContentMode) -> &'static str {
    match mode {
        StructuredContentMode::Always => "always",
        StructuredContentMode::OmitDefaultPath => "omit-default-path",
    }
}

fn structured_content_from(word: &str) -> Result<StructuredContentMode, PreambleError> {
    match word {
        "always" => Ok(StructuredContentMode::Always),
        "omit-default-path" => Ok(StructuredContentMode::OmitDefaultPath),
        _ => Err(invalid(
            "structured_content is not \"always\" or \"omit-default-path\"",
        )),
    }
}

/// The word for a concise-shape mode: the one `PRISM_MCP_CONCISE_SHAPE` uses.
pub(crate) fn concise_shape_word(mode: ConciseShapeMode) -> &'static str {
    match mode {
        ConciseShapeMode::Legacy => "legacy",
        ConciseShapeMode::Slim => "slim",
    }
}

fn concise_shape_from(word: &str) -> Result<ConciseShapeMode, PreambleError> {
    match word {
        "legacy" => Ok(ConciseShapeMode::Legacy),
        "slim" => Ok(ConciseShapeMode::Slim),
        _ => Err(invalid("concise_shape is not \"legacy\" or \"slim\"")),
    }
}

/// State of the daemon's index build, as `READY` reports it (spec §4.2 readiness coordinator).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildState {
    Idle,
    Unloaded,
    Building,
    Ready,
    Failed,
}

impl BuildState {
    /// The word `READY` and the operator's status reply use for this state.
    pub fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Unloaded => "unloaded",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    pub(super) fn from_word(word: &str) -> Result<Self, PreambleError> {
        match word {
            "idle" => Ok(Self::Idle),
            "unloaded" => Ok(Self::Unloaded),
            "building" => Ok(Self::Building),
            "ready" => Ok(Self::Ready),
            "failed" => Ok(Self::Failed),
            _ => Err(invalid("build is not a known index state")),
        }
    }
}

/// Split a line into its verb and payload after the checks every line gets: length, UTF-8,
/// shape, and that the verb is one of `verbs`.
pub(super) fn verb_and_payload<'l>(
    line: &'l [u8],
    verbs: &[&'static str],
) -> Result<(&'static str, &'l str), PreambleError> {
    if line.len() > PREAMBLE_MAX_BYTES {
        return Err(PreambleError::TooLong { len: line.len() });
    }
    let text = std::str::from_utf8(line).map_err(|_| PreambleError::NotUtf8)?;
    let (verb, payload) = text.split_once(' ').ok_or(PreambleError::Malformed)?;
    match verbs.iter().find(|known| **known == verb) {
        Some(known) => Ok((known, payload)),
        None => Err(PreambleError::UnknownVerb {
            verb: clip(verb, MAX_VERB_BYTES),
        }),
    }
}

/// Parse a payload as the JSON object `T`. Only an object is accepted: `serde` would also take
/// a positional array for a struct, and nothing here writes one.
pub(super) fn parse_object<'p, T: Deserialize<'p>>(payload: &'p str) -> Result<T, PreambleError> {
    if !payload.starts_with('{') {
        return Err(invalid("the payload is not a JSON object"));
    }
    serde_json::from_str(payload).map_err(|error| {
        let detail = clip(&printable(&error.to_string()), MAX_DETAIL_BYTES);
        match error.classify() {
            serde_json::error::Category::Data => PreambleError::Invalid { detail },
            _ => PreambleError::NotJson { detail },
        }
    })
}

/// Read `protocol` alone and refuse any version but this build's. This runs before the rest of
/// the payload is interpreted, because another version's payload may have another shape.
fn check_protocol(payload: &str) -> Result<(), PreambleError> {
    #[derive(Deserialize)]
    struct Versioned {
        protocol: u64,
    }
    let Versioned { protocol } = parse_object(payload)?;
    if protocol != u64::from(SHARED_PROTOCOL_VERSION) {
        return Err(PreambleError::UnsupportedProtocol { got: protocol });
    }
    Ok(())
}

pub(super) fn key_field(name: &str, hex: &str) -> Result<IdentityKey, PreambleError> {
    IdentityKey::from_hex(hex)
        .map_err(|error| invalid(format!("{name} is not an identity key: {error}")))
}

/// Refuse a decoded message whose line is not byte for byte what `encode` writes for it.
pub(super) fn require_canonical(line: &[u8], encoded: &str) -> Result<(), PreambleError> {
    if encoded.strip_suffix('\n').map(str::as_bytes) == Some(line) {
        Ok(())
    } else {
        Err(invalid(
            "the line is not spelled the way this build writes that message",
        ))
    }
}

/// What a shim sends: the identity it wants and its own connection settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello {
    pub identity: IdentityKey,
    pub settings: ClientSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HelloWire {
    #[allow(dead_code)] // read by `check_protocol`
    protocol: u64,
    identity: String,
    settings: SettingsWire,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsWire {
    structured_content: String,
    concise_shape: String,
    result_cap: u64,
    first_call_wait_secs: u64,
    warm_at_startup: bool,
}

impl Hello {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        let output = self.settings.output();
        format!(
            "HELLO {{\"protocol\":{SHARED_PROTOCOL_VERSION},\"identity\":\"{}\",\"settings\":{{\"structured_content\":\"{}\",\"concise_shape\":\"{}\",\"result_cap\":{},\"first_call_wait_secs\":{},\"warm_at_startup\":{}}}}}\n",
            self.identity,
            structured_content_word(output.structured_content),
            concise_shape_word(output.concise_shape),
            output.result_cap,
            self.settings.first_call_wait_secs(),
            self.settings.warm_at_startup(),
        )
    }

    /// Decode one received line (without its newline). `Ok` means: this protocol version, a
    /// full identity key, and settings that passed `ClientSettings::new`.
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (_, payload) = verb_and_payload(line, &["HELLO"])?;
        check_protocol(payload)?;
        let wire: HelloWire = parse_object(payload)?;
        let identity = key_field("identity", &wire.identity)?;
        let output = ConnectionSettings {
            result_cap: usize::try_from(wire.settings.result_cap)
                .map_err(|_| invalid("result_cap does not fit this platform's usize"))?,
            structured_content: structured_content_from(&wire.settings.structured_content)?,
            concise_shape: concise_shape_from(&wire.settings.concise_shape)?,
        };
        let settings = ClientSettings::new(
            output,
            wire.settings.first_call_wait_secs,
            wire.settings.warm_at_startup,
        )
        .map_err(PreambleError::Settings)?;
        let hello = Self { identity, settings };
        require_canonical(line, &hello.encode())?;
        Ok(hello)
    }

    /// `None` when this client asked for exactly the identity `served`; otherwise the
    /// `MISMATCH` reply it is owed. The comparison is of the full key: the 16 digits in the
    /// socket's file name only located the daemon (spec §4.2 "Refusal", §4.4).
    pub fn mismatch_against(&self, served: &IdentityKey) -> Option<Reply> {
        (self.identity != *served).then_some(Reply::Mismatch { expected: *served })
    }
}

/// The admitting reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ready {
    /// The identity the daemon serves, in full.
    pub identity: IdentityKey,
    pub pid: u32,
    /// When the daemon started, in seconds since the Unix epoch. With `pid` it names one
    /// daemon instance (spec §4.7).
    pub start_time: u64,
    /// The index generation at admission.
    pub generation: u64,
    pub build: BuildState,
    pub index_idle_timeout_secs: u64,
    pub index_idle_pressure_relief: bool,
}

/// What a daemon answers to a `HELLO`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// Admitted: MCP traffic may follow.
    Ready(Ready),
    /// The daemon is draining; start over (spec §3.1).
    Retiring,
    /// The daemon is at its connection bound (spec §4.2 `--max-clients`).
    Busy { max_clients: u32 },
    /// The daemon serves another identity; `expected` is the one it serves.
    Mismatch { expected: IdentityKey },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadyWire {
    #[allow(dead_code)] // read by `check_protocol`
    protocol: u64,
    identity: String,
    pid: u32,
    start_time: u64,
    generation: u64,
    build: String,
    index_idle_timeout_secs: u64,
    index_idle_pressure_relief: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetiringWire {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BusyWire {
    max_clients: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MismatchWire {
    expected: String,
}

impl Reply {
    /// The line to send, newline included.
    pub fn encode(&self) -> String {
        match self {
            Self::Ready(ready) => format!(
                "READY {{\"protocol\":{SHARED_PROTOCOL_VERSION},\"identity\":\"{}\",\"pid\":{},\"start_time\":{},\"generation\":{},\"build\":\"{}\",\"index_idle_timeout_secs\":{},\"index_idle_pressure_relief\":{}}}\n",
                ready.identity,
                ready.pid,
                ready.start_time,
                ready.generation,
                ready.build.word(),
                ready.index_idle_timeout_secs,
                ready.index_idle_pressure_relief,
            ),
            Self::Retiring => "RETIRING {}\n".to_string(),
            Self::Busy { max_clients } => format!("BUSY {{\"max_clients\":{max_clients}}}\n"),
            Self::Mismatch { expected } => format!("MISMATCH {{\"expected\":\"{expected}\"}}\n"),
        }
    }

    /// Decode one received line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Self, PreambleError> {
        let (verb, payload) = verb_and_payload(line, &["READY", "RETIRING", "BUSY", "MISMATCH"])?;
        let reply = match verb {
            "READY" => {
                check_protocol(payload)?;
                let wire: ReadyWire = parse_object(payload)?;
                Self::Ready(Ready {
                    identity: key_field("identity", &wire.identity)?,
                    pid: wire.pid,
                    start_time: wire.start_time,
                    generation: wire.generation,
                    build: BuildState::from_word(&wire.build)?,
                    index_idle_timeout_secs: wire.index_idle_timeout_secs,
                    index_idle_pressure_relief: wire.index_idle_pressure_relief,
                })
            }
            "RETIRING" => {
                let RetiringWire {} = parse_object(payload)?;
                Self::Retiring
            }
            "BUSY" => {
                let wire: BusyWire = parse_object(payload)?;
                Self::Busy {
                    max_clients: wire.max_clients,
                }
            }
            "MISMATCH" => {
                let wire: MismatchWire = parse_object(payload)?;
                Self::Mismatch {
                    expected: key_field("expected", &wire.expected)?,
                }
            }
            // `verb_and_payload` admits only the four verbs above; a verb added to its list
            // and not given an arm here must not be read as one of the others.
            other => {
                return Err(PreambleError::UnknownVerb {
                    verb: clip(other, MAX_VERB_BYTES),
                })
            }
        };
        require_canonical(line, &reply.encode())?;
        Ok(reply)
    }
}

#[cfg(test)]
#[path = "preamble_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "preamble_reply_tests.rs"]
mod reply_tests;
