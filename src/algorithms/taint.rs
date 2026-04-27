//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::data_flow::FlowPath;
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
/// invocation (e.g. `("sh", "-c", ...)`, `("pwsh", "-Command", ...)`).
///
/// Common Linux/macOS/Windows shells only; exotic absolute paths (`/usr/bin/sh`,
/// `/usr/local/bin/bash`) deliberately NOT included per spec §3.2 scope note.
fn is_shell_wrapper_at(call: &CallSite, name_idx: usize, flag_idx: usize) -> bool {
    let name = call.literal_arg(name_idx).unwrap_or("");
    let flag = call.literal_arg(flag_idx).unwrap_or("");
    match name {
        "sh" | "bash" | "/bin/sh" | "/bin/bash" => flag == "-c",
        "cmd.exe" => flag == "/c",
        "pwsh" | "powershell" | "powershell.exe" => {
            matches!(flag, "-c" | "-Command" | "-command")
        }
        _ => false,
    }
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
/// - Once for the tainted-binary form; tainted-arg index is the binary-path
///   argument itself. `semantic_check: None` because per-arg taint resolution
///   at sink-eval time (see `arg_is_tainted_in_path`) is the structural gate:
///   a literal binary has no identifier and is never tainted; a variable
///   bound to a non-tainted source isn't reached by any FlowPath edge at the
///   call line.
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
    // semantic_check requires the binary arg to be non-literal so a hardcoded
    // binary like exec.Command("ffmpeg", "-i", tainted) does NOT fire here.
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

/// Outcome of consulting the structured Go sink registry for a given call line.
///
/// Distinguishes three states that the previous `Option<&SinkPattern>` shape
/// conflated as `None`:
///
/// - `Match` — a pattern's `call_path` matched AND its `semantic_check` (if any)
///   accepted. The structured layer fires for this line; the flat-pattern catch-all
///   should also be allowed to add findings (subject to path-aware cleanser
///   suppression on the matched pattern's category).
/// - `SemanticallyExcluded` — at least one pattern's `call_path` matched but every
///   matching pattern either failed its `semantic_check` or had no relevant tainted
///   arg on this path. This outcome is not allowed to suppress the whole flat line:
///   the original PR #73 design did that, but reviewer feedback showed it hid
///   unmodeled shells and unrelated same-line sinks. Cleanser suppression, when
///   applicable, is scoped separately to identifiers inside the cleansed structured
///   call expression.
/// - `NoMatch` — no pattern's `call_path` matched. The structured layer has no
///   opinion; flat-pattern catch-all proceeds normally.
#[derive(Clone, Copy)]
enum SinkMatchOutcome {
    Match(&'static SinkPattern),
    SemanticallyExcluded,
    NoMatch,
}

/// Returns true if argument `arg_idx` of the call expression is tainted along `path`.
///
/// Resolution rules:
/// - Literal arg (string, int, bool, nil) → always false (literals can't be tainted).
/// - Bare identifier → check if any `FlowEdge` in `path` has this identifier as a `to`
///   location matching `parsed.path` (file scoping prevents cross-file collisions),
///   `call_line`, and `var_name()`. Without the file-scoping guard, an interprocedural
///   FlowEdge ending in another file at the same line/name could falsely register
///   as taint here.
/// - Complex expression (call, selector, binary, ...) → conservative recurse into
///   descendants; if ANY identifier descendant is tainted on the path (with file
///   scoping), the arg is considered tainted. Phase 1.5 keeps this conservative;
///   tightening (e.g., only considering specific positions in a selector chain) is
///   Phase 2+.
///
/// Returns false if the call has fewer than `arg_idx + 1` arguments.
fn arg_is_tainted_in_path(
    parsed: &ParsedFile,
    call: &Node<'_>,
    arg_idx: usize,
    path: &FlowPath,
) -> bool {
    let arguments = match call.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    let mut idx = 0usize;
    let mut target_arg: Option<Node<'_>> = None;
    for child in arguments.named_children(&mut cursor) {
        if idx == arg_idx {
            target_arg = Some(child);
            break;
        }
        idx += 1;
    }
    let arg_node = match target_arg {
        Some(n) => n,
        None => return false,
    };
    let call_line = call.start_position().row + 1;
    arg_node_taints_match(parsed, &arg_node, call_line, path)
}

/// Walk `arg_node` and any descendants for identifiers that are tainted on `path` at
/// `call_line` in `parsed`'s file. Returns true on first hit.
fn arg_node_taints_match(
    parsed: &ParsedFile,
    arg_node: &Node<'_>,
    call_line: usize,
    path: &FlowPath,
) -> bool {
    match arg_node.kind() {
        // Literal kinds — definitely not tainted.
        "interpreted_string_literal"
        | "raw_string_literal"
        | "rune_literal"
        | "int_literal"
        | "float_literal"
        | "imaginary_literal"
        | "true"
        | "false"
        | "nil" => false,

        // Bare identifier — direct check with file scoping.
        "identifier" => {
            let name = parsed.node_text(arg_node);
            path.edges.iter().any(|e| {
                e.to.file == parsed.path && e.to.line == call_line && e.to.var_name() == name
            })
        }

        // Composite expression — recurse into descendants. Conservative: any
        // tainted identifier within counts.
        _ => {
            let mut cursor = arg_node.walk();
            for child in arg_node.named_children(&mut cursor) {
                if arg_node_taints_match(parsed, &child, call_line, path) {
                    return true;
                }
            }
            false
        }
    }
}

/// Returns the first structured sink pattern on `line` whose tainted_arg subtrees
/// contain a descendant call_expression matching the active framework's source
/// patterns (e.g. `c.Param`, `r.URL.Query`). Used as a secondary fallback in the
/// source==sink loop to catch inline source==sink shapes that the per-arg DFG with
/// a real FlowPath cannot resolve — inline framework-source calls don't generate
/// FlowEdges because their results are consumed inline.
///
/// Scanning is exhaustive over (sink_pat, call) on this line, not first-match.
/// First-match would miss inline shapes when an unrelated structured sink earlier
/// on the line shadows the inline-bearing one (e.g. `exec.Command("ls"); c.File(c.Param("f"))`).
///
/// Request param names are scoped to the enclosing function of `line`, mirroring
/// `detect_framework_sources`. File-wide collection would treat unrelated handlers'
/// receiver names as valid binders here. The empty-collection short-circuit
/// (return None when `request_param_names.is_empty()`) is also load-bearing: it
/// matches the `detect_framework_sources` guard at L794, preventing non-prefixed
/// sources like `mux.Vars` from being recognized in functions that don't bind a
/// `*http.Request` parameter.
///
/// Phase 1.5 limitation: only framework sources are recognized, not IPC sources.
/// IPC source==sink shapes are rare and remain a Phase 1.5.1+ refinement.
fn find_sink_with_inline_framework_source(
    parsed: &ParsedFile,
    line: usize,
) -> Option<&'static SinkPattern> {
    let framework = parsed.framework()?;

    // Function-scoped request param name collection. Mirrors
    // `detect_framework_sources` — only binds receiver names that appear in the
    // enclosing function's signature.
    let func_node = parsed.enclosing_function(line)?;
    let target_types = framework_request_types(framework.name);
    if target_types.is_empty() {
        return None;
    }
    let request_param_names = collect_request_param_names(parsed, &func_node, target_types);
    // Empty-function-scope guard: mirrors the early `continue` in
    // `detect_framework_sources`. Without it, non-prefixed sources like
    // `mux.Vars` would still be inserted into `source_paths` even when the
    // enclosing function has no `*http.Request` / `*gin.Context` parameter,
    // wrongly recognizing them as framework sources for this line.
    if request_param_names.is_empty() {
        return None;
    }

    // Build the set of concrete framework-source call_paths for THIS function
    // (mirrors detect_framework_sources's prefix-substitution logic).
    let mut source_paths: BTreeSet<String> = BTreeSet::new();
    for src in framework.sources {
        if framework_prefixes(framework.name)
            .iter()
            .any(|p| src.call_path.starts_with(p))
        {
            for n in &request_param_names {
                source_paths.insert(substitute_prefix(src.call_path, n, framework.name));
            }
        } else {
            // No conventional prefix — match as-is (e.g. mux.Vars).
            source_paths.insert(src.call_path.to_string());
        }
    }
    if source_paths.is_empty() {
        return None;
    }

    // Walk all calls on `line`. For each, check against EVERY structured sink
    // pattern (priority order: GO_CWE78_SINKS, GO_CWE22_SINKS, framework SINKS).
    // First (call, sink_pat) pair where the tainted_arg subtree contains an
    // inline framework source returns its sink_pat.
    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };

        let pattern_iter = GO_CWE78_SINKS
            .iter()
            .chain(GO_CWE22_SINKS.iter())
            .chain(framework.sinks.iter());

        for pat in pattern_iter {
            if actual != pat.call_path {
                continue;
            }
            // Apply semantic_check (matches go_sink_outcome's gating). If
            // semantic_check rejects, this pattern doesn't describe THIS call —
            // skip and try the next pattern.
            if let Some(check) = pat.semantic_check {
                let cs = CallSite {
                    call_node: *call,
                    source: parsed.source.as_str(),
                };
                if !check(&cs) {
                    continue;
                }
            }
            let arguments = match call.child_by_field_name("arguments") {
                Some(n) => n,
                None => continue,
            };
            let mut cursor = arguments.walk();
            let mut idx = 0usize;
            for arg in arguments.named_children(&mut cursor) {
                if pat.tainted_arg_indices.contains(&idx)
                    && subtree_has_call_in(parsed, &arg, &source_paths)
                {
                    return Some(pat);
                }
                idx += 1;
            }
        }
    }
    None
}

