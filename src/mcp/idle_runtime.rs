//! Single-owner stdio lifecycle. All publication/disposal happens on its event channel.
use super::framing::ReadOutcome;
use super::idle::{IdlePolicy, Phase, Published};
use super::lazy::{Readiness, SessionBuilder};
use super::serving_events::{relief_pair, Context, Details, Stamp};
use super::session::{canonical_config, AutoRefreshSummary, RefreshSummary};
use super::transport::SessionRuntime;
use super::{ServerConfig, SessionProvider, StartupMode};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub(super) struct Input {
    pub outcome: anyhow::Result<Option<ReadOutcome>>,
    // Acknowledge only when this frame becomes the active request. This bounds reader
    // read-ahead, including while readiness is consuming the common channel.
    pub next: Option<mpsc::Sender<()>>,
}

pub(super) enum Event {
    Input(Input),
    Built {
        result: anyhow::Result<Box<SessionProvider>>,
        finished_at: Instant,
    },
    Tick(Instant),
    Disposed,
}

pub(super) struct IdleRuntime {
    pub tx: mpsc::Sender<Event>,
    pub rx: mpsc::Receiver<Event>,
    pub pending: VecDeque<Input>,
    phase: Phase,
    provider: Option<Box<SessionProvider>>,
    builder: SessionBuilder,
    policy: IdlePolicy,
    startup: StartupMode,
    wait: Duration,
    attempt: usize,
    previous: Option<Published>,
    published_at: Option<Instant>,
    active: Rc<Cell<usize>>,
    last_release: Rc<Cell<Instant>>,
    clock: Rc<dyn Fn() -> Instant>,
    log: LifecycleLog,
    events: Vec<Observation>,
    release: Rc<Cell<Option<(Stamp, usize, u64)>>>,
    ids: Rc<Cell<(usize, u64)>>,
    #[cfg(test)]
    build_workers: Vec<(usize, std::thread::JoinHandle<()>)>,
    disposing: bool,
    disposal_deadline: Option<Instant>,
    #[cfg(test)]
    before_dispose: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[derive(Clone)]
struct LifecycleLog {
    context: Context,
    attempt: usize,
}

pub(super) struct Activity {
    release: Rc<Cell<Option<(Stamp, usize, u64)>>>,
    ids: Rc<Cell<(usize, u64)>>,
    active: Rc<Cell<usize>>,
    last_release: Rc<Cell<Instant>>,
    clock: Rc<dyn Fn() -> Instant>,
}

impl Drop for Activity {
    fn drop(&mut self) {
        self.active.set(self.active.get() - 1);
        let at = (self.clock)();
        self.last_release.set(at);
        if self.active.get() == 0 {
            let (attempt, generation) = self.ids.get();
            self.release.set(Some((
                Stamp {
                    at,
                    wall: std::time::SystemTime::now(),
                },
                attempt,
                generation,
            )));
        }
    }
}

impl IdleRuntime {
    pub fn new(
        cfg: &ServerConfig,
        policy: IdlePolicy,
        builder: Option<SessionBuilder>,
    ) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        anyhow::ensure!(
            cfg.startup != StartupMode::Eager,
            "idle runtime requires Lazy/Background"
        );
        anyhow::ensure!(
            policy.timeout.is_some(),
            "idle runtime requires a nonzero timeout"
        );
        anyhow::ensure!(
            Instant::now()
                .checked_add(policy.timeout.unwrap())
                .is_some(),
            "idle timeout overflows the monotonic clock"
        );
        let builder_cfg = cfg.clone();
        let builder =
            builder.unwrap_or_else(|| Arc::new(move || SessionProvider::bootstrap(&builder_cfg)));
        let (tx, rx) = mpsc::channel();
        let context = Context::stdio(&cfg, policy);
        context.startup();
        let mut rt = Self {
            tx,
            rx,
            pending: VecDeque::new(),
            phase: Phase::Idle,
            provider: None,
            builder,
            policy,
            startup: cfg.startup,
            wait: cfg.first_call_wait,
            attempt: 0,
            previous: None,
            published_at: None,
            active: Rc::new(Cell::new(0)),
            last_release: Rc::new(Cell::new(Instant::now())),
            clock: Rc::new(Instant::now),
            log: LifecycleLog {
                context,
                attempt: 0,
            },
            events: Vec::new(),
            release: Rc::new(Cell::new(None)),
            ids: Rc::new(Cell::new((0, 0))),
            #[cfg(test)]
            build_workers: Vec::new(),
            disposing: false,
            disposal_deadline: None,
            #[cfg(test)]
            before_dispose: None,
        };
        if cfg.startup == StartupMode::Background {
            rt.spawn();
        }
        Ok(rt)
    }

