//! Parallel/serial oracle for the multi-line Step-5b argument fixtures.

use super::build::CodePropertyGraph;
use super::multiline_call_arg_tests::{fixture_source, languages};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;

#[test]
fn step5b_parallel_and_serial_collectors_match_on_multiline_fixtures() {
    for (language, file) in languages() {
        let source = fixture_source(language, true);
        let parsed = ParsedFile::parse(file, &source, language).unwrap();
        let files = BTreeMap::from([(file.to_string(), parsed)]);
        let cpg = CodePropertyGraph::build(&files);
        let parallel = CodePropertyGraph::collect_step5b_edges(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let serial = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        assert_eq!(parallel, serial, "{language:?}: Step-5b par != serial");
    }
}
