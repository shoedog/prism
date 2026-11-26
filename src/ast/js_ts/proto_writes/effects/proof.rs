//! Census evidence used to refuse identity and member-version claims.
use super::*;
use crate::ast::js_ts::proto_writes::operator;

const CENSUS_WORK_LIMIT: usize = 4_000_000;

#[derive(Default)]
pub(in super::super) struct Proof {
    pub(in super::super) eval_scopes: BTreeSet<usize>,
    pub(in super::super) eval_sites: BTreeSet<usize>,
    pub(in super::super) getters: BTreeSet<String>,
    pub(in super::super) opaque_calls: BTreeMap<usize, Arc<BTreeSet<AstSpan>>>,
    pub(in super::super) external_peer_owners: BTreeSet<usize>,
    pub(in super::super) external_origins: BTreeSet<usize>,
    pub(in super::super) external_stores: BTreeMap<usize, BTreeSet<AstSpan>>,
    pub(in super::super) work: usize,
    #[cfg(test)]
    pub(in super::super) rhs_nodes: usize,
}

#[derive(Default)]
pub(super) struct CensusProof<'a> {
    pub(super) evals: Vec<(Node<'a>, Node<'a>)>,
    calls: BTreeMap<usize, BTreeSet<AstSpan>>,
    external_bindings: BTreeSet<usize>,
}