    pub fn activity(&self) -> Activity {
        self.active.set(self.active.get() + 1);
        Activity {
            release: self.release.clone(),
            ids: self.ids.clone(),
            active: self.active.clone(),
            last_release: self.last_release.clone(),
            clock: self.clock.clone(),
        }
    }

    fn spawn(&mut self) {
        if let Phase::Unloaded { last_published } = self.phase {
            self.previous = Some(last_published);
        }
        self.attempt += 1;
        self.ids.set((self.attempt, 0));
        self.log.attempt = self.attempt;
        let builder = self.builder.clone();
        let tx = self.tx.clone();
        self.phase = Phase::Building {
            started: (self.clock)(),
            deadline: self.disposal_deadline.take(),
        };
        let log = self.log.clone();
        let start = log.record("build_start", "complete", Duration::ZERO, None, 0);
        let _worker = std::thread::spawn(move || {
            log.write(start);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder()))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("index build panicked")))
                .map(Box::new);
            let finished_at = Instant::now();
            let mut finish = log.record(
                "build_finish",
                if result.is_ok() { "complete" } else { "failed" },
                Duration::ZERO,
                None,
                0,
            );
            finish.time.at = finished_at;
            finish.details.build = result.as_ref().ok().map(|p| p.build_facts);
            // A retired owner has dropped its receiver. Failed send disposes even late results.
            let _ = tx.send(Event::Built {
                result,
                finished_at,
            });
            log.write(finish);
        });
        #[cfg(test)]
        self.build_workers.push((self.attempt, _worker));
    }

    #[cfg(test)]
    pub(super) fn join_build_for_tests(&mut self) -> usize {
        for (attempt, worker) in self.build_workers.drain(..) {
            assert!(attempt <= self.attempt);
            worker.join().unwrap();
        }
        self.attempt
    }

    pub fn accept(&mut self, event: Event) {
        match event {
            Event::Input(input) => self.pending.push_back(input),
            Event::Tick(now) => self.tick(now),
            Event::Disposed => self.disposing = false,
            Event::Built {
                result,
                finished_at,
            } => {
                let Phase::Building { started, .. } = self.phase else {
                    return;
                };
                let duration = finished_at.saturating_duration_since(started);
                let event = self.completion_event();
                match result {
                    Ok(mut provider) => {
                        if let Some(previous) = self.previous {
                            provider.resume_idle_generation(previous);
                        }
                        let publication = provider.idle_publication();
                        let changed = self.previous.is_some_and(|p| {
                            p.diverged || p.fingerprint != publication.fingerprint
                        });
                        let facts = provider.build_facts;
                        self.provider = Some(provider);
                        self.phase = Phase::Ready;
                        self.published_at = Some(finished_at);
                        self.ids.set((self.attempt, publication.generation));
                        let mut record = self.log.record(
                            event,
                            if changed { "changed_snapshot" } else { "ready" },
                            duration,
                            Some(publication),
                            0,
                        );
                        record.snapshot_reason = self.previous.map_or("demand_or_startup", |_| {
                            if changed {
                                "changed_snapshot"
                            } else {
                                "same_snapshot"
                            }
                        });
                        record.details.build = Some(facts);
                        let mut published = record.clone();
                        published.event = "publication";
                        self.events.extend([record, published]);
                    }
                    Err(error) => {
                        self.phase = Phase::Failed {
                            error: error.to_string(),
                            heard: false,
                        };
                        self.events.push(self.log.record(
                            event,
                            &format!("failed error={error}"),
                            duration,
                            None,
                            0,
                        ));
                    }
                }
            }
        }
        self.flush_events();
    }

    #[cfg(test)]
    pub(super) fn deferred_for_tests(cfg: &ServerConfig) -> (Self, mpsc::Sender<bool>) {
        let (send, receive) = mpsc::channel();
        let receive = std::sync::Mutex::new(receive);
        let build_cfg = cfg.clone();
        let builder = Arc::new(move || {
            anyhow::ensure!(
                receive.lock().unwrap().recv().unwrap(),
                "injected failed reload"
            );
            SessionProvider::bootstrap(&build_cfg)
        });
        let mut cfg = cfg.clone();
        cfg.first_call_wait = Duration::ZERO;
        let mut rt = Self::new(
            &cfg,
            IdlePolicy::enabled(Duration::from_secs(10)),
            Some(builder),
        )
        .unwrap();
        let activity = rt.activity();
        assert!(matches!(rt.ensure_ready(), Readiness::Warming { .. }));
        drop(activity);
        (rt, send)
    }
    #[cfg(test)]
    pub(super) fn fail_refresh_for_tests(&mut self) {
        self.provider
            .as_mut()
            .unwrap()
            .force_next_refresh_error_for_tests("injected refresh failure");
    }
    #[cfg(test)]
    pub(super) fn log_context(&mut self) -> &mut Context {
        &mut self.log.context
    }
    pub(super) fn flush_events(&mut self) {
        if self.active.get() == 0 {
            for event in self.events.drain(..) {
                self.log.write(event);
            }
        }
    }

    fn completion_event(&self) -> &'static str {
        if self.previous.is_some() {
            "reload"
        } else {
            "build"
        }
    }

    pub fn idle_deadline(&self) -> Option<Instant> {
        if !matches!(self.phase, Phase::Ready) || self.active.get() != 0 || !self.pending.is_empty()
        {
            return None;
        }
        self.published_at?
            .max(self.last_release.get())
            .checked_add(self.policy.timeout?)
    }

    pub fn tick(&mut self, now: Instant) {
        // Already queued input is pending work even if the timer won the channel race.
        while let Ok(event) = self.rx.try_recv() {
            if !matches!(event, Event::Tick(_)) {
                self.accept(event);
            }
        }
        if self.active.get() != 0 {
            return;
        }
        let Some(deadline) = self.idle_deadline() else {
            return;
        };
        if now < deadline {
            return;
        }
        let detached = Instant::now();
        let sidecar = self
            .provider
            .as_ref()
            .map_or("not_attempted", |p| p.session().index.sidecar_outcome());
        let Some(provider) = self.provider.take() else {
            return;
        };
        self.ids.set((self.attempt, 0));
        let last_published = provider.idle_publication();
        self.phase = Phase::Unloaded { last_published };
        self.disposing = true;
        self.disposal_deadline = None;
        let tx = self.tx.clone();
        let log = self.log.clone();
        let relief = self.policy.pressure_relief;
        let mut unload = log.record(
            "unload",
            "complete",
            detached.elapsed(),
            Some(last_published),
            0,
        );
        unload.details.sidecar = sidecar;
        let release = self.release.take().map(|(time, attempt, generation)| {
            let mut e = log.record(
                "last_release",
                "complete",
                Duration::ZERO,
                Some(last_published),
                0,
            );
            e.time = time;
            e.attempt = attempt;
            e.generation = generation;
            e
        });
        #[cfg(test)]
        let before_dispose = self.before_dispose.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let start = log.record(
                "disposal_start",
                "complete",
                Duration::ZERO,
                Some(last_published),
                0,
            );
            #[cfg(test)]
            if let Some(hook) = before_dispose {
                hook();
            }
            drop(provider); // whole session, including sticky diagnostics and lazy edge storage
            let end = log.record(
                "disposal_end",
                "complete",
                started.elapsed(),
                Some(last_published),
                0,
            );
            let (pair, released, duration) = relief_pair(relief, &log.context);
            let mut relief = log.record(
                "relief",
                "complete",
                duration,
                Some(last_published),
                released,
            );
            relief.details.pair = Some(pair);
            log.write(start);
            log.write(end);
            log.write(relief);
            let _ = tx.send(Event::Disposed);
        });
        // Ownership has been transferred; emit both records under the same stderr lock.
        let mut lines = Vec::new();
        if let Some(e) = release {
            lines.push(self.log.format(&e));
        }
        lines.push(self.log.format(&unload));
        self.log.context.emit_lines(&lines);
    }
}

