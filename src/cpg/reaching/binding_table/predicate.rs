#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Predicate {
    FieldTextIs {
        field: &'static str,
        any_of: &'static [&'static str],
    },
    FieldChildCountIs {
        field: &'static str,
        count: usize,
    },
    FieldChildKindIs {
        field: &'static str,
        any_of: &'static [&'static str],
    },
    FieldChildCountAtLeastExcludingKinds {
        field: &'static str,
        count: usize,
        excluded: &'static [&'static str],
    },
    TypeScriptParameterPattern {
        kind: &'static str,
    },
    NestedFieldChildKindIs {
        field: &'static str,
        nested_field: &'static str,
        any_of: &'static [&'static str],
    },
    OperatorIs {
        any_of: &'static [&'static str],
    },
    FieldChildCountAtLeast {
        field: &'static str,
        count: usize,
    },
    ParentKindIs {
        kind: &'static str,
    },
    AncestorKindIs {
        kind: &'static str,
    },
    JavaInstanceofGuard,
    HasField {
        field: &'static str,
    },
    JavaScriptDestructuringDeclarator {
        lexical: bool,
    },
    JavaScriptBindingPattern,
    CppReferenceCaptureOnly,
    IsImmediatelyInvoked,
}
