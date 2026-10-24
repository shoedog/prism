//! Nonblocking unload decisions and off-lock, off-loop destruction of a whole provider.
use super::activity::LastRelease;
use super::coordinator::{BuildJob, BuildPhase, DaemonState, Poisoned};
use super::fatal::{Domain, Fatal};
use crate::mcp::idle::Published;
use crate::mcp::SessionProvider;
use std::sync::{mpsc, Arc, TryLockError};
use std::time::{Duration, Instant};

pub(super) type Wake = Arc<dyn Fn() + Send + Sync>;
const REARM: Duration = Duration::from_millis(10);

#[derive(Default)]
pub(super) struct IdleState {
    pub(super) pending: u64,
    pub(super) leases: u64,
    pub(super) last_release: Option<Instant>,
    pub(super) last_release_event: Option<LastRelease>,
    pub(super) published_at: Option<Instant>,
    pub(super) previous: Option<Published>,
    pub(super) watermark: u64,
    pub(super) disposing: bool,
    pub(super) disposal_failed: bool,
    rearm: Option<Instant>,
    spawn_retry: Option<Duration>,
}

pub(super) fn spawn_disposal_thread(job: BuildJob) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("prism-idx-drop".into())
        .spawn(job)
        .map(|_| ())
}

impl DaemonState {
    pub(super) fn set_idle_waker(&self, wake: Wake) {
        *self.idle_waker.lock().expect("idle waker") = Some(wake);
    }
    pub(super) fn wake_idle_loop(&self) {
        if let Some(wake) = self.idle_waker.lock().expect("idle waker").as_ref() {
            wake();
        }
    }
    pub(super) fn notify_draining(&self) {
        self.changed.notify_all();
        self.wake_idle_loop();
    }

    pub(super) fn unload_deadline(&self) -> Result<Option<Instant>, Poisoned> {
        let Some(timeout) = self.idle_timeout else {
            return Ok(None);
        };
        let control = self.control_plane()?;
        let idle = &control.idle;
        if !matches!(control.build, BuildPhase::Ready)
            || idle.pending != 0
            || idle.leases != 0
            || control.waiting != 0
            || idle.disposing
            || idle.disposal_failed
            || control.admission.draining.is_some()
        {
            return Ok(None);
        }
        Ok(idle.published_at.and_then(|published| {
            published
                .max(idle.last_release.unwrap_or(published))
                .checked_add(timeout)
                .map(|deadline| deadline.max(idle.rearm.unwrap_or(deadline)))
        }))
    }

    pub(super) fn unload_tick(self: &Arc<Self>) -> Result<(), Poisoned> {
        self.flush_last_release()?;
        if self
            .unload_deadline()?
            .is_none_or(|deadline| self.now() < deadline)
        {
            return Ok(());
        }
        // Never wait for a reader/refresh on the server loop. Poison is a fatal domain,
        // not ordinary contention. Recheck eligibility after taking slot → control.
        let mut slot = match self.provider.try_write() {
            Ok(slot) => slot,
            Err(TryLockError::WouldBlock) => {
                let mut control = self.control_plane()?;
                control.idle.rearm = self.now().checked_add(REARM);
                return Ok(());
            }
            Err(TryLockError::Poisoned(_)) => return Err(self.poisoned("provider")),
        };
        let locked_started = Instant::now();
        let mut control = self.control_plane()?;
        let now = self.now();
        let idle = &control.idle;
        let origin = idle
            .published_at
            .map(|p| p.max(idle.last_release.unwrap_or(p)));
        if !matches!(control.build, BuildPhase::Ready)
            || idle.pending != 0
            || idle.leases != 0
            || control.waiting != 0
            || idle.disposing
            || idle.disposal_failed
            || control.admission.draining.is_some()
            || origin.is_none_or(|p| now.saturating_duration_since(p) < self.idle_timeout.unwrap())
        {
            return Ok(());
        }
        let Some(provider) = slot.as_ref() else {
            return Ok(());
        };
        let last_published = provider.idle_publication();
        let mut event = self.lifecycle_snapshot(
            &control,
            "unload",
            "idle",
            Duration::ZERO,
            "complete".into(),
        );
        let mut disposal_event = event.clone();
        // Start the worker before transferring ownership. Failed spawn cannot drop a provider
        // captured by a rejected job, and leaves Ready/slot/history intact.
        let (send, receive) = mpsc::channel::<SessionProvider>();
        let state = self.clone();
        let job = Box::new(move || {
            let _completion = DisposalCompletion(state.clone());
            let Ok(provider) = receive.recv() else {
                return;
            };
            let started = Instant::now();
            disposal_event.event = "disposal_start";
            disposal_event.at = state.now();
            state.lifecycle_event(disposal_event.clone());
            #[cfg(test)]
            if let Some(hook) = &state.hooks.dispose {
                hook();
            }
            drop(provider);
            disposal_event.event = "disposal_end";
            disposal_event.at = state.now();
            disposal_event.duration = started.elapsed();
            state.lifecycle_event(disposal_event.clone());
            state.relieve(disposal_event);
        });
        if (self.disposal_spawner)(job).is_err() {
            let retry = control
                .idle
                .spawn_retry
                .map_or(REARM, |delay| (delay * 2).min(Duration::from_secs(1)));
            control.idle.spawn_retry = Some(retry);
            control.idle.rearm = now.checked_add(retry);
            return Ok(());
        }
        control.idle.spawn_retry = None;
        let provider = slot.take().expect("checked resident provider");
        control.build = BuildPhase::Unloaded { last_published };
        control.generation = 0;
        control.idle.previous = Some(last_published);
        control.idle.disposing = true;
        control.idle.rearm = None;
        drop(control);
        drop(slot);
        event.duration = locked_started.elapsed();
        self.lifecycle_event(event);
        // The worker only receives ownership after both guards are gone.
        // A valid spawner never runs the job inline and starts it exactly once.
        if let Err(unsent) = send.send(provider) {
            // A spawner that discarded its job violated the contract. Recover ownership rather
            // than destroying the provider here. Production's thread spawner cannot do this.
            let mut slot = self
                .provider
                .write()
                .map_err(|_| self.poisoned("provider"))?;
            let mut control = self.control_plane()?;
            control.generation = unsent.0.generation();
            *slot = Some(unsent.0);
            control.build = BuildPhase::Ready;
            control.idle.disposing = false;
            control.idle.rearm = self.now().checked_add(REARM);
        }
        Ok(())
    }
}

/// Made before the received provider, so that even unwinding finishes destruction before
/// accounting is cleared. Panic never authorizes another reload; production aborts here.
struct DisposalCompletion(Arc<DaemonState>);
impl Drop for DisposalCompletion {
    fn drop(&mut self) {
        let failed = std::thread::panicking();
        if failed {
            (self.0.on_fatal)(Fatal::Panicked(Domain::Disposal));
        }
        if let Ok(mut control) = self.0.control_plane() {
            control.idle.disposing = false;
            control.idle.disposal_failed |= failed;
        }
        self.0.changed.notify_all();
        self.0.wake_idle_loop();
    }
}
