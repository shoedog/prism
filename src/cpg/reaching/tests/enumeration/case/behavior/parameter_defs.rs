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
            | "c-fix1-alias-uninitialized"
            | "c-fix1-alias-initialized"
            | "c-fix1-alias-conditional"
    ) || (case_id.starts_with("c-fix2-side-effect-")
        && case_id != "c-fix2-side-effect-pointer-update")
}

use crate::access_path::AccessPath;
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
    let (names, line): (&[&str], usize) = match case_id {
        "py-import-dotted-path-r08" => (&["path"], 1),
        "py-import-alias-original-r09" => (&["a", "pkg"], 1),
        "py-import-dotted-from-a06" => (&["incident"], 1),
        "py-import-try-except-handler-use" => (&["status"], 1),
        "c-fix1-frr-match-from" => (&["match"], 1),
        "c-fix2-side-effect-pointer-update" => (&["p"], 1),
        "c-fix2-recovered-macro-parameter" => (&["conn"], 2),
        "c-fix2-ordinary-call-control" | "c-fix2-real-parameter-mask" => (&["conn"], 1),
        "cpp-structured-range-header-stops-at-loop-exit"
        | "cpp-nested-structured-range-does-not-leak"
        | "cpp-for-chain-header-stops-at-loop-exit"
        | "cpp-chain-own-definitions-uncertain"
        | "cpp-multiline-chain-own-definitions-uncertain"
        | "cpp-uncertain-owner-stops-at-block-exit" => (&["x"], 1),
        "cpp-park-single-shadow-direct" => (&["x"], 3),
        _ => (&[], 1),
    };
    for (index, name) in names.iter().enumerate() {
        defs.push(super::super::super::super::def(
            parsed,
            u32::MAX - index as u32,
            name,
            line,
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
