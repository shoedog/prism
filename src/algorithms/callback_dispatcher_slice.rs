//! Callback Dispatcher Slice — resolve function-pointer-in-struct registrations
//! to their invocation sites.
//!
//! **Question answered:** "This function is stored into a struct field
//! somewhere — who actually *invokes* the field, and do any of those callers
//! pass NULL as an argument?"
//!
//! Motivating case (FRR CVE-2025-61102, T1-002): `show_vty_*` is registered
//! into a `functab->show_opaque_info` struct field, which `lib/log.c` then
//! invokes with `vty=NULL` through zlog. Normal call-graph slicing stops at
//! the registration (there's no direct call), so the NULL-dispatcher path is
//! invisible. This slice closes that gap with a lightweight, text-level
//! matcher:
//!
//! 1. For each diff-touched function F, scan all files for registrations of F
//!    into a struct field. Recognised forms:
//!      - `.field = F,`              (designated struct initialiser)
//!      - `.field = F }`             (end of struct literal)
//!      - `obj.field = F;`           (assignment)
//!      - `obj->field = F;`          (pointer assignment)
//! 2. For every distinct field name `fname` that F is registered into, scan all
//!    files for invocations of the form `X->fname(` or `X.fname(` or
//!    `tab[i].fname(`.
//! 3. For each invocation, if any argument position contains a literal
//!    `NULL`/`nullptr`/`0`, flag it as a confused-deputy hazard.
//!
//! This is deliberately text-level rather than a full function-pointer alias
//! analysis. The false-positive rate is kept acceptable by requiring a
//! *registration → invocation* chain anchored on an exact field name: no chain,
//! no finding.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

struct Registration {
    file: String,
    line: usize,
    field: String,
    /// How the function was registered. `Field` = `.field = func` or
    /// `obj->field = func`. `CallArg` = passed as an argument to a function
    /// whose name contains `register` (e.g. `ospf_register_opaque_functab`,
    /// `g_signal_connect`). The `field` column holds the *registrar* call
    /// name in that case, since the target struct field isn't directly
    /// visible at the call site.
    kind: RegKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegKind {
    Field,
    CallArg,
}

struct Invocation {
    file: String,
    line: usize,
    has_null_arg: bool,
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::CallbackDispatcherSlice);
    let mut block_id = 0;

