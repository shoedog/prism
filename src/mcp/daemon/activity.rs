//! Tool admission and serving lifetime. Socket and handshake lifetimes are independent.
use super::coordinator::{ControlState, DaemonState, Poisoned};
use std::sync::Arc;
use std::time::Instant;

/// A bounded latest-event slot for I0b-2's lifecycle formatter; produced at the real release.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // consumed by lifecycle formatting in the following slice
pub(super) struct LastRelease {
    pub(super) at: Instant,
    pub(super) attempt: u64,
    pub(super) generation: u64,
}

pub(super) struct ToolActivity {
    state: Arc<DaemonState>,
    serving: bool,
}

impl ToolActivity {
    /// Called by every Ready exit while it still holds the answering control critical section.
    pub(super) fn serving(&mut self, control: &mut ControlState) {
        debug_assert!(!self.serving);
        control.idle.pending -= 1;
        control.idle.leases += 1;
        self.serving = true;
    }
}

impl DaemonState {
    pub(super) fn admit_tool(self: &Arc<Self>) -> Result<ToolActivity, Poisoned> {
        let mut control = self.control_plane()?;
        control.idle.pending += 1;
        Ok(ToolActivity {
            state: self.clone(),
            serving: false,
        })
    }

    #[cfg(test)]
    pub(super) fn activity_counts(&self) -> (u64, u64, bool) {
        let control = self.control_plane().unwrap();
        (
            control.idle.pending,
            control.idle.leases,
            control.idle.disposing,
        )
    }

    #[cfg(test)]
    pub(super) fn dispatch_hook(&self) {
        if let Some(hook) = &self.hooks.dispatch {
            hook();
        }
    }
    #[cfg(test)]
    pub(super) fn response_hook(&self) {
        if let Some(hook) = &self.hooks.response {
            hook();
        }
    }
    #[cfg(test)]
    pub(super) fn returned_hook(&self) {
        if let Some(hook) = &self.hooks.returned {
            hook();
        }
    }
}

impl Drop for ToolActivity {
    fn drop(&mut self) {
        {
            // Every production poisoner has already fired its domain's fatal guard. Cleanup
            // never reads through poison and must not issue a second fatal report while that
            // failure unwinds (tests replace the otherwise nonreturning abort).
            let Ok(mut control) = self.state.control.lock() else {
                return;
            };
            let _fatal = self.state.guard(super::fatal::Domain::Control);
            if self.serving {
                control.idle.leases -= 1;
            } else {
                control.idle.pending -= 1;
            }
            // Sample after all slot waiting and response construction, even for errors/unwind.
            let now = self.state.now();
            control.idle.last_release = Some(now);
            if control.idle.pending == 0 && control.idle.leases == 0 {
                control.idle.last_release_event = Some(LastRelease {
                    at: now,
                    attempt: control.attempts,
                    generation: control.generation,
                });
            }
        }
        self.state.wake_idle_loop();
    }
}
