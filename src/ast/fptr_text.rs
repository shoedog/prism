//! Text-level function-pointer, dispatch-table and struct-field assignment scans (call graph levels 1, 2 and 4).

use std::collections::BTreeSet;

/// Scan the function-source prefix before a call for exactly one assignment of
/// the form `var_name = func_name`, where `func_name` is a known function.
/// Any second assignment or non-function RHS fails closed.
///
/// Used by call graph Level 1 to resolve local function pointer variables.
pub fn resolve_fptr_assignment(
    func_source: &str,
    var_name: &str,
    known_fns: &BTreeSet<String>,
) -> Option<String> {
    let mut resolved = None;
    let mut saw_assignment = false;
    for line in func_source.lines() {
        let trimmed = line.trim();
        // Match: var_name = identifier  (with optional trailing semicolon/comma)
        // Also match initialization: type (*var_name)(...) = identifier;
        //   and typedef:  callback_fn var_name = identifier;

        // Strategy: find `var_name` followed by `=` (but not `==`), then extract RHS identifier
        if let Some(eq_pos) = find_assignment_eq(trimmed) {
            let lhs = trimmed[..eq_pos].trim().trim_end_matches(':').trim();
            let rhs = trimmed[eq_pos + 1..].trim().trim_end_matches(';').trim();

            // Check if LHS contains var_name as the assigned variable
            // Handles: `var_name =`, `type var_name =`, `type (*var_name)(args) =`
            let lhs_has_var = lhs == var_name
                || lhs.ends_with(&format!(" {}", var_name))
                || lhs.ends_with(&format!("*{}", var_name))
                || lhs.contains(&format!("(*{})", var_name))
                || lhs.contains(&format!(" {} ", var_name));

            if !lhs_has_var {
                continue;
            }

            if saw_assignment {
                return None;
            }
            saw_assignment = true;

            // RHS should be a plain identifier (possibly with & prefix for address-of)
            let rhs_name = rhs.trim_start_matches('&');
            if !rhs_name.is_empty()
                && rhs_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && known_fns.contains(rhs_name)
            {
                resolved = Some(rhs_name.to_string());
            } else {
                return None;
            }
        }
    }
    resolved
}

/// Scan a function (or file-scope) source for an array initializer of the form:
///   `type array_name[] = { func_a, func_b, func_c };`
/// Returns all known function names that appear in the initializer list.
///
/// Used by call graph Phase 3 to resolve dispatch table calls.
pub fn resolve_array_dispatch(
    source: &str,
    array_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = Vec::new();

    // Find the array initializer: look for `array_name` followed by `[` ... `]` ... `=` ... `{`
    // This is a text heuristic — we look for lines containing the array name and an initializer
    let lines: Vec<&str> = source.lines().collect();
    let mut in_initializer = false;
    let mut brace_depth = 0;

    for line in &lines {
        let trimmed = line.trim();

        if !in_initializer {
            // Look for: array_name ... [] = { or array_name ... [N] = {
            if trimmed.contains(array_name) && trimmed.contains('[') && trimmed.contains('=') {
                in_initializer = true;
                // Count braces on this line
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth <= 0 {
                                in_initializer = false;
                            }
                        }
                        _ => {}
                    }
                }
                // Extract identifiers from this line's initializer portion
                if let Some(brace_start) = trimmed.find('{') {
                    extract_fn_names_from_init(&trimmed[brace_start..], known_fns, &mut targets);
                }
                continue;
            }
        }

        if in_initializer {
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth <= 0 {
                            in_initializer = false;
                        }
                    }
                    _ => {}
                }
            }
            extract_fn_names_from_init(trimmed, known_fns, &mut targets);
        }
    }

    targets
}

/// Extract known function names from an initializer fragment like `{func_a, func_b, .field = func_c}`.
fn extract_fn_names_from_init(text: &str, known_fns: &BTreeSet<String>, out: &mut Vec<String>) {
    // Split on commas and braces, then check each token
    for token in text.split(|c: char| c == ',' || c == '{' || c == '}' || c == '(' || c == ')') {
        let token = token.trim();
        // Handle designated initializers: `.field = func_name`
        let ident = if let Some(eq_pos) = token.find('=') {
            let rhs = token[eq_pos + 1..].trim();
            // Skip ==
            if token.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                continue;
            }
            rhs
        } else {
            token
        };
        let ident = ident.trim_start_matches('&');
        if !ident.is_empty()
            && ident.chars().all(|c| c.is_alphanumeric() || c == '_')
            && known_fns.contains(ident)
            && !out.contains(&ident.to_string())
        {
            out.push(ident.to_string());
        }
    }
}

