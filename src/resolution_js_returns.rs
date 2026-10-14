//! Slice V: resolution of a JS/TS site whose callee is a value another in-repo function
//! returned. This is the skeleton the tests are written against; the route proves nothing.
use super::{CallGraph, CallSite, ResolutionOutcome};
use crate::ast::JsFollow;
use crate::js_returns::JsReturnPath;

impl CallGraph {
    pub(super) fn js_ts_returned_route<'a>(
        &'a self,
        _hook: &JsFollow,
        _path: &JsReturnPath,
        _site: &CallSite,
    ) -> Option<ResolutionOutcome<'a>> {
        None
    }
}
