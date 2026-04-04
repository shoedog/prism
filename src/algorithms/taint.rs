//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Built-in taint sink patterns matched against AST identifier nodes.
///
/// **Matching convention** (see `matches_sink()`):
/// - Bare patterns (e.g., `"exec"`) use substring matching: `"exec"` matches `execFile`.
/// - `=`-prefixed patterns (e.g., `"=open"`) require an exact identifier match:
///   `"=open"` matches `open` but not `openFile` or `openConnection`.
const SINK_PATTERNS: &[&str] = &[
    // === Cross-language / generic ===
    "exec",
    "eval",
    "system",
    "query", // NOTE: also in provenance DATABASE_PATTERNS — intentional (both a data source and a sink)
    "execute",
    "raw_sql",
    "=open", // exact to avoid "openFile", "openConnection"; still fires on os.open (tree-sitter splits it)
    "write",
    "unlink",
    "remove",
    "rmdir",
    "send",
    "respond",
    "render",
    "redirect",
    // === C/C++ buffer overflow / unsafe string operations ===
    // Note: identifiers don't include '(' so patterns must be bare names.
    "strcpy",
    "strcat",
    "strncpy",
    "sprintf",
    "vsprintf",
    "gets",
    "scanf",
    "memcpy",
    "memmove",
    // C/C++ command / library injection
    "execv",
    "execve",
    "execvp",
    "dlopen",
    // C/C++ memory safety
    "free",
    // C/C++ format string sinks
    "printf",
    "fprintf",
    "snprintf",
    // C/C++ va_list format string sinks
    "vprintf",
    "vfprintf",
    "vsprintf",
    "vsnprintf",
    // === Python ===
    // Deserialization (arbitrary code execution)
    "=loads", // pickle.loads, marshal.loads, yaml.loads (exact to avoid "downloads", "preloads")
    "=load",  // pickle.load, yaml.load (exact to avoid "download", "upload")
    // Process execution
    "=Popen", // subprocess.Popen (exact; "subprocess" omitted — too generic)
    "=popen", // os.popen
    // Dynamic code execution
    "=compile", // compile() — creates executable code objects (exact to avoid "compiled")
    // Template injection
    "render_template_string", // Flask — renders user-supplied template
    "mark_safe",              // Django — marks string as safe HTML (bypass escaping)
    "=Markup",                // Jinja2/Flask — raw HTML wrapper (exact to avoid "markup")
    // Dynamic attribute access with untrusted names
    "=getattr", // (exact to avoid "getAttributes")
    "=setattr",
    // === JavaScript / TypeScript ===
    // DOM XSS sinks
    "innerHTML",
    "outerHTML",
    "dangerouslySetInnerHTML",
    "insertAdjacentHTML",
    // Dynamic code execution
    "Function", // new Function('return ' + userInput)
    // Command execution (Node.js child_process)
    "spawn",     // child_process.spawn
    "execFile",  // child_process.execFile
    "execSync",  // child_process.execSync
    "spawnSync", // child_process.spawnSync
    // File operations (Node.js fs)
    "writeFile",
    "writeFileSync",
    "unlinkSync",
    "rmdirSync",
    "appendFile",
    "appendFileSync",
    // SQL injection (ORM raw queries)
    "=raw",     // knex.raw() (exact to avoid "rawData", "withdrawal", "drawLine")
    "=literal", // Sequelize.literal()
    // === Go ===
    // Command execution
    "Command", // exec.Command
    "Exec",    // os.Exec, db.Exec
    // Template injection / XSS
    "=HTML",   // template.HTML() — exact to avoid "HTMLEscapeString", "HTMLAttr"
    "Fprintf", // fmt.Fprintf(w, userInput) — reflected XSS / format string
    "Sprintf", // fmt.Sprintf(userInput) — format string injection
    // File operations
    "Remove",    // os.Remove
    "RemoveAll", // os.RemoveAll
    "WriteFile", // os.WriteFile
    "Rename",    // os.Rename
    "Chmod",     // os.Chmod
    // SQL
    "Query",    // sql.Query (already covered by lowercase "query")
    "QueryRow", // sql.QueryRow
    // === Rust ===
    // Unsafe memory operations with tainted data
    "=transmute",          // std::mem::transmute — type-unsafe cast
    "from_raw_parts",      // slice::from_raw_parts — raw pointer to slice
    "=write_volatile",     // ptr::write_volatile — unchecked memory write
    "=read_volatile",      // ptr::read_volatile — unchecked memory read
    "from_utf8_unchecked", // String::from_utf8_unchecked — no validation
    // Command execution
    // "Command" already covered by Go section (exec.Command / std::process::Command)
    // File operations
    "set_permissions", // std::fs::set_permissions
    // SQL (diesel/sqlx)
    "sql_query", // diesel::sql_query — raw SQL
    "query_as",  // sqlx::query_as
    // Deserialization
    "=deserialize", // serde deserialize with untrusted input
    // FFI boundary
    "=CString", // CString::new — FFI string, null handling
    "=CStr",    // CStr::from_ptr — raw pointer to string
    // === Lua ===
    // Dynamic code execution (code injection)
    "=loadstring", // loadstring(user_input) — executes arbitrary Lua code
    "=dofile",     // dofile(path) — loads and executes a Lua file
    "=loadfile",   // loadfile(path) — loads a Lua file as a function
    // Command execution
    // "execute" already covered by generic; os.execute -> identifier "execute"
    // "=popen" already covered by Python section; io.popen -> identifier "popen"
    // Note: Lua string.format injection is a niche concern. Tree-sitter splits
    // "string.format" into separate identifier nodes, so substring sink matching
    // can't catch it. The high-severity Lua paths (loadstring, dofile, execute)
    // are already covered above.
    // === Terraform / HCL ===
    // Security-sensitive resource attributes where tainted variables can cause issues.
    // These are attribute names in resource blocks, not function calls.
    "cidr_blocks",          // Network ACL — tainted CIDRs open firewall holes
    "ipv6_cidr_blocks",     // IPv6 variant of above
    "ingress",              // Security group ingress rules
    "egress",               // Security group egress rules
    "=policy",              // IAM policy documents — tainted values grant unintended permissions
    "assume_role_policy",   // IAM assume role policy
    "user_data",            // EC2 user_data — shell injection vector
    "user_data_base64",     // Base64 variant of user_data
    "=inline",              // Provisioner inline commands
    "=command",             // Provisioner command execution
    "iam_instance_profile", // IAM instance profile attachment
    "role_arn",             // IAM role ARN — cross-account access
    // === Shell / Bash ===
    // Command injection sinks — where untrusted input causes code execution
    "=eval",   // eval "$VAR" — arbitrary code execution
    "=source", // source "$FILE" — code inclusion
    "xargs",   // echo $INPUT | xargs rm — argument injection
    "=su",     // su $USER — privilege escalation
    "=sudo",   // sudo $CMD — privilege escalation
    "=chmod",  // chmod $MODE $FILE — permission manipulation
    "=chown",  // chown $OWNER $FILE — ownership manipulation
    "sqlite3", // sqlite3 db "SELECT $INPUT" — SQL injection
    "=curl",   // curl $URL — SSRF / data exfiltration
    "=wget",   // wget $URL — SSRF / data exfiltration
    "=exec",   // exec $CMD — process replacement
    "=awk",    // awk "$PATTERN" — code injection in awk
    "=sed",    // sed "$EXPR" — code injection in sed
    "=find",   // find ... -exec — command injection via glob/args
    // === Busybox / Firmware shell ===
    // Flash and boot environment — can brick devices
    "=mtd",         // mtd write $IMAGE $PARTITION — flash write, wrong partition = bricked
    "=fw_setenv",   // fw_setenv $VAR $VAL — U-Boot env, can cause boot loop
    "=fw_printenv", // fw_printenv — reads boot env (lower risk, but info disclosure)
    // OpenWrt UCI config — persistent config injection
    "=uci", // uci set/commit with tainted values
    // Network interface and firewall — security bypass / disruption
    "=iptables",  // iptables $RULE — firewall manipulation
    "=ip6tables", // ip6tables $RULE — IPv6 firewall manipulation
    "=ifconfig",  // ifconfig $IFACE — network interface config
    "=ip",        // ip addr/route/link — iproute2 network config
    "=brctl",     // brctl addif/delif — bridge config, VLAN hopping
    "=bridge",    // bridge fdb/vlan — modern bridge config
    "=vconfig",   // vconfig add $IFACE $VLAN — VLAN segmentation bypass
    "=swconfig",  // swconfig set — switch chip L2 manipulation
    // Kernel module loading — rootkit installation vector
    "=insmod",   // insmod $MODULE — load kernel module
    "=modprobe", // modprobe $MODULE — load kernel module with deps
    "=rmmod",    // rmmod $MODULE — unload kernel module
    // Firmware daemon environment injection
    "procd_set_param", // procd_set_param env VAR=VAL — daemon config injection
    // === C/C++ kernel / embedded ===
    // User-space data ingress — kernel attack surface
    "copy_from_user", // Linux kernel: copies untrusted user-space data
    "get_user",       // Linux kernel: reads single value from user-space
    "__get_user",     // Linux kernel: unchecked user-space read
    "=ioctl",         // ioctl with user buffer — kernel I/O untrusted data path
];

