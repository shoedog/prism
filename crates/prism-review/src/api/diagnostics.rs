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
    context_applied: bool,
    notes: Vec<crate::algorithms::taint::boundary::records::ContextRunNote>,
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
    pub(super) fn apply_context(
        &mut self,
        collection: Option<BoundaryCollection>,
        notes: Vec<crate::algorithms::taint::boundary::records::ContextRunNote>,
    ) {
        if !self.context_applied {
            self.context_applied = true;
            self.complete = collection;
            self.notes = notes;
        }
    }
    pub(super) fn reconcile_context(
        &mut self,
        state: &super::context_run::ContextState,
        request: Option<&super::context_run::ContextRequest>,
        warnings: &[String],
    ) {
        let Some(request) = request else {
            return;
        };
        if self.context_applied || matches!(state, super::context_run::ContextState::NotRequested) {
            return;
        }
        let (collection, notes) = match self.complete.as_ref() {
            Some(collection) => {
                let (reconciled, notes) =
                    crate::algorithms::taint::boundary::context_notes::reconcile(
                        collection, state, request, warnings,
                    );
                (Some(reconciled), notes)
            }
            None => (
                None,
                crate::algorithms::taint::boundary::context_notes::notes_only(
                    state, request, warnings,
                ),
            ),
        };
        self.apply_context(collection, notes);
    }
    pub(super) fn project(&self) -> Option<ReviewBoundaryDiagnostics> {
        self.complete
            .as_ref()
            .map(|c| display::project_with_notes(c, self.notes.clone()))
            .or_else(|| {
                (!self.notes.is_empty()).then(|| display::BoundaryDisplay {
                    run_notes: self.notes.clone(),
                    ..Default::default()
                })
            })
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
        messages.extend(self.run_notes().iter().map(|n| n.message().to_string()));
        messages
    }
}
#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "context_notes_runner_tests.rs"]
mod context_notes_runner_tests;