/// Walk `node` and descendants; returns true if any `call_expression` node has
/// a call_path text in `paths`.
fn subtree_has_call_in(parsed: &ParsedFile, node: &Node<'_>, paths: &BTreeSet<String>) -> bool {
    if node.kind() == "call_expression" {
        if let Some(cp) = go_call_path_text(parsed, node) {
            if paths.contains(&cp) {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if subtree_has_call_in(parsed, &child, paths) {
            return true;
        }
    }
    false
}

/// Returns the structured-sink outcome for `sink_pat` on `line` of `parsed`,
/// using `path` to resolve per-argument taint via `arg_is_tainted_in_path`.
///
/// Outcome rules:
/// - `Match(sink_pat)` — call_path matches, semantic_check (if any) passes,
///   AND at least one arg in `sink_pat.tainted_arg_indices` is tainted on `path`.
/// - `SemanticallyExcluded` — call_path matches but EITHER `semantic_check`
///   rejects OR no arg in `tainted_arg_indices` is tainted on this path.
/// - `NoMatch` — no call expression on `line` matches `sink_pat.call_path`.
///
/// `path == None` is the source==sink no-originating-path fallback (see design
/// note §2.3); in that branch we trust call_path + semantic_check without per-arg
/// precision, preserving today's source==sink behavior for shapes like
/// `c.File(c.Param("f"))`.
///
/// Caller is responsible for confirming `parsed.language == Language::Go`
/// (the function returns `NoMatch` for non-Go files).
fn line_matches_structured_sink(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    if parsed.language != Language::Go {
        return SinkMatchOutcome::NoMatch;
    }
    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);
    let mut had_call_path_match = false;
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
        had_call_path_match = true;
        if let Some(check) = sink_pat.semantic_check {
            let cs = CallSite {
                call_node: *call,
                source: parsed.source.as_str(),
            };
            if !check(&cs) {
                continue;
            }
        }

        // Per-arg taint check — only when a FlowPath is provided. `path == None`
        // is the source==sink no-originating-path fallback (see design note §2.3);
        // in that case we trust the existing call_path + semantic_check gate
        // without per-arg precision, preserving today's source==sink behavior for
        // shapes like `c.File(c.Param("f"))`.
        if let Some(p) = path {
            let any_arg_tainted = sink_pat
                .tainted_arg_indices
                .iter()
                .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, p));
            if !any_arg_tainted {
                // call_path + semantic_check passed, but the relevant args
                // aren't tainted on this path. Mark as a structural match-but-
                // not-actually-firing; subsequent iterations may find a
                // different call on this line that DOES have tainted args.
                continue;
            }
        }

        return SinkMatchOutcome::Match(sink_pat);
    }
    if had_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

