//! The daemon's log: its standard error, which the shim that starts it points at `<key16>.log`
//! (spec §4.1 step 6, §4.3). The log is truncated at daemon start when it is over 1 MiB, and the
//! daemon stops adding its own lines past 8 MiB and says so once (§4.3). A panic message, which
//! the panic hook writes, is not counted: it is followed by the end of the process.
//! Lifecycle writes hold no control/slot guard; the affected provider is dropped or handed off
//! before its line. Stderr is one lock shared by every thread, including shutdown's `stopped`
//! line. A blocked disposal writer delays clearing `disposing`, so reload answers Warming.
//! Refresh scalars are captured under access and emitted after the caller's activity is released.
//! Production daemons write to the shim's `O_APPEND` regular file.
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
use crate::mcp::serving_events::{relief_pair, sequenced, Context, Details, Stamp};
use crate::mcp::transport::SessionRuntime;
use crate::mcp::{AutoRefreshSummary, RefreshPolicy, RefreshSummary, SessionProvider, StartupMode};
use crate::navigation::NavigationSession;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) struct LifecycleLog {
    pub(super) context: Context,
    origin: std::sync::OnceLock<Instant>,
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
            context: Context::new(identity, policy),
            origin: std::sync::OnceLock::new(),
            timeout: policy.timeout.unwrap_or_default(),
            // Reuse I0a applicability, with the ordinary lifecycle enabled for this daemon.
            relief: policy.pressure_relief && policy.pressure_relief_note(true, false).is_none(),
        }
    }
    pub(super) fn startup(&self) {
        let text = crate::mcp::serving_events::bounded(&format!("{} event=startup reason=startup attempt=0 generation=0 configured_idle_secs={:.3} observed_idle_secs=not_applicable duration_ns=not_applicable {} released_bytes=0 mono_ms=0", self.context.fields(Stamp::now()), self.timeout.as_secs_f64(), Details::default().fields("startup")), "complete");
        let mut stderr = std::io::stderr().lock();
        write_to(&sequenced(&text), &WRITTEN, &mut stderr);
    }
    /// Use the first build's existing guarded clock read; construction must not read the clock.
    pub(super) fn start_clock(&self, at: Instant) {
        let _ = self.origin.set(at);
    }
    pub(super) fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout.unwrap_or_default();
        self.relief &= !self.timeout.is_zero();
        self
    }
}

#[derive(Clone)]
pub(super) struct Event {
    pub(super) details: Details,
    pub(super) event: &'static str,
    reason: &'static str,
    pub(super) at: Instant,
    pub(super) wall: SystemTime,
    pub(super) duration: Duration,
    attempt: u64,
    generation: u64,
    observed_idle: Duration,
    outcome: String,
    pub(super) released: usize,
}

