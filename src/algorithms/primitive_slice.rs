//! Primitive Slice — deterministic security-primitive fingerprint detection.
//!
//! Scans diff-touched files (whole file, not just diff lines) for small AST
//! shapes whose security relevance is mechanical and well-known. Emits
//! pre-labelled `SliceFinding`s with a `rule_id` category, so the downstream
//! reviewer's job collapses from "remember all the rules, scan, decide" to
//! "verify or dispute this specific finding."
//!
//! Seed rules:
//!
//! * `HASH_TRUNCATED_BELOW_128_BITS` — direct:  `<digest>.hexdigest()[:N]`
//!   with `N < 32`, or `<digest>.digest()[:N]` with `N < 16`.
//! * `HASH_TRUNCATION_VIA_CALL`     — two-pass: a function whose body is
//!   `<digest>.hexdigest()[:PARAM]` is called with a literal int `N < 32`
//!   at `PARAM`'s position (either positional or keyword). This is the
//!   T1-005 / OpenWrt `get_str_hash(..., 12)` case — the literal `[:12]`
//!   never appears in the source; it's `[:length]` with `length=12` at the
//!   call site.
//! * `WEAK_HASH_FOR_IDENTITY`       — `hashlib.md5(...)` or
//!   `hashlib.sha1(...)` whose result is assigned to, or used as, a name
//!   matching `*_id`, `*_key`, `*_hash`, `*_token`, `cache*`, `session*`.
//! * `SHELL_TRUE_WITH_INTERPOLATION`— `subprocess.{run,Popen,call,check_call,check_output}`
//!   or `os.system(` with `shell=True` AND the command arg contains an
//!   f-string, `.format(`, or `%` interpolation.
//! * `CERT_VALIDATION_DISABLED`     — `verify=False`, `ssl.CERT_NONE`,
//!   `_create_unverified_context(`, `CURLOPT_SSL_VERIFYPEER, 0`.
//! * `HARDCODED_SECRET`             — assignment `NAME = "literal"` where
//!   `NAME` matches `*token*`, `*secret*`, `*password*`, `*api_key*`,
//!   `*apikey*`, `*access_key*`, `*private_key*` (case-insensitive) and the
//!   literal is non-empty / non-placeholder.
//!
//! Severity is seeded by blast radius, not diff proximity:
//!
//! * primitive on a diff line                       → `concern`
//! * primitive in a function that also contains a
//!   diff line (i.e. a reviewed function)           → `concern`
//! * primitive elsewhere in a diff-touched file     → `suggestion`
//!
//! The reviewer calibrates final severity from the downstream usage (hash
//! feeding a cache key vs local checksum, subprocess taking user input vs
//! static args, etc.).

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// One function that truncates a digest via a parameter, recorded during
/// pass 1 of the hash-truncation rule.
#[derive(Debug, Clone)]
struct TruncatingFunc {
    file: String,
    name: String,
    /// Zero-based position of the truncation parameter.
    param_index: usize,
    param_name: String,
    /// Which digest kind the body uses: `Hex` = `hexdigest()[:N]` (N counts
    /// hex chars → threshold 32); `Raw` = `digest()[:N]` (N counts bytes →
    /// threshold 16).
    digest_kind: DigestKind,
    def_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DigestKind {
    Hex,
    Raw,
}

impl DigestKind {
    fn threshold(&self) -> usize {
        match self {
            Self::Hex => 32,
            Self::Raw => 16,
        }
    }
    fn unit(&self) -> &'static str {
        match self {
            Self::Hex => "hex chars",
            Self::Raw => "bytes",
        }
    }
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::PrimitiveSlice);
    let mut block_id = 0usize;

    // Pass 1 (cross-file within the diff set): find truncating functions.
    let truncating = collect_truncating_functions(files, diff);

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(p) => p,
            None => continue,
        };

        // Pre-compute "reviewed function" line ranges — any function in this
        // file that contains at least one diff line.
        let dirty_ranges = dirty_function_ranges(parsed, diff_info);

        // Scan each line of the whole file (scope: diff-touched files).
        for (idx, line_text) in parsed.source.lines().enumerate() {
            let line_num = idx + 1;
            let code = strip_comment(line_text, parsed.language);

            scan_line_for_rules(
                parsed,
                &diff_info.file_path,
                diff_info,
                line_num,
                code,
                line_text,
                &dirty_ranges,
                &truncating,
                &mut result,
                &mut block_id,
            );
        }
    }

    // Rule 1b (two-pass): scan diff-touched files for call sites of
    // truncating functions.
    scan_call_sites_for_truncation(files, diff, &truncating, &mut result, &mut block_id);

    Ok(result)
}

