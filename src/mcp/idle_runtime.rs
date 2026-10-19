//! Single-owner stdio lifecycle. All publication/disposal happens on its event channel.
use super::framing::ReadOutcome;
use super::idle::{pressure_relief, IdlePolicy, Phase, Published};
use super::lazy::{Readiness, SessionBuilder};
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
    identity: String,
}

pub(super) struct Activity {
    active: Rc<Cell<usize>>,
    last_release: Rc<Cell<Instant>>,
    clock: Rc<dyn Fn() -> Instant>,
}

impl Drop for Activity {
    fn drop(&mut self) {
        self.active.set(self.active.get() - 1);
        self.last_release.set((self.clock)());
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
        use sha2::{Digest, Sha256};
        let identity = format!(
            "{:x}",
            Sha256::digest(cfg.repo_root.to_string_lossy().as_bytes())
        )[..12]
            .to_owned();
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
            identity,
        };
        if cfg.startup == StartupMode::Background {
            rt.spawn();
        }
        Ok(rt)
    }

    pub fn activity(&self) -> Activity {
        self.active.set(self.active.get() + 1);
        Activity {
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
        let builder = self.builder.clone();
        let tx = self.tx.clone();
        self.phase = Phase::Building {
            started: (self.clock)(),
            deadline: None,
        };
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder()))
                .unwrap_or_else(|_| Err(anyhow::anyhow!("index build panicked")))
                .map(Box::new);
            let finished_at = Instant::now();
            // A retired owner has dropped its receiver. Failed send disposes even late results.
            let _ = tx.send(Event::Built {
                result,
                finished_at,
            });
        });
    }

    pub fn accept(&mut self, event: Event) {
        match event {
            Event::Input(input) => self.pending.push_back(input),
            Event::Tick(now) => self.tick(now),
            Event::Built {
                result,
                finished_at,
            } => {
                let Phase::Building { started, .. } = self.phase else {
                    return;
                };
                let duration = finished_at.saturating_duration_since(started);
                match result {
                    Ok(mut provider) => {
                        if let Some(previous) = self.previous {
                            provider.resume_idle_generation(previous);
                        }
                        let publication = provider.idle_publication();
                        let changed = self.previous.is_some_and(|p| {
                            p.diverged || p.fingerprint != publication.fingerprint
                        });
                        self.provider = Some(provider);
                        self.phase = Phase::Ready;
                        self.published_at = Some(finished_at);
                        self.log(
                            "reload",
                            if changed { "changed_snapshot" } else { "ready" },
                            duration,
                            Some(publication),
                            0,
                        );
                    }
                    Err(error) => {
                        self.phase = Phase::Failed {
                            error: error.to_string(),
                            heard: false,
                        };
                        self.log("reload", "failed", duration, self.previous, 0);
                    }
                }
            }
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
        let started = (self.clock)();
        let Some(provider) = self.provider.take() else {
            return;
        };
        let last_published = provider.idle_publication();
        self.phase = Phase::Unloaded { last_published };
        drop(provider); // whole session, including sticky diagnostics and lazy edge storage
        let released = pressure_relief(self.policy.pressure_relief);
        self.log(
            "unload",
            "idle",
            (self.clock)().saturating_duration_since(started),
            Some(last_published),
            released,
        );
    }

    fn log(
        &self,
        event: &str,
        outcome: &str,
        duration: Duration,
        published: Option<Published>,
        released: usize,
    ) {
        eprintln!("prism-mcp: identity={} event={} reason={} idle_minutes={:.3} duration_ms={} attempt={} generation={} outcome={} released_bytes={}",
            self.identity, event, if event == "unload" { "idle" } else { "demand_or_startup" },
            self.policy.timeout.unwrap_or_default().as_secs_f64() / 60.0, duration.as_millis(),
            self.attempt, published.map_or(0, |p| p.generation), outcome, released);
    }
}

impl SessionRuntime for IdleRuntime {
    fn ensure_ready(&mut self) -> Readiness {
        if matches!(
            self.phase,
            Phase::Idle | Phase::Unloaded { .. } | Phase::Failed { heard: true, .. }
        ) {
            self.spawn();
        }
        loop {
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
                    let elapsed = now.saturating_duration_since(*started);
                    match self
                        .rx
                        .recv_timeout(deadline.saturating_duration_since(now))
                    {
                        Ok(event) => self.accept(event), // input/EOF stay queued in their reader order
                        Err(_) => return Readiness::Warming { elapsed },
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
        self.log(
            "refresh",
            if result.is_ok() { "complete" } else { "failed" },
            (self.clock)().saturating_duration_since(started),
            self.provider.as_ref().map(|p| p.idle_publication()),
            0,
        );
        result
    }
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        let started = (self.clock)();
        let result = self.provider.as_mut().expect("ready").auto_refresh();
        self.log(
            "auto_refresh",
            if result.is_ok() { "complete" } else { "failed" },
            (self.clock)().saturating_duration_since(started),
            self.provider.as_ref().map(|p| p.idle_publication()),
            0,
        );
        result
    }
}

#[cfg(test)]
#[path = "idle_tests.rs"]
mod tests;
