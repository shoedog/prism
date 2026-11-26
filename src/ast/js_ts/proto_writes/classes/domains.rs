//! Positive key-domain proofs. An unsupported expression leaves its selector live.
use super::{proofs::SHARED_LINKS, *};

mod collections;
mod type_tests;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Domain {
    Numeric,
    String,
    Benign,
}

impl<'a> Classifier<'a, '_> {
    pub(super) fn numeric_key(&self, node: Node<'a>) -> bool {
        matches!(self.key_domain(node, 0), Some(Domain::Numeric))
    }

    pub(super) fn benign_key(&self, node: Node<'a>) -> bool {
        self.key_domain(node, 0).is_some()
    }

    fn key_domain(&self, node: Node<'a>, depth: usize) -> Option<Domain> {
        if depth >= 32 || !self.proof_charge(1) {
            return None;
        }
        let node = peel(node);
        let next = |value| self.key_domain(value, depth + 1);
        match node.kind() {
            "number" => Some(Domain::Numeric),
            "true" | "false" | "null" => Some(Domain::Benign),
            "string" if !SHARED_LINKS.contains(&self.plain_string(node)?) => Some(Domain::String),
            "template_string" if self.anchored_template(node) => Some(Domain::String),
            "identifier" => self.identifier_domain(node, depth + 1),
            "binary_expression" => {
                let left = field(node, "left")?;
                let right = field(node, "right")?;
                match operator(node, self.file) {
                    "-" | "*" | "/" | "%" | "**" | "|" | "&" | "^" | "<<" | ">>" | ">>>" => {
                        Some(Domain::Numeric)
                    }
                    "+" if next(left) == Some(Domain::Numeric)
                        && next(right) == Some(Domain::Numeric) =>
                    {
                        Some(Domain::Numeric)
                    }
                    "+" if self.anchored_addition(left, right) => Some(Domain::Benign),
                    "==" | "!=" | "===" | "!==" | "<" | ">" | "<=" | ">=" | "in" | "instanceof" => {
                        Some(Domain::Benign)
                    }
                    "||" | "??" | "&&" => Some(join(next(left)?, next(right)?)),
                    _ => None,
                }
            }
            "unary_expression" => match operator(node, self.file) {
                "+" | "-" | "~" => Some(Domain::Numeric),
                "!" | "typeof" | "void" | "delete" => Some(Domain::Benign),
                _ => None,
            },
            "update_expression" => Some(Domain::Numeric),
            "ternary_expression" => Some(join(
                next(field(node, "consequence")?)?,
                next(field(node, "alternative")?)?,
            )),
            "sequence_expression" => {
                let mut tail = None;
                for child in children(node) {
                    if !self.proof_charge(1) {
                        return None;
                    }
                    tail = Some(child);
                }
                next(tail?)
            }
            "assignment_expression" => next(field(node, "right")?),
            "member_expression" | "subscript_expression" => self.member_domain(node, depth + 1),
            _ => None,
        }
    }

    fn identifier_domain(&self, node: Node<'a>, depth: usize) -> Option<Domain> {
        let read = self.analysis.read(node)?;
        let binding = read.binding.as_ref()?;
        if self.script_global_binding(binding) {
            return None;
        }
        if !read.resolution_complete {
            return self
                .numeric_counter(node, binding, depth)
                .then_some(Domain::Numeric);
        }
        if self.numeric_type_test_dominates(node) {
            return Some(Domain::Numeric);
        }
        let mut result = None;
        for site in &read.reaching_versions {
            if !self.proof_charge(1) || self.analysis.defaults.contains_key(&site.start_byte) {
                return None;
            }
            let domain = match self.origin(site) {
                Origin::Value(value) => self.key_domain(value, depth),
                Origin::Unknown => self.numeric_store(site, depth).then_some(Domain::Numeric),
                _ => None,
            }?;
            result = Some(result.map_or(domain, |prior| join(prior, domain)));
        }
        result
    }

    fn numeric_store(&self, site: &AstSpan, depth: usize) -> bool {
        let Some(store) = self
            .node_at(site)
            .and_then(|node| self.expression_use(node))
        else {
            return false;
        };
        match store.kind() {
            "update_expression" => true,
            "augmented_assignment_expression" if operator(store, self.file) == "+=" => {
                field(store, "right")
                    .is_some_and(|rhs| self.key_domain(rhs, depth) == Some(Domain::Numeric))
                    && field(store, "left")
                        .is_some_and(|lhs| self.key_domain(lhs, depth + 1) == Some(Domain::Numeric))
            }
            "augmented_assignment_expression" => matches!(
                operator(store, self.file),
                "-=" | "*=" | "/=" | "%=" | "**=" | "|=" | "&=" | "^=" | "<<=" | ">>=" | ">>>="
            ),
            _ => false,
        }
    }

    fn numeric_counter(&self, node: Node<'a>, binding: &AstSpan, depth: usize) -> bool {
        if depth >= 32 || !self.proof_charge(1) {
            return false;
        }
        let Some(writes) = self.analysis.proof_writes(binding) else {
            return false;
        };
        if self.numeric_declaration_dominates(node, binding, depth)
            && writes
                .iter()
                .all(|site| site == binding || self.preserves_numeric(site, depth))
        {
            return true;
        }
        let owner_kind = |kind: &str| kind == "program" || crate::ast::is_js_ts_function_like(kind);
        let Some(owner) = self.enclosing(node, owner_kind) else {
            return false;
        };
        for site in writes {
            if !self.proof_charge(1) {
                return false;
            }
            let Some(store) = self
                .node_at(site)
                .and_then(|node| self.expression_use(node))
            else {
                continue;
            };
            let Some(loop_node) = self.enclosing(store, |kind| {
                kind == "for_statement" || crate::ast::is_js_ts_function_like(kind)
            }) else {
                continue;
            };
            if loop_node.start_byte() > node.start_byte()
                || loop_node.end_byte() < node.end_byte()
                || self.enclosing(loop_node, owner_kind) != Some(owner)
            {
                continue;
            }
            if self.counter_loop(node, loop_node, binding, writes, depth) {
                return true;
            }
        }
        false
    }

    fn counter_loop(
        &self,
        node: Node<'a>,
        loop_node: Node<'a>,
        binding: &AstSpan,
        writes: &BTreeSet<AstSpan>,
        depth: usize,
    ) -> bool {
        let Some(init) = field(loop_node, "initializer").map(peel) else {
            return false;
        };
        let site = if matches!(init.kind(), "lexical_declaration" | "variable_declaration") {
            let mut decls = children(init);
            let Some(decl) = decls.next().filter(|_| decls.next().is_none()) else {
                return false;
            };
            field(decl, "name").zip(field(decl, "value"))
        } else if init.kind() == "assignment_expression" {
            field(init, "left").zip(field(init, "right"))
        } else {
            None
        };
        let Some((target, initial)) = site else {
            return false;
        };
        if !identifier(target)
            || self.binding_of(target).as_ref() != Some(binding)
            || node.start_byte() < init.end_byte()
            || self.key_domain(initial, depth + 1) != Some(Domain::Numeric)
        {
            return false;
        }
        writes.iter().all(|site| {
            if !self.proof_charge(1) {
                return false;
            }
            if site.start_byte < init.start_byte()
                || site.start_byte == target.start_byte()
                || site.start_byte >= loop_node.end_byte()
            {
                return true;
            }
            self.preserves_numeric(site, depth)
        })
    }

    fn numeric_declaration_dominates(
        &self,
        mut node: Node<'a>,
        binding: &AstSpan,
        depth: usize,
    ) -> bool {
        let Some(declaration) = self.node_at(binding).and_then(|name| name.parent()) else {
            return false;
        };
        if declaration.kind() != "variable_declarator" {
            return false;
        }
        let Some(statement) = declaration.parent() else {
            return false;
        };
        let Some(block) = statement
            .parent()
            .filter(|parent| parent.kind() == "statement_block")
        else {
            return false;
        };
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return false;
            }
            let Some(parent) = node.parent() else {
                return false;
            };
            if is_owner(parent) {
                return false;
            }
            if parent == block {
                return statement.end_byte() <= node.start_byte()
                    && field(declaration, "value").is_some_and(|initial| {
                        self.key_domain(initial, depth + 1) == Some(Domain::Numeric)
                    });
            }
            node = parent;
        }
        false
    }

    fn preserves_numeric(&self, site: &AstSpan, depth: usize) -> bool {
        let Some(store) = self
            .node_at(site)
            .and_then(|node| self.expression_use(node))
        else {
            return false;
        };
        match store.kind() {
            "update_expression" => true,
            "assignment_expression" | "augmented_assignment_expression" => {
                (store.kind() == "assignment_expression" || operator(store, self.file) == "+=")
                    && field(store, "right")
                        .is_some_and(|rhs| self.key_domain(rhs, depth + 1) == Some(Domain::Numeric))
            }
            _ => false,
        }
    }

    fn anchored_template(&self, node: Node<'a>) -> bool {
        let raw = self.text(node);
        let Some(body) = raw.get(1..raw.len().saturating_sub(1)) else {
            return false;
        };
        // Escapes require cooking, including continuations and escaped `${`.
        if !self.proof_charge(body.len()) || body.contains('\\') {
            return false;
        }
        let mut first = None;
        let mut last = None;
        for part in children(node) {
            if !self.proof_charge(1) {
                return false;
            }
            if part.kind() == "template_substitution" {
                first.get_or_insert(part.start_byte());
                last = Some(part.end_byte());
            }
        }
        match (first, last) {
            (Some(first), Some(last)) => incompatible(
                &raw[1..first - node.start_byte()],
                &raw[last - node.start_byte()..raw.len() - 1],
            ),
            _ => !SHARED_LINKS.contains(&body),
        }
    }

    fn anchored_addition(&self, left: Node<'a>, right: Node<'a>) -> bool {
        let decimal = |node| {
            let node = peel(node);
            let raw = self.text(node);
            (node.kind() == "number" && raw.len() == 1 && raw.as_bytes()[0].is_ascii_digit())
                .then_some(raw)
        };
        incompatible(
            self.plain_string(left)
                .or_else(|| decimal(left))
                .unwrap_or(""),
            self.plain_string(right)
                .or_else(|| decimal(right))
                .unwrap_or(""),
        )
    }
}

fn join(left: Domain, right: Domain) -> Domain {
    if left == right {
        left
    } else {
        Domain::Benign
    }
}

fn incompatible(prefix: &str, suffix: &str) -> bool {
    SHARED_LINKS
        .iter()
        .all(|name| !name.starts_with(prefix) || !name.ends_with(suffix))
}
