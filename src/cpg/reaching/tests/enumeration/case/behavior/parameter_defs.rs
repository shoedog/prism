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
            | "java-formal-parameter-masks"
            | "java-spread-parameter-masks"
    )
}

use crate::ast::ParsedFile;
use crate::cpg::reaching::DefSite;

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
