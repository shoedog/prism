//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

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
    "BUILD_FROM_FILE", // Dockerfile/build-system context injection
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
    "fopen", // C file open — path traversal / confused-deputy risk
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
    "=Popen",        // subprocess.Popen (exact; "subprocess" omitted — too generic)
    "=popen",        // os.popen
    "=run",          // subprocess.run (exact to avoid "running", "runner")
    "=check_call",   // subprocess.check_call
    "=check_output", // subprocess.check_output
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
    // === Logging sinks — format string injection ===
    "syslog",   // syslog(LOG_ERR, user_input) — format string injection
    "=openlog", // openlog(user_ident, ...) — ident string injection
    // === Network output — information leak / injection ===
    "sendto",  // sendto(fd, buf, ...) — untrusted data to network
    "sendmsg", // sendmsg(fd, &msg, ...) — untrusted data to network
    // === C/C++ kernel / embedded ===
    // User-space data ingress — kernel attack surface
    "copy_from_user", // Linux kernel: copies untrusted user-space data
    "get_user",       // Linux kernel: reads single value from user-space
    "__get_user",     // Linux kernel: unchecked user-space read
    "=ioctl",         // ioctl with user buffer — kernel I/O untrusted data path
    // Kernel copy-out — information leak to userspace
    "copy_to_user", // Linux kernel: copies potentially sensitive data to user-space
    "put_user",     // Linux kernel: writes single value to user-space
];

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 Go CWE-78 / CWE-22 structured sinks (spec §3.2 / §3.3).
//
// These coexist with `SINK_PATTERNS` above: the flat list uses substring
// identifier matching for cross-language coverage; the structured list below
// uses qualified call-path matching with optional `semantic_check` predicates
// for argument-shape discrimination (e.g., shell-wrapper detection).
//
// Both registries are consulted independently in the analysis pass.
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if `call`'s arguments at `name_idx` and `flag_idx` form a shell-wrapper
/// invocation (e.g. `("sh", "-c", ...)`).
///
/// Common Linux/Windows shells only; PowerShell (`pwsh`/`powershell.exe`) and exotic
/// absolute paths (`/usr/bin/sh`) deliberately NOT included per spec §3.2 scope note.
fn is_shell_wrapper_at(call: &CallSite, name_idx: usize, flag_idx: usize) -> bool {
    let name = call.literal_arg(name_idx).unwrap_or("");
    let flag = call.literal_arg(flag_idx).unwrap_or("");
    matches!(name, "sh" | "bash" | "cmd.exe" | "/bin/sh" | "/bin/bash")
        && matches!(flag, "-c" | "/c")
}

/// Adapter for `exec.Command("sh", "-c", X)`-shaped sinks
/// (function-pointer compatible — `semantic_check` is `Option<fn(...)>`, not a closure).
fn check_shell_wrapper(call: &CallSite) -> bool {
    is_shell_wrapper_at(call, 0, 1)
}

/// Adapter for `exec.CommandContext(ctx, "sh", "-c", X)`-shaped sinks
/// where the context arg shifts everything by one.
fn check_shell_wrapper_ctx(call: &CallSite) -> bool {
    is_shell_wrapper_at(call, 1, 2)
}

