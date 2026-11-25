//! Compact the per-file producer result before the parallel merge.
use super::Producer;
use crate::ast::ParsedFile;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
#[cfg(test)]
use std::collections::BTreeMap;

// Consuming boundary: expanded scratch dies on this worker before parallel collection.
impl Producer {
    pub(in crate::data_flow::build) fn compact_file_refs(
        file: &String,
        parsed: &ParsedFile,
        census: &RustMacroCensus,
        c_census: &CMacroCensus,
    ) -> crate::data_flow::compact::CompactDfg {
        let _profile_file = crate::profile::Phase::file("dfg.file", file);
        let parts = Self::file_refs_with_census(file, parsed, census, c_census).into_parts();
        #[cfg(test)]
        let legacy_adjacency_rows = parts
            .adjacency
            .as_ref()
            .map_or(0, |(f, b)| f.len() + b.len());
        let graph = crate::data_flow::compact::CompactDfg::from_parts(parts)
            .expect("DFG exceeds u32 location capacity");
        #[cfg(test)]
        if let Some(records) = FILE_RESULTS.lock().unwrap().as_mut() {
            if let Some(record) = records.get_mut(file) {
                *record = (
                    graph.arena.rows().len(),
                    graph.edges().len(),
                    legacy_adjacency_rows,
                    crate::data_flow::test_support::compact::storage(&graph),
                );
            }
        }
        graph
    }
}
#[cfg(test)]
pub(in crate::data_flow) static FILE_RESULTS: std::sync::Mutex<
    Option<BTreeMap<String, (usize, usize, usize, Vec<u8>)>>,
> = std::sync::Mutex::new(None);
