//! Frozen ordered aggregation and label insertion.
use super::super::build::{file_refs, FileRefs};
use super::*;
use rayon::prelude::*;

impl LegacyDataFlowGraph {
    pub(super) fn insert_label(
        labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        key: (VarLocation, VarLocation),
        label: FlowConfidence,
    ) {
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }

    /// Build a data flow graph from only the specified files.
    ///
    /// Identical to `build()` but skips files not in `only_files`. Used by
    /// incremental cache update to process only changed files.
    pub fn build_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        // Filter to only the requested files and delegate to build.
        let filtered: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        Self::build_from_refs(&filtered)
    }

    /// Build a data flow graph from parsed files.
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let refs: BTreeMap<String, &ParsedFile> =
            files.iter().map(|(k, v)| (k.clone(), v)).collect();
        Self::build_from_refs(&refs)
    }

    /// Build a DFG from file references (shared implementation for build and build_subset).
    fn build_from_refs(files: &BTreeMap<String, &ParsedFile>) -> Self {
        let mut defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut edges = Vec::new();
        let mut labels = BTreeMap::new();
        let mut exact_labels = BTreeMap::new();
        let mut rd_function_stats = BTreeMap::new();

        let ordered_files: Vec<(&String, &ParsedFile)> =
            files.iter().map(|(path, parsed)| (path, *parsed)).collect();
        let per_file_refs: Vec<FileRefs> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                file_refs(file_path, parsed)
            })
            .collect();

        for file_refs in per_file_refs {
            // Invariant: per-file passes only register keys for their own file —
            // cross-file registration would break merge disjointness (S1 review MAJOR 2).
            debug_assert!(file_refs
                .defs
                .keys()
                .chain(file_refs.uses.keys())
                .all(|key| key.0.as_str() == file_refs.file_path.as_str()));
            debug_assert!(file_refs.labels.keys().all(|(from, to)| {
                from.file.as_str() == file_refs.file_path.as_str()
                    && to.file.as_str() == file_refs.file_path.as_str()
            }));
            debug_assert!(file_refs.exact_labels.keys().all(|edge| {
                edge.from.file.as_str() == file_refs.file_path.as_str()
                    && edge.to.file.as_str() == file_refs.file_path.as_str()
            }));
            for (key, locs) in file_refs.defs {
                defs.entry(key).or_default().extend(locs);
            }
            for (key, locs) in file_refs.uses {
                uses.entry(key).or_default().extend(locs);
            }
            edges.extend(file_refs.edges);
            for (key, label) in file_refs.labels {
                Self::insert_label(&mut labels, key, label);
            }
            for (edge, label) in file_refs.exact_labels {
                exact_labels
                    .entry(edge)
                    .and_modify(|stored: &mut FlowConfidence| *stored = stored.worst(label))
                    .or_insert(label);
            }
            rd_function_stats.insert(file_refs.file_path, file_refs.rd_function_stats);
        }

        // Build adjacency maps
        let mut forward: BTreeMap<VarLocation, Vec<VarLocation>> = BTreeMap::new();
        let mut backward: BTreeMap<VarLocation, Vec<VarLocation>> = BTreeMap::new();
        for edge in &edges {
            forward
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
            backward
                .entry(edge.to.clone())
                .or_default()
                .push(edge.from.clone());
        }

        LegacyDataFlowGraph {
            edges,
            defs,
            uses,
            forward,
            backward,
            labels,
            exact_labels,
            rd_function_stats,
        }
    }
}