fn classify_severity(
    line: usize,
    diff_info: &DiffInfo,
    dirty_ranges: &[(usize, usize)],
) -> &'static str {
    if diff_info.diff_lines.contains(&line) {
        return "concern";
    }
    if dirty_ranges.iter().any(|(s, e)| *s <= line && line <= *e) {
        return "concern";
    }
    "suggestion"
}

fn dirty_function_ranges(parsed: &ParsedFile, diff_info: &DiffInfo) -> Vec<(usize, usize)> {
    let mut ranges = BTreeSet::new();
    for &line in &diff_info.diff_lines {
        if let Some(func) = parsed.enclosing_function(line) {
            let (s, e) = parsed.node_line_range(&func);
            ranges.insert((s, e));
        }
    }
    ranges.into_iter().collect()
}

// --- Rule dispatch ---------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn scan_line_for_rules(
    _parsed: &ParsedFile,
    file: &str,
    diff_info: &DiffInfo,
    line_num: usize,
    code: &str,
    raw_line: &str,
    dirty_ranges: &[(usize, usize)],
    _truncating: &[TruncatingFunc],
    result: &mut SliceResult,
    block_id: &mut usize,
) {
    // --- Rule 1a: direct hash truncation with integer literal ---
    if let Some((kind, n)) = detect_direct_truncation(code) {
        if n < kind.threshold() {
            let severity = classify_severity(line_num, diff_info, dirty_ranges);
            let bits = match kind {
                DigestKind::Hex => n * 4,
                DigestKind::Raw => n * 8,
            };
            let description = format!(
                "Digest truncated to {} {} (~{} bits). Below the 128-bit threshold required for cryptographic identifiers / cache keys: birthday-bound collisions become practical at realistic fleet sizes (one user's artifact served to another). If the value is used as a content-hash / non-security checksum only, dispute — otherwise widen the slice (use the full digest) or switch to a keyed construction.",
                n,
                kind.unit(),
                bits,
            );
            push_finding(
                result,
                block_id,
                file,
                line_num,
                severity,
                description,
                "HASH_TRUNCATED_BELOW_128_BITS",
                raw_line,
            );
        }
    }

    // --- Rule 2: weak hash (md5/sha1) flowing into identity-shaped name ---
    if let Some((algo, target)) = detect_weak_hash_identity(code) {
        let severity = classify_severity(line_num, diff_info, dirty_ranges);
        let description = format!(
            "{} digest flowing into an identity-shaped name (`{}`). MD5/SHA-1 are broken against collision resistance; using them as identifiers/cache keys/tokens exposes the same confusion-attack class as hash truncation. If this is a non-security fingerprint (content-addressed cache, cross-process dedupe with no trust boundary), dispute — otherwise switch to SHA-256+.",
            algo, target
        );
        push_finding(
            result,
            block_id,
            file,
            line_num,
            severity,
            description,
            "WEAK_HASH_FOR_IDENTITY",
            raw_line,
        );
    }

    // --- Rule 3: subprocess shell=True with interpolation ---
    if detect_shell_true_with_interpolation(code) {
        let severity = classify_severity(line_num, diff_info, dirty_ranges);
        let description = "Shell command built from an interpolated string with `shell=True`. If any component of the command is user-influenced (direct input, header, filename, downstream value), this is shell-injection-by-default. Prefer list-form args (`shell=False`) or `shlex.quote()` each interpolated field. Dispute only if every interpolated value is provably constant or already-shell-quoted at the boundary.".to_string();
        push_finding(
            result,
            block_id,
            file,
            line_num,
            severity,
            description,
            "SHELL_TRUE_WITH_INTERPOLATION",
            raw_line,
        );
    }

    // --- Rule 4: cert validation disabled ---
    if let Some(marker) = detect_cert_validation_disabled(code) {
        let severity = classify_severity(line_num, diff_info, dirty_ranges);
        let description = format!(
            "TLS peer-certificate validation disabled via `{}`. Traffic to this endpoint is vulnerable to active MITM; any credentials, tokens, or integrity assumptions on the response are void. Dispute only if the endpoint is loopback / a pinned local-only socket.",
            marker
        );
        push_finding(
            result,
            block_id,
            file,
            line_num,
            severity,
            description,
            "CERT_VALIDATION_DISABLED",
            raw_line,
        );
    }

    // --- Rule 5: hardcoded secrets ---
    if let Some((name, preview)) = detect_hardcoded_secret(code) {
        let severity = classify_severity(line_num, diff_info, dirty_ranges);
        let description = format!(
            "Credential-shaped name `{}` assigned to a string literal (`{}`). If this value is a real secret it is now in source control / build artefacts / log scrapes. Replace with an env lookup / secret manager. Dispute if the literal is a placeholder, test vector, or public constant (document which).",
            name, preview
        );
        push_finding(
            result,
            block_id,
            file,
            line_num,
            severity,
            description,
            "HARDCODED_SECRET",
            raw_line,
        );
    }
}

