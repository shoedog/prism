use super::*;
use crate::ast::JsBindingCache;
use crate::js_write_census::{
    identity::{ident, Comp, Ident},
    tables::Tables,
};
use petgraph::Direction;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::cpg::build) enum SourceRefusal {
    Ambiguous,
    Unconnected,
    EnclosingOwner,
    NotFound,
}

impl SourceRefusal {
    pub(in crate::cpg::build) fn counter(self) -> &'static str {
        match self {
            Self::Ambiguous => "refused_source_ambiguous",
            Self::Unconnected => "refused_source_unconnected",
            Self::EnclosingOwner => "refused_source_in_enclosing_owner",
            Self::NotFound => "refused_source_not_found",
        }
    }
}

pub(super) struct Sources<'a, 'site> {
    site: &'site JsxSite<'a>,
    bindings: JsBindingCache<'a>,
}

#[derive(Clone, Copy)]
struct Operand<'tree, 'path> {
    root: Node<'tree>,
    binding: Option<Node<'tree>>,
    owner: Node<'tree>,
    path: &'path AccessPath,
}

impl<'a, 'site> Sources<'a, 'site> {
    pub(super) fn new(site: &'site JsxSite<'a>) -> Self {
        Self {
            site,
            bindings: JsBindingCache::for_sites(),
        }
    }

    pub(super) fn source_node(&mut self, leaf: &Leaf) -> Result<NodeIndex, SourceRefusal> {
        let Some(path) = &leaf.path else {
            return Err(SourceRefusal::NotFound);
        };
        let parsed = self.site.caller;
        let value = syntax::exact_node(parsed, leaf.span.clone()).ok_or(SourceRefusal::NotFound)?;
        let root = syntax::operand_root(value).ok_or(SourceRefusal::NotFound)?;
        let owner = syntax::execution_owner(parsed, root).ok_or(SourceRefusal::NotFound)?;
        if crate::anon_owner::identity(parsed, &owner, &self.site.call.caller.file).as_ref()
            != Some(&self.site.call.caller)
        {
            return Err(SourceRefusal::NotFound);
        }
        let operand = Operand {
            root,
            binding: self.binding(root, &path.base),
            owner,
            path,
        };
        if matches!(root.kind(), "this" | "super")
            && operand
                .binding
                .is_some_and(syntax::is_class_receiver_region)
        {
            return Err(SourceRefusal::Unconnected);
        }
        if self.destructuring_store_can_precede(operand) {
            return Err(SourceRefusal::Ambiguous);
        }
        let lines = operand_lines(value);
        let full = self.tier(operand, path, lines.iter().copied());
        #[cfg(test)]
        let full = if self.site.fall_through_on_full_conflict
            && !path.fields.is_empty()
            && full == Err(SourceRefusal::Ambiguous)
        {
            Err(SourceRefusal::Unconnected)
        } else {
            full
        };
        let mut refusal = match full {
            Ok(node) => return Ok(node),
            Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
            Err(reason) => reason,
        };
        let base = AccessPath::simple(&path.base);
        if !path.fields.is_empty() {
            let root_line = root.start_position().row + 1;
            match self.tier(operand, &base, [root_line]) {
                Ok(node) => return Ok(node),
                Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
                Err(SourceRefusal::Unconnected) => refusal = SourceRefusal::Unconnected,
                _ => {}
            }
            match self.tier(
                operand,
                &base,
                lines.iter().copied().filter(|line| *line != root_line),
            ) {
                Ok(node) => return Ok(node),
                Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
                Err(SourceRefusal::Unconnected) => refusal = SourceRefusal::Unconnected,
                _ => {}
            }
        }
        if refusal == SourceRefusal::NotFound && self.enclosing(operand, &lines, &base) {
            return Err(SourceRefusal::EnclosingOwner);
        }
        Err(refusal)
    }

    fn tier(
        &mut self,
        operand: Operand<'a, '_>,
        path: &AccessPath,
        lines: impl IntoIterator<Item = usize>,
    ) -> Result<NodeIndex, SourceRefusal> {
        let mut unconnected = false;
        let mut first = None;
        for line in lines {
            let Some(node) = self.lookup(&self.site.call.caller, path, line) else {
                continue;
            };
            match self.candidate(operand, path, line, node) {
                Ok(()) => {
                    first.get_or_insert(node);
                }
                Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
                Err(_) => unconnected = true,
            }
        }
        first.ok_or(if unconnected {
            SourceRefusal::Unconnected
        } else {
            SourceRefusal::NotFound
        })
    }