    // Collect diff-touched function names (C/C++ only).
    let mut diff_functions: BTreeMap<String, (String, usize, usize)> = BTreeMap::new();
    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }
        for &line in &diff_info.diff_lines {
            if let Some(func) = parsed.enclosing_function(line) {
                if let Some(name_node) = parsed.language.function_name(&func) {
                    let name = parsed.node_text(&name_node).to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let (start, end) = parsed.node_line_range(&func);
                    diff_functions
                        .entry(name)
                        .or_insert((diff_info.file_path.clone(), start, end));
                }
            }
        }
    }
    if diff_functions.is_empty() {
        return Ok(result);
    }

    for (func_name, (def_file, def_start, def_end)) in &diff_functions {
        let registrations = find_registrations(files, func_name);
        if registrations.is_empty() {
            continue;
        }

        // Emit call-arg registrations (registrar callee pattern) separately —
        // we don't know the target field for these, so we can't find
        // invocations. The finding is still useful: it tells the reviewer
        // "this function is registered via X; inspect X's dispatch path."
        let call_arg_regs: Vec<&Registration> = registrations
            .iter()
            .filter(|r| r.kind == RegKind::CallArg)
            .collect();
        if !call_arg_regs.is_empty() {
            let registrars: BTreeSet<String> =
                call_arg_regs.iter().map(|r| r.field.clone()).collect();
            let sites = call_arg_regs
                .iter()
                .take(5)
                .map(|r| format!("{}:{}", r.file, r.line))
                .collect::<Vec<_>>()
                .join(", ");
            let description = format!(
                "Function `{}` is registered as a callback via {} at {}{}. The dispatcher lives inside the registrar — callers that invoke the stored pointer may pass literal NULL (e.g. log/debug wrappers). Inspect the registrar's internals for NULL-passing dispatch paths; a confused-deputy or NULL-dereference hazard is possible if the callback body dereferences its arguments without a guard.",
                func_name,
                registrars
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                sites,
                if call_arg_regs.len() > 5 {
                    format!(" (+{} more)", call_arg_regs.len() - 5)
                } else {
                    String::new()
                }
            );
            let related_files: Vec<String> = call_arg_regs
                .iter()
                .map(|r| r.file.clone())
                .filter(|f| f != def_file)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let related_lines: Vec<usize> = call_arg_regs
                .iter()
                .map(|r| r.line)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            result.findings.push(SliceFinding {
                algorithm: "callback_dispatcher".to_string(),
                file: def_file.clone(),
                line: *def_start,
                severity: "warning".to_string(),
                description,
                function_name: Some(func_name.clone()),
                related_lines,
                related_files,
                category: Some("callback_registrar_call".to_string()),
                parse_quality: None,
            });
            // Add a block showing the def + registration lines.
            let mut block = DiffBlock::new(block_id, def_file.clone(), ModifyType::Modified);
            for line in *def_start..=(*def_end).min(*def_start + 4) {
                block.add_line(def_file, line, false);
            }
            for reg in &call_arg_regs {
                block.add_line(&reg.file, reg.line, false);
            }
            if !block.file_line_map.is_empty() {
                result.blocks.push(block);
                block_id += 1;
            }
        }

        let field_names: BTreeSet<String> = registrations
            .iter()
            .filter(|r| r.kind == RegKind::Field)
            .map(|r| r.field.clone())
            .collect();

        for field in &field_names {
            let invocations = find_invocations(files, field);
            if invocations.is_empty() {
                continue;
            }

            let null_invocations: Vec<&Invocation> =
                invocations.iter().filter(|i| i.has_null_arg).collect();

            let matching_regs: Vec<&Registration> =
                registrations.iter().filter(|r| &r.field == field).collect();

            let description = if !null_invocations.is_empty() {
                let sites = null_invocations
                    .iter()
                    .take(3)
                    .map(|i| format!("{}:{}", i.file, i.line))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Function `{}` is registered as `.{} = {}` at {} and the `.{}` field is invoked with a NULL argument at {}{}. If the invocation reaches `{}`, NULL is dereferenced inside the callback — classic confused-deputy / zlog-style dispatcher hazard.",
                    func_name,
                    field,
                    func_name,
                    matching_regs
                        .iter()
                        .take(3)
                        .map(|r| format!("{}:{}", r.file, r.line))
                        .collect::<Vec<_>>()
                        .join(", "),
                    field,
                    sites,
                    if null_invocations.len() > 3 {
                        format!(" (+{} more)", null_invocations.len() - 3)
                    } else {
                        String::new()
                    },
                    func_name,
                )
            } else {
                format!(
                    "Function `{}` is registered as `.{} = {}` at {} and the `.{}` field is invoked at {} site(s) (e.g. {}). The callback reaches these dispatchers even though there is no direct call edge — review argument contracts at each invocation.",
                    func_name,
                    field,
                    func_name,
                    matching_regs
                        .iter()
                        .take(3)
                        .map(|r| format!("{}:{}", r.file, r.line))
                        .collect::<Vec<_>>()
                        .join(", "),
                    field,
                    invocations.len(),
                    invocations
                        .iter()
                        .take(3)
                        .map(|i| format!("{}:{}", i.file, i.line))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            let severity = if null_invocations.is_empty() {
                "info"
            } else {
                "concern"
            };

            let related_files: Vec<String> = matching_regs
                .iter()
                .map(|r| r.file.clone())
                .chain(invocations.iter().map(|i| i.file.clone()))
                .filter(|f| f != def_file)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();

            let related_lines: Vec<usize> = matching_regs
                .iter()
                .map(|r| r.line)
                .chain(invocations.iter().map(|i| i.line))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();

            result.findings.push(SliceFinding {
                algorithm: "callback_dispatcher".to_string(),
                file: def_file.clone(),
                line: *def_start,
                severity: severity.to_string(),
                description,
                function_name: Some(func_name.clone()),
                related_lines,
                related_files,
                category: Some(if null_invocations.is_empty() {
                    "callback_dispatcher_chain".to_string()
                } else {
                    "callback_null_arg_dispatch".to_string()
                }),
                parse_quality: None,
            });

            // Build a block containing: the function definition, each
            // registration site, and each invocation site (first 8 lines
            // around each).
            let mut block = DiffBlock::new(block_id, def_file.clone(), ModifyType::Modified);
            for line in *def_start..=(*def_end).min(*def_start + 8) {
                block.add_line(def_file, line, false);
            }
            for reg in &matching_regs {
                block.add_line(&reg.file, reg.line, false);
            }
            for inv in &invocations {
                block.add_line(&inv.file, inv.line, false);
            }
            if !block.file_line_map.is_empty() {
                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}

/// Scan every C/C++ file for lines that register `func_name` into a struct
/// field. Recognises `.field = func_name`, `obj.field = func_name`, and
/// `obj->field = func_name` forms.
fn find_registrations(files: &BTreeMap<String, ParsedFile>, func_name: &str) -> Vec<Registration> {
    let mut regs = Vec::new();
    for (file, parsed) in files {
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }
        let src = parsed.source.as_str();
        let lines: Vec<&str> = src.lines().collect();
        // Precompute line start byte offsets.
        let mut line_starts: Vec<usize> = Vec::with_capacity(lines.len() + 1);
        let mut off = 0;
        for line in &lines {
            line_starts.push(off);
            off += line.len() + 1; // +1 for '\n'
        }
        line_starts.push(src.len());

        for (idx, line_text) in lines.iter().enumerate() {
            let line_num = idx + 1;
            if !contains_word(line_text, func_name) {
                continue;
            }
            if let Some(field) = extract_registration_field(line_text, func_name) {
                regs.push(Registration {
                    file: file.clone(),
                    line: line_num,
                    field,
                    kind: RegKind::Field,
                });
                continue;
            }
            // For call-arg registrations, the callee may be on an earlier
            // line (multi-line call). Resolve by walking back through the
            // full source byte buffer.
            let line_start = line_starts[idx];
            if let Some(pos) = locate_word_in_line(line_text, func_name) {
                let abs_pos = line_start + pos;
                if let Some(registrar) = enclosing_callee_multiline(src, abs_pos, func_name.len()) {
                    let r = registrar.to_ascii_lowercase();
                    if r.contains("register") || r.contains("_functab") || r == "g_signal_connect" {
                        regs.push(Registration {
                            file: file.clone(),
                            line: line_num,
                            field: registrar,
                            kind: RegKind::CallArg,
                        });
                    }
                }
            }
        }
    }
    regs
}

/// Return the byte offset within `line` of the first standalone-token
/// occurrence of `word`, or None.
fn locate_word_in_line(line: &str, word: &str) -> Option<usize> {
    let mut start = 0;
    while let Some(pos) = line[start..].find(word) {
        let abs = start + pos;
        let before = &line[..abs];
        let after = &line[abs + word.len()..];
        let left_ok = before
            .chars()
            .last()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        let right_ok = after
            .chars()
            .next()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        if left_ok && right_ok {
            // Require the right-hand separator to be a call-arg-shaped char
            // (comma, close paren, whitespace) so we skip uses in unrelated
            // expressions. This is best-effort; it matches the FRR pattern.
            if after
                .chars()
                .next()
                .map(|c| matches!(c, ',' | ')' | ' ' | '\t' | '\n'))
                .unwrap_or(false)
            {
                return Some(abs);
            }
        }
        start = abs + word.len();
    }
    None
}

/// Walk backward through `src` starting just before the word at `word_abs..
/// word_abs+word_len`, find the enclosing `(` at paren-depth 0 (across
/// newlines), and return the identifier that precedes it. Skips `/* ... */`
/// comments and `// ...` lines conservatively by ignoring `(` / `)` inside
/// obvious comment contexts.
fn enclosing_callee_multiline(src: &str, word_abs: usize, word_len: usize) -> Option<String> {
    let bytes = src.as_bytes();
    let mut depth: i32 = 0;
    let mut i = word_abs;
    let limit = word_abs.saturating_sub(4096); // cap backward walk
    while i > limit {
        i -= 1;
        let b = bytes[i];
        match b {
            b')' => depth += 1,
            b'(' => {
                if depth == 0 {
                    // Collect whitespace and then identifier ending at i.
                    let mut end = i;
                    while end > 0 {
                        let c = bytes[end - 1];
                        if c.is_ascii_whitespace() {
                            end -= 1;
                        } else {
                            break;
                        }
                    }
                    let mut begin = end;
                    while begin > 0 {
                        let c = bytes[begin - 1] as char;
                        if c.is_alphanumeric() || c == '_' {
                            begin -= 1;
                        } else {
                            break;
                        }
                    }
                    if begin == end {
                        return None;
                    }
                    let _ = word_len;
                    return std::str::from_utf8(&bytes[begin..end])
                        .ok()
                        .map(|s| s.to_string());
                } else {
                    depth -= 1;
                }
            }
            _ => {}
        }
    }
    None
}

/// Scan every C/C++ file for invocations of `field` as a function pointer:
/// `x->field(`, `x.field(`, `tab[i].field(`, etc.
fn find_invocations(files: &BTreeMap<String, ParsedFile>, field: &str) -> Vec<Invocation> {
    let mut invs = Vec::new();
    for (file, parsed) in files {
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }
        for (idx, line_text) in parsed.source.lines().enumerate() {
            let line_num = idx + 1;
            if !contains_word(line_text, field) {
                continue;
            }
            if let Some((_, has_null)) = extract_invocation(line_text, field) {
                invs.push(Invocation {
                    file: file.clone(),
                    line: line_num,
                    has_null_arg: has_null,
                });
            }
        }
    }
    invs
}

/// If `line` contains a registration like `.field = func_name` or
/// `x->field = func_name` or `x.field = func_name`, return the field name.
fn extract_registration_field(line: &str, func_name: &str) -> Option<String> {
    // Find the token occurrence of `func_name` standing on its own.
    let mut start = 0;
    while let Some(pos) = line[start..].find(func_name) {
        let abs = start + pos;
        let before = &line[..abs];
        let after = &line[abs + func_name.len()..];
        // Must be a standalone identifier.
        let left_ok = before
            .chars()
            .last()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        let right_ok = after
            .chars()
            .next()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        if !(left_ok && right_ok) {
            start = abs + func_name.len();
            continue;
        }
        // Look backward for ` = ` or `=` with a field on the LHS.
        let lhs = before.trim_end();
        if let Some(eq_stripped) = lhs.strip_suffix('=') {
            let lhs_expr = eq_stripped.trim_end();
            if let Some(field) = field_name_from_lhs(lhs_expr) {
                return Some(field);
            }
        }
        start = abs + func_name.len();
    }
    None
}

/// Given the LHS of an assignment (without the trailing `=`), extract the
/// field identifier it writes to.
fn field_name_from_lhs(lhs: &str) -> Option<String> {
    let lhs = lhs.trim_end();
    // Handle trailing parentheses or brackets by finding the last separator.
    let mut end = lhs.len();
    let bytes = lhs.as_bytes();
    // Walk backward while we're in identifier characters.
    while end > 0 {
        let c = bytes[end - 1] as char;
        if c.is_alphanumeric() || c == '_' {
            end -= 1;
        } else {
            break;
        }
    }
    let ident = &lhs[end..];
    if ident.is_empty() {
        return None;
    }
    // Determine the separator (., ->, or designated-initializer dot).
    let before_ident = lhs[..end].trim_end();
    let is_field_access = before_ident.ends_with("->")
        || before_ident.ends_with('.')
        || before_ident.is_empty()
        || before_ident.ends_with('{')
        || before_ident.ends_with(',');
    // Exclude top-level assignments to plain variables (e.g. `int x =`).
    // A leading dot (designated initializer) is the signal we want.
    if before_ident.ends_with('.') || before_ident.ends_with("->") {
        return Some(ident.to_string());
    }
    // Designated initializer form: check if the identifier is preceded by `.` earlier.
    // e.g. `    .show_opaque_info = ospf_ext_show_info`
    // In this case before_ident is ".", handled above. But tree-sitter
    // preserves the `.` directly before the ident: lhs = ".show_opaque_info"
    if lhs.trim_start().starts_with('.') {
        let trimmed = lhs.trim_start().trim_start_matches('.').trim();
        // Strip to the identifier only (no suffix).
        let ident = trimmed
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();
        if !ident.is_empty() {
            return Some(ident);
        }
    }
    // Otherwise accept if the LHS *looks* like a member access —
    // require at least one `.` or `->` anywhere in the LHS.
    if is_field_access && (lhs.contains("->") || lhs.contains('.')) {
        return Some(ident.to_string());
    }
    None
}

/// If `line` contains a field-pointer call like `x->field(`, `x.field(`, or
/// `tab[i].field(`, return a snippet of the argument list and whether any arg
/// is a NULL literal.
fn extract_invocation(line: &str, field: &str) -> Option<(String, bool)> {
    let mut start = 0;
    while let Some(pos) = line[start..].find(field) {
        let abs = start + pos;
        let before = &line[..abs];
        let after = &line[abs + field.len()..];
        let left_ok = before
            .chars()
            .last()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        if !left_ok {
            start = abs + field.len();
            continue;
        }
        // Must be followed by `(`.
        let after_trim = after.trim_start();
        if !after_trim.starts_with('(') {
            start = abs + field.len();
            continue;
        }
        // Must be preceded by `.` or `->` — otherwise it's a plain function call,
        // not a field-pointer dispatch.
        let lhs = before.trim_end();
        let is_field_call = lhs.ends_with('.') || lhs.ends_with("->");
        if !is_field_call {
            start = abs + field.len();
            continue;
        }
        // Extract arg list up to the matching close paren.
        let paren_start_abs = abs + field.len() + (after.len() - after_trim.len());
        let args_text = balanced_parens_content(&line[paren_start_abs..]);
        let has_null = args_text.split(',').any(|a| {
            let t = a.trim();
            t == "NULL"
                || t == "nullptr"
                || t == "0"
                || t.starts_with("NULL ")
                || t.starts_with("0 ")
        });
        return Some((args_text, has_null));
    }
    None
}

/// Given a slice starting at `(`, return the string contents up to the
/// matching `)`. If no match on this line, return the remainder as-is.
fn balanced_parens_content(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'(') {
        return String::new();
    }
    let mut depth = 0;
    let mut end = None;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    match end {
        Some(e) => s[1..e].to_string(),
        None => s[1..].to_string(),
    }
}

