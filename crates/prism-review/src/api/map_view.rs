//! Borrowed maps for the completed taint context; original inputs retain ownership.

use prism::api::ReviewInputs;
use prism::ast::ParsedFile;
use prism::slice::FileParseQuality;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(crate) struct MapView<'a> {
    pub(crate) files: &'a BTreeMap<String, ParsedFile>,
    #[allow(dead_code)] // Source consumers arrive with S5 format adapters.
    pub(crate) sources: &'a BTreeMap<String, String>,
    pub(crate) parse_quality: &'a BTreeMap<String, FileParseQuality>,
}

impl<'a> MapView<'a> {
    pub(crate) fn original(inputs: &'a ReviewInputs) -> Self {
        Self::new(&inputs.files, &inputs.sources, &inputs.parse_quality)
    }

    pub(crate) fn new(
        files: &'a BTreeMap<String, ParsedFile>,
        sources: &'a BTreeMap<String, String>,
        parse_quality: &'a BTreeMap<String, FileParseQuality>,
    ) -> Self {
        Self {
            files,
            sources,
            parse_quality,
        }
    }
}

#[cfg(test)]
#[path = "map_view_tests.rs"]
pub(crate) mod tests;