    fn lookup(&self, id: &FunctionId, path: &AccessPath, line: usize) -> Option<NodeIndex> {
        self.site.lookup.get_ref(
            self.site.graph,
            VarKeyRef {
                file: &id.file,
                function: &id.name,
                function_start_line: id.start_line,
                line,
                path,
                access: VarAccess::Use,
            },
        )
    }

    fn binding(&mut self, at: Node<'a>, name: &str) -> Option<Node<'a>> {
        let parsed = self.site.caller;
        if matches!(at.kind(), "this" | "super") {
            return syntax::receiver_owner(parsed, at);
        }
        if at.parent().is_some_and(|parent| {
            parent.kind() == "arrow_function" && parent.child_by_field_name("parameter") == Some(at)
        }) {
            return Some(at);
        }
        parsed
            .js_ts_site_declaration(at, name, &mut self.bindings)
            .map(|(_, declaration)| declaration)
    }

    fn candidate(
        &mut self,
        operand: Operand<'a, '_>,
        path: &AccessPath,
        line: usize,
        node: NodeIndex,
    ) -> Result<(), SourceRefusal> {
        let CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        } = &self.site.graph[node]
        else {
            return Err(SourceRefusal::Unconnected);
        };
        let parsed = self.site.caller;
        let actual = syntax::exact_node(parsed, *start_byte..*end_byte)
            .and_then(syntax::operand_root)
            .filter(|root| {
                syntax::is_value_read(parsed, *root) && parsed.node_text(root) == path.base
            });
        let represented = match actual {
            Some(root) if root == operand.root => self.check_read(operand, root),
            Some(root) => match self.check_read(operand, root) {
                Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
                result => {
                    let line_proof = self.check_line(operand, path, line, Some(root));
                    if line_proof == Err(SourceRefusal::Ambiguous) {
                        return line_proof;
                    }
                    result.and(line_proof)
                }
            },
            None => self.check_line(operand, path, line, None),
        };
        if represented == Err(SourceRefusal::Ambiguous) {
            return represented;
        }
        let connected = self.contributors(&self.site.call.caller, operand, path, node)?;
        represented.and(connected.then_some(()).ok_or(SourceRefusal::Unconnected))
    }

    // Def spans on the immutable graph can name entry formals, pattern locals
    // or a later write/projection. Every relevant contributor must agree.
    fn contributors(
        &mut self,
        owner: &FunctionId,
        operand: Operand<'a, '_>,
        path: &AccessPath,
        node: NodeIndex,
    ) -> Result<bool, SourceRefusal> {
        let parsed = self.site.caller;
        let mut proved = false;
        let mut unproved = false;
        for edge in self.site.graph.edges_directed(node, Direction::Incoming) {
            if !edge.weight().is_data_flow() {
                continue;
            }
            let CpgNode::Variable {
                file,
                function,
                function_start_line,
                path: def_path,
                start_byte,
                end_byte,
                access: VarAccess::Def,
                ..
            } = &self.site.graph[edge.source()]
            else {
                continue;
            };
            if file != &owner.file
                || function != &owner.name
                || *function_start_line != owner.start_line
                || def_path.base != path.base
                || !comparable(&def_path.fields, &path.fields)
            {
                continue;
            }
            let binding = syntax::exact_node(parsed, *start_byte..*end_byte)
                .and_then(def_root)
                .filter(|root| parsed.node_text(root) == path.base)
                .and_then(|root| self.binding(root, &path.base));
            match (operand.binding, binding) {
                (Some(expected), Some(got)) if expected != got => {
                    return Err(SourceRefusal::Ambiguous)
                }
                (Some(_), Some(_)) => proved = true,
                _ => unproved = true,
            }
        }
        Ok(proved && !unproved)
    }