/// Cross-cutting Go CWE-78 (OS command injection) sinks. See spec §3.2.
///
/// Both `exec.Command` and `exec.CommandContext` appear twice:
/// - Once for the shell-wrapper form (`semantic_check` filters to shell calls);
///   tainted-arg index points at the `X` payload after `"-c"`.
/// - Once for the tainted-binary form (no `semantic_check`); tainted-arg index
///   is the binary-path argument itself.
///
/// `syscall.Exec(argv0, argv, envv)` checks both `argv0` (literal-or-tainted)
/// and the `argv` slice (DFG-conservative: any tainted slice element taints
/// the slice as a whole). Per-element tracking is out of scope for Phase 1.
pub const GO_CWE78_SINKS: &[SinkPattern] = &[
    // Shell-wrapped variants — payload is the arg after "-c".
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[2],
        semantic_check: Some(check_shell_wrapper),
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[3],
        semantic_check: Some(check_shell_wrapper_ctx),
    },
    // Tainted-binary variants — first non-ctx arg is the binary path.
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    // syscall.Exec — argv0 + argv slice.
    SinkPattern {
        call_path: "syscall.Exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];

/// Cross-cutting Go CWE-22 (path traversal) sinks. See spec §3.3.
///
/// `os.Rename(old, new)` checks both arguments; everything else is single-arg.
/// `filepath.Join` is *not* a sink — it's a path-construction primitive that
/// taint flows through; the downstream `os.*` call is what fires.
pub const GO_CWE22_SINKS: &[SinkPattern] = &[
    // Read sinks
    SinkPattern {
        call_path: "os.Open",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.OpenFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.ReadFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "ioutil.ReadFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    // Write sinks
    SinkPattern {
        call_path: "os.Create",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.WriteFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "ioutil.WriteFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    // Mutation sinks
    SinkPattern {
        call_path: "os.Remove",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.RemoveAll",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.Mkdir",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.MkdirAll",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.Rename",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];

/// GLib/D-Bus IPC accessor patterns.
///
/// These function-call patterns read values from IPC messages (D-Bus) or
/// GLib hash-tables populated from IPC. Any value returned is user-controlled
/// and constitutes a taint source for confused-deputy analysis.
const IPC_SOURCE_PATTERNS: &[&str] = &[
    "g_hash_table_lookup(",         // GLib hash table keyed on IPC-supplied data
    "g_variant_get_",               // GLib Variant D-Bus field accessor
    "g_variant_dup_",               // GLib Variant D-Bus field accessor (dup/alloc variant)
    "dbus_message_get_args(",       // libdbus raw message argument extraction
    "dbus_message_iter_get_basic(", // libdbus iterator-based argument extraction
];

/// Detect lines in diff-touched C/C++ files that match GLib/D-Bus IPC patterns.
///
/// Returns `(file_path, line_number)` pairs for every line that reads from an
/// IPC source. These are added to `taint_sources` so the engine can trace
/// confused-deputy flows (e.g. `str = g_hash_table_lookup(settings->data, "usercert")`
/// → `BUILD_FROM_FILE, str`).
///
/// Only processes files from the diff to avoid flooding unrelated files with
/// sources. Only processes C/C++ files because GLib/D-Bus is a C API.
fn detect_ipc_sources(ctx: &CpgContext, diff: &DiffInput) -> Vec<(String, usize)> {
    let diff_files: std::collections::BTreeSet<&str> =
        diff.files.iter().map(|f| f.file_path.as_str()).collect();
    let mut sources = Vec::new();

    for (file_path, parsed) in ctx.files {
        if !diff_files.contains(file_path.as_str()) {
            continue;
        }
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }
        for (idx, line_text) in parsed.source.lines().enumerate() {
            let line_num = idx + 1;
            if IPC_SOURCE_PATTERNS.iter().any(|p| line_text.contains(p)) {
                sources.push((file_path.clone(), line_num));
            }
        }
    }

    sources
}

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

// ─────────────────────────────────────────────────────────────────────────────
// Framework-aware source detection (spec §2.6 / §2.8 — pull model).
//
// For each Go file with a detected framework, walk every function definition
// in the file:
//   1. Collect parameter names whose type matches the framework's request type
//      (`*http.Request` for net/http and gorilla/mux; `*gin.Context` for gin).
//   2. For each `SourcePattern` in the framework spec, substitute each matched
//      parameter name into the pattern's `call_path` prefix.
//   3. Scan the function body for call expressions whose textual prefix matches
//      the substituted path. Each match's start line becomes a taint source.
//
// Patterns without a conventional prefix (like gorilla/mux's `mux.Vars`) are
// matched as-is — `mux.Vars(r)` is a free function that takes the request as
// an argument rather than living on a method receiver.
// ─────────────────────────────────────────────────────────────────────────────

/// Type strings that bind to a request-like parameter for each framework.
/// `*http.Request` covers net/http + gorilla/mux; `*gin.Context` covers gin.
fn framework_request_types(framework_name: &str) -> &'static [&'static str] {
    match framework_name {
        "gin" => &["*gin.Context"],
        "net/http" | "gorilla/mux" => &["*http.Request"],
        _ => &[],
    }
}

/// The conventional receiver-name prefixes a framework expects in its source
/// patterns. When the bound parameter name differs, we substitute these
/// prefixes textually. Patterns whose `call_path` starts with neither prefix
/// (e.g. gorilla/mux's `mux.Vars`) are matched without substitution.
fn framework_prefixes(framework_name: &str) -> &'static [&'static str] {
    match framework_name {
        "gin" => &["c."],
        "net/http" | "gorilla/mux" => &["r."],
        _ => &[],
    }
}

/// Substitute the conventional framework prefix in `call_path` with the bound
/// parameter name. If `call_path` doesn't start with any framework prefix,
/// returns it unchanged (covers free-function patterns like `mux.Vars`).
fn substitute_prefix(call_path: &str, param_name: &str, framework_name: &str) -> String {
    for prefix in framework_prefixes(framework_name) {
        if let Some(rest) = call_path.strip_prefix(prefix) {
            return format!("{}.{}", param_name, rest);
        }
    }
    call_path.to_string()
}

/// Collect names of parameters in `func_node` whose type matches one of `target_types`.
/// Per spec §2.6, ALL matching parameters bind (not just the first), to handle
/// pathological signatures like `func cmp(a, b *http.Request)`.
fn collect_request_param_names(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    target_types: &[&str],
) -> Vec<String> {
    let mut names = Vec::new();
    let params = match func_node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return names,
    };
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        if param.kind() != "parameter_declaration" {
            continue;
        }
        let type_text = match param.child_by_field_name("type") {
            Some(t) => parsed.node_text(&t).trim().to_string(),
            None => continue,
        };
        if !target_types.contains(&type_text.as_str()) {
            continue;
        }
        // A single parameter_declaration may declare multiple names sharing one type
        // (Go: `func f(a, b *http.Request)`). Collect every identifier child.
        let mut name_cursor = param.walk();
        for child in param.named_children(&mut name_cursor) {
            if child.kind() == "identifier" {
                names.push(parsed.node_text(&child).to_string());
            }
        }
    }
    names
}

