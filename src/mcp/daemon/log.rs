//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The log is truncated at daemon start when it is over 1 MiB, and the
//! daemon stops adding its own lines past 8 MiB and says so once (§4.3). A panic message, which
//! the panic hook writes, is not counted: it is followed by the end of the process.
use std::io::Write;
use std::os::fd::AsFd;
use std::sync::atomic::{AtomicU64, Ordering};

/// A log over this size at daemon start is emptied first.
const TRUNCATE_AT_START_BYTES: u64 = 1 << 20;
/// The daemon writes no line of its own past this size.
const CAP_BYTES: u64 = 8 << 20;
/// Longest line the daemon writes; text that comes from elsewhere (an error's message) is cut.
const MAX_LINE_BYTES: usize = 1000;

/// Bytes in the log as far as this process knows: what was there at start, plus its own lines.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// At daemon start: if standard error is a regular file over 1 MiB, empty it; count what is left.
/// Standard error that is not a regular file (a terminal, a pipe in a test) is left alone.
pub(super) fn start() {
    let Ok(stderr) = std::io::stderr().as_fd().try_clone_to_owned() else {
        return;
    };
    let file = std::fs::File::from(stderr);
    let Ok(metadata) = file.metadata() else {
        return;
    };
    if !metadata.file_type().is_file() {
        return;
    }
    let size = if metadata.len() > TRUNCATE_AT_START_BYTES && file.set_len(0).is_ok() {
        0
    } else {
        metadata.len()
    };
    WRITTEN.store(size, Ordering::SeqCst);
}

/// Write one line, `prism-mcp daemon: <text>`, with control characters replaced and at most
/// 1000 bytes of `text`, unless the log has reached its cap. The line that would cross the cap
/// is replaced by one note that the cap was reached. A failed write is ignored: there is nowhere
/// else to report it.
pub(super) fn line(text: &str) {
    write_to(text, &WRITTEN, &mut std::io::stderr().lock());
}

#[cfg(test)]
pub(super) type EventObserver = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

pub(super) fn write_to(text: &str, written: &AtomicU64, stderr: &mut impl Write) {
    let text = printable_prefix(text, MAX_LINE_BYTES);
    let line = format!("prism-mcp daemon: {text}\n");
    let len = u64::try_from(line.len()).unwrap_or(u64::MAX);
    let before = written.fetch_add(len, Ordering::SeqCst);
    if before.saturating_add(len) <= CAP_BYTES {
        let _ = stderr.write_all(line.as_bytes());
    } else if before <= CAP_BYTES {
        let _ = writeln!(
            stderr,
            "prism-mcp daemon: the log reached its cap of {} MiB; later lines are dropped",
            CAP_BYTES >> 20
        );
    }
}

/// At most `max` bytes of `text`, cut on a character boundary, with every control character
/// replaced: a line must not be able to start another or drive a terminal.
pub(super) fn printable_prefix(text: &str, max: usize) -> String {
    let mut out = String::with_capacity(text.len().min(max));
    for c in text.chars() {
        let c = if c.is_control() { '\u{fffd}' } else { c };
        if out.len() + c.len_utf8() > max {
            break;
        }
        out.push(c);
    }
    out
}

use super::coordinator::{BuildPhase, ControlState, DaemonState};
use crate::mcp::freshness::{FreshnessProbe, FreshnessReport};
use crate::mcp::idle::IdlePolicy;
use crate::mcp::transport::SessionRuntime;
use crate::mcp::{AutoRefreshSummary, RefreshPolicy, RefreshSummary, SessionProvider, StartupMode};
use crate::navigation::NavigationSession;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) struct LifecycleLog {
    identity: String,
    origin: Instant,
    epoch_ms: u128,
    timeout: Duration,
    pub(super) relief: bool,
}
impl Default for LifecycleLog {
    fn default() -> Self {
        Self::new("unbound".into(), IdlePolicy::default())
    }
}
impl LifecycleLog {
    pub(super) fn new(identity: String, policy: IdlePolicy) -> Self {
        Self {
            identity,
            origin: Instant::now(),
            epoch_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            timeout: policy.timeout.unwrap_or_default(),
            // Reuse I0a applicability, with the ordinary lifecycle enabled for this daemon.
            relief: policy.pressure_relief && policy.pressure_relief_note(true, false).is_none(),
        }
    }
    pub(super) fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout.unwrap_or_default();
        self.relief &= !self.timeout.is_zero();
        self
    }
}

#[derive(Clone)]
pub(super) struct Event {
    pub(super) event: &'static str,
    reason: &'static str,
    pub(super) at: Instant,
    pub(super) duration: Duration,
    attempt: u64,
    generation: u64,
    observed_idle: Duration,
    outcome: String,
    pub(super) released: usize,
}

