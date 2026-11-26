use super::{ComparisonScope, DiffBasis, DiffError, DiffSide, ProducerInventory};
use crate::element::{CanonicalPath, PathToken};
use crate::SchemaIndex;

pub(super) struct BoundSide<'a> {
    pub basis: DiffBasis<'a>,
    pub schema: Option<&'a SchemaIndex>,
    pub producer: Option<&'a ProducerInventory<'a>>,
}

impl<'a> BoundSide<'a> {
    pub fn admit(side: &DiffSide<'a>) -> Result<Self, DiffError> {
        let schema = side.schema.filter(|index| index.source.is_some());
        let schema_basis = schema
            .and_then(|index| index.source.as_ref())
            .map(|source| DiffBasis {
                api: &source.api,
                authority: &source.authority,
                tree: &source.tree,
                scope: ComparisonScope::Api,
            });
        let producer_basis = side.producer.map(|inventory| &inventory.basis);
        let basis = side
            .basis
            .as_ref()
            .or(producer_basis)
            .or(schema_basis.as_ref())
            .filter(|base| {
                !base.api.is_empty() && !base.authority.is_empty() && !base.tree.is_empty()
            })
            .ok_or(DiffError::NoCommonScope)?;
        for source in schema_basis.iter().chain(producer_basis) {
            if (source.api, source.authority, source.tree)
                != (basis.api, basis.authority, basis.tree)
            {
                return Err(DiffError::ContradictorySlot);
            }
        }
        Ok(Self {
            basis: basis.clone(),
            schema,
            producer: side.producer,
        })
    }
}

pub(super) fn common_scope(
    before: &DiffBasis<'_>,
    after: &DiffBasis<'_>,
) -> Result<ComparisonScope, DiffError> {
    if before.api != after.api {
        return Err(DiffError::IncompatibleApi);
    }
    if before.authority != after.authority {
        return Err(DiffError::IncompatibleAuthority);
    }
    match (&before.scope, &after.scope) {
        (ComparisonScope::Api, scope) | (scope, ComparisonScope::Api) => Ok(scope.clone()),
        (ComparisonScope::PathPrefix(b), ComparisonScope::PathPrefix(a)) => {
            if under(b, a) {
                Ok(before.scope.clone())
            } else if under(a, b) {
                Ok(after.scope.clone())
            } else {
                Err(DiffError::NoCommonScope)
            }
        }
    }
}

pub(super) fn in_scope(path: &CanonicalPath, scope: &ComparisonScope) -> bool {
    match scope {
        ComparisonScope::Api => true,
        ComparisonScope::PathPrefix(prefix) => under(path, prefix),
    }
}

pub(super) fn same_path(before: &CanonicalPath, after: &CanonicalPath) -> bool {
    before.segments.len() == after.segments.len() && starts_with(before, after)
}

fn under(path: &CanonicalPath, prefix: &CanonicalPath) -> bool {
    prefix.segments.as_slice() == [Vec::new()] || starts_with(path, prefix)
}

fn starts_with(path: &CanonicalPath, prefix: &CanonicalPath) -> bool {
    path.segments.len() >= prefix.segments.len()
        && path
            .segments
            .iter()
            .zip(&prefix.segments)
            .all(|(path, prefix)| {
                path.len() == prefix.len()
                    && path
                        .iter()
                        .zip(prefix)
                        .all(|(path, prefix)| match (path, prefix) {
                            (PathToken::Literal(path), PathToken::Literal(prefix)) => {
                                path == prefix
                            }
                            (PathToken::Capture(path), PathToken::Capture(prefix)) => {
                                path.width == prefix.width
                                    && path.optional_separator == prefix.optional_separator
                            }
                            _ => false,
                        })
            })
}