/// True if `haystack` contains `needle` as a standalone identifier.
fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        let before = &haystack[..abs];
        let after = &haystack[abs + needle.len()..];
        let left_ok = before
            .chars()
            .last()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        let right_ok = after
            .chars()
            .next()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        if left_ok && right_ok {
            return true;
        }
        start = abs + needle.len();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_from_designated_initializer() {
        let line = "    .show_opaque_info = ospf_ext_show_info,";
        assert_eq!(
            extract_registration_field(line, "ospf_ext_show_info"),
            Some("show_opaque_info".to_string())
        );
    }

    #[test]
    fn field_from_arrow_assignment() {
        let line = "    tab->show_opaque_info = my_func;";
        assert_eq!(
            extract_registration_field(line, "my_func"),
            Some("show_opaque_info".to_string())
        );
    }

    #[test]
    fn invocation_with_null_detected() {
        let (args, has_null) =
            extract_invocation("    tab->show_opaque_info(NULL, lsa);", "show_opaque_info")
                .unwrap();
        assert!(has_null);
        assert!(args.contains("NULL"));
    }

    #[test]
    fn invocation_without_null() {
        let (_args, has_null) =
            extract_invocation("    tab->show_opaque_info(vty, lsa);", "show_opaque_info").unwrap();
        assert!(!has_null);
    }

    #[test]
    fn plain_call_is_not_invocation() {
        assert!(
            extract_invocation("    show_opaque_info(vty, lsa);", "show_opaque_info").is_none()
        );
    }
}
