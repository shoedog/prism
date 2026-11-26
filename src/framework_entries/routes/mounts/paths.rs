use super::*;

pub(super) fn suffix(path: &mut Exposure, reason: Reason) {
    if matches!(path.status, ExposureStatus::Full { .. }) {
        cut_path(path, reason);
    }
}

pub(super) fn cut_path(path: &mut Exposure, reason: Reason) {
    path.status = match path.status.value() {
        Some(value) => ExposureStatus::Suffix {
            value: value.into(),
            reason,
        },
        None => ExposureStatus::Dynamic { reason },
    };
}

pub(super) fn apply_guards(mount: &Mount, path: &mut Exposure) -> bool {
    if let Some(guard) = &mount.setup_guard {
        path.setup_guard = Some(match path.setup_guard.take() {
            Some(child) if &child != guard => format!("({guard}) && ({child})"),
            _ => guard.clone(),
        });
    }
    match (&path.method_guard, &mount.method_guard) {
        (_, None | Some(MethodSet::Any(_))) => {}
        (MethodSet::Any(_), Some(guard)) => path.method_guard = guard.clone(),
        (MethodSet::Known { known }, Some(MethodSet::Known { known: guard })) => {
            let joined: BTreeSet<_> = known.intersection(guard).cloned().collect();
            if joined.is_empty() && !known.is_empty() {
                return false;
            }
            path.method_guard = MethodSet::Known { known: joined };
        }
        (child, Some(parent)) if child != parent => {
            let text = |methods: &MethodSet| match methods {
                MethodSet::Known { known } => known.iter().cloned().collect::<Vec<_>>().join(", "),
                MethodSet::Unknown { text, .. } => text.clone(),
                MethodSet::Any(_) => unreachable!(),
            };
            path.method_guard =
                MethodSet::unknown(format!("({}) && ({})", text(parent), text(child)));
        }
        _ => {}
    }
    true
}

pub(super) fn tokenize<K>(path: &mut Exposure, relation: &impl Relation<K>) {
    path.path_tokens = path
        .status
        .value()
        .and_then(|value| relation.path_tokens(value))
        .map(|tokens| {
            let mut joined = Vec::new();
            for token in tokens {
                match (joined.last_mut(), token) {
                    (Some(PathToken::Literal(left)), PathToken::Literal(right)) => {
                        left.push_str(&right)
                    }
                    (_, token) => joined.push(token),
                }
            }
            joined
        });
    path.normalized = path.path_tokens.as_ref().map(|tokens| {
        tokens
            .iter()
            .map(|token| match token {
                PathToken::Literal(text) => text.as_str(),
                PathToken::Parameter => "{}",
            })
            .collect()
    });
}

/// Preserve the known upper bound on eligible methods across unknown guards.
pub(super) fn eligible(methods: &MethodSet, guard: Option<&MethodSet>) -> Option<MethodSet> {
    match (methods, guard) {
        (MethodSet::Known { known }, Some(MethodSet::Known { known: guard })) => {
            let joined: BTreeSet<_> = known.intersection(guard).cloned().collect();
            (!joined.is_empty() || known.is_empty()).then_some(MethodSet::Known { known: joined })
        }
        (MethodSet::Any(_) | MethodSet::Unknown { .. }, Some(guard @ MethodSet::Known { .. })) => {
            Some(guard.clone())
        }
        _ => Some(methods.clone()),
    }
}
