use super::*;

pub(super) fn is_capture_copy(case: &Case, is_binding: bool) -> bool {
    is_binding && case.language == Language::Cpp && case.kind == "lambda_capture_specifier"
}

pub(super) fn assert_capture_copy(case: &Case) {
    assert!(
        case.expect
            .iter()
            .any(|(_, _, label)| matches!(label, FlowConfidence::Exact))
            && case.expect.iter().any(|(_, _, label)| matches!(
                label,
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
            )),
        "{}: capture copy needs an ordinary specifier read and a deferred body read",
        case.id
    );
}

pub(super) fn has_required_kill(row: &BindingRow, cases: &[&&Case]) -> bool {
    row.declaration == Some(DeclarationKind::CaptureCopy)
        || cases
            .iter()
            .flat_map(|case| case.expect)
            .any(|(_, _, label)| {
                matches!(label, FlowConfidence::NameOnly(FlowDoubt::Killed { .. }))
            })
}
