//! The pure mapping tables behind the SARIF serializer (design §2.2): rule
//! descriptions, related-line attribution, severity → `level`, URI encoding
//! and fingerprints. Every function here is total and takes only borrowed
//! primitives, so each is unit-testable without building a document;
//! `sarif.rs` assembles what they return.
//!
//! Split out of `sarif.rs` to keep both files under the repo's 600-line limit
//! (CLAUDE.md §7).
//!
//! The `fullDescription` table's 29 categories are the complete production set
//! surveyed in `grounding/finding-inventory.md` §4. A category outside the
//! table falls back to `"{algorithm}: {category}"` — a new algorithm gets a
//! usable rule entry without a code change here, and adding its sentence is a
//! pure documentation follow-up. `contract_violation` names BOTH shapes it is
//! emitted for (a modified guard clause and a modified return path), because
//! one category covers two construction sites (`contract_slice.rs:183`, `:213`).

use crate::slice::SliceFinding;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Rule id / `properties.category` used when a finding carries no category.
/// No production construction site does today; this exists so a future one
/// cannot produce a document with a missing key.
pub const UNCATEGORIZED: &str = "uncategorized";

/// One sentence describing what a rule flags. Never empty.
pub fn rule_description(algorithm: &str, category: &str) -> String {
    let sentence = match category {
        // --- absence ---
        "missing_counterpart" => {
            "A paired resource operation is opened on the changed line but no matching close or \
             release call was found on any path out of the enclosing function."
        }
        "close_only_on_error_path" => {
            "The resource opened on the changed line is released only on the error (goto) path; \
             the normal return path leaves it open."
        }
        "missing_close_on_error_path" => {
            "The resource opened on the changed line is not released on an error path that jumps \
             to a cleanup label."
        }
        "double_close" => {
            "The same resource appears to be released twice — once inline and once again through \
             a cleanup label — on a path that can reach both."
        }
        // --- callback_dispatcher ---
        "callback_registrar_call" => {
            "The function is registered as a callback through a registrar call, so it has an \
             indirect caller that no direct call edge records."
        }
        "callback_dispatcher_chain" => {
            "A registered callback is invoked indirectly through a dispatcher, so a change to its \
             signature or contract affects call sites the call graph does not show."
        }
        "callback_null_arg_dispatch" => {
            "A registered callback is dispatched with a NULL argument at one or more sites, so it \
             must tolerate a null parameter."
        }
        // --- contract ---
        "contract_violation" => {
            "The changed function's contract moved: either a guard clause on an argument was \
             modified (precondition) or a return path was modified (postcondition), so callers \
             written against the old contract may now be wrong."
        }
        "contract_postcondition_new_null" => {
            "A new null-returning path was added to a function that returns a value on its other \
             paths, so callers may not handle null."
        }
        "contract_postcondition" => {
            "Summary of the changed function's postconditions — the result shapes it can produce \
             — for reviewing what callers must handle."
        }
        "contract" => {
            "Summary of the changed function's preconditions, and postconditions when present, \
             for reviewing what callers must satisfy."
        }
        "contract_precondition_weakened" => {
            "A precondition guard was removed or loosened against the old revision, so callers \
             that relied on it rejecting invalid values now receive unvalidated results."
        }
        "contract_precondition_strengthened" => {
            "A precondition guard was added or tightened against the old revision, so callers now \
             face stricter argument checking than before."
        }
        "contract_postcondition_weakened" => {
            "A postcondition was loosened against the old revision — a new null path, a changed \
             return type, or a weaker return shape — so callers may not handle the new result."
        }
        "contract_postcondition_strengthened" => {
            "A postcondition was tightened against the old revision, so callers that defended \
             against the old, looser result can be simplified."
        }
        // --- echo / membrane ---
        "missing_error_handling" => {
            "A caller of the changed function neither checks its result nor wraps the call in \
             error handling, so a failure introduced by the change propagates unnoticed."
        }
        "unprotected_caller" => {
            "A cross-file caller invokes the changed module-boundary function without the guard, \
             wrapper, or error handling its peers apply."
        }
        // --- peer_consistency ---
        "peer_guard_divergence" => {
            "Sibling functions in the same file guard the same value inconsistently: at least one \
             peer omits a check the others perform."
        }
        // --- primitive (SCREAMING_SNAKE rule ids, kept verbatim) ---
        "HASH_TRUNCATED_BELOW_128_BITS" => {
            "A hash digest is truncated below 128 bits, leaving too little collision resistance \
             for an identity or integrity check."
        }
        "WEAK_HASH_FOR_IDENTITY" => {
            "A cryptographically broken hash (MD5 or SHA-1) is used where the digest serves as an \
             identity or integrity token."
        }
        "SHELL_TRUE_WITH_INTERPOLATION" => {
            "A shell command is executed with shell interpretation enabled over an interpolated \
             string, so any caller-controlled fragment becomes shell syntax."
        }
        "CERT_VALIDATION_DISABLED" => {
            "TLS certificate or hostname validation is disabled, so the remote peer is not \
             authenticated."
        }
        "HARDCODED_SECRET" => {
            "A credential, key, or token is embedded as a literal in source, exposing it to \
             everyone who can read the repository."
        }
        "HASH_TRUNCATION_VIA_CALL" => {
            "A hash digest is truncated below 128 bits by a caller passing a short length to a \
             helper, so the truncation is invisible at the hashing site itself."
        }
        // --- provenance ---
        "untrusted_origin" => {
            "The variable used on the changed line traces back to an untrusted or unverified \
             origin, so its value should be validated before use."
        }
        // --- symmetry ---
        "broken_symmetry" => {
            "The changed function has a symmetric counterpart — its open/close, add/remove, or \
             encode/decode partner — that was not changed with it."
        }
        // --- taint ---
        "taint_sink" => {
            "A tainted value reaches a dangerous sink without passing through a cleanser on the \
             traced data-flow path."
        }
        "unquoted_expansion" => {
            "An unquoted shell expansion is used as a command argument, so word splitting and \
             globbing can alter the command that runs."
        }
        "taint_source" => {
            "This line introduces tainted data that reaches a reported sink; it is emitted to \
             make the flow's origin reviewable."
        }
        // --- forward compatibility: a category this table does not know ---
        _ => return format!("{algorithm}: {category}"),
    };
    sentence.to_string()
}