// --- Rule 1a: direct truncation --------------------------------------------

/// Match `.hexdigest()[:N]` or `.digest()[:N]` where N is a literal int.
fn detect_direct_truncation(code: &str) -> Option<(DigestKind, usize)> {
    let patterns = [
        (".hexdigest()[:", DigestKind::Hex),
        (".hexdigest() [:", DigestKind::Hex),
        (".digest()[:", DigestKind::Raw),
        (".digest() [:", DigestKind::Raw),
    ];
    for (pat, kind) in patterns {
        if let Some(pos) = code.find(pat) {
            let rest = &code[pos + pat.len()..];
            let end = rest.find(']')?;
            let num_text = rest[..end].trim();
            if let Ok(n) = num_text.parse::<usize>() {
                return Some((kind, n));
            }
        }
    }
    None
}

// --- Rule 1b: two-pass truncation via call ---------------------------------

fn collect_truncating_functions(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> Vec<TruncatingFunc> {
    let mut out = Vec::new();
    let diff_files: BTreeSet<&str> = diff.files.iter().map(|f| f.file_path.as_str()).collect();

    for (file_path, parsed) in files {
        if !diff_files.contains(file_path.as_str()) {
            continue;
        }
        if !matches!(parsed.language, Language::Python) {
            continue;
        }
        for func in parsed.all_functions() {
            let name_node = match parsed.language.function_name(&func) {
                Some(n) => n,
                None => continue,
            };
            let func_name = parsed.node_text(&name_node).to_string();
            let params = parsed.function_parameter_names(&func);
            if params.is_empty() {
                continue;
            }
            let body_text = parsed.node_text(&func);
            for (idx, pname) in params.iter().enumerate() {
                if pname.is_empty() {
                    continue;
                }
                let hex_pat = format!(".hexdigest()[:{}]", pname);
                let hex_pat_sp = format!(".hexdigest()[: {}]", pname);
                let raw_pat = format!(".digest()[:{}]", pname);
                let raw_pat_sp = format!(".digest()[: {}]", pname);
                if body_text.contains(&hex_pat) || body_text.contains(&hex_pat_sp) {
                    let (def_line, _) = parsed.node_line_range(&func);
                    out.push(TruncatingFunc {
                        file: file_path.clone(),
                        name: func_name.clone(),
                        param_index: idx,
                        param_name: pname.clone(),
                        digest_kind: DigestKind::Hex,
                        def_line,
                    });
                    break;
                }
                if body_text.contains(&raw_pat) || body_text.contains(&raw_pat_sp) {
                    let (def_line, _) = parsed.node_line_range(&func);
                    out.push(TruncatingFunc {
                        file: file_path.clone(),
                        name: func_name.clone(),
                        param_index: idx,
                        param_name: pname.clone(),
                        digest_kind: DigestKind::Raw,
                        def_line,
                    });
                    break;
                }
            }
        }
    }
    out
}

fn scan_call_sites_for_truncation(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    truncating: &[TruncatingFunc],
    result: &mut SliceResult,
    block_id: &mut usize,
) {
    if truncating.is_empty() {
        return;
    }
    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(p) => p,
            None => continue,
        };
        if !matches!(parsed.language, Language::Python) {
            continue;
        }
        let dirty_ranges = dirty_function_ranges(parsed, diff_info);
        let src = parsed.source.as_str();

        for tf in truncating {
            let needle = format!("{}(", tf.name);
            let mut search_from = 0;
            while let Some(pos) = src[search_from..].find(&needle) {
                let abs = search_from + pos;
                // Reject if it's actually the function *definition* (preceded by `def `).
                let line_start = src[..abs].rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_prefix = &src[line_start..abs];
                if line_prefix.trim_end().ends_with("def") {
                    search_from = abs + needle.len();
                    continue;
                }
                // Make sure the char before `needle` is not an identifier char.
                let left_ok = src[..abs]
                    .chars()
                    .last()
                    .map(|c| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or(true);
                if !left_ok {
                    search_from = abs + needle.len();
                    continue;
                }
                // Extract argument list (balanced parens, across lines).
                let args_start = abs + needle.len() - 1; // position of `(`
                let args = balanced_args(&src[args_start..]);
                let arg_list = split_top_level_commas(&args);
                let literal = arg_at_position(&arg_list, tf.param_index, &tf.param_name);
                if let Some(n) = literal {
                    if n < tf.digest_kind.threshold() {
                        let line_num = 1 + src[..abs].matches('\n').count();
                        let severity = classify_severity(line_num, diff_info, &dirty_ranges);
                        let bits = match tf.digest_kind {
                            DigestKind::Hex => n * 4,
                            DigestKind::Raw => n * 8,
                        };
                        let description = format!(
                            "Call to `{}(…, {}={}) `: the callee truncates its digest to the value of `{}` ({} {}). At {} the output is {} bits — below the 128-bit threshold for cryptographic identifiers / cache keys. {}:{} contains the truncation site (`{}`). Birthday-bound collisions become practical; if the value is used as a content-hash only, dispute — otherwise widen the literal (32 hex chars / 16 raw bytes) or switch to a keyed construction.",
                            tf.name,
                            tf.param_name,
                            n,
                            tf.param_name,
                            n,
                            tf.digest_kind.unit(),
                            n,
                            bits,
                            tf.file,
                            tf.def_line,
                            tf.name,
                        );
                        let raw = parsed.source.lines().nth(line_num - 1).unwrap_or("");
                        let mut finding = make_finding(
                            &diff_info.file_path,
                            line_num,
                            severity,
                            description,
                            "HASH_TRUNCATION_VIA_CALL",
                            raw,
                        );
                        finding.related_files = if tf.file != diff_info.file_path {
                            vec![tf.file.clone()]
                        } else {
                            vec![]
                        };
                        finding.related_lines = vec![tf.def_line];
                        result.findings.push(finding);

                        // Block: callsite + callee def (first 8 lines).
                        let mut block = DiffBlock::new(
                            *block_id,
                            diff_info.file_path.clone(),
                            ModifyType::Modified,
                        );
                        block.add_line(&diff_info.file_path, line_num, false);
                        for l in tf.def_line..=(tf.def_line + 12) {
                            block.add_line(&tf.file, l, false);
                        }
                        if !block.file_line_map.is_empty() {
                            result.blocks.push(block);
                            *block_id += 1;
                        }
                    }
                }
                search_from = abs + needle.len();
            }
        }
    }
}