/// Returns the structured-sink outcome for `line` across the full Go sink registry
/// (cross-cutting CWE-78/22 + framework-gated). Used during path-aware suppression.
///
/// `path` is forwarded to `line_matches_structured_sink` for per-argument taint
/// resolution (Phase 1.5 #1). Pass `Some(path)` from forward-flow callers; the
/// source==sink loop passes `None` for the no-originating-path branch (canonical
/// `c.File(c.Param("f"))` shape) so the engine falls back to call_path +
/// semantic_check matching without per-arg precision.
///
/// Aggregation rules:
/// - If any pattern returns `Match`, the first such pattern wins (priority order:
///   GO_CWE78_SINKS, GO_CWE22_SINKS, framework SINKS). The matched pattern's
///   category drives `FlowPath.cleansed_for` consultation.
/// - Else if any pattern returned `SemanticallyExcluded`, aggregate is
///   `SemanticallyExcluded`.
/// - Else `NoMatch`. Flat-pattern catch-all proceeds normally.
///
/// Note: when multiple patterns share a `call_path` (exec.Command shell-wrapper +
/// tainted-binary), the shell-wrapper variant is listed first; if it `Match`-es
/// it wins, otherwise the tainted-binary variant is checked next.
fn go_sink_outcome(parsed: &ParsedFile, line: usize, path: Option<&FlowPath>) -> SinkMatchOutcome {
    let mut any_call_path_match = false;
    for pat in GO_CWE78_SINKS {
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    for pat in GO_CWE22_SINKS {
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    if let Some(spec) = parsed.framework() {
        for pat in spec.sinks {
            match line_matches_structured_sink(parsed, line, pat, path) {
                SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
                SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
                SinkMatchOutcome::NoMatch => {}
            }
        }
    }
    if any_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

fn call_passes_sink_semantics(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
) -> bool {
    if let Some(check) = sink_pat.semantic_check {
        let cs = CallSite {
            call_node: *call,
            source: parsed.source.as_str(),
        };
        check(&cs)
    } else {
        true
    }
}

fn push_cleansed_structured_sink_range(
    ranges: &mut Vec<(usize, usize)>,
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool {
    if actual != sink_pat.call_path || !path.cleansed_for.contains(&sink_pat.category) {
        return false;
    }
    if !call_passes_sink_semantics(parsed, call, sink_pat) {
        return false;
    }
    ranges.push((call.start_byte(), call.end_byte()));
    true
}

/// Returns byte ranges for structured sink calls on `line` whose own flat
/// identifier matches should be suppressed because this flow is cleansed for the
/// sink's category. Suppression is intentionally scoped to the call expression:
/// unrelated flat sinks that happen to share the same source line still run.
fn cleansed_structured_sink_call_ranges(
    parsed: &ParsedFile,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    if parsed.language != Language::Go || path.cleansed_for.is_empty() {
        return Vec::new();
    }

    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);

    let mut ranges = Vec::new();
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };

        let mut pushed = false;
        for pat in GO_CWE78_SINKS {
            if push_cleansed_structured_sink_range(&mut ranges, parsed, call, &actual, pat, path) {
                pushed = true;
                break;
            }
        }
        if pushed {
            continue;
        }
        for pat in GO_CWE22_SINKS {
            if push_cleansed_structured_sink_range(&mut ranges, parsed, call, &actual, pat, path) {
                pushed = true;
                break;
            }
        }
        if pushed {
            continue;
        }
        if let Some(spec) = parsed.framework() {
            for pat in spec.sinks {
                if push_cleansed_structured_sink_range(
                    &mut ranges,
                    parsed,
                    call,
                    &actual,
                    pat,
                    path,
                ) {
                    break;
                }
            }
        }
    }
    ranges
}

fn node_in_ranges(node: &Node<'_>, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= node.start_byte() && node.end_byte() <= *end)
}

/// Returns true if the function body containing `line` in `parsed` has at least one
/// active sanitizer recognizer call whose category equals `category`. Walks the
/// enclosing function for the line, applies each recognizer's `semantic_check` and
/// textual `paired_check`, and returns on first match.
///
/// Used both by `apply_cleansers` (per-FlowPath, all categories) and by the
/// source==sink fallback (single-category check when no FlowPath exists for the line).
///
/// The walk is intraprocedural (cleanser must live in same function as `line`);
/// cross-function cleansing is a Phase 1.5+ concern. Phase 1 is Go-only — callers
/// must gate by language; this helper does not re-check.
fn function_body_cleansed_for(
    parsed: &ParsedFile,
    line: usize,
    category: SanitizerCategory,
) -> bool {
    let func_node = match parsed.enclosing_function(line) {
        Some(n) => n,
        None => return false,
    };
    let func_text = func_node.utf8_text(parsed.source.as_bytes()).unwrap_or("");

    let mut calls = Vec::new();
    collect_go_calls(func_node, &mut calls);

    for recognizer in crate::sanitizers::active_recognizers() {
        if recognizer.category != category {
            continue;
        }
        // Look for a call to the recognizer's call_path within the function.
        let mut matched = false;
        for call in &calls {
            let actual = match go_call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if actual != recognizer.call_path {
                continue;
            }
            if let Some(check) = recognizer.semantic_check {
                let cs = CallSite {
                    call_node: *call,
                    source: parsed.source.as_str(),
                };
                if !check(&cs) {
                    continue;
                }
            }
            matched = true;
            break;
        }
        if !matched {
            continue;
        }
        // For paired-check recognizers, the second-half check must also appear
        // in the function body (textual co-occurrence per §3.4 / §3.8).
        if let Some(paired) = recognizer.paired_check {
            if !crate::sanitizers::paired_check_satisfied(func_text, paired) {
                continue;
            }
        }
        return true;
    }
    false
}

/// Apply cleansers to a `FlowPath`, mutating `cleansed_for` in place per spec §3.6.
///
/// For each active sanitizer recognizer category, calls
/// `function_body_cleansed_for` on the flow's source line. If a recognizer in
/// that category fires (with `semantic_check` and `paired_check` satisfied per
/// §3.4 / §3.8), the category is inserted into `path.cleansed_for`.
///
/// The walk is intraprocedural (cleanser must live in same function as source);
/// cross-function cleansing is a Phase 1.5+ concern.
fn apply_cleansers(path: &mut crate::data_flow::FlowPath, files: &BTreeMap<String, ParsedFile>) {
    if path.edges.is_empty() {
        return;
    }
    // The source location is the `from` of the first edge (FlowPaths are
    // single-source fans built by taint_forward_cfg).
    let src = &path.edges[0].from;
    let parsed = match files.get(&src.file) {
        Some(p) => p,
        None => return,
    };
    if parsed.language != Language::Go {
        return; // Phase 1: Go-only sanitizer registry.
    }

    // Iterate distinct recognizer categories so each is checked at most once.
    let categories: BTreeSet<SanitizerCategory> = crate::sanitizers::active_recognizers()
        .map(|r| r.category)
        .collect();
    for category in categories {
        if path.cleansed_for.contains(&category) {
            continue;
        }
        if function_body_cleansed_for(parsed, src.line, category) {
            path.cleansed_for.insert(category);
        }
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
    // Lines whose identifiers are recognized framework SOURCE calls (e.g.
    // `r.URL.Query()`, `c.Query()`, `mux.Vars()`). These overlap textually with
    // the cross-language flat sink registry — `Query` is in SINK_PATTERNS as a
    // generic `sql.Query` substring matcher — so without this set, a tainted
    // source line would double-fire as a sink. Used during sink evaluation to
    // suppress flat substring matches on lines positively identified as sources.
    let framework_source_set: BTreeSet<(String, usize)> =
        framework_sources.iter().cloned().collect();

    // Forward propagation from each source (CFG-constrained when available)
    let mut paths = ctx.cpg.taint_forward_cfg(&taint_sources);

    // Sanitizer propagation hook (spec §3.6): for each path, walk the function
    // body containing its source and mark `cleansed_for` for any cleanser whose
    // call_path occurs there (with paired_check satisfied if required). This
    // happens after path construction but before sink evaluation so the
    // suppression check below can consult the cleansed-for set.
    for path in &mut paths {
        apply_cleansers(path, ctx.files);
    }

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
                // Consult the structured Go sink registry. Three outcomes:
                // - `Match(p)`: structured sink fires (modulo cleanser
                //   suppression by category).
                // - `SemanticallyExcluded`: a structured pattern's call_path
                //   matched but did not fire on this path. This outcome is NOT
                //   used to suppress every flat-pattern catch-all on the line —
                //   see PR #73 review feedback (P1: unmodeled shells; P2:
                //   unrelated same-line sinks). Cleanser suppression below is
                //   scoped to identifiers inside the cleansed structured call.
                // - `NoMatch`: no structured opinion; flat-pattern catch-all
                //   runs normally.
                //
                // Path-aware cleanser suppression (spec §3.7): if `Match(p)` and
                // the path is cleansed for `p.category`, suppress the structured
                // finding and flat identifiers inside that structured call. Do
                // not suppress unrelated flat sinks that share the same line.
                let outcome = if parsed.language == Language::Go {
                    go_sink_outcome(parsed, edge.to.line, Some(path))
                } else {
                    SinkMatchOutcome::NoMatch
                };
                let cleansed_structured_ranges =
                    cleansed_structured_sink_call_ranges(parsed, edge.to.line, path);
                let structured_suppressed_by_cleanser = match outcome {
                    SinkMatchOutcome::Match(p) => path.cleansed_for.contains(&p.category),
                    _ => false,
                };

                // Suppress flat substring matches on lines that are recognized
                // framework SOURCE calls — e.g., `Query` would otherwise fire
                // as a flat sink on `r.URL.Query()` even though that's the
                // source, not a sink. Structured Go sinks (which we know are
                // sinks, not sources) are not affected by this filter.
                let is_framework_source_line =
                    framework_source_set.contains(&(edge.to.file.clone(), edge.to.line));
                if !is_framework_source_line {
                    let ids = parsed.identifiers_on_line(edge.to.line);
                    for id in &ids {
                        if node_in_ranges(id, &cleansed_structured_ranges) {
                            continue;
                        }
                        let text = parsed.node_text(id);
                        if all_sinks.iter().any(|s| matches_sink(text, s)) {
                            sink_lines.insert((edge.to.file.clone(), edge.to.line));
                        }
                    }
                }

                // Phase 1 structured Go sinks (cross-cutting + framework-gated).
                if matches!(outcome, SinkMatchOutcome::Match(_))
                    && !structured_suppressed_by_cleanser
                {
                    sink_lines.insert((edge.to.file.clone(), edge.to.line));
                }
            }
        }
    }

    // Source lines themselves are taint-bearing — a structured sink on the
    // exact source line still fires. (E.g., `c.File(c.Param("f"))` — the
    // c.Param source and the c.File sink share a line.)
    //
    // Three branches (see design note §2.3):
    //   1. originating.is_empty() → primary Option::None fallback for pure
    //      source==sink shapes (no FlowEdge connects source to sink because
    //      they share a line and the source result is consumed inline).
    //   2. originating non-empty → per-path Match-and-cleansing combined loop.
    //      Fire iff at least one matching path is uncleansed. Non-matching
    //      paths are skipped — their cleansing state is irrelevant because
    //      per-arg DFG already says they don't fire this sink.
    //   3. originating non-empty + branch 2 found no fire → secondary inline-
    //      source fallback for mixed same-line shapes (the line hosts both a
    //      non-inline source driving the FlowPath AND an inline source==sink
    //      shape that conservative recursion can't recognize).
    for (file, line) in &taint_sources {
        if let Some(parsed) = ctx.files.get(file) {
            if parsed.language != Language::Go {
                continue;
            }
            // Find every path whose source is this (file, line).
            let originating: Vec<&FlowPath> = paths
                .iter()
                .filter(|p| {
                    p.edges
                        .first()
                        .map(|e| e.from.file == *file && e.from.line == *line)
                        .unwrap_or(false)
                })
                .collect();

            if originating.is_empty() {
                // No FlowPath originates — pure source==sink shape (e.g.
                // c.File(c.Param("f"))). Pass None to skip per-arg DFG; the
                // engine falls back to call_path + semantic_check matching.
                // Preserves today's source==sink behavior.
                let sink_pat = match go_sink_outcome(parsed, *line, None) {
                    SinkMatchOutcome::Match(p) => p,
                    SinkMatchOutcome::SemanticallyExcluded | SinkMatchOutcome::NoMatch => continue,
                };
                // No FlowPath cleansing applies. Fall back to function-body scan.
                let cleansed = function_body_cleansed_for(parsed, *line, sink_pat.category);
                if !cleansed {
                    sink_lines.insert((file.clone(), *line));
                }
            } else {
                // Per-arg DFG applies. Walk originating paths; for each that
                // Matches, check whether its FlowPath is cleansed for the
                // matched category. Fire iff AT LEAST ONE matching path is
                // not cleansed.
                //
                // Crucially, we ONLY consult cleansing for paths that actually
                // Match (per-arg DFG: relevant args are tainted on this path).
                // A non-matching path's cleansing state is irrelevant — per-arg
                // DFG already says it doesn't fire this sink, so its cleansing-
                // or-not can't move the decision.
                let mut any_matching_uncleansed = false;
                for p in &originating {
                    if let SinkMatchOutcome::Match(pat) = go_sink_outcome(parsed, *line, Some(p)) {
                        if !p.cleansed_for.contains(&pat.category) {
                            any_matching_uncleansed = true;
                            break;
                        }
                    }
                }

                // Secondary inline-source fallback for mixed same-line shapes.
                // The line may host both a non-inline source (driving an
                // originating FlowPath) AND an inline source==sink shape (e.g.
                // `c.File(c.Param("f"))`) that the primary per-arg DFG can't
                // recognize because its conservative recursion only checks
                // identifiers against FlowPath edges, and the inline c.Param
                // call generates no FlowEdge.
                //
                // The helper scans ALL (call, sink_pat) combinations on the
                // line and returns the first sink pattern whose tainted_arg
                // subtree contains an inline framework-source call. This is
                // intentionally NOT routed through go_sink_outcome's first-
                // match-wins aggregation — that would only consider the first
                // matching sink and miss inline shapes when an unrelated
                // structured sink earlier on the line shadows them.
                if !any_matching_uncleansed {
                    if let Some(pat) = find_sink_with_inline_framework_source(parsed, *line) {
                        let cleansed = function_body_cleansed_for(parsed, *line, pat.category);
                        if !cleansed {
                            any_matching_uncleansed = true;
                        }
                    }
                }

                if any_matching_uncleansed {
                    sink_lines.insert((file.clone(), *line));
                }
                // else: every matching path is cleansed (or no path matches
                // AND no inline source==sink shape was detected). Suppress.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::data_flow::{FlowEdge, VarAccessKind, VarLocation};

    fn flow_path_to(line: usize, var_name: &str) -> FlowPath {
        let from = VarLocation {
            file: "main.go".to_string(),
            function: "handler".to_string(),
            line: 3,
            path: AccessPath::simple(var_name),
            kind: VarAccessKind::Def,
        };
        let to = VarLocation {
            file: "main.go".to_string(),
            function: "handler".to_string(),
            line,
            path: AccessPath::simple(var_name),
            kind: VarAccessKind::Use,
        };
        FlowPath {
            edges: vec![FlowEdge { from, to }],
            cleansed_for: BTreeSet::new(),
        }
    }

    #[test]
    fn powershell_shell_wrappers_match_structured_registry() {
        let source = r#"package main
import "os/exec"
func handler(input string) {
	_ = exec.Command("pwsh", "-c", input).Run()
	_ = exec.Command("powershell", "-command", input).Run()
	_ = exec.Command("powershell.exe", "-Command", input).Run()
	_ = exec.CommandContext(ctx, "pwsh", "-Command", input).Run()
}
"#;
        let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();

        for line in [4, 5, 6, 7] {
            let path = flow_path_to(line, "input");
            assert!(matches!(
                go_sink_outcome(&parsed, line, Some(&path)),
                SinkMatchOutcome::Match(p) if p.category == SanitizerCategory::OsCommand
            ));
        }
    }

    #[test]
    fn shell_wrapper_flags_are_shell_family_specific() {
        let source = r#"package main
import "os/exec"
func handler(input string) {
	_ = exec.Command("sh", "-Command", input).Run()
	_ = exec.Command("cmd.exe", "-c", input).Run()
}
"#;
        let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();

        for line in [4, 5] {
            let path = flow_path_to(line, "input");
            assert!(matches!(
                go_sink_outcome(&parsed, line, Some(&path)),
                SinkMatchOutcome::SemanticallyExcluded
            ));
        }
    }
}
