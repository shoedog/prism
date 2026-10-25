//! Ordinary-session idle policy. Kept outside ServerConfig and the shared identity key.
use super::{ServerConfig, StartupMode};
use std::time::{Duration, Instant};

/// Provisional measurement setting; activation remains an explicit owner decision.
pub const PROVISIONAL_IDLE_SECS: u64 = 600;

#[derive(Clone, Copy, Debug, Default)]
pub struct IdlePolicy {
    pub(crate) timeout: Option<Duration>,
    pub(crate) pressure_relief: bool,
}

pub struct IdleResolution {
    pub policy: IdlePolicy,
    pub note: Option<&'static str>,
}

impl IdlePolicy {
    pub fn enabled(timeout: Duration) -> Self {
        Self {
            timeout: (!timeout.is_zero()).then_some(timeout),
            pressure_relief: false,
        }
    }

    pub fn with_pressure_relief(mut self, enabled: bool) -> Self {
        self.pressure_relief = enabled;
        self
    }

    /// One entry-point warning when the requested measurement arm cannot run.
    pub fn pressure_relief_note(&self, requested: bool, _shared: bool) -> Option<&'static str> {
        (requested && (self.timeout.is_none() || !cfg!(target_os = "macos"))).then_some(
            "prism-mcp: --index-idle-pressure-relief is unused: requires enabled I0 on macOS",
        )
    }

    /// Resolve once at the entry point. Unsupported modes ignore ambient values, including
    /// malformed ones, but reject an explicit nonzero request.
    pub fn resolve(
        explicit: Option<&str>,
        ambient: Option<&str>,
        cfg: &ServerConfig,
        owner: bool,
    ) -> anyhow::Result<IdleResolution> {
        let unsupported = owner || cfg.startup == StartupMode::Eager;
        let seconds = explicit.map(parse_seconds).transpose()?;
        if unsupported {
            anyhow::ensure!(
                seconds.unwrap_or(0) == 0,
                "--index-idle-secs must be 0 in Eager/owner mode"
            );
            return Ok(IdleResolution {
                policy: Self::default(),
                note: ambient
                    .map(|_| "prism-mcp: ignoring PRISM_MCP_INDEX_IDLE_SECS in Eager/owner mode"),
            });
        }
        let timeout = match seconds {
            Some(secs) => Duration::from_secs(secs),
            None => match ambient {
                Some(value) => Duration::from_secs(parse_seconds(value)?),
                None => Duration::ZERO,
            },
        };
        Ok(IdleResolution {
            policy: Self::enabled(timeout),
            note: None,
        })
    }
}

fn parse_seconds(value: &str) -> anyhow::Result<u64> {
    anyhow::ensure!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "index idle seconds must be an unsigned integer"
    );
    let secs = value
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("index idle seconds overflow"))?;
    anyhow::ensure!(
        Instant::now()
            .checked_add(Duration::from_secs(secs))
            .is_some(),
        "index idle seconds overflow the monotonic clock"
    );
    Ok(secs)
}

/// The only state retained from a disposed provider. No paths, sources, graph or Arc shadows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Published {
    pub fingerprint: [u8; 32],
    pub generation: u64,
    pub diverged: bool,
}

/// The stdio owner uses this phase; I0b will adapt the daemon's separate control state.
pub(crate) enum Phase {
    Idle,
    Building {
        started: Instant,
        deadline: Option<Instant>,
    },
    Ready,
    Failed {
        error: String,
        heard: bool,
    },
    Unloaded {
        last_published: Published,
    },
}

/// Hidden A+C experiment. Call only after the entire provider has been disposed.
pub(crate) fn pressure_relief(enabled: bool) -> usize {
    #[cfg(target_os = "macos")]
    if enabled {
        unsafe extern "C" {
            fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
        }
        // SAFETY: Apple's API accepts NULL for all zones and 0 to release all eligible pages.
        return unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    }
    let _ = enabled;
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::CacheMode;
    #[test]
    fn idle_config_validation_and_precedence() {
        let mut cfg = ServerConfig::new(".".into());
        let resolve =
            |flag, env, cfg: &ServerConfig, owner| IdlePolicy::resolve(flag, env, cfg, owner);
        assert_eq!(
            resolve(None, None, &cfg, false).unwrap().policy.timeout,
            None
        );
        assert_eq!(
            resolve(Some("0"), Some("bad"), &cfg, false)
                .unwrap()
                .policy
                .timeout,
            None
        );
        assert_eq!(
            resolve(Some("3"), Some("9"), &cfg, false)
                .unwrap()
                .policy
                .timeout,
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            resolve(None, Some("9"), &cfg, false)
                .unwrap()
                .policy
                .timeout,
            Some(Duration::from_secs(9))
        );
        for value in [
            "",
            "-1",
            "+1",
            "1.5",
            " 1",
            "18446744073709551616",
            "18446744073709551615",
        ] {
            assert!(resolve(None, Some(value), &cfg, false).is_err(), "{value}");
        }
        cfg.cache = CacheMode::NoCache;
        assert!(resolve(None, None, &cfg, false)
            .unwrap()
            .policy
            .timeout
            .is_none());
        assert!(resolve(None, Some("1"), &cfg, false)
            .unwrap()
            .policy
            .timeout
            .is_some());
        assert!(resolve(Some("1"), None, &cfg, false)
            .unwrap()
            .policy
            .timeout
            .is_some());
        for owner in [false, true] {
            cfg.startup = if owner {
                StartupMode::Lazy
            } else {
                StartupMode::Eager
            };
            let ignored = resolve(None, Some("bad"), &cfg, owner).unwrap();
            assert!(ignored.note.is_some());
            assert!(ignored.policy.timeout.is_none());
            assert!(resolve(Some("1"), None, &cfg, owner).is_err());
            assert!(resolve(Some("0"), None, &cfg, owner).is_ok());
        }
    }
}
