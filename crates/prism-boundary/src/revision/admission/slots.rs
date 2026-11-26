use super::*;

impl Scope {
    pub(crate) fn contains_prefix(&self, path: &[Vec<crate::element::PathToken>]) -> bool {
        match self {
            Self::Api => true,
            Self::PathPrefix { segments } => path.starts_with(segments),
        }
    }

    pub(crate) fn intersect(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Api, scope) | (scope, Self::Api) => Some(scope.clone()),
            (Self::PathPrefix { segments: a }, Self::PathPrefix { segments: b }) => {
                if a.starts_with(b) {
                    Some(self.clone())
                } else if b.starts_with(a) {
                    Some(other.clone())
                } else {
                    None
                }
            }
        }
    }
}

pub(super) fn compare_slot(expected: &InputSlot, actual: &InputSlot) -> Result<(), Refusal> {
    let id = &actual.input_id;
    for (dimension, left, right) in [
        (
            "input_id",
            Some(expected.input_id.as_str()),
            Some(actual.input_id.as_str()),
        ),
        (
            "api",
            Some(expected.api.as_str()),
            Some(actual.api.as_str()),
        ),
        (
            "repo",
            Some(expected.repo.as_str()),
            Some(actual.repo.as_str()),
        ),
        (
            "authority",
            Some(expected.authority.as_str()),
            Some(actual.authority.as_str()),
        ),
        ("rev", expected.rev.as_deref(), actual.rev.as_deref()),
        (
            "tree",
            Some(expected.tree.as_str()),
            Some(actual.tree.as_str()),
        ),
        (
            "branch",
            expected.branch.as_deref(),
            actual.branch.as_deref(),
        ),
    ] {
        if left != right {
            return Err(Refusal::mismatch(id, dimension, left, right));
        }
    }
    if expected.role != actual.role {
        return Err(Refusal::new("slot_mismatch", id).dimension("role"));
    }
    if expected.service_side.is_some()
        && actual.service_side.is_some()
        && expected.service_side != actual.service_side
    {
        return Err(Refusal::new("slot_mismatch", id).dimension("service_side"));
    }
    if expected.config != actual.config {
        let mut dimension = "config".to_string();
        if let (Some(expected), Some(actual)) = (&expected.config, &actual.config) {
            let expected = serde_json::to_value(expected).expect("serializable config");
            let actual = serde_json::to_value(actual).expect("serializable config");
            if let Some((key, _)) = expected
                .as_object()
                .into_iter()
                .flatten()
                .find(|(key, value)| actual.get(key.as_str()) != Some(value))
            {
                dimension = format!("config.{key}");
            }
        }
        return Err(Refusal::new("slot_mismatch", id).dimension(&dimension));
    }
    Ok(())
}

pub(super) fn project_scope(
    slot: &InputSlot,
    input: &InputSlot,
    projection: Option<&ScopeProjection>,
) -> Result<(Scope, Option<ScopeRemainder>), Refusal> {
    let fail = || Refusal::new("slot_mismatch", &input.input_id).dimension("scope");
    let Some(projection) = projection else {
        return (slot.scope == input.scope)
            .then(|| (slot.scope.clone(), None))
            .ok_or_else(fail);
    };
    let scope = slot
        .scope
        .intersect(&input.scope)
        .and_then(|scope| scope.intersect(&projection.requested))
        .ok_or_else(fail)?;
    let remainder = (scope != projection.requested).then(|| ScopeRemainder {
        requested: projection.requested.clone(),
        excluded: scope.clone(),
    });
    Ok((scope, remainder))
}

pub(super) fn validate_input(
    request: &AdmissionRequest,
    input: &AdmissionInput,
) -> Result<(), Refusal> {
    let binding = &input.binding;
    let id = &binding.input_id;
    let (tree, rev) = input
        .snapshot
        .identity()
        .ok_or_else(|| Refusal::new("unsupported_kind", id))?;
    let slot = unique(
        request
            .slots
            .iter()
            .filter(|slot| slot.slot == binding.slot),
        id,
        "missing_slot",
    )?;
    compare_slot(slot, binding)?;
    if tree != binding.tree || tree.is_empty() {
        return Err(Refusal::mismatch(
            id,
            "tree",
            Some(&binding.tree),
            Some(tree),
        ));
    }
    if rev != binding.rev.as_deref() || rev == Some("") {
        return Err(Refusal::mismatch(id, "rev", binding.rev.as_deref(), rev));
    }
    if slot.scope.intersect(&binding.scope).is_none() {
        return Err(Refusal::new("slot_mismatch", id).dimension("scope"));
    }
    let supported = matches!(
        (binding.role, binding.authority.as_str()),
        (
            Role::Producer,
            "schema" | "route_table" | "registration" | "annotation"
        ) | (Role::Consumer, "calls")
    );
    if !supported {
        return Err(Refusal::new("unsupported_kind", id).dimension("authority"));
    }
    Ok(())
}

pub(super) fn unique<'a, T: PartialEq + 'a>(
    mut records: impl Iterator<Item = &'a T>,
    id: &str,
    missing: &str,
) -> Result<&'a T, Refusal> {
    let first = records.next().ok_or_else(|| Refusal::new(missing, id))?;
    if records.any(|record| record != first) {
        return Err(Refusal::new("receipt_conflict", id));
    }
    Ok(first)
}
