use super::{Case, FlowConfidence, FlowDoubt};

pub(super) fn assert_switch_group(case: &Case) {
    assert!(
        case.expect.iter().any(|(_, _, label)| matches!(
            label,
            FlowConfidence::NameOnly(FlowDoubt::Killed { .. })
        )) && case
            .expect
            .iter()
            .any(|(_, _, label)| matches!(label, FlowConfidence::Exact)),
        "{}: Java switch groups share a switch-block kill and preserve the outer binding after the switch",
        case.id
    );
}

pub(super) fn assert_existing_resource(case: &Case) {
    assert!(
        case.expect.iter().any(|(_, _, label)| matches!(
            label,
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
        )) && case
            .expect
            .iter()
            .any(|(_, _, label)| matches!(label, FlowConfidence::Exact))
            && case.expect.iter().all(|(_, _, label)| matches!(
                label,
                FlowConfidence::Exact | FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
            )),
        "{}: existing Java resources are uses with a deferred header and Exact body/exit preservation",
        case.id
    );
}

pub(super) fn assert_assignment_reuse(case: &Case) {
    assert!(
        case.expect
            .iter()
            .all(|(_, _, label)| matches!(label, FlowConfidence::Exact)),
        "{}: an assignment beneath a Java declaration reuses the declared name",
        case.id
    );
}

pub(super) fn assert_scope(case: &Case) {
    assert!(
        case.expect.iter().all(|(_, _, label)| matches!(
            label,
            FlowConfidence::Exact
                | FlowConfidence::NameOnly(FlowDoubt::Killed { .. })
                | FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
        )),
        "{}: Java lexical scopes need Exact, Killed, or deferred CfgIncomplete evidence",
        case.id
    );
}