/// Compute the textual call path for a Go call expression by joining selector
/// segments. For `r.URL.Query()`, returns `Some("r.URL.Query")`. For
/// unqualified or non-selector callees, returns the bare identifier or `None`.
fn go_call_path_text(parsed: &ParsedFile, call_node: &Node<'_>) -> Option<String> {
    let func = call_node.child_by_field_name("function")?;
    Some(parsed.node_text(&func).to_string())
}

/// Walk `root` collecting every Go `call_expression` node.
fn collect_go_calls<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "call_expression" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_go_calls(child, out);
    }
}

/// Detect taint sources from per-file framework specs (e.g., `c.Query` for gin,
/// `r.URL.Query` for net/http).
///
/// **Phase 1 scope:** only Go files contribute (see early `continue` for non-Go
/// languages). Future phases extend this by registering Python (Flask/Django/FastAPI),
/// JS (Express), or Java framework specs in `src/frameworks/` — no engine change
/// needed; this loop will pick them up via the framework registry.
///
/// Unlike `detect_ipc_sources`, framework sources are file-wide (not diff-restricted):
/// handler-shaped functions are stable taint origins regardless of whether the diff
/// touched them.
///
/// Returns `(file, line)` pairs for every call expression that matches a
/// framework `SourcePattern` (after prefix substitution) and lives inside a
/// function whose signature exposes the framework's request type.
fn detect_framework_sources(ctx: &CpgContext) -> Vec<(String, usize)> {
    let mut sources: Vec<(String, usize)> = Vec::new();
    for (file_path, parsed) in ctx.files {
        if parsed.language != Language::Go {
            continue;
        }
        let spec = match parsed.framework() {
            Some(s) => s,
            None => continue,
        };
        let target_types = framework_request_types(spec.name);
        if target_types.is_empty() {
            continue;
        }

        for func in parsed.all_functions() {
            let param_names = collect_request_param_names(parsed, &func, target_types);
            if param_names.is_empty() {
                continue;
            }
            let mut calls = Vec::new();
            collect_go_calls(func, &mut calls);

            for source_pat in spec.sources {
                // Compute every concrete call path to look for in this function.
                let concrete_paths: Vec<String> = if framework_prefixes(spec.name)
                    .iter()
                    .any(|p| source_pat.call_path.starts_with(p))
                {
                    param_names
                        .iter()
                        .map(|n| substitute_prefix(source_pat.call_path, n, spec.name))
                        .collect()
                } else {
                    // No conventional prefix — match as-is (e.g. mux.Vars).
                    vec![source_pat.call_path.to_string()]
                };

                for call in &calls {
                    let actual = match go_call_path_text(parsed, call) {
                        Some(s) => s,
                        None => continue,
                    };
                    if concrete_paths.contains(&actual) {
                        let line = call.start_position().row + 1;
                        sources.push((file_path.clone(), line));
                    }
                }
            }
        }
    }
    sources
}

// ─────────────────────────────────────────────────────────────────────────────
// Structured sink matching (spec §3.1 / §3.2 / §3.3).
//
// A sink fires when:
//   1. The call's qualified path equals `sink_pat.call_path`, AND
//   2. The optional `semantic_check` returns true (or is `None`), AND
//   3. The taint engine has flagged the line as carrying tainted data.
//
// (3) is checked by the existing taint pass — this helper handles (1) and (2).
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if any call expression on `line` matches `sink_pat`.
/// Caller is responsible for confirming taint flow reaches this line.
///
/// Note: `tainted_arg_indices` is *not* checked here. The current taint engine
/// tracks taint at line granularity, not per-argument. Treating "taint reaches
/// the line containing this call" as evidence is a conservative approximation:
/// it may over-fire when an unrelated tainted statement shares the line, but
/// such cases are vanishingly rare in normal Go style and tightening this
/// requires per-argument taint precision (out of scope for Phase 1, see spec
/// §3.2 slice-taint behavior note).
fn line_matches_structured_sink(parsed: &ParsedFile, line: usize, sink_pat: &SinkPattern) -> bool {
    if parsed.language != Language::Go {
        return false;
    }
    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);
    for call in &calls {
        let call_line = call.start_position().row + 1;
        if call_line != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if actual != sink_pat.call_path {
            continue;
        }
        if let Some(check) = sink_pat.semantic_check {
            let cs = CallSite {
                call_node: *call,
                source: parsed.source.as_str(),
            };
            if !check(&cs) {
                continue;
            }
        }
        return true;
    }
    false
}

