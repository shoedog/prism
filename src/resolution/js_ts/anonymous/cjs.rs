//! #597: anonymous callables calling a module-scope CommonJS `require` binding.
//!
//! The consumer half is evidence recorded with the site (`js_ts_cjs_anonymous_require`: a direct
//! top-level `const` or `var`, declared once in the resolved program scope, with no write
//! resolving to that binding, not shadowed between the site and the program scope, one plain
//! relative literal). The producer half is the proof `resolution/js_ts/cjs.rs` gives named callers
//! (export-object barrier, terminal proof, no ESM export). The anonymous boundary adds its own
//! requirement that the producer's file environment is supported (no `with`, `eval`, escaped
//! identifiers or parse errors, nested `import =`, and the same for the files it re-exports
//! through).
//!
//! The route answers only with a certificate: whenever one cannot be issued it returns `None`,
//! so the site continues down the existing ladder and keeps its earlier outcome. Returning the
//! uncertified producer outcome would make `admit_anonymous_exact` demote it to a NameOnly
//! `import_member` edge, a grade the earlier ladder never gave the site. The producer-environment
//! check below is therefore redundant with the target-owner check (`anonymous_target_supported`)
//! and is kept as defence in depth.
use super::*;

/// What an anonymous site takes from a top-level `const` or `var` CommonJS `require`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CjsRequire {
    module_path: String,
    /// `None`: the module value itself (`const m = require('./x'); m()`).
    member: Option<String>,
}

/// Recorded with the call site, inside the dependency trace window.
pub(super) fn evidence<'a>(
    parsed: &'a ParsedFile,
    node: tree_sitter::Node<'a>,
    cache: &mut JsBindingCache<'a>,
) -> Option<CjsRequire> {
    if node.kind() != "call_expression" {
        return None;
    }
    let (module_path, member) = parsed.js_ts_cjs_anonymous_require(node, cache)?;
    Some(CjsRequire {
        module_path,
        member,
    })
}

impl CallGraph {
    /// The route for a site whose caller is an anonymous or module-level owner.
    pub(crate) fn anonymous_cjs_route(&self, site: &CallSite) -> Option<ResolutionOutcome<'_>> {
        let proof = self.js_anonymous.site(site)?;
        let cjs = proof.cjs.as_ref()?;
        let file = self.resolve_js_ts_member_module(&cjs.module_path, &site.caller.file)?;
        if !self.anonymous_module_supported(&file, &mut BTreeSet::new()) {
            return None;
        }
        let mut outcome = self.js_ts_cjs_producer_outcome(
            &cjs.module_path,
            cjs.member.as_deref(),
            &site.caller.file,
        )?;
        outcome.via_alias_route = true;
        let [one] = outcome.resolved.as_slice() else {
            return None;
        };
        if one.confidence != ResolutionConfidence::Exact
            || !self.anonymous_target_supported(one.target)
        {
            return None;
        }
        outcome.anonymous_certificate = Some(Certificate {
            site: site.clone(),
            target: one.target.clone(),
            evidence: proof.clone(),
            export: None,
        });
        Some(outcome)
    }
}
