use super::session::canonical_config;
use super::{ServerConfig, SessionProvider, StartupMode, FIRST_CALL_WAIT_MAX};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub(crate) type SessionBuilder = Arc<dyn Fn() -> anyhow::Result<SessionProvider> + Send + Sync>;

pub enum Readiness {
    Ready,
    Warming { elapsed: Duration },
    Failed { error: String },
}

#[cfg(test)]
pub(crate) type LazyTestHook = Arc<dyn Fn() + Send + Sync>;

#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct LazyTestHooks {
    pub(crate) published: Option<LazyTestHook>,
    pub(crate) before_wait: Option<LazyTestHook>,
}

pub struct LazySessionProvider {
    state: LazyState,
    /// `Lazy` (build on the first valid `tools/call`) or `Background` (build at startup).
    startup: StartupMode,
    wait: Duration,
    last_error: Option<String>,
    attempts: usize,
    builder: SessionBuilder,
    #[cfg(test)]
    hooks: LazyTestHooks,
}

enum LazyState {
    /// No build has been requested yet. The default (`StartupMode::Lazy`) construction state:
    /// the index is built only once a valid `tools/call` asks for it, so a client that merely
    /// handshakes and lists tools never pays for the repository index.
    Idle,
    Building {
        rx: mpsc::Receiver<anyhow::Result<SessionProvider>>,
        started: Instant,
        deadline: Option<Instant>,
    },
    Ready(Box<SessionProvider>),
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
        Self::from_canonical_config(
            wait,
            builder,
            cfg.startup,
            #[cfg(test)]
            LazyTestHooks::default(),
        )
    }

    #[cfg(test)]
    pub(crate) fn with_builder(
        cfg: &ServerConfig,
        wait: Duration,
        builder: SessionBuilder,
    ) -> anyhow::Result<Self> {
        Self::with_builder_and_hooks(cfg, wait, builder, LazyTestHooks::default())
    }

    #[cfg(test)]
    pub(crate) fn with_builder_and_hooks(
        cfg: &ServerConfig,
        wait: Duration,
        builder: SessionBuilder,
        hooks: LazyTestHooks,
    ) -> anyhow::Result<Self> {
        canonical_config(cfg)?;
        Self::from_canonical_config(wait, builder, cfg.startup, hooks)
    }

    fn from_canonical_config(
        wait: Duration,
        builder: SessionBuilder,
        startup: StartupMode,
        #[cfg(test)] hooks: LazyTestHooks,
    ) -> anyhow::Result<Self> {
        validate_wait(wait)?;
        // This provider never builds before `initialize`; `StartupMode::Eager` is served by
        // `SessionProvider::bootstrap` (see `mcp::run`), so asking for it here is a caller error.
        if startup == StartupMode::Eager {
            anyhow::bail!(
                "StartupMode::Eager builds before `initialize` via SessionProvider::bootstrap; \
                 the lazy provider serves Lazy or Background only"
            );
        }
        let mut provider = Self {
            state: LazyState::Idle,
            startup,
            wait,
            last_error: None,
            attempts: 0,
            builder,
            #[cfg(test)]
            hooks,
        };
        if provider.startup == StartupMode::Background {
            provider.spawn_build();
        }
        Ok(provider)
    }

    pub(crate) fn startup_mode(&self) -> StartupMode {
        self.startup
    }

    pub fn ensure_ready(&mut self) -> Readiness {
        // Idle starts the first build; Failed retries it. Both then wait like Building.
        let start_build = match &self.state {
            LazyState::Idle => true,
            LazyState::Failed { error, at } => {
                let _ = (error, at);
                true
            }
            LazyState::Building { .. } | LazyState::Ready(_) => false,
        };
        if start_build {
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
                let remaining = deadline.saturating_duration_since(now);
                #[cfg(test)]
                if !remaining.is_zero() {
                    if let Some(before_wait) = &self.hooks.before_wait {
                        before_wait();
                    }
                }
                (rx.recv_timeout(remaining), started.elapsed())
            }
            LazyState::Ready(_) => return Readiness::Ready,
            LazyState::Idle | LazyState::Failed { .. } => {
                unreachable!("idle and failed states always start a build before waiting")
            }
        };

        match result {
            Ok(Ok(provider)) => {
                self.state = LazyState::Ready(Box::new(provider));
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
        #[cfg(test)]
        let published = self.hooks.published.clone();
        let started = Instant::now();
        std::thread::spawn(move || {
            let result = builder();
            let _ = tx.send(result);
            #[cfg(test)]
            if let Some(published) = published {
                published();
            }
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
            LazyState::Ready(provider) => Some(provider.as_ref()),
            LazyState::Idle | LazyState::Building { .. } | LazyState::Failed { .. } => None,
        }
    }

    pub(crate) fn ready_mut(&mut self) -> Option<&mut SessionProvider> {
        match &mut self.state {
            LazyState::Ready(provider) => Some(provider.as_mut()),
            LazyState::Idle | LazyState::Building { .. } | LazyState::Failed { .. } => None,
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
            LazyState::Idle => "idle",
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
    use crate::mcp::{CacheMode, ServerConfig, StartupMode};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    struct BlockingBuild {
        release: Option<mpsc::Sender<()>>,
        published: Option<mpsc::Receiver<()>>,
    }

    impl BlockingBuild {
        fn new(release: mpsc::Sender<()>, published: mpsc::Receiver<()>) -> Self {
            Self {
                release: Some(release),
                published: Some(published),
            }
        }

        fn release(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }

        fn wait_next(&mut self) {
            self.published
                .as_ref()
                .expect("blocking builder publication must be awaited once")
                .recv_timeout(Duration::from_secs(5))
                .expect("blocking builder must publish after release");
        }

        fn wait(&mut self) {
            self.wait_next();
            self.published.take();
        }

        fn finish(&mut self) {
            self.release();
            self.wait();
        }
    }

    impl Drop for BlockingBuild {
        fn drop(&mut self) {
            self.release();
            if let Some(published) = self.published.take() {
                let _ = published.recv_timeout(Duration::from_secs(5));
            }
        }
    }

    fn blocking_builder(
        cfg: ServerConfig,
        release: Arc<Mutex<mpsc::Receiver<()>>>,
    ) -> SessionBuilder {
        Arc::new(move || {
            if let Ok(release) = release.lock() {
                let _ = release.recv();
            }
            SessionProvider::bootstrap(&cfg)
        })
    }

    fn publication_hooks(
        published: mpsc::Sender<()>,
        before_wait: Option<mpsc::Sender<()>>,
    ) -> LazyTestHooks {
        LazyTestHooks {
            published: Some(Arc::new(move || {
                let _ = published.send(());
            })),
            before_wait: before_wait.map(|before_wait| -> LazyTestHook {
                Arc::new(move || {
                    let _ = before_wait.send(());
                })
            }),
        }
    }

    fn blocking_lazy(wait: Duration) -> (tempfile::TempDir, LazySessionProvider, BlockingBuild) {
        blocking_lazy_with_before_wait(wait, None)
    }

    fn blocking_lazy_with_before_wait(
        wait: Duration,
        before_wait: Option<mpsc::Sender<()>>,
    ) -> (tempfile::TempDir, LazySessionProvider, BlockingBuild) {
        blocking_lazy_with_startup(wait, before_wait, StartupMode::Lazy)
    }

    fn blocking_lazy_with_startup(
        wait: Duration,
        before_wait: Option<mpsc::Sender<()>>,
        startup: StartupMode,
    ) -> (tempfile::TempDir, LazySessionProvider, BlockingBuild) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        cfg.startup = startup;
        let (release, rx) = mpsc::channel();
        let (published, published_rx) = mpsc::channel();
        let provider = LazySessionProvider::with_builder_and_hooks(
            &cfg,
            wait,
            blocking_builder(cfg.clone(), Arc::new(Mutex::new(rx))),
            publication_hooks(published, before_wait),
        )
        .unwrap();
        (dir, provider, BlockingBuild::new(release, published_rx))
    }

    #[test]
    fn lazy_construction_starts_no_build_until_the_first_readiness_check() {
        let (_dir, mut provider, mut build) = blocking_lazy(Duration::from_secs(1));

        assert_eq!(provider.attempts(), 0);
        assert_eq!(provider.builds(), 0);
        assert_eq!(provider.state_kind(), "idle");
        assert!(provider.ready().is_none());

        // Release before the first check: the build the check starts completes at once.
        build.release();
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
        build.wait();
        assert_eq!(provider.attempts(), 1);
        assert_eq!(provider.state_kind(), "ready");
    }

    #[test]
    fn background_startup_starts_one_build_at_construction() {
        let (_dir, mut provider, mut build) =
            blocking_lazy_with_startup(Duration::from_secs(1), None, StartupMode::Background);

        assert_eq!(provider.attempts(), 1);
        assert_eq!(provider.builds(), 1);
        assert_eq!(provider.state_kind(), "building");

        build.release();
        build.wait();
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
        assert_eq!(provider.attempts(), 1);
    }

    #[test]
    fn zero_wait_reports_warming_without_starting_another_build() {
        let (before_wait, before_wait_rx) = mpsc::channel();
        let (_dir, mut provider, mut build) =
            blocking_lazy_with_before_wait(Duration::ZERO, Some(before_wait));

        assert!(matches!(
            provider.ensure_ready(),
            Readiness::Warming { elapsed } if elapsed < Duration::from_secs(1)
        ));
        assert_eq!(provider.attempts(), 1);
        assert_eq!(provider.state_kind(), "building");
        assert!(matches!(
            before_wait_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        build.finish();
    }

    #[test]
    fn queued_calls_share_one_cumulative_wait_budget() {
        let (_dir, mut provider, mut build) = blocking_lazy(Duration::from_millis(200));

        let started = Instant::now();
        for _ in 0..4 {
            assert!(matches!(provider.ensure_ready(), Readiness::Warming { .. }));
        }
        assert!(
            started.elapsed() < Duration::from_millis(400),
            "queued calls must not multiply the one build attempt's wait budget"
        );
        assert_eq!(provider.attempts(), 1);

        build.release();
        build.wait();
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
        assert_eq!(provider.attempts(), 1);
    }

    #[test]
    fn retry_starts_with_a_fresh_full_wait_budget() {
        use std::collections::VecDeque;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        let wait = Duration::from_millis(100);
        let outcomes = Arc::new(Mutex::new(VecDeque::from(["fail", "block"])));
        let (release, rx) = mpsc::channel();
        let (published, published_rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        let builder_cfg = cfg.clone();
        let builder: SessionBuilder =
            Arc::new(
                move || match outcomes.lock().unwrap().pop_front().unwrap() {
                    "fail" => anyhow::bail!("first build failure"),
                    "block" => {
                        if let Ok(release) = rx.lock() {
                            let _ = release.recv();
                        }
                        SessionProvider::bootstrap(&builder_cfg)
                    }
                    _ => unreachable!(),
                },
            );
        let mut build = BlockingBuild::new(release, published_rx);
        let mut provider = LazySessionProvider::with_builder_and_hooks(
            &cfg,
            wait,
            builder,
            publication_hooks(published, None),
        )
        .unwrap();

        assert!(matches!(provider.ensure_ready(), Readiness::Failed { .. }));
        assert_eq!(provider.attempts(), 1);
        build.wait_next();

        let retry_started = Instant::now();
        match provider.ensure_ready() {
            Readiness::Warming { elapsed } => {
                assert!(elapsed >= wait, "the retry receives a fresh full deadline");
            }
            _ => panic!("retry must wait for its new build"),
        }
        assert!(
            retry_started.elapsed() >= wait,
            "the retry must wait for its own full deadline"
        );
        assert_eq!(provider.attempts(), 2);

        build.finish();
        assert!(matches!(provider.ensure_ready(), Readiness::Ready));
    }

    #[test]
    fn eager_startup_is_rejected_by_the_lazy_provider() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = ServerConfig::new(dir.path().to_path_buf());
        cfg.cache = CacheMode::NoCache;
        cfg.startup = StartupMode::Eager;

        let error = match LazySessionProvider::new(&cfg) {
            Ok(_) => panic!("eager startup must not be served by the lazy provider"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("Eager"));
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