#[derive(Clone)]
struct Observation {
    event: &'static str,
    outcome: String,
    time: Stamp,
    duration: Duration,
    attempt: usize,
    generation: u64,
    released: usize,
    details: Details,
    snapshot_reason: &'static str,
}
impl LifecycleLog {
    fn record(
        &self,
        event: &'static str,
        outcome: &str,
        duration: Duration,
        published: Option<Published>,
        released: usize,
    ) -> Observation {
        Observation {
            event,
            outcome: outcome.into(),
            time: Stamp::now(),
            duration,
            attempt: self.attempt,
            generation: published.map_or(0, |p| p.generation),
            released,
            details: Details::default(),
            snapshot_reason: "demand_or_startup",
        }
    }
    fn write(&self, event: Observation) {
        self.context.emit(&self.format(&event));
    }
    fn format(&self, e: &Observation) -> String {
        let reason = match e.event {
            "unload" | "disposal_start" | "disposal_end" | "relief" => "idle",
            "last_release" => "tool_complete",
            "refresh" | "auto_refresh" => "requested",
            "build" | "reload" | "publication" if e.outcome.starts_with("failed") => "build_failed",
            "reload" | "publication" => e.snapshot_reason,
            _ => "demand_or_startup",
        };
        let outcome = if matches!(e.outcome.as_str(), "ready" | "changed_snapshot") {
            "complete"
        } else {
            &e.outcome
        };
        let fields = self.context.fields(e.time);
        let details = e.details.fields(e.event);
        let timestamp_ms = e
            .time
            .wall
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mono_ms = e.time.mono() / 1_000_000;
        super::serving_events::bounded(
            &format!(
                concat!(
                    "{} event={} reason={} configured_idle_secs={:.3} observed_idle_secs=unknown ",
                    "timestamp_ms={} duration_ns={} duration_ms={} attempt={} generation={} ",
                    "{} released_bytes={} mono_ms={}",
                ),
                fields,
                e.event,
                reason,
                self.context.timeout.as_secs_f64(),
                timestamp_ms,
                e.duration.as_nanos(),
                e.duration.as_millis(),
                e.attempt,
                e.generation,
                details,
                e.released,
                mono_ms,
            ),
            outcome,
        )
    }
    #[cfg(test)]
    fn line(
        &self,
        event: &'static str,
        outcome: &str,
        duration: Duration,
        published: Option<Published>,
        released: usize,
    ) -> String {
        self.format(&self.record(event, outcome, duration, published, released))
    }
}

