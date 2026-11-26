//! Incoming Step 5b admission for an eligible bare-arrow formal.
use super::{callee_params::BareSlot, *};
use std::io::{self, Write};

pub(super) struct Admission {
    fixed_literal: bool,
    bare_literal: bool,
    refusal: Option<&'static str>,
}

impl Admission {
    pub(super) fn new(
        bare: Option<BareSlot>,
        site: &CallSite,
        cg: &CallGraph,
        resolved: &ResolvedCallee<'_>,
        exact_singleton: bool,
        cross_file_free_single_exact: bool,
    ) -> Self {
        let refusal = bare.and_then(|bare| {
            // This order assigns one reason even if multiple Exact checks fail.
            if cross_file_free_single_exact {
                Some("cross_file_free_single_exact")
            } else if bare.inferred_from_enclosing_call {
                Some("inferred_from_enclosing_call")
            } else if resolved.confidence == ResolutionConfidence::Exact {
                if !exact_singleton {
                    Some("non_singleton_exact")
                } else if resolved.kind == ResolutionKind::ImportQualified
                    && !cg.js_ts_namespace_export_matches(site, resolved.target)
                {
                    Some("missing_actual_export_witness")
                } else {
                    None
                }
            } else if site.caller.file != resolved.target.file {
                Some("foreign_ordinary_name_only")
            } else {
                None
            }
        });
        Self {
            fixed_literal: exact_singleton,
            bare_literal: resolved.confidence == ResolutionConfidence::Exact && refusal.is_none(),
            refusal,
        }
    }

    pub(super) fn ordinary(&self, bare: Option<BareSlot>) -> bool {
        bare.is_none() || self.refusal.is_none()
    }

    pub(super) fn literal(&self, bare: Option<BareSlot>) -> bool {
        if bare.is_some() {
            self.bare_literal
        } else {
            self.fixed_literal
        }
    }

    pub(super) fn report<'a>(
        &self,
        seen: &mut TraceKeys<'a>,
        site: &CallSite,
        resolved: &ResolvedCallee<'a>,
        bare: Option<BareSlot>,
    ) {
        if let (Some(reason), Some(bare)) = (self.refusal, bare) {
            seen.report(site, resolved, bare, reason);
        }
    }
}

pub(super) fn trace_enabled() -> bool {
    #[cfg(test)]
    if capture::active() {
        return true;
    }
    std::env::var_os("PRISM_C_BARE_TRACE").is_some_and(|value| value == "1")
}

/// Test-only capture of the trace records (a sink beside stderr, so tests can read them).
#[cfg(test)]
pub(super) mod capture {
    use std::sync::Mutex;

    static LINES: Mutex<Option<Vec<String>>> = Mutex::new(None);
    static SERIAL: Mutex<()> = Mutex::new(());

    pub(super) fn active() -> bool {
        LINES.lock().map(|l| l.is_some()).unwrap_or(false)
    }

    pub(in crate::cpg) fn push(line: &str) -> bool {
        match LINES.lock().ok().as_deref_mut() {
            Some(Some(lines)) => {
                lines.push(line.to_owned());
                true
            }
            _ => false,
        }
    }

    /// Run `f` with capture on; other tests running meanwhile may add their own records.
    pub(in crate::cpg) fn run<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
        let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        *LINES.lock().unwrap() = Some(Vec::new());
        let out = f();
        let lines = LINES.lock().unwrap().take().unwrap_or_default();
        (out, lines)
    }
}

#[derive(Default)]
pub(super) struct TraceKeys<'a>(BTreeSet<(&'a FunctionId, &'static str, ResolutionConfidence)>);

impl<'a> TraceKeys<'a> {
    fn report(
        &mut self,
        site: &CallSite,
        resolved: &ResolvedCallee<'a>,
        bare: BareSlot,
        reason: &str,
    ) {
        if !self
            .0
            .insert((resolved.target, resolved.kind.as_str(), resolved.confidence))
        {
            return;
        }
        let record = serde_json::json!({
            "caller_file": site.caller.file,
            "call": [site.start_byte, site.end_byte],
            "callee": site.callee_name,
            "target_file": resolved.target.file,
            "target_owner": resolved.target.name,
            "declaration": bare.declaration,
            "formal": bare.formal,
            "route": resolved.kind.as_str(),
            "confidence": resolved.confidence,
            "reason": reason,
        });
        let line = format!("PRISM_C_BARE_TRACE {record}\n");
        #[cfg(test)]
        if capture::push(&line) {
            return;
        }
        let _ = io::stderr().lock().write_all(line.as_bytes());
    }
}
