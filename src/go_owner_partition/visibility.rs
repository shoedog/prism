//! Clone-free caller-to-declaration build/test visibility.
use crate::go_build_profile::{self, GoBuildProfile};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

pub(super) fn uncached(
    caller_file: &str,
    defining_file: &str,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> (bool, bool) {
    let (Some(caller), Some(defining)) = (profiles.get(caller_file), profiles.get(defining_file))
    else {
        return (true, false);
    };
    let owns_test_namespace = crate::resolution::dir_of(caller_file)
        == crate::resolution::dir_of(defining_file)
        && caller.package_clause == defining.package_clause;
    if defining.is_test_file && !owns_test_namespace {
        return (false, true);
    }
    let visibility =
        go_build_profile::go_visible_with_package(caller, defining, &defining.package_clause);
    let exact = visibility.visible
        && go_build_profile::profile_allows_exact(Some(caller))
        && go_build_profile::visibility_allows_exact(Some(defining), &visibility);
    (visibility.visible, exact)
}