    fn check_read(
        &mut self,
        operand: Operand<'a, '_>,
        read: Node<'a>,
    ) -> Result<(), SourceRefusal> {
        let parsed = self.site.caller;
        if syntax::execution_owner(parsed, read) != Some(operand.owner) {
            return Err(SourceRefusal::Ambiguous);
        }
        match (operand.binding, self.binding(read, &operand.path.base)) {
            (Some(expected), Some(got)) if expected != got => return Err(SourceRefusal::Ambiguous),
            (Some(_), Some(_)) => {}
            _ => return Err(SourceRefusal::Unconnected),
        }
        if self.intervening_write(operand, read) {
            return Err(SourceRefusal::Ambiguous);
        }
        Ok(())
    }

    fn check_line(
        &mut self,
        operand: Operand<'a, '_>,
        path: &AccessPath,
        line: usize,
        selected: Option<Node<'a>>,
    ) -> Result<(), SourceRefusal> {
        let parsed = self.site.caller;
        let mut candidates = Vec::new();
        syntax::walk(vec![operand.owner], |node| {
            let (start, end) = parsed.node_line_range(&node);
            if !(start..=end).contains(&line) {
                return false;
            }
            if start == line
                && syntax::is_value_read(parsed, node)
                && parsed.node_text(&node) == path.base
                && syntax::execution_owner(parsed, node) == Some(operand.owner)
            {
                let (_, fields, bracket) = syntax::receiver_chain(parsed, node);
                if path.fields.is_empty() || (!bracket && fields.starts_with(&path.fields)) {
                    candidates.push(node);
                }
            }
            true
        });
        let mut proved = selected.is_some() || !candidates.is_empty();
        for read in candidates {
            match self.check_read(operand, read) {
                Err(SourceRefusal::Ambiguous) => return Err(SourceRefusal::Ambiguous),
                Err(_) => proved = false,
                Ok(()) => {}
            }
        }
        if proved {
            Ok(())
        } else {
            Err(SourceRefusal::Unconnected)
        }
    }

    fn destructuring_store_can_precede(&mut self, operand: Operand<'a, '_>) -> bool {
        let mut current = operand.root;
        while let Some(parent) = current.parent().filter(|parent| *parent != operand.owner) {
            if parent.kind() == "assignment_expression"
                && matches!(current.kind(), "array_pattern" | "object_pattern")
                && parent.child_by_field_name("left") == Some(current)
                && self.pattern_store_can_precede(operand, current)
            {
                return true;
            }
            current = parent;
        }
        false
    }

    fn pattern_store_can_precede(&mut self, operand: Operand<'a, '_>, target: Node<'a>) -> bool {
        match target.kind() {
            "array_pattern" | "object_pattern" | "rest_pattern" => target
                .named_children(&mut target.walk())
                .filter(|child| !child.is_extra())
                .any(|child| self.pattern_store_can_precede(operand, child)),
            "pair_pattern" => target
                .child_by_field_name("value")
                .is_some_and(|value| self.pattern_store_can_precede(operand, value)),
            "assignment_pattern" | "object_assignment_pattern" => {
                // Every store in this subpattern follows evaluation of its own default.
                let in_default = target.child_by_field_name("right").is_some_and(|default| {
                    default.start_byte() <= operand.root.start_byte()
                        && operand.root.end_byte() <= default.end_byte()
                });
                !in_default
                    && target
                        .child_by_field_name("left")
                        .is_some_and(|left| self.pattern_store_can_precede(operand, left))
            }
            // Sibling targets are visited in source order. A member target
            // resolves its receiver and computed key before storing.
            _ => {
                target.end_byte() <= operand.root.start_byte()
                    && self.overlaps_write(operand, target)
            }
        }
    }