/// Given a slice starting at `(`, return the string between the opening and
/// matching closing paren (balanced across newlines).
fn balanced_args(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'(') {
        return String::new();
    }
    let mut depth = 0i32;
    let mut in_str: Option<u8> = None;
    let mut end = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
        } else {
            match b {
                b'"' | b'\'' => in_str = Some(b),
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
        i += 1;
    }
    match end {
        Some(e) => s[1..e].to_string(),
        None => s[1..].to_string(),
    }
}

/// Split a call-argument blob on top-level commas (respecting nested parens,
/// brackets, braces, and quoted strings).
fn split_top_level_commas(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str: Option<u8> = None;
    let bytes = args.as_bytes();
    let mut last = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
        } else {
            match b {
                b'"' | b'\'' => in_str = Some(b),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                b',' if depth == 0 => {
                    out.push(args[last..i].trim().to_string());
                    last = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    let tail = args[last..].trim().to_string();
    if !tail.is_empty() || !out.is_empty() {
        out.push(tail);
    }
    out
}

/// Extract the integer literal at `pos` (positional) or at keyword
/// `param_name=...`. Returns `None` if the arg isn't a bare integer literal.
fn arg_at_position(args: &[String], pos: usize, param_name: &str) -> Option<usize> {
    // Keyword form wins (most explicit).
    let kw_prefix = format!("{}=", param_name);
    for a in args {
        if let Some(rest) = a.strip_prefix(&kw_prefix) {
            return rest.trim().parse::<usize>().ok();
        }
    }
    // Positional form: arg at `pos`, must not contain `=` (that'd be a kwarg
    // for some other parameter, which means `pos` slot was default-filled).
    let a = args.get(pos)?;
    if a.contains('=') {
        return None;
    }
    a.trim().parse::<usize>().ok()
}

// --- Rule 2: weak hash for identity ----------------------------------------

fn detect_weak_hash_identity(code: &str) -> Option<(&'static str, String)> {
    let (algo, algo_pat): (&str, &str) = if code.contains("hashlib.md5(") {
        ("MD5", "hashlib.md5(")
    } else if code.contains("hashlib.sha1(") {
        ("SHA-1", "hashlib.sha1(")
    } else {
        return None;
    };
    // Take the assignment LHS on the same line if present.
    let before = &code[..code.find(algo_pat).unwrap()];
    let lhs = before.split('=').next().map(|s| s.trim()).unwrap_or("");
    if lhs.is_empty() {
        return None;
    }
    // Support attribute access (e.g. `self.cache_key = hashlib.md5(...)`).
    let name = lhs.rsplit('.').next().unwrap_or(lhs);
    let lower = name.to_ascii_lowercase();
    let tokens = [
        "_id",
        "_key",
        "_hash",
        "_token",
        "cache",
        "session",
        "ident",
        "fingerprint",
    ];
    if tokens.iter().any(|t| lower.contains(t)) {
        Some((algo, name.to_string()))
    } else {
        None
    }
}

// --- Rule 3: shell=True with interpolation ---------------------------------

fn detect_shell_true_with_interpolation(code: &str) -> bool {
    let shell_callees = [
        "subprocess.run(",
        "subprocess.Popen(",
        "subprocess.call(",
        "subprocess.check_call(",
        "subprocess.check_output(",
        "os.system(",
    ];
    let has_shell_call = shell_callees.iter().any(|p| code.contains(p));
    if !has_shell_call {
        return false;
    }
    // os.system is always shell-level; subprocess.* needs explicit shell=True.
    let shell_true =
        code.contains("shell=True") || code.contains("shell = True") || code.contains("os.system(");
    if !shell_true {
        return false;
    }
    // Interpolation markers.
    code.contains("f\"") || code.contains("f'") || code.contains(".format(") || code.contains(" % ")
}

// --- Rule 4: cert validation disabled --------------------------------------

fn detect_cert_validation_disabled(code: &str) -> Option<&'static str> {
    let markers = [
        "verify=False",
        "verify = False",
        "ssl.CERT_NONE",
        "_create_unverified_context(",
        "CURLOPT_SSL_VERIFYPEER, 0",
        "CURLOPT_SSL_VERIFYHOST, 0",
        "check_hostname=False",
        "rejectUnauthorized: false",
        "InsecureSkipVerify: true",
    ];
    markers.iter().copied().find(|m| code.contains(m))
}

// --- Rule 5: hardcoded secret ----------------------------------------------

fn detect_hardcoded_secret(code: &str) -> Option<(String, String)> {
    // Look for NAME = "literal" at top of the line (allow indentation).
    let trimmed = code.trim_start();
    if trimmed.starts_with('#') || trimmed.is_empty() {
        return None;
    }
    let eq_pos = trimmed.find('=')?;
    // Rule out ==, !=, <=, >=, etc.
    let rest = &trimmed[eq_pos..];
    if rest.starts_with("==") || rest.starts_with("=>") {
        return None;
    }
    let lhs = trimmed[..eq_pos].trim().trim_end_matches(':').trim();
    let rhs = trimmed[eq_pos + 1..].trim();

    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    // Extract the rightmost name component (supports `self.secret`).
    let name = lhs.rsplit('.').next().unwrap_or(lhs);
    // Strip trailing type annotation like `API_KEY: str = "..."`.
    let name = name.split(':').next().unwrap_or(name).trim();
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    {
        return None;
    }
    let lname = name.to_ascii_lowercase();
    let secret_tokens = [
        "token",
        "secret",
        "password",
        "passwd",
        "api_key",
        "apikey",
        "access_key",
        "private_key",
        "auth_key",
        "session_key",
    ];
    if !secret_tokens.iter().any(|t| lname.contains(t)) {
        return None;
    }
    // RHS must be a string literal.
    let (q, body_start) = if rhs.starts_with('"') {
        ('"', 1)
    } else if rhs.starts_with('\'') {
        ('\'', 1)
    } else {
        return None;
    };
    let body_end = rhs[body_start..].find(q)? + body_start;
    let body = &rhs[body_start..body_end];
    if body.is_empty() {
        return None;
    }
    // Placeholders / obvious non-secrets.
    let placeholders = [
        "",
        "none",
        "null",
        "changeme",
        "changeit",
        "your-token",
        "your_token",
        "example",
        "placeholder",
        "xxx",
        "todo",
        "fixme",
    ];
    let body_lower = body.to_ascii_lowercase();
    if placeholders.iter().any(|p| &body_lower == p) {
        return None;
    }
    // Env-lookup shape (`os.getenv(...)`, `os.environ[...]`) — RHS wouldn't
    // actually be a string literal here; skip.
    if body.contains("${") {
        return None;
    }
    let preview = if body.len() > 20 {
        format!("{}…", &body[..20])
    } else {
        body.to_string()
    };
    Some((name.to_string(), preview))
}

// --- Helpers ---------------------------------------------------------------

fn strip_comment<'a>(line: &'a str, lang: Language) -> &'a str {
    match lang {
        Language::Python | Language::Bash => {
            if let Some(i) = line.find('#') {
                &line[..i]
            } else {
                line
            }
        }
        _ => {
            if let Some(i) = line.find("//") {
                &line[..i]
            } else {
                line
            }
        }
    }
}