/// Check whether an identifier text matches a sink pattern.
///
/// Most patterns use substring matching (e.g. "exec" matches "execFile").
/// Patterns prefixed with '=' require an exact identifier match
/// (e.g. "=raw" matches "raw" but not "rawData" or "withdrawal").
fn matches_sink(identifier: &str, pattern: &str) -> bool {
    if let Some(exact) = pattern.strip_prefix('=') {
        identifier == exact
    } else {
        identifier.contains(pattern)
    }
}
#[derive(Debug, Clone)]
pub struct TaintConfig {
    /// Explicit taint source locations.
    pub sources: Vec<(String, usize)>,
    /// If true, auto-taint all variables assigned on diff lines.
    pub taint_from_diff: bool,
    /// Additional sink patterns to check. Prefix with '=' for exact identifier match.
    pub extra_sinks: Vec<String>,
}

impl Default for TaintConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            taint_from_diff: true,
            extra_sinks: Vec::new(),
        }
    }
}

/// A taint finding: a path from source to sink.
#[derive(Debug, Clone)]
pub struct TaintFinding {
    pub source_file: String,
    pub source_line: usize,
    pub source_var: String,
    pub sink_file: String,
    pub sink_line: usize,
    pub sink_pattern: String,
    pub path_lines: Vec<(String, usize)>,
}

