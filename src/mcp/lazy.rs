use super::session::canonical_config;
use super::{ServerConfig, SessionProvider, FIRST_CALL_WAIT_MAX};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub(crate) type SessionBuilder = Arc<dyn Fn() -> anyhow::Result<SessionProvider> + Send + Sync>;

pub enum Readiness {
    Ready,
    Warming { elapsed: Duration },
    Failed { error: String },
}

pub struct LazySessionProvider {
    #[allow(dead_code)]
    cfg: ServerConfig,
    state: LazyState,
    wait: Duration,
    last_error: Option<String>,
    attempts: usize,
    builder: SessionBuilder,
}

enum LazyState {
    Building {
        rx: mpsc::Receiver<anyhow::Result<SessionProvider>>,
        started: Instant,
        deadline: Option<Instant>,
    },
    Ready(SessionProvider),
    Failed {
        error: String,
        at: Instant,
    },
}

impl LazySessionProvider {
    pub fn new(cfg: &ServerConfig) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        let builder_cfg = cfg.clone();
        let builder: SessionBuilder = Arc::new(move || SessionProvider::bootstrap(&builder_cfg));
        let wait = cfg.first_call_wait;
        Self::from_canonical_config(cfg, wait, builder)
    }

    #[cfg(test)]
    pub(crate) fn with_builder(
        cfg: &ServerConfig,
        wait: Duration,
        builder: SessionBuilder,
    ) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        Self::from_canonical_config(cfg, wait, builder)
    }

    fn from_canonical_config(
        mut cfg: ServerConfig,
        wait: Duration,
        builder: SessionBuilder,
    ) -> anyhow::Result<Self> {
        validate_wait(wait)?;
        cfg.first_call_wait = wait;
        let mut provider = Self {
            cfg,
            state: LazyState::Failed {
                error: String::new(),
                at: Instant::now(),
            },
            wait,
            last_error: None,
            attempts: 0,
            builder,
        };
        provider.spawn_build();
        Ok(provider)
    }

    pub fn ensure_ready(&mut self) -> Readiness {
        if matches!(&self.state, LazyState::Failed { .. }) {
            self.spawn_build();
        }

        if matches!(&self.state, LazyState::Ready(_)) {
            return Readiness::Ready;
        }

        let now = Instant::now();
        let (result, elapsed) = match &mut self.state {
            LazyState::Building {
                rx,
                started,
                deadline,
            } => {
                let deadline = deadline.get_or_insert_with(|| {
                    now.checked_add(self.wait)
                        .unwrap_or_else(|| now + FIRST_CALL_WAIT_MAX)
                });
                (
                    rx.recv_timeout(deadline.saturating_duration_since(now)),
                    started.elapsed(),
                )
            }
            LazyState::Ready(_) => return Readiness::Ready,
            LazyState::Failed { .. } => unreachable!("failed state always restarts before waiting"),
        };

        match result {
            Ok(Ok(provider)) => {
                self.state = LazyState::Ready(provider);
                self.last_error = None;
                Readiness::Ready
            }
            Ok(Err(error)) => self.failed(error.to_string()),
            Err(mpsc::RecvTimeoutError::Timeout) => Readiness::Warming { elapsed },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.failed("index build panicked".to_string())
            }
        }
    }

    fn spawn_build(&mut self) {
        self.attempts += 1;
        let (tx, rx) = mpsc::channel();
        let builder = Arc::clone(&self.builder);
        let started = Instant::now();
        std::thread::spawn(move || {
            let _ = tx.send(builder());
        });
        self.state = LazyState::Building {
            rx,
            started,
            deadline: None,
        };
    }

    fn failed(&mut self, error: String) -> Readiness {
        self.last_error = Some(error.clone());
        self.state = LazyState::Failed {
            error: error.clone(),
            at: Instant::now(),
        };
        Readiness::Failed { error }
    }

    pub(crate) fn ready(&self) -> Option<&SessionProvider> {
        match &self.state {
            LazyState::Ready(provider) => Some(provider),
            LazyState::Building { .. } | LazyState::Failed { .. } => None,
        }
    }

    pub(crate) fn ready_mut(&mut self) -> Option<&mut SessionProvider> {
        match &mut self.state {
            LazyState::Ready(provider) => Some(provider),
            LazyState::Building { .. } | LazyState::Failed { .. } => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn attempts(&self) -> usize {
        self.attempts
    }

    #[cfg(test)]
    pub(crate) fn builds(&self) -> usize {
        self.attempts
    }

    #[cfg(test)]
    pub(crate) fn state_kind(&self) -> &'static str {
        match self.state {
            LazyState::Building { .. } => "building",
            LazyState::Ready(_) => "ready",
            LazyState::Failed { .. } => "failed",
        }
    }

    #[cfg(test)]
    pub(crate) fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

fn validate_wait(wait: Duration) -> anyhow::Result<()> {
    if wait > FIRST_CALL_WAIT_MAX {
        anyhow::bail!(
            "first_call_wait must not exceed {} seconds",
            FIRST_CALL_WAIT_MAX.as_secs()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{CacheMode, ServerConfig};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    fn blocking_builder(
        cfg: ServerConfig,
        release: Arc<Mutex<mpsc::Receiver<()>>>,
    ) -> SessionBuilder {
        Arc::new(move || {
            release.lock().unwrap().recv().unwrap();
            SessionProvider::bootstrap(&cfg)
        })
    }

    fn blocking_lazy(wait: Duration) -> (tempfile::TempDir, LazySessionProvider, mpsc::Sender<()>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        let (release, rx) = mpsc::channel();
        let provider = LazySessionProvider::with_builder(
            &cfg,
            wait,
            blocking_builder(cfg.clone(), Arc::new(Mutex::new(rx))),
        )
        .unwrap();
        (dir, provider, release)
    }

    #[test]
    fn construction_starts_one_background_build_without_waiting_for_readiness() {
        let (_dir, mut provider, release) = blocking_lazy(Duration::from_secs(1));

        assert_eq!(provider.attempts(), 1);
        assert_eq!(provider.builds(), 1);
        assert_eq!(provider.state_kind(), "building");

        release.send(()).unwrap();
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
        assert_eq!(provider.attempts(), 1);
    }

    #[test]
    fn zero_wait_reports_warming_without_starting_another_build() {
        let (_dir, mut provider, _release) = blocking_lazy(Duration::ZERO);

        assert!(matches!(
            provider.ensure_ready(),
            Readiness::Warming { elapsed } if elapsed < Duration::from_secs(1)
        ));
        assert_eq!(provider.attempts(), 1);
        assert_eq!(provider.state_kind(), "building");
    }

    #[test]
    fn queued_calls_share_one_cumulative_wait_budget() {
        let (_dir, mut provider, release) = blocking_lazy(Duration::from_millis(200));

        let started = Instant::now();
        for _ in 0..4 {
            assert!(matches!(provider.ensure_ready(), Readiness::Warming { .. }));
        }
        assert!(
            started.elapsed() < Duration::from_millis(400),
            "queued calls must not multiply the one build attempt's wait budget"
        );
        assert_eq!(provider.attempts(), 1);

        release.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
        assert_eq!(provider.attempts(), 1);
    }

    #[test]
    fn injected_builder_rejects_an_unbounded_wait() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ServerConfig::new(dir.path().to_path_buf());
        let builder: SessionBuilder = Arc::new(|| anyhow::bail!("not reached"));

        let error = match LazySessionProvider::with_builder(&cfg, Duration::MAX, builder) {
            Ok(_) => panic!("unbounded wait must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("first_call_wait"));
    }
}