impl<'a> Census<'a, '_> {
    pub(super) fn census_charge(&mut self, work: usize) -> bool {
        self.effects.proof.work = self.effects.proof.work.saturating_add(work);
        self.effects.refused_file |= self.effects.proof.work > CENSUS_WORK_LIMIT;
        !self.effects.refused_file
    }

    pub(super) fn getter(&mut self, node: Node<'a>) {
        if node.kind() == "method_definition" && has_token(node, "get") {
            let name = field(node, "name")
                .filter(|name| name.kind() == "property_identifier")
                .map_or("*", |name| {
                    self.lexical.canonical(self.file.node_text(&name))
                });
            self.effects.proof.getters.insert(name.to_owned());
        }
    }

    pub(super) fn note_external(&mut self, binding: usize, site: usize) {
        self.proof.external_bindings.insert(binding);
        self.effects.proof.external_origins.insert(site);
    }

    pub(super) fn external_pattern(&mut self, target: Node<'a>, pos: Position<'a>) {
        let mut pending = vec![target];
        while let Some(part) = pending.pop() {
            if !self.census_charge(1) {
                return;
            }
            if identifier(part) {
                if let Some(binding) =
                    flatten_lookup(self.lexical, pos.context.scope, self.file.node_text(&part))
                {
                    self.note_external(binding, part.start_byte());
                }
            } else {
                pending.extend(super::super::pattern_parts(part));
            }
        }
    }

    pub(super) fn receiver_roots(&mut self, expression: Node<'a>) -> Vec<Node<'a>> {
        self.roots(expression, false)
    }

    fn roots(&mut self, expression: Node<'a>, containers: bool) -> Vec<Node<'a>> {
        let mut pending = vec![expression];
        let mut roots = Vec::new();
        while let Some(source) = pending.pop().map(peel) {
            if !self.census_charge(1) {
                break;
            }
            #[cfg(test)]
            if !containers {
                self.effects.proof.rhs_nodes += 1;
            }
            if identifier(source) {
                roots.push(source);
                continue;
            }
            match source.kind() {
                "member_expression" | "subscript_expression" => {
                    pending.extend(field(source, "object"))
                }
                "ternary_expression" => pending.extend(
                    [field(source, "consequence"), field(source, "alternative")]
                        .into_iter()
                        .flatten(),
                ),
                "sequence_expression" => {
                    let siblings = source.named_child_count();
                    if !self.census_charge(siblings) {
                        break;
                    }
                    #[cfg(test)]
                    if !containers {
                        self.effects.proof.rhs_nodes += siblings;
                    }
                    pending.extend(children(source).last());
                }
                "assignment_expression" => pending.extend(field(source, "right")),
                "binary_expression" | "augmented_assignment_expression" => {
                    let operator = operator(source, self.file);
                    if matches!(operator, "||" | "??" | "||=" | "??=" | "&&=") {
                        pending.extend(field(source, "left"));
                    }
                    if matches!(operator, "||" | "??" | "&&" | "||=" | "??=" | "&&=") {
                        pending.extend(field(source, "right"));
                    }
                }
                "array" | "object" | "spread_element" | "pair" if containers => {
                    if source.kind() == "pair" {
                        pending.extend(field(source, "value"));
                    } else {
                        pending.extend(children(source));
                    }
                }
                _ => {}
            }
        }
        roots
    }

    pub(super) fn opaque_arguments(&mut self, call: Node<'a>, pos: Position<'a>) {
        let effect = call_effect(self.file, self.lexical, call, pos.context);
        // Reflection can invoke accessors or proxy traps. Its classification
        // as a read by the transfer does not prove its arguments unmodified.
        if effect.pure {
            return;
        }
        let Some(args) = field(call, "arguments").filter(|args| args.kind() == "arguments") else {
            return;
        };
        for arg in children(args) {
            // Bare arguments of opaque calls were already handled by transfer.
            // Record the other value forms separately for refusal accounting.
            if identifier(peel(arg)) && effect.unknown {
                continue;
            }
            for base in self.roots(arg, true) {
                if let Some(binding) =
                    flatten_lookup(self.lexical, pos.context.scope, self.file.node_text(&base))
                {
                    self.proof
                        .calls
                        .entry(binding)
                        .or_default()
                        .insert(call.into());
                }
            }
        }
    }

    pub(super) fn finish_proof(&mut self) {
        for (call, scope) in self.proof.evals.drain(..) {
            let function = self.lexical.binding(scope, "eval").is_some_and(|binding| {
                self.lexical.eval_functions.contains(&binding.start_byte)
                    && !self.lexical.escaped_names.contains("eval")
                    && self
                        .effects
                        .known
                        .get(&binding.start_byte)
                        .is_some_and(|writes| writes.len() == 1)
            });
            if function {
                continue;
            }
            self.effects.proof.eval_sites.insert(call.start_byte());
            let mut scope = Some(scope.id());
            while let Some(id) = scope {
                if !self.effects.proof.eval_scopes.insert(id) {
                    break;
                }
                scope = self.lexical.scopes[&id].parent;
            }
        }

        let mut external_owners = BTreeSet::new();
        for &binding in &self.proof.external_bindings {
            let owner = self.lexical.binding_owner_ids[&binding];
            if !external_owners.insert(owner) {
                self.effects.proof.external_peer_owners.insert(owner);
            }
        }

        // Traverse each explicit alias component at most once. One shared set
        // describes all opaque argument sites for that component.
        let mut visited = BTreeSet::new();
        let seeds: Vec<_> = self
            .proof
            .calls
            .keys()
            .chain(&self.proof.external_bindings)
            .copied()
            .collect();
        for seed in seeds {
            if !visited.insert(seed) {
                continue;
            }
            let mut pending = vec![seed];
            let mut members = Vec::new();
            let mut calls = BTreeSet::new();
            let mut external = false;
            while let Some(binding) = pending.pop() {
                members.push(binding);
                external |= self.proof.external_bindings.contains(&binding);
                calls.extend(
                    self.proof
                        .calls
                        .get(&binding)
                        .into_iter()
                        .flatten()
                        .cloned(),
                );
                pending.extend(
                    self.aliases
                        .get(&binding)
                        .into_iter()
                        .flatten()
                        .copied()
                        .filter(|alias| visited.insert(*alias)),
                );
            }
            let calls = Arc::new(calls);
            for binding in members {
                if external {
                    self.proof.external_bindings.insert(binding);
                }
                if !calls.is_empty() {
                    self.effects
                        .proof
                        .opaque_calls
                        .insert(binding, calls.clone());
                }
            }
        }

        for (&binding, calls) in &self.proof.calls {
            if self.proof.external_bindings.contains(&binding) {
                self.effects
                    .proof
                    .external_stores
                    .entry(self.lexical.binding_owner_ids[&binding])
                    .or_default()
                    .extend(calls.iter().cloned());
            }
        }
        for write in self.effects.writes.values().filter(|write| write.member) {
            if self.proof.external_bindings.contains(&write.binding) {
                self.effects
                    .proof
                    .external_stores
                    .entry(self.lexical.binding_owner_ids[&write.binding])
                    .or_default()
                    .insert(write.site.clone());
            }
        }
    }
}
