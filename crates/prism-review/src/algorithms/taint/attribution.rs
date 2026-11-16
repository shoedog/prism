//! Review-private emission attribution (#602 S4d1 part E).
//!
//! The taint run decides, privately, which source reaches which sink and with what evidence.
//! This module keeps that relation. The run records it at the lines where it decides, with the
//! value it uses there: nothing is rediscovered, no description is read, no line numbers are
//! compared afterwards. With no sidecar every helper returns before it clones or allocates.
//!
//! The sidecar is crate-private and is returned only by the scoped run. Public results, the
//! legacy entry points and `TaintTrace` are unchanged.
use prism::finding_confidence::EvidencePath;

/// What the run emitted: one entry per sink-style finding and per `taint_source` finding.
#[derive(Debug, Clone, Default)]
pub(crate) struct EmissionAttribution {
    pub(crate) sinks: Vec<SinkEmission>,
    pub(crate) sources: Vec<SourceEmission>,
}

/// One sink-style finding (`taint_sink` or `unquoted_expansion`).
// Nothing reads these fields in production yet: S4d1 part U is the first reader.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SinkEmission {
    /// Position of the finding in `SliceResult::findings` (and `evidence`).
    pub(crate) finding_index: usize,
    pub(crate) category: String,
    pub(crate) file: String,
    pub(crate) line: usize,
    /// The source the finding's description names, as the run chose it.
    pub(crate) selected_source: Option<(String, usize)>,
    /// Every source this finding licensed, in decision order, with the evidence used for it.
    pub(crate) licensed: Vec<LicensedSource>,
}

/// One source licensed by one sink-style finding, with the evidence of THAT pair.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct LicensedSource {
    pub(crate) file: String,
    pub(crate) line: usize,
    pub(crate) evidence: EvidencePath,
    pub(crate) route: LicenseRoute,
}

/// Which decision of the run licensed the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LicenseRoute {
    /// A source recorded for the sink by a real flow path.
    Path,
    /// The nearest-source fallback for a sink with no path linkage.
    Fallback,
    /// The nearest preceding same-file source of an unquoted expansion.
    Unquoted,
}

/// One emitted `taint_source` finding.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SourceEmission {
    pub(crate) finding_index: usize,
    pub(crate) file: String,
    pub(crate) line: usize,
}

#[cfg(test)]
thread_local! {
    /// Test-only: stores made by this thread's helpers, so a test can see that `None` stored none.
    static STORED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn note_store() {
    STORED.with(|stored| stored.set(stored.get() + 1));
}

#[cfg(not(test))]
fn note_store() {}

/// The run's handle on an optional sidecar. Licenses are held until the sink-style finding they
/// belong to is recorded; with `None` nothing is held.
pub(super) struct Recorder<'a> {
    sidecar: Option<&'a mut EmissionAttribution>,
    licensed: Vec<LicensedSource>,
}

impl<'a> Recorder<'a> {
    pub(super) fn new(sidecar: Option<&'a mut EmissionAttribution>) -> Self {
        Self {
            sidecar,
            licensed: Vec::new(),
        }
    }

    /// A source licensed for the sink the run is deciding; `evidence` is the value the run hands
    /// to `keep_best_evidence` for this pair.
    pub(super) fn license(
        &mut self,
        source: (&str, usize),
        evidence: &EvidencePath,
        route: LicenseRoute,
    ) {
        if self.sidecar.is_none() {
            return;
        }
        self.licensed.push(LicensedSource {
            file: source.0.to_string(),
            line: source.1,
            evidence: evidence.clone(),
            route,
        });
        note_store();
    }

    /// The sink-style finding about to be pushed at `finding_index`, with the licenses held so far.
    pub(super) fn sink(
        &mut self,
        finding_index: usize,
        category: &str,
        at: (&str, usize),
        selected: Option<(&str, usize)>,
    ) {
        let Some(sidecar) = self.sidecar.as_mut() else {
            return;
        };
        sidecar.sinks.push(SinkEmission {
            finding_index,
            category: category.to_string(),
            file: at.0.to_string(),
            line: at.1,
            selected_source: selected.map(|(file, line)| (file.to_string(), line)),
            licensed: std::mem::take(&mut self.licensed),
        });
        note_store();
    }

    /// The `taint_source` finding about to be pushed at `finding_index`.
    pub(super) fn source(&mut self, finding_index: usize, at: (&str, usize)) {
        let Some(sidecar) = self.sidecar.as_mut() else {
            return;
        };
        sidecar.sources.push(SourceEmission {
            finding_index,
            file: at.0.to_string(),
            line: at.1,
        });
        note_store();
    }
}

#[cfg(test)]
mod tests;