/// Detect variadic wrapper functions that forward arguments to known format string sinks.
///
/// Scans all parsed files for functions with a variadic parameter (`...`) that
/// call any known format string sink (vprintf, vfprintf, vsprintf, vsnprintf,
/// sprintf, snprintf, fprintf, printf). These wrappers should be treated as
/// sinks themselves, since the intraprocedural DFG cannot trace arguments
/// across function boundaries.
///
/// Returns wrapper function names as exact-match sink patterns (prefixed with `=`).
///
/// Known limitations:
/// - Only detects 1-hop wrappers. If `my_log(...)` calls `internal_log(...)` which
///   calls `vsnprintf`, only `internal_log` is detected. Could be extended by
///   iterating to a fixed point over discovered wrappers.
/// - A variadic function that calls printf for debug logging but whose `...` args
///   are unrelated to the printf call will be misclassified as a wrapper. Rare in
///   practice since most variadic+printf combos are genuine format wrappers.
fn detect_format_string_wrappers(files: &BTreeMap<String, ParsedFile>) -> Vec<String> {
    /// Format string sinks that variadic wrappers typically forward to.
    const FORMAT_SINKS: &[&str] = &[
        "vprintf",
        "vfprintf",
        "vsprintf",
        "vsnprintf",
        "sprintf",
        "snprintf",
        "fprintf",
        "printf",
    ];

    let mut wrappers = Vec::new();

    for parsed in files.values() {
        let func_types = parsed.language.function_node_types();
        let root = parsed.tree.root_node();
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            if func_types.contains(&node.kind()) {
                if parsed.is_variadic_function(&node) {
                    // Check if this function calls any format sink
                    let callees = parsed.callees_in_function(&node);
                    let calls_format_sink =
                        callees.iter().any(|c| FORMAT_SINKS.contains(&c.as_str()));
                    if calls_format_sink {
                        if let Some(name_node) = parsed.language.function_name(&node) {
                            let name = parsed.node_text(&name_node).to_string();
                            wrappers.push(format!("={}", name));
                        }
                    }
                }
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }
    }

    wrappers
}