impl DaemonState {
    /// Sample wall time at production, through the same seam for all owned events.
    pub(super) fn wall_now(&self) -> SystemTime {
        #[cfg(test)]
        if let Some(now) = &self.hooks.wall_time {
            return now();
        }
        SystemTime::now()
    }
    pub(super) fn lifecycle_snapshot(
        &self,
        control: &ControlState,
        event: &'static str,
        reason: &'static str,
        duration: Duration,
        outcome: String,
    ) -> Event {
        let at = self.now();
        let wall = self.wall_now();
        let idle = &control.idle;
        let origin = idle
            .published_at
            .map(|p| p.max(idle.last_release.unwrap_or(p)));
        Event {
            details: Details::default(),
            event,
            reason,
            at,
            wall,
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
        let timestamp = e
            .wall
            .duration_since(UNIX_EPOCH)
            .ok()
            .unwrap_or_default()
            .as_millis();
        let mono_ms =
            e.at.saturating_duration_since(*log.origin.get_or_init(|| e.at))
                .as_millis();
        // Reserve the numeric trailer and outcome marker before capping free-text error data.
        let suffix = format!(" mono_ms={mono_ms} outcome=");
        let mut text = printable_prefix(&format!("{} event={} reason={} observed_idle_secs={:.3} configured_idle_secs={:.3} timestamp_ms={} duration_ns={} attempt={} generation={} {} released_bytes={}",
            log.context.fields(Stamp { at: e.at, wall: e.wall }), e.event, e.reason, e.observed_idle.as_secs_f64(), log.timeout.as_secs_f64(), timestamp,
            e.duration.as_nanos(), e.attempt, e.generation, e.details.fields(e.event), e.released), MAX_LINE_BYTES - suffix.len() - 30);
        text.push_str(&suffix);
        text.push_str(&printable_prefix(
            &e.outcome,
            MAX_LINE_BYTES - text.len() - 30,
        ));
        let _text = {
            let mut stderr = std::io::stderr().lock();
            let text = sequenced(&text);
            write_to(&text, &WRITTEN, &mut stderr);
            text
        };
        #[cfg(test)]
        {
            // Keep I0b's terminal-event seam; O-lite boundaries have their own observer.
            let observer = if matches!(e.event, "build_start" | "build_finish") {
                &log.context.observe
            } else {
                &self.hooks.lifecycle
            };
            if let Some(observe) = observer {
                observe(&_text);
            }
        }
        if matches!(e.event, "initial_build" | "reload") && e.outcome == "complete" {
            let mut published = e;
            published.event = "publication";
            self.lifecycle_event(published);
        }
    }
    pub(super) fn build_boundary(&self, attempt: u64, time: Stamp, event: &'static str) -> Event {
        Event {
            event,
            reason: "demand_or_startup",
            at: time.at,
            wall: time.wall,
            duration: Duration::ZERO,
            attempt,
            generation: 0,
            observed_idle: Duration::ZERO,
            outcome: "complete".into(),
            released: 0,
            details: Details::default(),
        }
    }
    pub(super) fn finished_build(
        &self,
        attempt: u64,
        built: &anyhow::Result<SessionProvider>,
    ) -> Event {
        let mut e = self.build_boundary(attempt, self.build_stamp(self.now()), "build_finish");
        e.details.build = built.as_ref().ok().map(|p| p.build_facts);
        if let Err(error) = built {
            e.outcome = format!("failed error={error}");
        }
        e
    }
    pub(super) fn build_stamp(&self, at: Instant) -> Stamp {
        Stamp {
            at,
            wall: self.wall_now(),
        }
    }
    pub(super) fn enrich_build(
        &self,
        event: &mut Option<Event>,
        provider: Option<&SessionProvider>,
        control: &ControlState,
    ) {
        if let Some(e) = event
            .as_mut()
            .filter(|_| matches!(control.build, BuildPhase::Ready))
        {
            e.details.build = provider.map(|p| p.build_facts);
            e.at = control.idle.published_at.unwrap();
            e.wall = self.wall_now();
        }
    }
    /// Capture only; callers emit after settlement and guard/provider release.
    pub(super) fn log_build(&self, control: &ControlState, result: &Result<(), String>) -> Event {
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
                if p.diverged || p.generation != control.generation {
                    "changed_snapshot"
                } else {
                    "same_snapshot"
                }
            })
        };
        let outcome = result
            .as_ref()
            .map_or_else(|e| format!("failed error={e}"), |_| "complete".into());
        self.lifecycle_snapshot(control, event, reason, duration, outcome)
    }
    /// Consume only for an accepted unload, retaining the release's producer identity/time.
    pub(super) fn take_last_release(&self, control: &mut ControlState) -> Option<Event> {
        control.idle.last_release_event.take().map(|release| {
            let mut event = self.lifecycle_snapshot(
                control,
                "last_release",
                "tool_complete",
                Duration::ZERO,
                "complete".into(),
            );
            event.at = release.at;
            event.wall = release.wall;
            event.attempt = release.attempt;
            event.generation = release.generation;
            event.observed_idle = Duration::ZERO;
            event
        })
    }
    pub(super) fn relieve(&self, mut event: Event) {
        event.event = "relief";
        event.at = self.now();
        event.wall = self.wall_now();
        let (pair, released, duration) =
            relief_pair(self.lifecycle.relief, &self.lifecycle.context);
        event.released = released;
        event.duration = duration;
        event.details.pair = Some(pair);
        self.lifecycle_event(event);
    }
    fn log_refresh<T>(
        &self,
        event: &'static str,
        started: Instant,
        result: &anyhow::Result<T>,
        generation: u64,
    ) -> Option<Event> {
        if let Ok(control) = self.control_plane() {
            let outcome = result
                .as_ref()
                .map_or_else(|e| format!("failed error={e}"), |_| "complete".into());
            let mut event =
                self.lifecycle_snapshot(&control, event, "requested", started.elapsed(), outcome);
            event.generation = generation;
            Some(event)
        } else {
            None
        }
    }
}

/// Delegates the actual refresh and observes its Result before response shaping or caps.
/// Used only inside the existing exclusive dispatch; no lease/admission/lock changes.
pub(super) struct LoggedRefresh<'a>(
    pub(super) &'a DaemonState,
    pub(super) &'a mut SessionProvider,
    pub(super) &'a mut Option<Event>,
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
        *self.2 = self
            .0
            .log_refresh("refresh", started, &result, self.1.generation());
        result
    }
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        let started = Instant::now();
        let result = self.1.auto_refresh_index();
        *self.2 = self
            .0
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
