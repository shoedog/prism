use super::ReviewBoundaryDiagnostics;
use crate::algorithms::taint::boundary::{
    display,
    records::{Availability, BoundaryCollection},
};
use prism::slice::SliceResult;

/// One algorithm result; diagnostics do not modify the typed result's warnings.
#[derive(Debug)]
#[non_exhaustive]
pub struct AlgorithmOutcome {
    pub result: SliceResult,
    pub diagnostics: Option<ReviewBoundaryDiagnostics>,
}
impl AlgorithmOutcome {
    pub fn new(result: SliceResult) -> Self {
        Self {
            result,
            diagnostics: None,
        }
    }
    pub(super) fn into_legacy_result(mut self) -> SliceResult {
        if let Some(diagnostics) = &self.diagnostics {
            self.result.warnings.extend(diagnostics.messages());
        }
        self.result
    }
}
#[derive(Default)]
pub(super) struct RunDiagnostics {
    complete: Option<BoundaryCollection>,
}
impl RunDiagnostics {
    pub(super) fn needs_collection(&self) -> bool {
        self.complete.is_none()
    }
    pub(super) fn capture_once(&mut self, capture: impl FnOnce() -> BoundaryCollection) {
        if self.complete.is_none() {
            self.complete = Some(capture());
        }
    }
    pub(super) fn project(&self) -> Option<ReviewBoundaryDiagnostics> {
        self.complete.as_ref().map(display::project)
    }
}
impl ReviewBoundaryDiagnostics {
    /// Render only new diagnostics, leaving ordinary warning composition to callers.
    pub fn messages(&self) -> Vec<String> {
        let mut messages: Vec<_> = self
            .located()
            .iter()
            .map(|note| note.message().to_owned())
            .collect();
        messages.extend(self.summary());
        if let Availability::ListingUnavailable(message) = self.availability() {
            messages.push(format!(
                "taint-context: boundary diagnostics unavailable: {}",
                display::escape_content(message)
            ));
        }
        messages
    }
}
#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
