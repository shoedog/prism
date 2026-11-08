//! #724: the anchored spelling of a Rust macro-argument call, for the
//! authoritative route only.
//!
//! The macro-argument extractor records `super::super::f(..)` inside
//! `assert!(..)` as `callee_name = "f"` (the chain from its first identifier,
//! because `crate`/`self`/`super` are not `identifier` nodes) and keeps the
//! written prefix aside in `CallSite::path_anchor`. Every consumer other than
//! the authoritative route (the legacy `::` branch, the callers index, the
//! dumps) reads `callee_name` as recorded, so only this accessor composes the
//! path the way rustc reads it.
use crate::call_graph::CallSite;

/// The site as the authoritative route should read it: the written keyword
/// prefix followed by the recorded name (`"super::super"` + `"f"` is
/// `super::super::f`). `None` when the site has no anchor.
pub(crate) fn anchored_site(site: &CallSite) -> Option<CallSite> {
    let anchor = site.path_anchor.as_deref()?;
    let mut anchored = site.clone();
    anchored.callee_name = format!("{anchor}::{}", site.callee_name);
    anchored.path_anchor = None;
    Some(anchored)
}
