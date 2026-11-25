use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteTable {
    routes: Vec<Route>,
    mounts: Vec<Mount>,
    summary: Summary,
}

impl RouteTable {
    /// Build the listing and enforce receiver and callback proof together.
    pub fn new(
        mut routes: Vec<Route>,
        mut mounts: Vec<Mount>,
        mut diagnostics: Vec<Diagnostic>,
        mut failed_roots: Vec<FailedRoot>,
        enabled_adapters: impl IntoIterator<Item = Framework>,
    ) -> Self {
        let failed: BTreeSet<_> = failed_roots.iter().map(|event| &event.root).collect();
        let mut unproven = 0;
        let mut candidates = 0;
        for route in &mut routes {
            if let Receiver::Unproven { reason } = route.receiver {
                route.candidate = Some(true);
                route.paths.clear();
                for callback in &mut route.callbacks {
                    match &mut callback.status {
                        CallbackStatus::Resolved { argument_text, .. }
                        | CallbackStatus::Anonymous { argument_text, .. } => {
                            let text = if argument_text.is_empty() {
                                let diagnostic = Diagnostic {
                                    site: route.site.clone(),
                                    reason: Reason::CallbackUnproven,
                                };
                                if !diagnostics.contains(&diagnostic) {
                                    diagnostics.push(diagnostic);
                                }
                                "<argument unavailable>".into()
                            } else {
                                std::mem::take(argument_text)
                            };
                            callback.status = CallbackStatus::Unresolved { text, reason };
                        }
                        CallbackStatus::Unresolved {
                            reason: slot_reason,
                            ..
                        } => {
                            // The unproven-receiver wire contract requires
                            // the receiver's refusal on every callback slot.
                            *slot_reason = reason;
                        }
                    }
                    callback.delegate = None;
                }
                unproven += 1;
            }
            candidates += usize::from(route.candidate == Some(true));
            for exposure in &mut route.paths {
                if exposure
                    .root
                    .as_ref()
                    .is_some_and(|root| failed.contains(root))
                {
                    exposure.binding_blocked = Some(MethodSet::ANY);
                }
            }
            route.paths.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        }
        routes.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        mounts.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        diagnostics.sort();
        failed_roots.sort();
        let summary = Summary {
            enabled_adapters: enabled_adapters.into_iter().collect(),
            registrations: routes.len(),
            proven: routes.len() - unproven,
            unproven,
            candidates,
            mounts: mounts.len(),
            diagnostics,
            failed_roots,
        };
        Self {
            routes,
            mounts,
            summary,
        }
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }
}