fn push_finding(
    result: &mut SliceResult,
    block_id: &mut usize,
    file: &str,
    line: usize,
    severity: &str,
    description: String,
    rule_id: &str,
    raw_line: &str,
) {
    result.findings.push(make_finding(
        file,
        line,
        severity,
        description,
        rule_id,
        raw_line,
    ));
    let mut block = DiffBlock::new(*block_id, file.to_string(), ModifyType::Modified);
    block.add_line(file, line, false);
    result.blocks.push(block);
    *block_id += 1;
}

fn make_finding(
    file: &str,
    line: usize,
    severity: &str,
    description: String,
    rule_id: &str,
    _raw_line: &str,
) -> SliceFinding {
    SliceFinding {
        algorithm: "primitive".to_string(),
        file: file.to_string(),
        line,
        severity: severity.to_string(),
        description,
        function_name: None,
        related_lines: vec![],
        related_files: vec![],
        category: Some(rule_id.to_string()),
        parse_quality: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_trunc_hex_below_threshold() {
        let (k, n) = detect_direct_truncation("    return h.hexdigest()[:12]").unwrap();
        assert_eq!(n, 12);
        assert!(matches!(k, DigestKind::Hex));
    }

    #[test]
    fn direct_trunc_hex_at_threshold_none() {
        // 32 is exactly 128 bits → not under threshold, so detect still matches
        // but caller filters. We only check the parse succeeds here.
        let (_k, n) = detect_direct_truncation("x = h.hexdigest()[:32]").unwrap();
        assert_eq!(n, 32);
    }

    #[test]
    fn weak_hash_identity() {
        let (a, name) =
            detect_weak_hash_identity("    cache_key = hashlib.md5(b).hexdigest()").unwrap();
        assert_eq!(a, "MD5");
        assert_eq!(name, "cache_key");
    }

    #[test]
    fn weak_hash_non_identity_none() {
        assert!(detect_weak_hash_identity("    checksum = hashlib.md5(b).hexdigest()").is_none());
    }

    #[test]
    fn shell_true_with_fstring() {
        assert!(detect_shell_true_with_interpolation(
            "subprocess.run(f\"cp {src} {dst}\", shell=True)"
        ));
    }

    #[test]
    fn shell_true_without_interp_none() {
        assert!(!detect_shell_true_with_interpolation(
            "subprocess.run(\"ls -la\", shell=True)"
        ));
    }

    #[test]
    fn cert_verify_false() {
        assert_eq!(
            detect_cert_validation_disabled("    requests.get(url, verify=False)"),
            Some("verify=False")
        );
    }

    #[test]
    fn hardcoded_api_key() {
        let (name, _) = detect_hardcoded_secret("API_KEY = \"sk-real-looking-1234\"").unwrap();
        assert_eq!(name, "API_KEY");
    }

    #[test]
    fn hardcoded_placeholder_rejected() {
        assert!(detect_hardcoded_secret("TOKEN = \"changeme\"").is_none());
    }

    #[test]
    fn args_split_basic() {
        let args = split_top_level_commas(" \"a b\" , 12 ");
        assert_eq!(args, vec!["\"a b\"".to_string(), "12".to_string()]);
    }

    #[test]
    fn args_split_keyword() {
        let args = split_top_level_commas("s, length=12");
        assert_eq!(args, vec!["s".to_string(), "length=12".to_string()]);
    }

    #[test]
    fn arg_at_pos_keyword_wins() {
        let args = vec!["s".to_string(), "length=12".to_string()];
        assert_eq!(arg_at_position(&args, 1, "length"), Some(12));
    }

    #[test]
    fn arg_at_pos_positional() {
        let args = vec!["s".to_string(), "12".to_string()];
        assert_eq!(arg_at_position(&args, 1, "length"), Some(12));
    }
}
