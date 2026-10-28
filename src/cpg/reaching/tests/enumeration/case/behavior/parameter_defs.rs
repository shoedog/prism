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
            | "c-fix1-alias-uninitialized"
            | "c-fix1-alias-initialized"
            | "c-fix1-alias-conditional"
    )
}

use crate::access_path::AccessPath;
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
        "c-fix1-frr-match-from" => &["match"],
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

pub(super) fn add_alias_derived_definition(
    case_id: &str,
    parsed: &ParsedFile,
    defs: &mut Vec<DefSite>,
) {
    let (name, anchor) = match case_id {
        "c-fix1-alias-uninitialized" | "c-fix1-alias-initialized" | "c-fix1-alias-conditional" => {
            ("x", "y")
        }
        "c-fix1-frr-match-from" => ("match", "from"),
        _ => return,
    };
    let mut alias = super::super::super::super::def(parsed, u32::MAX - 64, anchor, 2, 0, true);
    alias.path = AccessPath::from_expr(name);
    defs.push(alias);
}
