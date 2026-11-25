//! Completeness checks whose absence of proof preserves candidate stores.
use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MemberHistory {
    pub(super) members: Option<usize>,
    pub(super) calls: Option<usize>,
    pub(super) aliases: Option<(usize, usize)>,
}

impl MemberHistory {
    pub(super) fn merge(&mut self, other: Self) {
        self.members = self.members.max(other.members);
        self.calls = self.calls.max(other.calls);
        self.aliases = self.aliases.max(other.aliases);
    }
}

impl<'a> Analysis<'a> {
    pub(super) fn binding_refusal(&self, base: &str, context: Context<'a>) -> Option<&'static str> {
        let spelling = self.lexical.canonical(base);
        if self.lexical.escaped_names.contains(spelling) {
            return Some("identity_escape");
        }
        if self.lexical.implicit_arguments(context.scope, base) {
            return Some("implicit_arguments");
        }
        let binding = self.lexical.binding(context.scope, base)?;
        self.effects
            .proof
            .eval_scopes
            .contains(&self.lexical.binding_scopes[&binding.start_byte])
            .then_some("direct_eval")
    }

    pub(super) fn refuse_read(
        &mut self,
        node: Node<'a>,
        path: &str,
        base: &str,
        context: Context<'a>,
        version: &mut Version,
    ) -> (Option<&'static str>, MemberHistory) {
        let mut history = MemberHistory::default();
        let binding_reason = self.binding_refusal(base, context);
        let mut reason = binding_reason
            .or(version.input_reason)
            .or(version.unproven_flow.then_some("control_flow_unproven"));
        let binding = self.lexical.binding(context.scope, base);
        if binding_reason.is_some() {
            self.extend_binding_stores(context, binding.as_ref(), &mut version.sites);
        }
        if path == base || self.skipped(context.owner) {
            return (reason, history);
        }
        let Some(binding) = binding else {
            return (reason, history);
        };
        let id = binding.start_byte;
        let owner = self.lexical.binding_owner_ids[&id];
        let deferred = self.deferred_initializer.is_some()
            || !self.effects.runs_with(context.owner.id(), owner);
        if deferred {
            reason = reason.or(Some("member_deferred"));
        }
        // An opaque call can only affect its argument after evaluation.
        // A loop or deferred evaluation can revisit an earlier source span.
        let limit = if deferred {
            usize::MAX
        } else {
            self.member_effect_limit(node)
        };
        if self
            .effects
            .proof
            .opaque_calls
            .get(&id)
            .is_some_and(|sites| {
                sites
                    .iter()
                    .next()
                    .is_some_and(|call| call.end_byte <= limit)
            })
        {
            reason = reason.or(Some("member_call"));
            history.calls = Some(limit);
        }
        if self.possible_accessor(node, path, base, version) {
            reason = reason.or(Some("member_accessor"));
        }
        let external = version
            .objects
            .iter()
            .any(|object| self.effects.proof.external_origins.contains(object));
        if external && self.effects.proof.external_peer_owners.contains(&owner) {
            reason = reason.or(Some("member_alias"));
            history.aliases = Some((owner, limit));
        }
        if reason.is_some() {
            history.members = Some(limit);
        }
        (reason, history)
    }

    fn member_effect_limit(&self, node: Node<'a>) -> usize {
        let mut limit = node.start_byte();
        let mut parent = node.parent();
        for _ in 0..128 {
            let Some(ancestor) = parent else { return limit };
            if is_loop(ancestor) {
                limit = ancestor.end_byte();
            }
            if is_owner(ancestor) {
                return if self.inline_owners.contains_key(&AstSpan::from(ancestor)) {
                    usize::MAX
                } else {
                    limit
                };
            }
            parent = ancestor.parent();
        }
        usize::MAX
    }

    pub(super) fn extend_binding_stores(
        &mut self,
        context: Context<'a>,
        binding: Option<&AstSpan>,
        into: &mut BTreeSet<AstSpan>,
    ) {
        let Some(binding) = binding else { return };
        if self.skipped(context.owner) {
            return;
        }
        let id = binding.start_byte;
        let count = self.effects.known.get(&id).map_or(0, BTreeSet::len);
        if count > 0 && self.charge(context.owner, count) {
            into.extend(self.effects.known[&id].iter().cloned());
        }
    }

    fn possible_accessor(&self, node: Node<'a>, path: &str, base: &str, version: &Version) -> bool {
        let getters = &self.effects.proof.getters;
        if getters.is_empty() {
            return false;
        }
        let segments = path.get(base.len() + 1..).unwrap_or_default();
        if !getters.contains("*")
            && !segments
                .split('.')
                .any(|key| key == "[]" || getters.contains(key))
        {
            return false;
        }
        if segments.contains('.')
            || segments == "[]"
            || version.objects.len() != 1
            || !field(node, "object").is_some_and(|object| identifier(peel(object)))
        {
            return true;
        }
        // An own literal data property shadows any inherited accessor.
        let Some(Origin::Value(value)) = version
            .objects
            .first()
            .and_then(|object| self.origins.get(object))
        else {
            return true;
        };
        let object = peel(*value);
        if object.kind() != "object" || object.named_child_count() > 32 {
            return true;
        }
        let entries: Vec<_> = children(object).collect();
        for entry in entries.into_iter().rev() {
            if entry.kind() != "pair" {
                return true;
            }
            let Some(key) = field(entry, "key") else {
                return true;
            };
            let name = match key.kind() {
                "property_identifier" => self.file.node_text(&key),
                "string" | "number" => match super::super::literal_index(self.file, key) {
                    Some(name) => name,
                    None => return true,
                },
                _ => return true,
            };
            if name == segments {
                return false;
            }
        }
        true
    }
}