/// Which file a finding's `related_lines` belong to (§2.2.2, sol #8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    /// Lines are in `finding.file`; `related_files` are separate artifacts.
    SameFile,
    /// Lines are in `related_files[0]` (symmetry's counterpart function,
    /// primitive's callee).
    CounterpartFile,
    /// Lines can be located only when there is no other candidate file.
    Ambiguous,
}

pub fn attribution(algorithm: &str) -> Attribution {
    match algorithm {
        "echo" | "membrane" | "absence" | "contract" | "provenance" | "taint"
        | "peer_consistency" => Attribution::SameFile,
        "symmetry" | "primitive" => Attribution::CounterpartFile,
        _ => Attribution::Ambiguous,
    }
}

/// SARIF `level` for a prism severity. An unknown future severity becomes
/// `error`, never invisible (§5.5); the original string stays in
/// `properties.severity`.
pub fn level_for_severity(severity: &str) -> &'static str {
    match severity {
        "concern" => "error",
        "warning" => "warning",
        "suggestion" | "info" => "note",
        _ => "error",
    }
}

/// `(encoded uri, escapes_repo_root)`. Backslashes normalise to `/` and each
/// segment is percent-encoded per RFC 3986 (unreserved `A-Za-z0-9-._~` kept).
/// An absolute or `..`-containing path is emitted as given after encoding and
/// flagged so the caller can raise a notification.
pub fn sarif_uri(path: &str) -> (String, bool) {
    let normalized = path.replace('\\', "/");
    let escapes = path.starts_with('/')
        || Path::new(path).is_absolute()
        || normalized.split('/').any(|s| s == "..");
    let encoded = normalized
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/");
    (encoded, escapes)
}