/// Find the position of an assignment `=` in a trimmed line, skipping `==`, `!=`, `<=`, `>=`.
/// Returns `None` if no plain assignment is found.
pub fn find_assignment_eq(trimmed: &str) -> Option<usize> {
    if let Some(eq_pos) = trimmed.find('=') {
        // Skip ==
        if eq_pos + 1 < trimmed.len() {
            if trimmed.as_bytes().get(eq_pos + 1) == Some(&b'=') {
                return None;
            }
        }
        // Skip !=, <=, >=
        if eq_pos > 0 {
            let before = trimmed.as_bytes().get(eq_pos - 1);
            if before == Some(&b'!') || before == Some(&b'<') || before == Some(&b'>') {
                return None;
            }
        }
        Some(eq_pos)
    } else {
        None
    }
}

/// The legacy per-line, per-field matcher — extracted verbatim from
/// resolve_struct_field_assignment so the Level-4 index (call_graph) and the
/// legacy oracle share ONE core. Quirks (arrow-anywhere priority,
/// prefix-consumption, single-= anchor, RHS stop set, &-strip) are pinned by
/// the level4_legacy_* tests and are CONTRACT until retired by a separate
/// measured change.
pub(crate) fn line_field_targets(
    trimmed: &str,
    field_name: &str,
    known_fns: &BTreeSet<String>,
    targets: &mut BTreeSet<String>,
) {
    let arrow_pattern = format!("->{}", field_name);
    let dot_pattern = format!(".{}", field_name);
    let mut search_from = 0usize;
    while search_from < trimmed.len() {
        let field_pos = trimmed[search_from..]
            .find(&arrow_pattern)
            .map(|p| (p + search_from, arrow_pattern.len()))
            .or_else(|| {
                trimmed[search_from..]
                    .find(&dot_pattern)
                    .map(|p| (p + search_from, dot_pattern.len()))
            });
        let (pos, pat_len) = match field_pos {
            Some(v) => v,
            None => break,
        };
        let after_field = pos + pat_len;
        search_from = after_field;
        let rest = trimmed[after_field..].trim_start();
        if !rest.starts_with('=') || rest.starts_with("==") {
            continue;
        }
        let rhs = rest[1..].trim();
        let rhs_end = rhs
            .find(|c: char| c == ';' || c == ',' || c == '}' || c == ')' || c.is_whitespace())
            .unwrap_or(rhs.len());
        let rhs_token = rhs[..rhs_end].trim().trim_start_matches('&');
        if !rhs_token.is_empty()
            && rhs_token.chars().all(|c| c.is_alphanumeric() || c == '_')
            && known_fns.contains(rhs_token)
        {
            targets.insert(rhs_token.to_string());
        }
    }
}

/// Every maximal identifier immediately preceded by `->` or `.` on the line,
/// under the PINNED predicate: char::is_alphanumeric(c) || c == '_' — the same
/// Unicode-aware class the Level-4 call-site filter applies to callee names —
/// scanned over RAW source, including comments and strings (legacy semantics).
///
/// Completeness: a field can only produce a target when the accessor occurrence
/// is followed (after optional whitespace) by `=`, which terminates the
/// identifier run. Prefix occurrences (`->cbx` when querying `cb`) produce no
/// targets; the legacy core retains their consumption side-effects.
#[cfg(test)]
pub(crate) fn candidate_fields_on_line(trimmed: &str) -> BTreeSet<String> {
    fn ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    let mut out = BTreeSet::new();
    let mut rest = trimmed;
    loop {
        let arrow = rest.find("->");
        let dot = rest.find('.');
        let (pos, len) = match (arrow, dot) {
            (Some(a), Some(d)) if a <= d => (a, 2),
            (Some(a), None) => (a, 2),
            (_, Some(d)) => (d, 1),
            (None, None) => break,
        };
        let after = &rest[pos + len..];
        let end = after.find(|c: char| !ident_char(c)).unwrap_or(after.len());
        if end > 0 {
            out.insert(after[..end].to_string());
            rest = &after[end..];
        } else {
            rest = after;
        }
    }
    out
}

/// Scan source text for struct field assignments that assign a known function
/// to a field with the given name.
/// Post-S1 this is the differential-test oracle; production Level-4 resolution
/// uses the inverted index in call_graph.rs.
///
/// Matches patterns:
///   `anything->field_name = known_func;`
///   `anything.field_name = known_func;`
///   `.field_name = known_func` (designated initializer)
///
/// Returns all unique known function names found assigned to this field.
pub fn resolve_struct_field_assignment(
    source: &str,
    field_name: &str,
    known_fns: &BTreeSet<String>,
) -> Vec<String> {
    let mut targets = BTreeSet::new();
    let arrow_pattern = format!("->{}", field_name);
    let dot_pattern = format!(".{}", field_name);
    for line in source.lines() {
        let trimmed = line.trim();
        if !(trimmed.contains(&arrow_pattern) || trimmed.contains(&dot_pattern)) {
            continue;
        }
        line_field_targets(trimmed, field_name, known_fns, &mut targets);
    }
    targets.into_iter().collect()
}
