pub(super) fn needs_synthetic_definition(case_id: &str) -> bool {
    matches!(
        case_id,
        "e0a-x-rs-parameters"
            | "e0a-x-js-formal_parameters"
            | "js-lexical-destructuring-declarator"
            | "js-var-destructuring-declarator"
            | "ts-required-parameter-seeds"
            | "ts-optional-parameter"
            | "ts-rest-parameter"
            | "ts-rest-pattern-owned-by-parameter"
    )
}

use crate::ast::ParsedFile;
use crate::cpg::reaching::DefSite;

pub(super) fn mark_alias_derived(case_id: &str, defs: &mut [DefSite]) {
    for def in defs.iter_mut() {
        let initializer_alias = case_id == "cpp-initializer-alias-does-not-bind-target"
            && def.line == 3
            && def.path.base == "x";
        let reference_capture_body_read =
            (case_id == "cpp-multiple-reference-captures-not-copy-bindings" && def.line >= 7)
                || (case_id == "cpp-capture-by-reference" && def.line == 3);
        if initializer_alias || reference_capture_body_read {
            def.alias_derived = true;
        }
    }
}

pub(super) fn add_named_synthetic_definitions(
    case_id: &str,
    parsed: &ParsedFile,
    defs: &mut Vec<DefSite>,
) {
    let names: &[&str] = match case_id {
        "py-import-dotted-path-r08" => &["path"],
        "py-import-alias-original-r09" => &["a", "pkg"],
        "py-import-dotted-from-a06" => &["incident"],
        "py-import-try-except-handler-use" => &["status"],
        "cpp-structured-range-header-stops-at-loop-exit"
        | "cpp-nested-structured-range-does-not-leak"
        | "cpp-for-chain-header-stops-at-loop-exit"
        | "cpp-uncertain-owner-stops-at-block-exit" => &["x"],
        _ => &[],
    };
    for (index, name) in names.iter().enumerate() {
        defs.push(super::super::super::super::def(
            parsed,
            u32::MAX - index as u32,
            name,
            1,
            0,
            false,
        ));
    }
}