impl DaemonState {
    pub(super) fn lifecycle_snapshot(
        &self,
        control: &ControlState,
        event: &'static str,
        reason: &'static str,
        duration: Duration,
        outcome: String,
    ) -> Event {
        let at = self.now();
        let idle = &control.idle;
        let origin = idle
            .published_at
            .map(|p| p.max(idle.last_release.unwrap_or(p)));
        Event {
            event,
            reason,
            at,
            duration,
            attempt: control.attempts,
            generation: control.generation,
            observed_idle: origin.map_or(Duration::ZERO, |p| at.saturating_duration_since(p)),
            outcome,
            released: 0,
        }
    }
    pub(super) fn lifecycle_event(&self, e: Event) {
        let log = &self.lifecycle;
        let timestamp = log.epoch_ms + e.at.saturating_duration_since(log.origin).as_millis();
        let text = printable_prefix(&format!("identity={} pid={} event={} reason={} observed_idle_secs={:.3} configured_idle_secs={} timestamp_ms={} duration_ns={} attempt={} generation={} released_bytes={} outcome={}",
            log.identity, std::process::id(), e.event, e.reason, e.observed_idle.as_secs_f64(), log.timeout.as_secs(), timestamp,
            e.duration.as_nanos(), e.attempt, e.generation, e.released, e.outcome), MAX_LINE_BYTES);
        line(&text);
        #[cfg(test)]
        if let Some(observe) = &self.hooks.lifecycle {
            observe(&text);
        }
    }
    pub(super) fn log_build(&self, control: &ControlState, result: &Result<(), String>) {
        let duration = match control.build {
            BuildPhase::Building { started, .. } => self.now().saturating_duration_since(started),
            _ => Duration::ZERO,
        };
        let event = if control.idle.previous.is_some() {
            "reload"
        } else {
            "initial_build"
        };
        let reason = if result.is_err() {
            "build_failed"
        } else {
            control.idle.previous.map_or("demand_or_startup", |p| {
                if p.generation == control.generation {
                    "same_snapshot"
                } else {
                    "changed_snapshot"
                }
            })
        };
        let outcome = result
            .as_ref()
            .map_or_else(|e| format!("failed error={e}"), |_| "complete".into());
        self.lifecycle_event(self.lifecycle_snapshot(control, event, reason, duration, outcome));
    }
    pub(super) fn flush_last_release(&self) -> Result<(), super::coordinator::Poisoned> {
        let mut control = self.control_plane()?;
        let event = control.idle.last_release_event.take().map(|release| {
            let mut event = self.lifecycle_snapshot(
                &control,
                "last_release",
                "tool_complete",
                Duration::ZERO,
                "complete".into(),
            );
            event.at = release.at;
            event.attempt = release.attempt;
            event.generation = release.generation;
            event.observed_idle = Duration::ZERO;
            event
        });
        drop(control);
        if let Some(event) = event {
            self.lifecycle_event(event);
        }
        Ok(())
    }
    pub(super) fn relieve(&self, mut event: Event) {
        event.event = "relief";
        event.at = self.now();
        let started = Instant::now();
        event.released = crate::mcp::idle::pressure_relief(self.lifecycle.relief);
        event.duration = started.elapsed();
        self.lifecycle_event(event);
    }
    fn log_refresh<T>(
        &self,
        event: &'static str,
        started: Instant,
        result: &anyhow::Result<T>,
        generation: u64,
    ) {
        if let Ok(control) = self.control_plane() {
            let outcome = result
                .as_ref()
                .map_or_else(|e| format!("failed error={e}"), |_| "complete".into());
            let mut event =
                self.lifecycle_snapshot(&control, event, "requested", started.elapsed(), outcome);
            event.generation = generation;
            drop(control);
            self.lifecycle_event(event);
        }
    }
}

/// Delegates the actual refresh and observes its Result before response shaping or caps.
/// Used only inside the existing exclusive dispatch; no lease/admission/lock changes.
pub(super) struct LoggedRefresh<'a>(
    pub(super) &'a DaemonState,
    pub(super) &'a mut SessionProvider,
);
impl SessionRuntime for LoggedRefresh<'_> {
    fn ensure_ready(&mut self) -> crate::mcp::lazy::Readiness {
        self.1.ensure_ready()
    }
    fn startup_mode(&self) -> StartupMode {
        self.1.startup_mode()
    }
    fn session(&self) -> &NavigationSession {
        self.1.session()
    }
    fn freshness(&self) -> Option<&FreshnessProbe> {
        Some(self.1.freshness())
    }
    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.1.known_stale_after_refresh()
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        self.1.refresh_policy()
    }
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        let started = Instant::now();
        let result = self.1.refresh_index();
        self.0
            .log_refresh("refresh", started, &result, self.1.generation());
        result
    }
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        let started = Instant::now();
        let result = self.1.auto_refresh_index();
        self.0
            .log_refresh("auto_refresh", started, &result, self.1.generation());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text from elsewhere cannot start a new line in the log or grow it past a line's bound.
    #[test]
    fn a_line_has_no_control_characters_and_a_bounded_length() {
        assert_eq!(
            printable_prefix("a\nb\x1b[2Jc", 100),
            "a\u{fffd}b\u{fffd}[2Jc"
        );
        assert_eq!(printable_prefix("ééé", 5), "éé");
        assert_eq!(printable_prefix(&"x".repeat(5000), 1000).len(), 1000);
    }
}