/// Returns true if any cross-cutting Go sink (`GO_CWE78_SINKS`, `GO_CWE22_SINKS`)
/// or active framework's `sinks` matches a call on `line`. Used by the taint
/// pass to flag a tainted line as a structured sink.
fn line_matches_any_go_sink(parsed: &ParsedFile, line: usize) -> bool {
    for pat in GO_CWE78_SINKS {
        if line_matches_structured_sink(parsed, line, pat) {
            return true;
        }
    }
    for pat in GO_CWE22_SINKS {
        if line_matches_structured_sink(parsed, line, pat) {
            return true;
        }
    }
    if let Some(spec) = parsed.framework() {
        for pat in spec.sinks {
            if line_matches_structured_sink(parsed, line, pat) {
                return true;
            }
        }
    }
    false
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

    // Add GLib/D-Bus IPC accessor lines as explicit taint sources.
    // This enables tracing confused-deputy paths where user-controlled IPC data
    // (e.g. from `g_hash_table_lookup(settings->data, "usercert")`) flows into
    // a privileged sink (e.g. `BUILD_FROM_FILE`) that runs as root.
    // These are additional sources that extend (not replace) diff-line sources.
    let ipc_sources: Vec<(String, usize)> = detect_ipc_sources(ctx, diff);
    for ipc_src in &ipc_sources {
        taint_sources.push(ipc_src.clone());
    }
    let ipc_source_set: BTreeSet<(String, usize)> = ipc_sources.into_iter().collect();

    // Add framework-aware taint sources (Phase 1 Go: net/http, gin, gorilla/mux).
    // For each Go file with a detected framework, every call to a framework
    // SourcePattern (`c.Query`, `r.URL.Query`, `mux.Vars`, …) is a taint source.
    // These extend (not replace) diff-line and IPC sources.
    let framework_sources: Vec<(String, usize)> = detect_framework_sources(ctx);
    for fw_src in &framework_sources {
        if !taint_sources.contains(fw_src) {
            taint_sources.push(fw_src.clone());
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

                // Phase 1 structured Go sinks (cross-cutting + framework-gated).
                // A tainted line whose call matches one of these patterns fires a sink.
                if parsed.language == Language::Go && line_matches_any_go_sink(parsed, edge.to.line)
                {
                    sink_lines.insert((edge.to.file.clone(), edge.to.line));
                }
            }
        }
    }

    // Source lines themselves are taint-bearing — a structured sink on the
    // exact source line still fires. (E.g., `c.File(c.Param("f"))` — the
    // c.Param source and the c.File sink share a line.)
    for (file, line) in &taint_sources {
        if let Some(parsed) = ctx.files.get(file) {
            if parsed.language == Language::Go && line_matches_any_go_sink(parsed, *line) {
                sink_lines.insert((file.clone(), *line));
            }
        }
    }

    // Also check source lines for sinks (taint at source)
    for (file, line) in &taint_sources {
        all_tainted.entry(file.clone()).or_default().insert(*line);
    }

    // Emit findings for each taint source
    for (file, line) in &taint_sources {
        result.findings.push(SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: "info".to_string(),
            description: format!("taint source: origin of tainted data at line {}", line),
            function_name: None,
            related_lines: vec![],
            related_files: vec![],
            category: Some("taint_source".to_string()),
            parse_quality: None,
        });
    }

    // Emit findings for each taint sink reached
    for (file, line) in &sink_lines {
        // Find a source that reaches this sink (use first taint source as representative)
        // Pick the most descriptive source for this sink.
        // Prefer the nearest IPC source before the sink (user-controlled IPC reads
        // are the semantically interesting starting point for confused-deputy analysis).
        // Fall back to the nearest diff-line source, then any source in the same file.
        let source_desc = ipc_source_set
            .iter()
            .filter(|(sf, sl)| sf == file && *sl < *line)
            .max_by_key(|(_, sl)| *sl)
            .or_else(|| {
                taint_sources
                    .iter()
                    .filter(|(sf, sl)| sf == file && *sl < *line)
                    .max_by_key(|(_, sl)| *sl)
            })
            .or_else(|| taint_sources.iter().find(|(sf, _)| sf == file))
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
            category: Some("taint_sink".to_string()),
            parse_quality: None,
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
                parse_quality: None,
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
