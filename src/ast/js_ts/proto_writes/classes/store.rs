//! Prove that retaining a store's value also retains its selected slot.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn advances(&self, store: Node<'a>, target: Node<'a>) -> bool {
        if field(store, "left").map(peel) != Some(target) {
            return false;
        }
        let mut enclosing = store;
        while let Some(parent) = enclosing.parent() {
            if !self.charge(1) {
                return false;
            }
            match parent.kind() {
                "parenthesized_expression"
                | "non_null_expression"
                | "as_expression"
                | "satisfies_expression" => enclosing = parent,
                "assignment_expression" => {
                    if field(parent, "right") != Some(enclosing)
                        || !field(parent, "left").is_some_and(|left| identifier(peel(left)))
                    {
                        return false;
                    }
                    return self.retains_slot(store, target);
                }
                "variable_declarator" if field(parent, "value") == Some(enclosing) => {
                    return self.retains_slot(store, target);
                }
                "return_statement" | "arrow_function" => {
                    let mut current = Some(parent);
                    while let Some(node) = current {
                        if !self.charge(1) {
                            return false;
                        }
                        if is_owner(node) {
                            let accumulator = inline_iteration(self.file, node)
                                .and_then(|call| self.reducer(call))
                                .map(|(_, _, accumulator)| accumulator);
                            let reachable = parent.kind() != "return_statement"
                                || self.analysis.reachable_return(parent);
                            return reachable
                                && accumulator.is_some()
                                && field(target, "object").is_some_and(|object| {
                                    self.from_accumulator(object, accumulator, &mut BTreeSet::new())
                                })
                                && self.retains_slot(store, target);
                        }
                        current = node.parent();
                    }
                    return false;
                }
                _ => return false,
            }
        }
        false
    }

    fn retains_slot(&self, store: Node<'a>, target: Node<'a>) -> bool {
        if matches!(operator(store, self.file), "||=" | "??=") {
            // The branch retaining the selected child does not perform the store.
            return false;
        }
        let mut values: Vec<_> = field(store, "right").into_iter().collect();
        while let Some(value) = values.pop() {
            if !self.charge(1) {
                return false;
            }
            let value = peel(value);
            match value.kind() {
                "binary_expression" if matches!(operator(value, self.file), "||" | "??" | "&&") => {
                    if operator(value, self.file) != "&&" {
                        values.extend(field(value, "left"));
                    }
                    values.extend(field(value, "right"));
                }
                "ternary_expression" => {
                    values.extend(field(value, "consequence"));
                    values.extend(field(value, "alternative"));
                }
                "sequence_expression" => values.extend(children(value).last()),
                "subscript_expression" if self.same_slot(target, value) => return true,
                _ => {}
            }
        }
        false
    }

    fn same_slot(&self, write: Node<'a>, read: Node<'a>) -> bool {
        ["object", "index"].into_iter().all(|part| {
            let Some(left) = field(write, part).map(peel) else {
                return false;
            };
            let Some(right) = field(read, part).map(peel) else {
                return false;
            };
            if self.file.node_text(&left) != self.file.node_text(&right) {
                return false;
            }
            match (self.analysis.read(left), self.analysis.read(right)) {
                (Some(a), Some(b)) => {
                    if !self.charge(a.reaching_versions.len() + b.reaching_versions.len())
                        || a.binding.is_none()
                        || a.binding != b.binding
                    {
                        return false;
                    }
                    let stable = a.reaching_versions == b.reaching_versions
                        && !a.reaching_versions.is_empty()
                        && ((a.resolution_complete && b.resolution_complete)
                            || (identifier(left)
                                && identifier(right)
                                && self.analysis.correlated_identifiers(a, b)));
                    // A shared version is a candidate when a completeness
                    // check refused equality. Keep the fact without using
                    // this possibility to promote the final property role.
                    stable
                        || ((self.analysis.refused_read_proof(a)
                            || self.analysis.refused_read_proof(b))
                            && a.reaching_versions
                                .iter()
                                .any(|site| b.reaching_versions.binary_search(site).is_ok()))
                }
                _ => self.fixed_key(left).is_some(),
            }
        })
    }
}
