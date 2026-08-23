use super::{
    collect_profile_tags, is_goarch, is_goos, profile_satisfied, GoBuildProfile, GOARCH, GOOS,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoBuildImplication {
    Proven,
    Disproven,
    Uncertain,
}

/// Prove `consumer => declaration` by looking for a concrete build satisfying
/// `consumer && !declaration`. The shared custom-tag cap keeps this exactness
/// boundary aligned with ordinary build-profile visibility.
pub(crate) fn build_profile_implies(
    consumer: &GoBuildProfile,
    declaration: &GoBuildProfile,
) -> GoBuildImplication {
    if consumer.build_unparsed || declaration.build_unparsed {
        return GoBuildImplication::Uncertain;
    }
    let mut tags = BTreeSet::new();
    collect_profile_tags(consumer, &mut tags);
    collect_profile_tags(declaration, &mut tags);
    let free = tags
        .into_iter()
        .filter(|tag| !is_goos(tag) && !is_goarch(tag) && tag != "unix")
        .collect::<Vec<_>>();
    if free.len() > 8 {
        return GoBuildImplication::Uncertain;
    }
    for goos in GOOS {
        for goarch in GOARCH {
            for mask in 0..(1usize << free.len()) {
                if profile_satisfied(consumer, goos, goarch, &free, mask)
                    && !profile_satisfied(declaration, goos, goarch, &free, mask)
                {
                    return GoBuildImplication::Disproven;
                }
            }
        }
    }
    GoBuildImplication::Proven
}