impl SessionRuntime for IdleRuntime {
    fn ensure_ready(&mut self) -> Readiness {
        loop {
            // Disposal owns the old session until its completion. Do not overlap a reload.
            if self.disposing {
                let now = (self.clock)();
                let deadline = *self.disposal_deadline.get_or_insert(now + self.wait);
                match self
                    .rx
                    .recv_timeout(deadline.saturating_duration_since(now))
                {
                    Ok(event) => self.accept(event),
                    // No build has started yet, so its elapsed time is still zero.
                    Err(_) => {
                        // A timed-out call cannot spend a later call's reload budget.
                        self.disposal_deadline.take();
                        return Readiness::Warming {
                            elapsed: Duration::ZERO,
                        };
                    }
                }
                continue;
            }
            if matches!(
                self.phase,
                Phase::Idle | Phase::Unloaded { .. } | Phase::Failed { heard: true, .. }
            ) {
                self.spawn();
            }
            match &mut self.phase {
                Phase::Ready => return Readiness::Ready,
                Phase::Failed { error, heard } => {
                    *heard = true;
                    return Readiness::Failed {
                        error: error.clone(),
                    };
                }
                Phase::Building { started, deadline } => {
                    let now = (self.clock)();
                    let deadline = *deadline.get_or_insert_with(|| now + self.wait);
                    let started = *started;
                    match self
                        .rx
                        .recv_timeout(deadline.saturating_duration_since(now))
                    {
                        Ok(event) => self.accept(event), // input/EOF stay queued in their reader order
                        Err(_) => {
                            return Readiness::Warming {
                                elapsed: (self.clock)().saturating_duration_since(started),
                            }
                        }
                    }
                }
                _ => unreachable!("idle and heard failures spawn before waiting"),
            }
        }
    }
    fn startup_mode(&self) -> StartupMode {
        self.startup
    }
    fn session(&self) -> &crate::navigation::NavigationSession {
        self.provider.as_ref().expect("ready").session()
    }
    fn freshness(&self) -> Option<&super::freshness::FreshnessProbe> {
        Some(self.provider.as_ref().expect("ready").freshness())
    }
    fn known_stale_after_refresh(&self) -> Option<&super::freshness::FreshnessReport> {
        self.provider
            .as_ref()
            .expect("ready")
            .known_stale_after_refresh()
    }
    fn refresh_policy(&self) -> super::RefreshPolicy {
        self.provider.as_ref().expect("ready").refresh_policy()
    }
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        let started = (self.clock)();
        let result = self.provider.as_mut().expect("ready").refresh();
        let event = self.log.record(
            "refresh",
            if result.is_ok() { "complete" } else { "failed" },
            (self.clock)().saturating_duration_since(started),
            self.provider.as_ref().map(|p| p.idle_publication()),
            0,
        );
        self.ids.set((
            self.attempt,
            self.provider.as_ref().expect("ready").generation(),
        ));
        self.events.push(event);
        self.flush_events();
        result
    }
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        let started = (self.clock)();
        let result = self.provider.as_mut().expect("ready").auto_refresh();
        let event = self.log.record(
            "auto_refresh",
            if result.is_ok() { "complete" } else { "failed" },
            (self.clock)().saturating_duration_since(started),
            self.provider.as_ref().map(|p| p.idle_publication()),
            0,
        );
        self.ids.set((
            self.attempt,
            self.provider.as_ref().expect("ready").generation(),
        ));
        self.events.push(event);
        self.flush_events();
        result
    }
}

#[cfg(test)]
#[path = "idle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "idle_fix_tests.rs"]
mod fix_tests;
