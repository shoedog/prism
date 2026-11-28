use super::ResolutionKind::*;

/// The `as_str` spelling is the wire name call-stats and navigation evidence
/// emit (`kinds`, `kind_exact`, ...), so every variant is pinned to its exact
/// string; an arm that returns another variant's name fails here.
#[test]
fn every_resolution_kind_has_its_exact_wire_name() {
    let table = [
        (StaticLinkage, "static_linkage"),
        (QualifiedOwner, "qualified_owner"),
        (SelfReceiver, "self_receiver"),
        (ImportQualified, "import_qualified"),
        (QualifierOwner, "qualifier_owner"),
        (LocalDef, "local_def"),
        (SamePackage, "same_package"),
        (ImplicitThis, "implicit_this"),
        (FreeSingle, "free_single"),
        (FreeMulti, "free_multi"),
        (TypedParam, "typed_param"),
        (ConstructorLocal, "constructor_local"),
        (TraitCha, "trait_cha"),
        (R6SingleOwner, "r6_single_owner"),
        (StemSingle, "stem_single"),
        (StemMulti, "stem_multi"),
        (EmbeddedPromotion, "embedded_promotion"),
        (InterfaceDispatch, "interface_dispatch"),
        (ImportMember, "import_member"),
        (R6MultiOwnerCandidate, "r6_multi_owner_candidate"),
        (CallbackRegistration, "callback_registration"),
        (FuncValueField, "func_value_field"),
        (PropertyAccess, "property_access"),
        (FrameworkEntry, "framework_entry"),
        (ReturnTyped, "return_typed"),
        (FieldTyped, "field_typed"),
        (ParameterCallback, "parameter_callback"),
        (ArgumentFollow, "argument_follow"),
        (ContextualOwner, "contextual_owner"),
    ];
    for (kind, name) in table {
        assert_eq!(kind.as_str(), name, "{kind:?}");
    }
    let distinct: std::collections::BTreeSet<_> = table.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(distinct.len(), table.len(), "wire names must be unique");
}