fn encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Occurrence discriminator over `[algorithm, category, file, function_name,
/// masked_description, line_text]` serialized as a canonical JSON array (no
/// delimiter ambiguity). The line number is excluded and digit runs in the
/// description are masked, so the value survives line shifts. See `sarif.rs`'s
/// module docs for what it does NOT guarantee.
pub fn fingerprint(finding: &SliceFinding, line_text: &str) -> String {
    let masked = mask_digits(&finding.description);
    let parts = [
        finding.algorithm.as_str(),
        finding.category.as_deref().unwrap_or(UNCATEGORIZED),
        finding.file.as_str(),
        finding.function_name.as_deref().unwrap_or(""),
        masked.as_str(),
        line_text,
    ];
    let canonical = serde_json::to_vec(&parts).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    format!("{:x}", hasher.finalize())
}

/// Every maximal run of ASCII digits becomes a single `#`.
fn mask_digits(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_run = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !in_run {
                out.push('#');
                in_run = true;
            }
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// Whitespace-trimmed source text of `line` in `file`; empty when unavailable.
pub fn line_text_of(sources: &BTreeMap<String, String>, file: &str, line: usize) -> String {
    if line == 0 {
        return String::new();
    }
    sources
        .get(file)
        .and_then(|s| s.lines().nth(line - 1))
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(algorithm: &str, description: &str) -> SliceFinding {
        SliceFinding {
            algorithm: algorithm.to_string(),
            file: "a.py".to_string(),
            line: 12,
            severity: "warning".to_string(),
            description: description.to_string(),
            function_name: Some("read".to_string()),
            related_lines: vec![],
            related_files: vec![],
            category: Some("missing_counterpart".to_string()),
            parse_quality: None,
            diagrams: vec![],
        }
    }

    /// §7.2.7: the four-value vocabulary plus the conservative default.
    #[test]
    fn severity_maps_to_level() {
        assert_eq!(level_for_severity("concern"), "error");
        assert_eq!(level_for_severity("warning"), "warning");
        assert_eq!(level_for_severity("suggestion"), "note");
        assert_eq!(level_for_severity("info"), "note");
        // §5.5: an unknown severity becomes louder, not quieter.
        assert_eq!(level_for_severity("critical"), "error");
        assert_eq!(level_for_severity(""), "error");
    }

    /// §7.2.9: URI encoding and repo-root escape detection.
    #[test]
    fn uri_encoding_keeps_unreserved_and_normalises_separators() {
        assert_eq!(
            sarif_uri("dir with space/a b.py"),
            ("dir%20with%20space/a%20b.py".to_string(), false)
        );
        assert_eq!(sarif_uri("a\\b.py"), ("a/b.py".to_string(), false));
        assert_eq!(sarif_uri("src/mod.rs"), ("src/mod.rs".to_string(), false));
        assert_eq!(sarif_uri("a#b?c%d.py").0, "a%23b%3Fc%25d.py");
        assert_eq!(sarif_uri("caf\u{e9}.py").0, "caf%C3%A9.py", "UTF-8 bytes");
        assert_eq!(sarif_uri("a-b._~.py").0, "a-b._~.py", "unreserved kept");

        assert!(sarif_uri("../x.py").1, "`..` escapes the repo root");
        assert!(sarif_uri("a/../../x.py").1);
        assert!(sarif_uri("/etc/passwd").1, "absolute escapes the root");
        assert!(
            !sarif_uri("a..b/x.py").1,
            "`..` inside a name is not a segment"
        );
    }

    /// §7.2.8: the fingerprint survives a line shift but not a change of
    /// evidence.
    #[test]
    fn fingerprint_is_line_shift_stable_and_evidence_sensitive() {
        let a = finding("absence", "file open without close in 'read' (line 12)");
        let b = finding("absence", "file open without close in 'read' (line 13)");
        let text = "f = open(\"x\")";
        assert_eq!(
            fingerprint(&a, text),
            fingerprint(&b, text),
            "digit runs are masked and the line number is excluded"
        );
        assert_ne!(
            fingerprint(&a, text),
            fingerprint(&a, "f = open(\"y\")"),
            "different line text is a different occurrence"
        );
        let mut c = a.clone();
        c.function_name = Some("other".to_string());
        assert_ne!(
            fingerprint(&a, text),
            fingerprint(&c, text),
            "different function is a different occurrence"
        );
        let hex = fingerprint(&a, "");
        assert_eq!(hex.len(), 64, "lowercase sha256 hex, untruncated");
        assert!(hex
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn mask_digits_collapses_maximal_runs() {
        assert_eq!(mask_digits("line 12 and 3"), "line # and #");
        assert_eq!(mask_digits("no digits"), "no digits");
        assert_eq!(mask_digits("1a2"), "#a#");
    }

    /// §2.2.2 attribution table, verbatim.
    #[test]
    fn attribution_table_matches_the_spec() {
        for algorithm in [
            "echo",
            "membrane",
            "absence",
            "contract",
            "provenance",
            "taint",
            "peer_consistency",
        ] {
            assert_eq!(attribution(algorithm), Attribution::SameFile, "{algorithm}");
        }
        for algorithm in ["symmetry", "primitive"] {
            assert_eq!(
                attribution(algorithm),
                Attribution::CounterpartFile,
                "{algorithm}"
            );
        }
        for algorithm in ["callback_dispatcher", "", "not_an_algorithm"] {
            assert_eq!(
                attribution(algorithm),
                Attribution::Ambiguous,
                "{algorithm}"
            );
        }
    }

    #[test]
    fn line_text_is_trimmed_and_absent_sources_are_empty() {
        let mut sources = BTreeMap::new();
        sources.insert("a.py".to_string(), "one\n    two  \nthree\n".to_string());
        assert_eq!(line_text_of(&sources, "a.py", 2), "two");
        assert_eq!(line_text_of(&sources, "a.py", 99), "");
        assert_eq!(line_text_of(&sources, "a.py", 0), "");
        assert_eq!(line_text_of(&sources, "missing.py", 1), "");
    }

    /// Every category in `grounding/finding-inventory.md` §4 has a real
    /// sentence, not the `{algorithm}: {category}` fallback.
    #[test]
    fn every_known_category_has_a_sentence() {
        let categories = [
            "missing_counterpart",
            "close_only_on_error_path",
            "missing_close_on_error_path",
            "double_close",
            "callback_registrar_call",
            "callback_dispatcher_chain",
            "callback_null_arg_dispatch",
            "contract_violation",
            "contract_postcondition_new_null",
            "contract_postcondition",
            "contract",
            "contract_precondition_weakened",
            "contract_precondition_strengthened",
            "contract_postcondition_weakened",
            "contract_postcondition_strengthened",
            "missing_error_handling",
            "unprotected_caller",
            "peer_guard_divergence",
            "HASH_TRUNCATED_BELOW_128_BITS",
            "WEAK_HASH_FOR_IDENTITY",
            "SHELL_TRUE_WITH_INTERPOLATION",
            "CERT_VALIDATION_DISABLED",
            "HARDCODED_SECRET",
            "HASH_TRUNCATION_VIA_CALL",
            "untrusted_origin",
            "broken_symmetry",
            "taint_sink",
            "unquoted_expansion",
            "taint_source",
        ];
        assert_eq!(categories.len(), 29, "the inventory lists 29 categories");
        for category in categories {
            let text = rule_description("algo", category);
            assert_ne!(
                text,
                format!("algo: {category}"),
                "{category} fell through to the fallback"
            );
            assert!(text.ends_with('.'), "{category}: not a sentence: {text}");
            assert!(text.len() > 40, "{category}: sentence too thin: {text}");
        }
    }

    #[test]
    fn contract_violation_names_both_shapes() {
        let text = rule_description("contract", "contract_violation");
        assert!(text.contains("precondition"), "{text}");
        assert!(text.contains("postcondition"), "{text}");
    }

    #[test]
    fn unknown_category_falls_back_to_algorithm_and_category() {
        assert_eq!(
            rule_description("newalgo", "brand_new"),
            "newalgo: brand_new"
        );
        assert_eq!(
            rule_description("taint", "uncategorized"),
            "taint: uncategorized"
        );
    }
}
