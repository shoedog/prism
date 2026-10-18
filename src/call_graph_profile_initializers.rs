//! Keep whole-program initializer work out of the struct-move interval.
use super::*;

pub(super) struct Initializers {
    pub(super) props: crate::js_ts_props::ImportedPropsProofs,
    pub(super) scope: Option<ScopeGraph>,
    pub(super) proofs: crate::js_anonymous::ProofIndex,
    pub(super) macro_shadow: BTreeSet<String>,
}

pub(super) fn collect(
    files: &BTreeMap<String, ParsedFile>,
    scope_inputs: Option<&ScopeGraphBuildInputs>,
    macro_shadow: &BTreeSet<String>,
    profile: &mut crate::profile::Stages,
) -> Initializers {
    // Preserve the literal's evaluation order. Each builder reads shared inputs
    // and owns its output; intervening empty collections/defaults have no effects
    // visible to these builders. Scope construction also emits its nested row.
    profile.next("call_graph.js_ts_props");
    let props = crate::js_ts_props::collect(files);
    profile.next("call_graph.populate_scope_graph");
    let scope = CallGraph::populate_scope_graph(files, scope_inputs);
    profile.next("call_graph.js_anonymous_proofs");
    let proofs = crate::js_anonymous::ProofIndex::build(files);
    profile.next("call_graph.rust_macro_shadow_intersection");
    let macro_shadow = crate::rust_macro_args::transparent_shadow_intersection(macro_shadow);
    profile.next("call_graph.struct_init");
    Initializers {
        props,
        scope,
        proofs,
        macro_shadow,
    }
}
