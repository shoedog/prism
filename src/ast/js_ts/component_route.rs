//! Test overlay for the component-value proof seam.
#![cfg(test)]

use crate::ast::ParsedFile;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SiteKind {
    Literal,
    Jsx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Direct,
    LocalHookOperand,
    JsxRenderOperand,
    UnknownWrapper,
    UnprovedIdentity,
}

// S2b-STUB: classify the actual value expression against the selected syntax.
pub(super) fn js_ts_component_route(
    _site_kind: SiteKind,
    _lexical_file: &ParsedFile,
    _lexical_site: Node<'_>,
    _selected_file: &ParsedFile,
    _selected_callable: Node<'_>,
    _export_proved: bool,
) -> Route {
    Route::UnprovedIdentity
}

// S2b-STUB: use recorded attempts to count F9's JSX numerator and denominator.
pub(super) fn wrapper_census_counts(_attempts: &[(SiteKind, bool, Route)]) -> (usize, usize) {
    (0, 0)
}
