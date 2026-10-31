//! Two smaller per-site walks have distinct fences: local-type shadowing
//! traverses nested declarations, and literal parameters require containment.
use super::{intervals::Intervals, SpanValue};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct Facts {
    local_types: BTreeMap<String, Intervals<()>>,
    literal_parameters: BTreeMap<String, Intervals<()>>,
}
impl Facts {
    pub(super) fn new(p: &ParsedFile, root: Node<'_>) -> Self {
        fn walk(
            p: &ParsedFile,
            root: Node<'_>,
            n: Node<'_>,
            types: &mut BTreeMap<String, Vec<SpanValue<()>>>,
            params: &mut BTreeMap<String, Vec<SpanValue<()>>>,
        ) {
            if matches!(n.kind(), "type_spec" | "type_alias") {
                if let Some(name) = n.child_by_field_name("name") {
                    let mut parent = n.parent();
                    while let Some(scope) = parent {
                        if matches!(
                            scope.kind(),
                            "block" | "expression_case" | "type_case" | "communication_case"
                        ) {
                            types
                                .entry(p.node_text(&name).trim().to_owned())
                                .or_default()
                                .push(SpanValue {
                                    lo: (n.start_byte() + 1).max(scope.start_byte()),
                                    hi: scope.end_byte(),
                                    value: (),
                                });
                            break;
                        }
                        parent = scope.parent();
                    }
                }
            }
            if n.kind() == "func_literal" {
                for (name, _) in super::collect::parameters(p, n, false) {
                    params.entry(name).or_default().push(SpanValue {
                        lo: n.start_byte().max(root.start_byte()),
                        hi: n.end_byte().min(root.end_byte()),
                        value: (),
                    });
                }
            }
            let mut c = n.walk();
            for child in n.children(&mut c) {
                walk(p, root, child, types, params);
            }
        }
        let (mut types, mut params) = (BTreeMap::new(), BTreeMap::new());
        walk(p, root, root, &mut types, &mut params);
        Self {
            local_types: types
                .into_iter()
                .map(|(name, rows)| (name, Intervals::new(rows)))
                .collect(),
            literal_parameters: params
                .into_iter()
                .map(|(name, rows)| (name, Intervals::new(rows)))
                .collect(),
        }
    }
    pub(super) fn at(&self, name: &str, byte: usize, literal: bool) -> bool {
        let rows = if literal {
            &self.literal_parameters
        } else {
            &self.local_types
        };
        let mut found = false;
        if let Some(row) = rows.get(name) {
            row.visit(byte, |_| {
                found = true;
                false
            });
        }
        found
    }
}
