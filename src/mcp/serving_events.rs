//! Bounded scalar facts only: no graph, provider, serving lock or forcing accessor.
use super::{
    idle::IdlePolicy,
    mem_sample::{self, MemSample},
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A bounded token, including for empty, non-Unicode or operator-supplied free text.
pub(super) fn malloc_space_efficient() -> &'static str {
    match std::env::var_os("MallocSpaceEfficient").as_deref() {
        None => "unset",
        Some(value) if value == "1" => "1",
        Some(value) if value == "0" => "0",
        Some(_) => "other",
    }
}

static ORIGIN: OnceLock<Stamp> = OnceLock::new();
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Copy)]
pub(super) struct Stamp {
    pub at: Instant,
    pub wall: SystemTime,
}
impl Stamp {
    pub fn now() -> Self {
        Self {
            at: Instant::now(),
            wall: SystemTime::now(),
        }
    }
    pub fn mono(self) -> u128 {
        self.at.saturating_duration_since(origin().at).as_nanos()
    }
    pub fn epoch(self) -> String {
        self.wall
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| "unknown".into(), |d| d.as_nanos().to_string())
    }
}
pub(super) fn origin() -> Stamp {
    *ORIGIN.get_or_init(Stamp::now)
}
#[derive(Clone, Copy)]
pub(super) struct BuildFacts {
    pub cpg: &'static str,
    pub stages: [u128; 4],
}
#[derive(Clone, Default)]
pub(super) struct Details {
    pub build: Option<BuildFacts>,
    pub sidecar: &'static str,
    pub pair: Option<ReliefPair>,
}
impl Details {
    pub fn fields(&self, event: &str) -> String {
        let stages = self
            .build
            .map(|f| f.stages.map(|v| v.to_string()))
            .unwrap_or_else(|| std::array::from_fn(|_| "unknown".into()));
        let mut text = format!(
            "cpg={} load_parse_ns={} f_validation_ns={} index_ns={} bootstrap_ns={} sidecar={}",
            self.build.map_or("unknown", |f| f.cpg),
            stages[0],
            stages[1],
            stages[2],
            stages[3],
            match event {
                "publication" => "not_attempted",
                "unload" if !self.sidecar.is_empty() => self.sidecar,
                "unload" => "not_attempted",
                _ => "not_applicable",
            }
        );
        if let Some(p) = &self.pair {
            let before = p.times[0];
            let after = p.times[1];
            let heap_before = optional(p.samples[0].heap_in_use_bytes);
            let heap_after = optional(p.samples[1].heap_in_use_bytes);
            let footprint_before = optional(p.samples[0].phys_footprint_bytes);
            let footprint_after = optional(p.samples[1].phys_footprint_bytes);
            text.push_str(&format!(
                concat!(
                    " sample_before_ns={} sample_after_ns={} sample_before_epoch_ns={} sample_after_epoch_ns={} ",
                    "heap_before={} heap_after={} footprint_before={} footprint_after={}",
                ),
                before.mono(),
                after.mono(),
                before.epoch(),
                after.epoch(),
                heap_before,
                heap_after,
                footprint_before,
                footprint_after,
            ));
        }
        text
    }
}
fn optional(n: Option<u64>) -> String {
    n.map_or_else(|| "unknown".into(), |v| v.to_string())
}
#[derive(Clone)]
pub(super) struct ReliefPair {
    times: [Stamp; 2],
    samples: [MemSample; 2],
}
pub(super) fn relief_pair(enabled: bool, context: &Context) -> (ReliefPair, usize, Duration) {
    let before = Stamp::now();
    let first = context.sample();
    let started = Instant::now();
    let released = super::idle::pressure_relief(enabled);
    let duration = started.elapsed();
    let second = context.sample();
    let after = Stamp::now();
    (
        ReliefPair {
            times: [before, after],
            samples: [first, second],
        },
        released,
        duration,
    )
}
#[derive(Clone)]
pub(super) struct Context {
    pub identity: String,
    pub instance_start: String,
    pub timeout: Duration,
    pub relief: bool,
    #[cfg(test)]
    pub observe: Option<Observer>,
    #[cfg(test)]
    pub sampler: Option<std::sync::Arc<dyn Fn() -> MemSample + Send + Sync>>,
}
impl Context {
    pub fn new(identity: String, policy: IdlePolicy) -> Self {
        let start = origin();
        Self {
            identity,
            instance_start: start.epoch(),
            timeout: policy.timeout.unwrap_or_default(),
            relief: policy.pressure_relief && policy.pressure_relief_note(true, false).is_none(),
            #[cfg(test)]
            observe: observer(),
            #[cfg(test)]
            sampler: None,
        }
    }
    fn sample(&self) -> MemSample {
        #[cfg(test)]
        if let Some(sample) = &self.sampler {
            return sample();
        }
        mem_sample::sample()
    }
    pub fn fields(&self, time: Stamp) -> String {
        format!(
            "identity={} pid={} instance_start={} epoch_ns={} mono_ns={} effective_relief={}",
            self.identity,
            std::process::id(),
            self.instance_start,
            time.epoch(),
            time.mono(),
            self.relief
        )
    }
}

pub(super) fn sequenced(text: &str) -> String {
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    text.replacen(
        " released_bytes=",
        &format!(" sequence={seq} released_bytes="),
        1,
    )
}
pub(super) fn bounded(text: &str, outcome: &str) -> String {
    let mut out = printable(text, 950); // reserve sequence (30), outcome marker and prefix budget
    out.push_str(" outcome=");
    out.push_str(&printable(outcome, 970usize.saturating_sub(out.len())));
    out
}
fn printable(text: &str, max: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let c = if c.is_control() { '\u{fffd}' } else { c };
        if out.len() + c.len_utf8() > max {
            break;
        }
        out.push(c);
    }
    out
}
#[cfg(test)]
use parity_tests::{observer, Observer};
#[cfg(test)]
pub(crate) mod build_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod parity_tests;