    fn intervening_write(&mut self, operand: Operand<'a, '_>, read: Node<'a>) -> bool {
        if read == operand.root {
            return false;
        }
        let parsed = self.site.caller;
        let earlier = read.start_byte().min(operand.root.start_byte());
        let later = read.start_byte().max(operand.root.start_byte());
        let mut conflict = false;
        syntax::walk(vec![operand.owner], |node| {
            if conflict || node.end_byte() <= earlier || node.start_byte() >= later {
                return false;
            }
            if let Some(target) = parsed.js_ts_write_target(node) {
                // An iteration target is written after evaluating the iterable,
                // before executing that iteration's body.
                let moment = if node.kind() == "for_in_statement" {
                    node.child_by_field_name("right")
                        .map_or(node.start_byte(), |right| right.end_byte())
                } else {
                    node.end_byte()
                };
                if (earlier < moment && moment <= later
                    || (target.start_byte() > earlier
                        && target.start_byte() < later
                        && moment > later
                        && node.kind() == "for_in_statement"))
                    && syntax::execution_owner(parsed, target) == Some(operand.owner)
                {
                    conflict = self.overlaps_write(operand, target);
                }
            }
            true
        });
        conflict
    }

    fn overlaps_write(&mut self, operand: Operand<'a, '_>, target: Node<'a>) -> bool {
        let parsed = self.site.caller;
        if let Some(root) = syntax::operand_root(target) {
            if parsed.node_text(&root) != operand.path.base {
                return false;
            }
            let path = ident(Tables::shared(), parsed.source.as_bytes(), target, true);
            let overlaps =
                match path {
                    Ident::Path(path) => path.comps.iter().zip(&operand.path.fields).all(
                        |(left, right)| match left {
                            Comp::Name(name) => name == right,
                            Comp::Index => true,
                        },
                    ),
                    Ident::NoWrite => false,
                    Ident::Refuse(_) => true,
                };
            return overlaps
                && self
                    .binding(root, &operand.path.base)
                    .is_none_or(|binding| Some(binding) == operand.binding);
        }
        parsed.recorded_write_paths(target).iter().any(|path| {
            path.base == operand.path.base && comparable(&path.fields, &operand.path.fields)
        })
    }

    fn enclosing(&mut self, operand: Operand<'a, '_>, lines: &[usize], base: &AccessPath) -> bool {
        let mut current = operand.owner;
        let parsed = self.site.caller;
        while let Some(outer) = syntax::execution_owner(parsed, current) {
            current = outer;
            let Some(id) = crate::anon_owner::identity(parsed, &outer, &self.site.call.caller.file)
            else {
                continue;
            };
            for path in [operand.path, base] {
                for &line in lines {
                    let Some(index) = self.lookup(&id, path, line) else {
                        continue;
                    };
                    let CpgNode::Variable {
                        start_byte,
                        end_byte,
                        ..
                    } = &self.site.graph[index]
                    else {
                        continue;
                    };
                    let root = syntax::exact_node(parsed, *start_byte..*end_byte)
                        .and_then(syntax::operand_root)
                        .filter(|root| {
                            syntax::is_value_read(parsed, *root)
                                && parsed.node_text(root) == path.base
                        });
                    if let Some(root) = root {
                        if operand.binding.is_some()
                            && self.binding(root, &path.base) == operand.binding
                        {
                            return true;
                        }
                    } else if self.contributors(&id, operand, path, index) == Ok(true) {
                        // The enclosing producer may represent the captured operand with a
                        // line anchor. Its local Def supplies the binding witness.
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn def_root(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "shorthand_property_identifier_pattern" => Some(node),
        "rest_pattern" => node.named_child(0).and_then(def_root),
        _ => syntax::operand_root(node),
    }
}

fn comparable(left: &[String], right: &[String]) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

/// Value-token lines in source order; type operands, comments and wrappers do not add lines.
fn operand_lines(node: Node<'_>) -> Vec<usize> {
    let mut lines = BTreeSet::new();
    syntax::walk(vec![node], |node| {
        if object_literal_args::is_transparent_wrapper(node.kind()) {
            if let Some(value) = props_binding::first_expression(node) {
                lines.extend(operand_lines(value));
            }
            return false;
        }
        if node.kind() == "member_expression" {
            let object = node.child_by_field_name("object");
            let property = node.child_by_field_name("property");
            for child in node.children(&mut node.walk()) {
                if Some(child) == object || Some(child) == property {
                    lines.extend(operand_lines(child));
                } else if matches!(child.kind(), "." | "optional_chain") {
                    lines.insert(child.start_position().row + 1);
                }
            }
            return false;
        }
        lines.insert(node.start_position().row + 1);
        false
    });
    lines.into_iter().collect()
}
