//! Build-local syntax inventories; graph endpoints are resolved only at emission.
use super::*;
use crate::ast::ReturnInfo;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
#[path = "callee_return_cache_tests.rs"]
mod tests;

type Functions<'a> = BTreeMap<(usize, String), tree_sitter::Node<'a>>;
type FactCell = Arc<OnceLock<Option<Vec<ReturnInfo>>>>;

pub(super) struct ReturnCache<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    functions: BTreeMap<&'a str, OnceLock<Functions<'a>>>,
    entries: Mutex<BTreeMap<FunctionId, FactCell>>,
}

impl<'a> ReturnCache<'a> {
    pub(super) fn new(files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self {
            files,
            functions: files
                .keys()
                .map(|file| (file.as_str(), OnceLock::new()))
                .collect(),
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    pub(super) fn get(&self, callee: &FunctionId) -> FactCell {
        #[cfg(test)]
        if super::callee_legacy::enabled() {
            return Arc::new(OnceLock::from(
                self.files
                    .get(&callee.file)
                    .and_then(|p| super::callee_legacy::returns(p, callee)),
            ));
        }
        let cell = self
            .entries
            .lock()
            .expect("return cache lock")
            .entry(callee.clone())
            .or_default()
            .clone();
        // The map lock is released before syntax work. Cache missing owners and
        // empty inventories too; simultaneous first demand runs the walk once.
        cell.get_or_init(|| self.compute(callee));
        cell
    }

    fn compute(&self, callee: &FunctionId) -> Option<Vec<ReturnInfo>> {
        let parsed = self.files.get(&callee.file)?;
        let functions = self.functions[callee.file.as_str()].get_or_init(|| {
            #[cfg(test)]
            super::callee_cost_tests::record("return-lookups", callee);
            let mut out = BTreeMap::new();
            for node in parsed.all_functions() {
                if let Some(name) = parsed.language.function_name(&node) {
                    let key = (
                        parsed.node_line_range(&node).0,
                        parsed.node_text(&name).to_string(),
                    );
                    // The original find ignores end_line and keeps the FIRST
                    // matching node, even when several owners share this key.
                    out.entry(key).or_insert(node);
                }
            }
            out
        });
        let node = functions.get(&(callee.start_line, callee.name.clone()))?;
        #[cfg(test)]
        {
            super::callee_cost_tests::record("returns", callee);
            let visits = super::callee_census_tests::visits(parsed, *node, node.id());
            super::callee_cost_tests::record_n("return-visits", callee, visits);
        }
        Some(parsed.return_value_nodes(node))
    }
}