/// Detect unquoted variable expansions in Bash command arguments.
///
/// In shell scripts, `$VAR` without quotes undergoes word splitting and glob
/// expansion, making it a command injection / path traversal vector. This
/// function walks tainted lines in Bash files and reports any `simple_expansion`
/// or `expansion` node that appears inside a `command` without being wrapped
/// in a `string` (double-quote) node.
///
/// Returns findings as (file, line, var_name) tuples.
fn detect_unquoted_expansions(
    files: &BTreeMap<String, ParsedFile>,
    tainted_lines: &BTreeMap<String, BTreeSet<usize>>,
) -> Vec<(String, usize, String)> {
    let mut findings = Vec::new();

    for (file, lines) in tainted_lines {
        let parsed = match files.get(file) {
            Some(p) if p.language == Language::Bash => p,
            _ => continue,
        };

        for &line in lines {
            find_unquoted_on_line(parsed, parsed.tree.root_node(), line, &mut findings, file);
        }
    }
    findings
}

/// Walk the AST looking for unquoted expansions on a specific line.
fn find_unquoted_on_line(
    parsed: &ParsedFile,
    node: tree_sitter::Node,
    target_line: usize,
    findings: &mut Vec<(String, usize, String)>,
    file: &str,
) {
    let node_line = node.start_position().row + 1;

    // Only descend into nodes that overlap our target line
    let node_end_line = node.end_position().row + 1;
    if node_line > target_line || node_end_line < target_line {
        return;
    }

    let kind = node.kind();

    // Found a variable expansion on our target line
    if (kind == "simple_expansion" || kind == "expansion") && node_line == target_line {
        // Walk up to check if we're inside a "string" (quoted) or directly in a "command"
        let mut parent = node.parent();
        let mut is_quoted = false;
        let mut in_command = false;
        while let Some(p) = parent {
            match p.kind() {
                "string" | "raw_string" => {
                    is_quoted = true;
                    break;
                }
                "command" => {
                    in_command = true;
                    break;
                }
                // Stop at statement boundaries
                "function_definition" | "program" | "subshell" => break,
                _ => {}
            }
            parent = p.parent();
        }

        if in_command && !is_quoted {
            // Extract variable name from the expansion
            let var_name = parsed.node_text(&node).to_string();
            findings.push((file.to_string(), target_line, var_name));
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_unquoted_on_line(parsed, child, target_line, findings, file);
    }
}

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    taint_config: &TaintConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);

    // Collect taint sources
    let mut taint_sources: Vec<(String, usize)> = taint_config.sources.clone();

    if taint_config.taint_from_diff {
        for diff_info in &diff.files {
            for &line in &diff_info.diff_lines {
                taint_sources.push((diff_info.file_path.clone(), line));
            }
        }
    }

    // Forward propagation from each source (CFG-constrained when available)
    let paths = ctx.cpg.taint_forward_cfg(&taint_sources);

    // Detect variadic wrapper functions and add them as dynamic sinks
    let wrapper_sinks = detect_format_string_wrappers(ctx.files);

    // Collect all tainted lines and identify sinks
    let mut all_tainted: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut sink_lines: BTreeSet<(String, usize)> = BTreeSet::new();

    let all_sinks: Vec<&str> = SINK_PATTERNS
        .iter()
        .copied()
        .chain(taint_config.extra_sinks.iter().map(|s| s.as_str()))
        .chain(wrapper_sinks.iter().map(|s| s.as_str()))
        .collect();

    for path in &paths {
        for edge in &path.edges {
            all_tainted
                .entry(edge.from.file.clone())
                .or_default()
                .insert(edge.from.line);
            all_tainted
                .entry(edge.to.file.clone())
                .or_default()
                .insert(edge.to.line);

            // Check if the target location involves a sink
            if let Some(parsed) = ctx.files.get(&edge.to.file) {
                let ids = parsed.identifiers_on_line(edge.to.line);
                for id in &ids {
                    let text = parsed.node_text(id);
                    if all_sinks.iter().any(|s| matches_sink(text, s)) {
                        sink_lines.insert((edge.to.file.clone(), edge.to.line));
                    }
                }
            }
        }
    }

    // Also check source lines for sinks (taint at source)
    for (file, line) in &taint_sources {
        all_tainted.entry(file.clone()).or_default().insert(*line);
    }

    // Emit findings for each taint sink reached
    for (file, line) in &sink_lines {
        // Find a source that reaches this sink (use first taint source as representative)
        let source_desc = taint_sources
            .iter()
            .find(|(sf, _)| sf == file)
            .map(|(_, sl)| format!("line {}", sl))
            .unwrap_or_else(|| "diff lines".to_string());
        result.findings.push(SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: "warning".to_string(),
            description: format!(
                "tainted value from {} reaches sink at line {}",
                source_desc, line
            ),
            function_name: None,
            related_lines: taint_sources
                .iter()
                .filter(|(sf, _)| sf == file)
                .map(|(_, sl)| *sl)
                .collect(),
            related_files: vec![],
            category: Some("tainted_value".to_string()),
        });
    }

    // Bash-specific: detect unquoted variable expansions on tainted lines.
    // In shell, unquoted $VAR in command arguments is a word-splitting / injection vector.
    let unquoted = detect_unquoted_expansions(ctx.files, &all_tainted);
    for (file, line, var_name) in &unquoted {
        // Avoid duplicate findings if line is already flagged as a sink
        if !sink_lines.contains(&(file.clone(), *line)) {
            sink_lines.insert((file.clone(), *line));
            result.findings.push(SliceFinding {
                algorithm: "taint".to_string(),
                file: file.clone(),
                line: *line,
                severity: "warning".to_string(),
                description: format!(
                    "unquoted expansion {} in command argument — word splitting / injection risk",
                    var_name,
                ),
                function_name: None,
                related_lines: taint_sources
                    .iter()
                    .filter(|(sf, _)| sf == file)
                    .map(|(_, sl)| *sl)
                    .collect(),
                related_files: vec![],
                category: Some("unquoted_expansion".to_string()),
            });
        }
    }

    // Build output blocks
    let mut block_id = 0;
    for (file, lines) in &all_tainted {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);

        for &line in lines {
            let is_source = taint_sources.iter().any(|(f, l)| f == file && *l == line);
            let is_sink = sink_lines.contains(&(file.clone(), line));
            // Mark sources and sinks as diff lines for highlighting
            block.add_line(file, line, is_source || is_sink);
        }

        if !lines.is_empty() {
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
