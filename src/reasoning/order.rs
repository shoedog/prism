//! Same-line recovered def-use ordering guard for witness-bearing reasoning traces.

use std::collections::BTreeMap;

use petgraph::graph::NodeIndex;

use crate::ast::ParsedFile;
use crate::cpg::{
    CodePropertyGraph, OrderingDecision, OrderingUnavailableReason, OrderingWarning,
    SameLineOrderView,
};

pub struct AstOrderView<'a> {
    cpg: &'a CodePropertyGraph,
    files: &'a BTreeMap<String, ParsedFile>,
}

impl<'a> AstOrderView<'a> {
    pub fn new(cpg: &'a CodePropertyGraph, files: &'a BTreeMap<String, ParsedFile>) -> Self {
        Self { cpg, files }
    }
}

impl SameLineOrderView for AstOrderView<'_> {
    fn admit_same_line_recovered_def_use(
        &self,
        def: NodeIndex,
        use_: NodeIndex,
    ) -> OrderingDecision {
        let Some(def_loc) = self.cpg.to_var_location(def) else {
            return admit_with_warning(
                None,
                "<unknown>",
                0,
                "",
                OrderingUnavailableReason::AstUnavailable,
            );
        };
        let Some(use_loc) = self.cpg.to_var_location(use_) else {
            return admit_with_warning(
                None,
                "<unknown>",
                0,
                "",
                OrderingUnavailableReason::AstUnavailable,
            );
        };
        if def_loc.file != use_loc.file || def_loc.line != use_loc.line {
            return OrderingDecision::Admit;
        }
        let path = def_loc.path.to_string();
        if path != use_loc.path.to_string() {
            return admit_with_warning(
                self.files.get(&def_loc.file),
                &def_loc.file,
                def_loc.line,
                &path,
                OrderingUnavailableReason::OccurrenceMismatch,
            );
        }
        let Some(_parsed) = self.files.get(&def_loc.file) else {
            return admit_with_warning(
                None,
                &def_loc.file,
                def_loc.line,
                &path,
                OrderingUnavailableReason::AstUnavailable,
            );
        };
        if use_loc.start_byte >= def_loc.start_byte {
            OrderingDecision::Admit
        } else {
            admit_with_warning(
                self.files.get(&def_loc.file),
                &def_loc.file,
                def_loc.line,
                &path,
                OrderingUnavailableReason::OccurrenceMismatch,
            )
        }
    }
}

fn admit_with_warning(
    parsed: Option<&ParsedFile>,
    file: &str,
    line: usize,
    path: &str,
    reason: OrderingUnavailableReason,
) -> OrderingDecision {
    OrderingDecision::AdmitWithWarning {
        warning: OrderingWarning {
            file: file.to_string(),
            line,
            path: path.to_string(),
            reason: if parsed.is_none() {
                OrderingUnavailableReason::AstUnavailable
            } else {
                reason
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::cpg::CpgNode;
    use crate::languages::Language;

    fn build(src: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
        let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
        let mut files = BTreeMap::new();
        files.insert("app.py".to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);
        (cpg, files)
    }

    fn vars(cpg: &CodePropertyGraph, path: &str, access: crate::cpg::VarAccess) -> Vec<NodeIndex> {
        cpg.node_indices()
            .filter(|&idx| {
                matches!(
                    cpg.node(idx),
                    CpgNode::Variable { path: p, access: a, .. } if p.to_string() == path && *a == access
                )
            })
            .collect()
    }

    fn var(cpg: &CodePropertyGraph, path: &str, access: crate::cpg::VarAccess) -> NodeIndex {
        vars(cpg, path, access).into_iter().next().unwrap()
    }

    #[test]
    fn unique_same_line_def_before_use_is_admitted() {
        let (cpg, files) = build("def f(u):\n    y = u; sink(y)\n");
        let view = AstOrderView::new(&cpg, &files);
        let def = var(&cpg, "y", crate::cpg::VarAccess::Def);
        let use_ = var(&cpg, "y", crate::cpg::VarAccess::Use);
        assert!(matches!(
            view.admit_same_line_recovered_def_use(def, use_),
            OrderingDecision::Admit
        ));
    }

    #[test]
    fn same_line_use_before_def_fails_open_with_warning() {
        let (cpg, files) = build("def f(u):\n    sink(y); y = u; baz(y)\n");
        let view = AstOrderView::new(&cpg, &files);
        let def = var(&cpg, "y", crate::cpg::VarAccess::Def);
        let uses = vars(&cpg, "y", crate::cpg::VarAccess::Use);
        assert!(!uses.is_empty());
        assert!(matches!(
            view.admit_same_line_recovered_def_use(def, uses[0]),
            OrderingDecision::AdmitWithWarning { .. }
        ));
    }

    #[test]
    fn identifier_substrings_in_other_tokens_do_not_create_duplicate_occurrences() {
        let (cpg, files) = build("def f(u):\n    id = u; valid(id)\n");
        let view = AstOrderView::new(&cpg, &files);
        let def = var(&cpg, "id", crate::cpg::VarAccess::Def);
        let use_ = var(&cpg, "id", crate::cpg::VarAccess::Use);
        assert!(matches!(
            view.admit_same_line_recovered_def_use(def, use_),
            OrderingDecision::Admit
        ));
    }
}
