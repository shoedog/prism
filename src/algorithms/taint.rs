//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

mod go;
mod js_ts;
mod python;

pub use go::sinks::{GO_CWE22_SINKS, GO_CWE78_SINKS};
pub use js_ts::sink_tables::{
    JS_CWE22_SINKS, JS_CWE502_SINKS, JS_CWE78_SINKS, JS_CWE79_SINKS, JS_CWE89_SINKS,
    JS_CWE918_SINKS,
};
pub use python::sink_tables::{PY_CWE502_SINKS, PY_CWE79_SINKS, PY_CWE89_SINKS, PY_CWE918_SINKS};

use crate::access_path::AccessPath;
use crate::algorithms::taint_sinks;
use crate::ast::tree_helpers::{
    call_arg_node, call_path_matches, call_path_text, collect_calls, is_js_ts_language,
};
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgContext};
use crate::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use crate::languages::Language;
use crate::output::mermaid::safe_node_id;
use crate::sanitizers::matching::function_body_cleansed_for;
use crate::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceFinding, SliceGraph, SliceResult,
    SlicingAlgorithm,
};
use anyhow::Result;
use go::sanitizers::{go_path_traversal_cleansed_for_sink, PathSanitizerKind};
use go::sinks::{
    collect_go_calls, detect_go_framework_sources, find_sink_with_inline_framework_source,
    go_call_path_text, go_sink_outcome,
};
use js_ts::alias_edges::{
    js_ts_request_source_seed_framework_and_params, js_ts_seed_is_koa_request_object_alias,
    js_ts_variable_reference_ranges_scoped, synthesize_js_ts_assignment_alias_edges,
};
use js_ts::exec_sql::{
    js_ts_exec_file_is_literal_binary, js_ts_sql_call_is_parametrized,
    js_ts_yaml_load_uses_safe_schema,
};
use js_ts::framework_params::detect_js_ts_framework_sources;
use js_ts::imports_ssrf::js_ts_ssrf_sink_call_path_matches;
use js_ts::request_aliases::{js_ts_seed_scope_contains, js_ts_seed_scope_contains_range};
use js_ts::sanitizers::{js_ts_path_traversal_cleansed_for_sink, js_ts_ssrf_cleansed_for_sink};
use js_ts::sink_tables::js_yaml_load_call_uses_unsafe_schema;
use js_ts::sinks::{
    is_js_yaml_load_sink_pattern, js_ts_flat_sink_identifier_has_tainted_request_source,
    js_ts_inline_framework_source_yaml_sink_lines, js_ts_js_yaml_bare_load_call_matches,
    js_ts_safe_structured_sink_call_ranges, js_ts_sink_call_has_inline_framework_source_arg,
    js_ts_sink_outcome,
};
use js_ts::source_access::{
    js_ts_framework_source_access_ranges_by_line, js_ts_request_source_identifier,
    node_contains_js_ts_source_access_on_line_with_request_object_aliases,
};
use js_ts::source_ranges::{
    js_ts_framework_source_target_ranges_by_line,
    js_ts_request_source_access_ranges_for_alias_on_line,
};
use python::clients::{
    python_is_aiohttp_client_session_method_call, python_is_aiohttp_request_call,
    python_is_urllib3_pool_manager_request_call,
};
use python::sanitizers::{
    python_safe_structured_sink_call_ranges, python_ssrf_cleansed_for_sink,
    python_xss_cleansed_for_sink,
};
use python::sinks::{
    python_sink_outcome, python_sink_with_inline_flask_source, python_sql_call_is_parametrized,
    python_yaml_load_uses_safe_loader,
};
use python::sources::{detect_python_framework_sources, python_is_inner_decorated_function};
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
    // Deserialization is handled by explicit structured PY_CWE502_SINKS entries.
    // Do not add broad `=loads` / `=load` flat fallbacks here: `json.loads` and
    // `json.load` parse data without code execution and are not CWE-502 sinks.
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
pub(super) fn matches_sink(identifier: &str, pattern: &str) -> bool {
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
    /// Opt in to singleton-Exact callee-return flow. Default-off preserves the
    /// Tier-1/review wire until the separate default-on decision.
    pub return_flow: bool,
}

impl Default for TaintConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            taint_from_diff: true,
            extra_sinks: Vec::new(),
            return_flow: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TaintSeed {
    file: String,
    line: usize,
    target: Option<AccessPath>,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
}

impl TaintSeed {
    fn line(file: String, line: usize) -> Self {
        Self {
            file,
            line,
            target: None,
            start_byte: None,
            scope: None,
            byte_scope: None,
        }
    }

    fn target(file: String, line: usize, target: AccessPath) -> Self {
        Self::target_scoped(file, line, target, None, None, None)
    }

    fn target_scoped(
        file: String,
        line: usize,
        target: AccessPath,
        start_byte: Option<usize>,
        scope: Option<(usize, usize)>,
        byte_scope: Option<(usize, usize)>,
    ) -> Self {
        Self {
            file,
            line,
            target: Some(target),
            start_byte,
            scope,
            byte_scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct JsTsRequestDataSource {
    line: usize,
    target: AccessPath,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JsTsAliasKill {
    line: usize,
    byte: usize,
    byte_scope: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsTsAliasDef {
    line: usize,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    kills: Vec<JsTsAliasKill>,
}

type JsTsAliasDefs = BTreeMap<String, Vec<JsTsAliasDef>>;

impl JsTsAliasKill {
    fn applies_to_range(&self, line: usize, start_byte: usize, end_byte: usize) -> bool {
        (self.line < line || (self.line == line && self.byte < end_byte))
            && self
                .byte_scope
                .map(|(start, end)| start <= start_byte && end_byte <= end)
                .unwrap_or(true)
    }
}

impl JsTsAliasDef {
    fn visible_on(&self, line: usize) -> bool {
        self.line <= line
            && self
                .scope
                .map(|(start, end)| start <= line && line <= end)
                .unwrap_or(true)
    }

    fn visible_range(&self, line: usize, start_byte: usize, end_byte: usize) -> bool {
        self.visible_on(line)
            && (self.line < line
                || self
                    .start_byte
                    .map(|def_start| def_start <= start_byte)
                    .unwrap_or(true))
            && self
                .byte_scope
                .map(|(start, end)| start <= start_byte && end_byte <= end)
                .unwrap_or(true)
            && !self
                .kills
                .iter()
                .any(|kill| kill.applies_to_range(line, start_byte, end_byte))
    }

    fn same_binding_as(&self, other: &Self) -> bool {
        self.line == other.line
            && self.start_byte == other.start_byte
            && self.scope == other.scope
            && self.byte_scope == other.byte_scope
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

fn detect_framework_sources(ctx: &CpgContext) -> Vec<TaintSeed> {
    let mut sources: Vec<TaintSeed> = Vec::new();
    for (file_path, parsed) in ctx.files {
        match parsed.language {
            Language::Go => detect_go_framework_sources(file_path, parsed, &mut sources),
            Language::Python => detect_python_framework_sources(file_path, parsed, &mut sources),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                detect_js_ts_framework_sources(file_path, parsed, &mut sources)
            }
            _ => {}
        }
    }
    sources.sort();
    sources.dedup();
    sources
}

#[derive(Clone)]
struct JsTsParam {
    name: String,
    has_nest_source_decorator: bool,
}

#[derive(Default)]
struct PythonDjangoRequestDataSources {
    has_access: bool,
    targets: BTreeSet<(usize, AccessPath)>,
}

#[derive(Default)]
struct PythonFlaskRequestDataSources {
    targets: BTreeSet<(usize, AccessPath)>,
    lines: BTreeSet<usize>,
}

#[derive(Debug)]
struct PythonParam {
    name: String,
    annotation: Option<String>,
}

fn synthesize_target_seed_paths(seeds: &[TaintSeed], ctx: &CpgContext, paths: &mut Vec<FlowPath>) {
    for seed in seeds {
        let target = match &seed.target {
            Some(t) => t,
            None => continue,
        };
        let parsed = match ctx.files.get(&seed.file) {
            Some(p) => p,
            None => continue,
        };
        let func = match parsed.enclosing_function(seed.line) {
            Some(f) => f,
            None => continue,
        };
        let func =
            if parsed.language == Language::Python && python_is_inner_decorated_function(&func) {
                func.parent().unwrap_or(func)
            } else {
                func
            };
        let func_name = parsed
            .language
            .function_name(&func)
            .map(|n| parsed.node_text(&n).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let reachable = if ctx.cpg.has_cfg_edges() {
            Some(ctx.cpg.cfg_reachable_lines(&seed.file, seed.line))
        } else {
            None
        };
        let from = VarLocation {
            file: seed.file.clone(),
            function: func_name.clone(),
            function_start_line: func.start_position().row + 1,
            line: seed.line,
            path: target.clone(),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Def,
        };
        let mut edges = Vec::new();
        let reachable_dfg = ctx.cpg.dfg_forward_reachable(&from);
        for target_loc in reachable_dfg {
            if !js_ts_seed_scope_contains(seed, target_loc.line) {
                continue;
            }
            if target_loc.file == seed.file
                && target_loc.function == func_name
                && target_loc.function_start_line == from.function_start_line
                && target_loc.line <= seed.line
            {
                continue;
            }
            if let Some(cfg_set) = &reachable {
                if target_loc.file == seed.file
                    && target_loc.function == func_name
                    && target_loc.function_start_line == from.function_start_line
                    && !reference_line_cfg_reachable(
                        parsed,
                        &func,
                        &seed.file,
                        target_loc.line,
                        cfg_set,
                    )
                {
                    continue;
                }
            }
            edges.push(FlowEdge {
                from: from.clone(),
                to: target_loc,
            });
        }
        let synth_ctx = TargetSeedSynthesisContext {
            seed,
            parsed,
            func,
            func_name: &func_name,
            target,
            reachable: reachable.as_ref(),
            from: &from,
        };
        synthesize_js_ts_assignment_alias_edges(&synth_ctx, &mut edges);
        synthesize_direct_target_reference_edges(&synth_ctx, &mut edges);
        if !edges.is_empty() {
            paths.push(FlowPath {
                edges,
                cleansed_for: BTreeSet::new(),
            });
        }
    }
}

struct TargetSeedSynthesisContext<'a> {
    seed: &'a TaintSeed,
    parsed: &'a ParsedFile,
    func: Node<'a>,
    func_name: &'a str,
    target: &'a AccessPath,
    reachable: Option<&'a BTreeSet<(String, usize)>>,
    from: &'a VarLocation,
}

fn synthesize_direct_target_reference_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
) {
    let js_ts_request_source =
        js_ts_request_source_seed_framework_and_params(ctx).map(|(framework, params)| {
            let koa_request_object_aliases =
                if framework == "koa" && js_ts_seed_is_koa_request_object_alias(ctx) {
                    std::iter::once(ctx.target.base.clone()).collect()
                } else {
                    BTreeSet::new()
                };
            (
                framework,
                std::iter::once(ctx.target.base.clone())
                    .chain(params)
                    .collect::<BTreeSet<_>>(),
                koa_request_object_aliases,
            )
        });
    let allow_same_line_refs = is_js_ts_language(ctx.parsed.language);
    let refs = if allow_same_line_refs {
        js_ts_variable_reference_ranges_scoped(
            ctx.parsed,
            ctx.func,
            &ctx.target.base,
            ctx.seed.line,
        )
    } else {
        ctx.parsed
            .find_variable_references_scoped(&ctx.func, &ctx.target.base, ctx.seed.line)
            .into_iter()
            .map(|line| (line, 0, usize::MAX))
            .collect()
    };
    for (ref_line, ref_start, ref_end) in refs {
        if ref_line < ctx.seed.line || (!allow_same_line_refs && ref_line == ctx.seed.line) {
            continue;
        }
        if allow_same_line_refs
            && ref_line == ctx.seed.line
            && ctx.seed.start_byte == Some(ref_start)
        {
            continue;
        }
        if !js_ts_seed_scope_contains_range(ctx.parsed, ctx.seed, ref_line, ref_start, ref_end) {
            continue;
        }
        if ref_line != ctx.seed.line {
            if let Some(cfg_set) = ctx.reachable {
                if !reference_line_cfg_reachable(
                    ctx.parsed,
                    &ctx.func,
                    &ctx.seed.file,
                    ref_line,
                    cfg_set,
                ) {
                    continue;
                }
            }
        }
        if let Some((framework, source_params, koa_request_object_aliases)) = &js_ts_request_source
        {
            let source_ranges = js_ts_request_source_access_ranges_for_alias_on_line(
                ctx.parsed,
                ref_line,
                &ctx.target.base,
            );
            if source_ranges.is_empty() {
                continue;
            }
            if !node_contains_js_ts_source_access_on_line_with_request_object_aliases(
                ctx.parsed,
                ctx.func,
                ctx.func.id(),
                ref_line,
                framework,
                source_params,
                koa_request_object_aliases,
            ) {
                continue;
            }
        }
        if edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file
                && edge.to.line == ref_line
                && edge.to.var_name() == ctx.target.base
        }) {
            continue;
        }
        edges.push(FlowEdge {
            from: ctx.from.clone(),
            to: VarLocation {
                file: ctx.seed.file.clone(),
                function: ctx.func_name.to_string(),
                function_start_line: ctx.func.start_position().row + 1,
                line: ref_line,
                path: AccessPath::simple(ctx.target.base.clone()),
                start_byte: 0,
                end_byte: 0,
                kind: VarAccessKind::Use,
            },
        });
    }
}

fn reference_line_cfg_reachable(
    parsed: &ParsedFile,
    func: &Node<'_>,
    file: &str,
    ref_line: usize,
    cfg_set: &BTreeSet<(String, usize)>,
) -> bool {
    if cfg_set.contains(&(file.to_string(), ref_line)) {
        return true;
    }
    reachable_multiline_node_contains_line(parsed, *func, file, ref_line, cfg_set)
}

fn reachable_multiline_node_contains_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    file: &str,
    ref_line: usize,
    cfg_set: &BTreeSet<(String, usize)>,
) -> bool {
    if !node_contains_line(&node, ref_line) {
        return false;
    }

    let start_line = node.start_position().row + 1;
    if start_line < ref_line
        && cfg_set.contains(&(file.to_string(), start_line))
        && (parsed.language.is_call_node(node.kind())
            || parsed.language.is_assignment_node(node.kind())
            || matches!(node.kind(), "return_statement" | "expression_statement"))
    {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if reachable_multiline_node_contains_line(parsed, child, file, ref_line, cfg_set) {
            return true;
        }
    }
    false
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
pub(crate) enum SinkMatchOutcome {
    Match(&'static SinkPattern),
    SemanticallyExcluded,
    NoMatch,
}

fn call_literal_arg(parsed: &ParsedFile, call: &Node<'_>, arg_idx: usize) -> Option<String> {
    let arg = call_arg_node(call, arg_idx)?;
    let text = parsed.node_text(&arg).trim();
    if matches!(
        arg.kind(),
        "interpreted_string_literal" | "raw_string_literal" | "string"
    ) || ((text.starts_with('"') || text.starts_with('\'') || text.starts_with('`'))
        && text.len() >= 2)
    {
        if text.starts_with('`') && text.contains("${") {
            return None;
        }
        let quote_idx = text.find(['"', '\'', '`']).unwrap_or(0);
        let prefix = &text[..quote_idx];
        if prefix.chars().any(|c| c == 'f' || c == 'F') {
            return None;
        }
        let without_prefix = &text[quote_idx..];
        let trimmed = without_prefix
            .strip_prefix("\"\"\"")
            .and_then(|s| s.strip_suffix("\"\"\""))
            .or_else(|| {
                without_prefix
                    .strip_prefix("'''")
                    .and_then(|s| s.strip_suffix("'''"))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('`')
                    .and_then(|s| s.strip_suffix('`'))
            })
            .unwrap_or(without_prefix);
        return Some(trimmed.to_string());
    }
    None
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
    let arg_node = match call_arg_node(call, arg_idx) {
        Some(n) => n,
        None => return false,
    };
    arg_node_taints_match(parsed, &arg_node, path)
}

/// Walk `arg_node` and any descendants for identifiers that are tainted on `path` at
/// the identifier's own source line in `parsed`'s file. Returns true on first hit.
fn arg_node_taints_match(parsed: &ParsedFile, arg_node: &Node<'_>, path: &FlowPath) -> bool {
    match arg_node.kind() {
        // Literal kinds — definitely not tainted.
        "interpreted_string_literal"
        | "raw_string_literal"
        | "rune_literal"
        | "int_literal"
        | "float_literal"
        | "number"
        | "string"
        | "imaginary_literal"
        | "true"
        | "false"
        | "null"
        | "undefined"
        | "nil" => false,

        // Bare identifier — direct check with file scoping.
        "identifier" => {
            let name = parsed.node_text(arg_node);
            let identifier_line = arg_node.start_position().row + 1;
            path.edges.iter().any(|e| {
                if e.to.file != parsed.path
                    || e.to.line != identifier_line
                    || e.to.var_name() != name
                {
                    return false;
                }
                let source_ranges = js_ts_request_source_access_ranges_for_alias_on_line(
                    parsed,
                    identifier_line,
                    name,
                );
                source_ranges.is_empty() || node_in_ranges(arg_node, &source_ranges)
            })
        }

        // Composite expression — recurse into descendants. Conservative: any
        // tainted identifier within counts.
        _ => {
            let mut cursor = arg_node.walk();
            for child in arg_node.named_children(&mut cursor) {
                if arg_node_taints_match(parsed, &child, path) {
                    return true;
                }
            }
            false
        }
    }
}

struct PathSanitizerBinding {
    kind: PathSanitizerKind,
    result_var: String,
    call_line: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GuardControl {
    RejectBranch,
    AllowBranch,
}

struct UrlSanitizerBinding {
    url_var: String,
    result_var: String,
    call_line: usize,
}

struct JsTsPathSanitizerBinding {
    result_var: String,
    call_line: usize,
}

struct SafeFormatHtmlBinding {
    result_var: String,
    call_line: usize,
}

fn flow_path_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            Some(path),
            sink_line,
            Some(sink_pat),
            None,
        );
    }
    path.cleansed_for.contains(&sink_pat.category)
}

fn flow_path_cleansed_for_sink_call(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    call: &Node<'_>,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            Some(path),
            sink_line,
            Some(sink_pat),
            Some(call),
        );
    }
    if parsed.language == Language::Python {
        if sink_pat.category == SanitizerCategory::Sqli
            && python_sql_call_is_parametrized(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Deserialization
            && python_yaml_load_uses_safe_loader(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Xss
            && python_xss_cleansed_for_sink(parsed, path, sink_line, sink_pat, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Ssrf {
            return python_ssrf_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call);
        }
    }
    if is_js_ts_language(parsed.language) {
        if sink_pat.category == SanitizerCategory::Sqli
            && js_ts_sql_call_is_parametrized(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Deserialization
            && js_ts_yaml_load_uses_safe_schema(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Ssrf {
            return js_ts_ssrf_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call);
        }
        if sink_pat.category == SanitizerCategory::PathTraversal
            && js_ts_path_traversal_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::OsCommand
            && js_ts_exec_file_is_literal_binary(parsed, call)
        {
            return true;
        }
    }
    path.cleansed_for.contains(&sink_pat.category)
}

fn source_line_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            None,
            sink_line,
            Some(sink_pat),
            None,
        );
    }
    function_body_cleansed_for(parsed, sink_line, sink_pat.category)
}

fn structured_sink_line_cleansed_for_path(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Python {
        let mut calls = Vec::new();
        collect_calls(parsed, parsed.tree.root_node(), &mut calls);

        let mut matched = false;
        for call in &calls {
            if call.start_position().row + 1 != line {
                continue;
            }
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !sink_call_path_matches(parsed, call, &actual, sink_pat)
                || !call_passes_sink_semantics(parsed, call, sink_pat)
            {
                continue;
            }
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
                continue;
            }
            matched = true;
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
                return false;
            }
        }

        return matched;
    }

    if is_js_ts_language(parsed.language) {
        let mut calls = Vec::new();
        collect_calls(parsed, parsed.tree.root_node(), &mut calls);

        let mut matched = false;
        for call in &calls {
            if !node_contains_line(call, line) {
                continue;
            }
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !sink_call_path_matches(parsed, call, &actual, sink_pat)
                || !call_passes_sink_semantics(parsed, call, sink_pat)
            {
                continue;
            }
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
                continue;
            }
            matched = true;
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
                return false;
            }
        }

        return matched || path.cleansed_for.contains(&sink_pat.category);
    }

    if parsed.language != Language::Go || sink_pat.category != SanitizerCategory::PathTraversal {
        return path.cleansed_for.contains(&sink_pat.category);
    }

    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);

    let mut matched = false;
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if actual != sink_pat.call_path || !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
        }
        if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
            continue;
        }
        matched = true;
        if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
            return false;
        }
    }

    matched
}

fn assignment_lhs_identifiers(parsed: &ParsedFile, lhs: &Node<'_>) -> Vec<String> {
    if lhs.kind() == "identifier" {
        return vec![parsed.node_text(lhs).to_string()];
    }
    let mut names = Vec::new();
    let mut cursor = lhs.walk();
    for child in lhs.named_children(&mut cursor) {
        if child.kind() == "identifier" {
            names.push(parsed.node_text(&child).to_string());
        }
    }
    names
}

fn assignment_rhs_expressions<'a>(rhs: &Node<'a>) -> Vec<Node<'a>> {
    if rhs.kind() == "call_expression" {
        return vec![*rhs];
    }
    let mut items = Vec::new();
    let mut cursor = rhs.walk();
    for child in rhs.named_children(&mut cursor) {
        items.push(child);
    }
    if items.is_empty() {
        items.push(*rhs);
    }
    items
}

fn path_targets_var_at_line(
    parsed: &ParsedFile,
    path: &FlowPath,
    line: usize,
    var_name: &str,
) -> bool {
    path.edges
        .iter()
        .any(|e| e.to.file == parsed.path && e.to.line == line && e.to.var_name() == var_name)
}

fn sink_call_has_tainted_arg_in_path(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool {
    sink_pat
        .tainted_arg_indices
        .iter()
        .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, path))
}

fn sink_call_uses_var_in_tainted_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
    var_name: &str,
) -> bool {
    let arguments = match call.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if sink_pat.tainted_arg_indices.contains(&idx)
            && node_contains_identifier(parsed, &arg, var_name)
        {
            return true;
        }
    }
    false
}

fn node_contains_identifier(parsed: &ParsedFile, node: &Node<'_>, var_name: &str) -> bool {
    if node.kind() == "identifier" && parsed.node_text(node) == var_name {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_identifier(parsed, &child, var_name) {
            return true;
        }
    }
    false
}

fn collect_if_statements<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "if_statement" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_if_statements(child, out);
    }
}

fn first_statement_line(parsed: &ParsedFile, node: &Node<'_>) -> Option<usize> {
    if parsed.language.is_statement_node(node.kind()) {
        return Some(node.start_position().row + 1);
    }
    for child in crate::go_statements::children(*node, parsed.language) {
        if parsed.language.is_statement_node(child.kind()) {
            return Some(child.start_position().row + 1);
        }
        if parsed.language.is_scope_block(child.kind()) {
            if let Some(line) = first_statement_line(parsed, &child) {
                return Some(line);
            }
        }
    }
    None
}

fn block_ends_with_return(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    last_statement_node(parsed, node).is_some_and(|n| parsed.language.is_return_node(n.kind()))
}

fn last_statement_node<'a>(parsed: &ParsedFile, node: &Node<'a>) -> Option<Node<'a>> {
    let mut last = None;
    for child in crate::go_statements::children(*node, parsed.language) {
        if parsed.language.is_statement_node(child.kind()) {
            last = Some(child);
        } else if parsed.language.is_scope_block(child.kind()) {
            if let Some(n) = last_statement_node(parsed, &child) {
                last = Some(n);
            }
        }
    }
    last
}

fn safe_successor_line(
    cpg: &CodePropertyGraph,
    parsed: &ParsedFile,
    if_node: &Node<'_>,
    reject_entry: usize,
) -> Option<usize> {
    let if_line = if_node.start_position().row + 1;
    let if_idx = cpg.statement_at(&parsed.path, if_line)?;
    cpg.cfg_successors(if_idx)
        .into_iter()
        .map(|idx| cpg.node(idx).line())
        .find(|line| *line != reject_entry)
}

fn cfg_line_reaches(
    cpg: &CodePropertyGraph,
    file: &str,
    start_line: usize,
    target_line: usize,
) -> bool {
    start_line == target_line
        || cpg
            .cfg_reachable_lines(file, start_line)
            .contains(&(file.to_string(), target_line))
}

fn node_contains_line(node: &Node<'_>, line: usize) -> bool {
    let row = line.saturating_sub(1);
    node.start_position().row <= row && row <= node.end_position().row
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
pub(crate) fn line_matches_structured_sink(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let mut had_call_path_match = false;
    for call in &calls {
        let call_on_line = if matches!(
            parsed.language,
            Language::Python | Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            node_contains_line(call, line)
        } else {
            call.start_position().row + 1 == line
        };
        if !call_on_line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if !sink_call_path_matches(parsed, call, &actual, sink_pat) {
            continue;
        }
        had_call_path_match = true;
        if !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
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
                .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, p))
                || js_ts_sink_call_has_inline_framework_source_arg(parsed, call, sink_pat);
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

fn sink_call_path_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
) -> bool {
    if is_js_ts_language(parsed.language) && sink_pat.category == SanitizerCategory::Ssrf {
        return js_ts_ssrf_sink_call_path_matches(parsed, call, actual, sink_pat.call_path);
    }
    if is_js_ts_language(parsed.language)
        && sink_pat.category == SanitizerCategory::Deserialization
        && sink_pat.call_path == "load"
    {
        return js_ts_js_yaml_bare_load_call_matches(parsed, call, actual);
    }
    if call_path_matches(parsed, actual, sink_pat.call_path) {
        return true;
    }
    if parsed.language != Language::Python || sink_pat.category != SanitizerCategory::Ssrf {
        return false;
    }
    if sink_pat.call_path == "urllib3.PoolManager.request" {
        return python_is_urllib3_pool_manager_request_call(parsed, call);
    }
    if sink_pat.call_path == "aiohttp.request" {
        return python_is_aiohttp_request_call(parsed, call);
    }
    let Some(method) = sink_pat.call_path.strip_prefix("aiohttp.ClientSession.") else {
        return false;
    };
    python_is_aiohttp_client_session_method_call(parsed, call, method)
}

fn structured_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    match parsed.language {
        Language::Go => go_sink_outcome(parsed, line, path),
        Language::Python => python_sink_outcome(parsed, line, path),
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            js_ts_sink_outcome(parsed, line, path)
        }
        _ => SinkMatchOutcome::NoMatch,
    }
}

fn collect_assignments<'a>(node: Node<'a>, parsed: &ParsedFile, out: &mut Vec<Node<'a>>) {
    if parsed.language.is_assignment_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_assignments(child, parsed, out);
    }
}

fn call_passes_sink_semantics(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
) -> bool {
    if is_js_yaml_load_sink_pattern(parsed, sink_pat) {
        return js_yaml_load_call_uses_unsafe_schema(parsed, call);
    }
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
    cpg: &CodePropertyGraph,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool {
    if actual != sink_pat.call_path {
        return false;
    }
    if !call_passes_sink_semantics(parsed, call, sink_pat) {
        return false;
    }
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        let call_line = call.start_position().row + 1;
        if sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, call_line, sink_pat, call) {
                return false;
            }
        } else if path
            .cleansed_for
            .contains(&SanitizerCategory::PathTraversal)
        {
            // Flat substring matching has no per-arg precision. For diff-line
            // artifact paths whose structured sink is SemanticallyExcluded, still
            // suppress only identifiers inside this specific safely-guarded call.
            if !go_path_traversal_cleansed_for_sink(
                parsed,
                cpg,
                None,
                call_line,
                Some(sink_pat),
                Some(call),
            ) {
                return false;
            }
        } else {
            return false;
        }
    } else if !path.cleansed_for.contains(&sink_pat.category) {
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
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    if parsed.language == Language::Python {
        return python_safe_structured_sink_call_ranges(parsed, cpg, line, path);
    }
    if is_js_ts_language(parsed.language) {
        return js_ts_safe_structured_sink_call_ranges(parsed, cpg, line, path);
    }
    if parsed.language != Language::Go {
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
            if push_cleansed_structured_sink_range(
                &mut ranges,
                parsed,
                cpg,
                call,
                &actual,
                pat,
                path,
            ) {
                pushed = true;
                break;
            }
        }
        if pushed {
            continue;
        }
        for pat in GO_CWE22_SINKS {
            if push_cleansed_structured_sink_range(
                &mut ranges,
                parsed,
                cpg,
                call,
                &actual,
                pat,
                path,
            ) {
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
                    cpg,
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
    if !crate::sanitizers::sanitizer_supported(parsed.language) {
        return;
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

/// Build a Chain-shaped `SliceGraph` representing one source-to-sink taint path.
///
/// `intermediate` contains any intermediate step nodes (file, line, line-text)
/// ordered from source to sink. For Plan 1 the caller passes `&[]`; later plans
/// can populate it from `FlowPath` edges.
fn build_taint_chain_diagram(
    source_file: &str,
    source_line: usize,
    source_text: &str,
    sink_file: &str,
    sink_line: usize,
    sink_text: &str,
    intermediate: &[(String, usize, String)],
) -> SliceGraph {
    // Degenerate case: source == sink with no intermediates. Emit a single
    // Sink node rather than a duplicate-id pair.
    if intermediate.is_empty() && source_file == sink_file && source_line == sink_line {
        let node = GraphNode {
            id: safe_node_id(sink_file, sink_line),
            label: format!("{}:{}\n{}", sink_file, sink_line, sink_text),
            kind: NodeKind::Sink,
            file: Some(sink_file.to_string()),
            line: Some(sink_line),
        };
        return SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![node],
            edges: vec![],
            clusters: vec![],
            mermaid: String::new(),
        };
    }
    // Normal path: build [source, ...intermediate, sink] chain.
    let mut nodes = vec![GraphNode {
        id: safe_node_id(source_file, source_line),
        label: format!("{}:{}\n{}", source_file, source_line, source_text),
        kind: NodeKind::Source,
        file: Some(source_file.to_string()),
        line: Some(source_line),
    }];
    for (f, l, t) in intermediate {
        nodes.push(GraphNode {
            id: safe_node_id(f, *l),
            label: format!("{}:{}\n{}", f, l, t),
            kind: NodeKind::Step,
            file: Some(f.clone()),
            line: Some(*l),
        });
    }
    nodes.push(GraphNode {
        id: safe_node_id(sink_file, sink_line),
        label: format!("{}:{}\n{}", sink_file, sink_line, sink_text),
        kind: NodeKind::Sink,
        file: Some(sink_file.to_string()),
        line: Some(sink_line),
    });
    let edges: Vec<GraphEdge> = nodes
        .windows(2)
        .map(|pair| GraphEdge {
            from: pair[0].id.clone(),
            to: pair[1].id.clone(),
            label: Some("tainted".to_string()),
            style: EdgeStyle::Solid,
        })
        .collect();
    SliceGraph {
        title: Some("Data flow".to_string()),
        shape: GraphShape::Chain,
        nodes,
        edges,
        clusters: vec![],
        mermaid: String::new(),
    }
}

/// Extract the trimmed text of a 1-indexed line from a source string.
/// Returns an empty string if the line number is out of range.
fn source_line_text(source: &str, line: usize) -> String {
    source
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .trim()
        .to_string()
}

fn evidence_rank(evidence: &EvidencePath) -> u8 {
    if evidence.crossed_unlabeled || evidence.hops.is_empty() {
        0
    } else if evidence.hops.iter().all(|hop| match hop {
        EvidenceHop::DataFlow { confidence, .. } => confidence.is_exact(),
        EvidenceHop::Call { confidence, .. } => {
            *confidence == crate::resolution::ResolutionConfidence::Exact
        }
    }) {
        2
    } else {
        1
    }
}

fn keep_best_evidence<K: Ord>(map: &mut BTreeMap<K, EvidencePath>, key: K, evidence: EvidencePath) {
    match map.get(&key) {
        Some(current) if evidence_rank(current) >= evidence_rank(&evidence) => {}
        _ => {
            map.insert(key, evidence);
        }
    }
}

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    taint_config: &TaintConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);

    // Collect taint sources. Line-scoped seeds preserve existing behavior;
    // target-scoped seeds are used by framework handler parameters.
    let mut taint_seeds: Vec<TaintSeed> = taint_config
        .sources
        .iter()
        .map(|(file, line)| TaintSeed::line(file.clone(), *line))
        .collect();

    if taint_config.taint_from_diff {
        for diff_info in &diff.files {
            // Files prism cannot parse (e.g. Cargo.toml, .md) are never keys
            // of `ctx.files`, so they can never be traced; seeding them only
            // produces an untraceable `taint_source` finding (P1 Change 1).
            if !ctx.files.contains_key(&diff_info.file_path) {
                continue;
            }
            for &line in &diff_info.diff_lines {
                taint_seeds.push(TaintSeed::line(diff_info.file_path.clone(), line));
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
        taint_seeds.push(TaintSeed::line(ipc_src.0.clone(), ipc_src.1));
    }
    let ipc_source_set: BTreeSet<(String, usize)> = ipc_sources.into_iter().collect();

    // Add framework-aware taint sources (Phase 1 Go: net/http, gin, gorilla/mux).
    // For each Go file with a detected framework, every call to a framework
    // SourcePattern (`c.Query`, `r.URL.Query`, `mux.Vars`, …) is a taint source.
    // These extend (not replace) diff-line and IPC sources.
    let framework_sources: Vec<TaintSeed> = detect_framework_sources(ctx);
    for fw_src in &framework_sources {
        taint_seeds.push(fw_src.clone());
    }
    taint_seeds.sort();
    taint_seeds.dedup();
    let taint_sources: Vec<(String, usize)> = taint_seeds
        .iter()
        .filter(|s| s.target.is_none())
        .map(|s| (s.file.clone(), s.line))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // Lines whose identifiers are recognized framework SOURCE calls or JS/TS
    // request-data accesses (e.g. `r.URL.Query()`, `c.Query()`,
    // `req.query.term`). These overlap textually with the cross-language flat
    // sink registry — `Query` is in SINK_PATTERNS as a generic `sql.Query`
    // substring matcher — so without this set, a tainted source line would
    // double-fire as a sink. Used during sink evaluation to suppress flat
    // substring matches on lines positively identified as sources; structured
    // sink checks still run for true source==sink lines.
    let framework_source_set: BTreeSet<(String, usize)> = framework_sources
        .iter()
        .filter(|s| {
            s.target.is_none()
                || ctx
                    .files
                    .get(&s.file)
                    .is_some_and(|parsed| is_js_ts_language(parsed.language))
        })
        .map(|s| (s.file.clone(), s.line))
        .collect();
    let framework_source_target_ranges =
        js_ts_framework_source_target_ranges_by_line(ctx.files, &framework_sources);
    let framework_source_access_ranges =
        js_ts_framework_source_access_ranges_by_line(ctx.files, &framework_source_set);

    // Forward propagation from each source (CFG-constrained when available)
    let (mut paths, flow_evidence) = if taint_config.return_flow {
        ctx.cpg
            .taint_forward_cfg_with_return_flow_labeled(&taint_sources)
    } else {
        ctx.cpg.taint_forward_cfg_labeled(&taint_sources)
    };
    synthesize_target_seed_paths(&framework_sources, ctx, &mut paths);

    let mut evidence_by_lines: BTreeMap<(String, usize, String, usize), EvidencePath> =
        BTreeMap::new();
    for path in &paths {
        for edge in &path.edges {
            let evidence = flow_evidence
                .get(&(edge.from.clone(), edge.to.clone()))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
            keep_best_evidence(
                &mut evidence_by_lines,
                (
                    edge.from.file.clone(),
                    edge.from.line,
                    edge.to.file.clone(),
                    edge.to.line,
                ),
                evidence,
            );
        }
    }

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
    // For each sink location, the set of source locations that reach it via a FlowPath.
    // Empty set means "added by non-path code" — caller falls back to file-scan heuristic.
    let mut sink_to_path_sources: BTreeMap<(String, usize), BTreeSet<(String, usize)>> =
        BTreeMap::new();
    // Selected witness retained at the moment a FlowPath target is accepted as
    // a sink. This is independent of the same-file source chosen for legacy
    // display text.
    let mut sink_to_path_evidence: BTreeMap<(String, usize), EvidencePath> = BTreeMap::new();

    // Flat built-in sinks are decided by `taint_sinks` (call position and
    // receiver, #622 S1); caller-supplied and wrapper sinks keep name matching.
    let sink_classifier = taint_sinks::SinkClassifier::default();
    let extra_sinks: Vec<&str> = taint_config
        .extra_sinks
        .iter()
        .map(|s| s.as_str())
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
                let outcome = structured_sink_outcome(parsed, edge.to.line, Some(path));
                let cleansed_structured_ranges =
                    cleansed_structured_sink_call_ranges(parsed, &ctx.cpg, edge.to.line, path);
                let structured_suppressed_by_cleanser = match outcome {
                    SinkMatchOutcome::Match(p) => structured_sink_line_cleansed_for_path(
                        parsed,
                        &ctx.cpg,
                        path,
                        edge.to.line,
                        p,
                    ),
                    _ => false,
                };

                // Suppress flat substring matches on lines that are recognized
                // framework SOURCE calls — e.g., `Query` would otherwise fire
                // as a flat sink on `r.URL.Query()` even though that's the
                // source, not a sink. Structured Go sinks (which we know are
                // sinks, not sources) are not affected by this filter.
                let is_framework_source_line =
                    framework_source_set.contains(&(edge.to.file.clone(), edge.to.line));
                let framework_source_access_ranges =
                    framework_source_access_ranges.get(&(edge.to.file.clone(), edge.to.line));
                let framework_source_target_ranges =
                    framework_source_target_ranges.get(&(edge.to.file.clone(), edge.to.line));
                let is_js_ts_framework_source_line =
                    is_framework_source_line && is_js_ts_language(parsed.language);
                let ids = parsed.identifiers_on_line(edge.to.line);
                for id in &ids {
                    if node_in_ranges(id, &cleansed_structured_ranges) {
                        continue;
                    }
                    let text = parsed.node_text(id);
                    if is_framework_source_line
                        && (!is_js_ts_language(parsed.language)
                            || js_ts_request_source_identifier(text)
                            || framework_source_target_ranges
                                .is_some_and(|ranges| node_in_ranges(id, ranges))
                            || framework_source_access_ranges
                                .is_some_and(|ranges| node_in_ranges(id, ranges)))
                    {
                        continue;
                    }
                    let flat_sink = sink_classifier
                        .classify(&edge.to.file, parsed, *id, text, SINK_PATTERNS)
                        .is_some();
                    if !flat_sink && !extra_sinks.iter().any(|s| matches_sink(text, s)) {
                        continue;
                    }
                    if is_js_ts_framework_source_line
                        && !js_ts_flat_sink_identifier_has_tainted_request_source(
                            parsed,
                            id,
                            path,
                            framework_source_access_ranges
                                .map(|ranges| ranges.as_slice())
                                .unwrap_or(&[]),
                        )
                    {
                        continue;
                    }
                    {
                        let sink = (edge.to.file.clone(), edge.to.line);
                        sink_lines.insert(sink.clone());
                        if let Some(first_edge) = path.edges.first() {
                            sink_to_path_sources
                                .entry(sink.clone())
                                .or_default()
                                .insert((first_edge.from.file.clone(), first_edge.from.line));
                        }
                        if let Some(evidence) = evidence_by_lines
                            .get(&(
                                edge.from.file.clone(),
                                edge.from.line,
                                edge.to.file.clone(),
                                edge.to.line,
                            ))
                            .cloned()
                        {
                            keep_best_evidence(&mut sink_to_path_evidence, sink, evidence);
                        }
                    }
                }

                // Structured sinks (Go Phase 1 plus Python Phase 2).
                if matches!(outcome, SinkMatchOutcome::Match(_))
                    && !structured_suppressed_by_cleanser
                {
                    let sink = (edge.to.file.clone(), edge.to.line);
                    sink_lines.insert(sink.clone());
                    if let Some(first_edge) = path.edges.first() {
                        sink_to_path_sources
                            .entry(sink.clone())
                            .or_default()
                            .insert((first_edge.from.file.clone(), first_edge.from.line));
                    }
                    if let Some(evidence) = evidence_by_lines
                        .get(&(
                            edge.from.file.clone(),
                            edge.from.line,
                            edge.to.file.clone(),
                            edge.to.line,
                        ))
                        .cloned()
                    {
                        keep_best_evidence(&mut sink_to_path_evidence, sink, evidence);
                    }
                }
            }
        }
    }

    for (file, parsed) in ctx.files {
        if is_js_ts_language(parsed.language) {
            for line in js_ts_inline_framework_source_yaml_sink_lines(parsed) {
                sink_lines.insert((file.clone(), line));
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
            if parsed.language == Language::Python {
                if let Some(pat) = python_sink_with_inline_flask_source(parsed, *line) {
                    let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, pat);
                    if !cleansed {
                        sink_lines.insert((file.clone(), *line));
                    }
                }
                continue;
            }
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
                let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, sink_pat);
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
                        if !flow_path_cleansed_for_sink(parsed, &ctx.cpg, p, *line, pat) {
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
                        let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, pat);
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

    // Sources whose location was actually selected as the reported origin of
    // an emitted sink finding (P1 Change 2). `taint_source` findings are only
    // emitted for members of this set — built up below as each sink finding's
    // `source_location` is resolved — instead of unconditionally for every
    // seed (most diff-line seeds never reach a sink and are noise).
    let mut sources_with_emitted_sinks: BTreeSet<(String, usize)> = BTreeSet::new();
    let mut evidence_for_sources: BTreeMap<(String, usize), EvidencePath> = BTreeMap::new();

    // Emit findings for each taint sink reached
    for (file, line) in &sink_lines {
        // Find a source that reaches this sink.
        // Prefer a source that the actual FlowPath identified as flowing into this sink.
        // Pick the one nearest to the sink (largest line number that's still <= sink line)
        // to remain consistent with the existing "nearest-in-file" semantics for the
        // reviewer-friendly source description.
        let path_derived_source: Option<(&str, usize)> = sink_to_path_sources
            .get(&(file.clone(), *line))
            .and_then(|set| {
                set.iter()
                    .filter(|(sf, sl)| sf == file && *sl <= *line)
                    .max_by_key(|(_, sl)| *sl)
                    // #602: no same-file path source -> name a cross-file one.
                    .or_else(|| set.iter().find(|(sf, _)| sf != file))
                    .map(|(sf, sl)| (sf.as_str(), *sl))
            });

        // Fall back to existing heuristic for sinks added without path linkage
        // (e.g., YAML inline-framework-source sinks, IPC sources).
        // Prefer the nearest IPC source before the sink (user-controlled IPC reads
        // are the semantically interesting starting point for confused-deputy analysis).
        // Then fall back to the nearest diff-line source, then any source in the same file.
        let source_location: Option<(&str, usize)> = path_derived_source.or_else(|| {
            ipc_source_set
                .iter()
                .filter(|(sf, sl)| sf == file && *sl < *line)
                .max_by_key(|(_, sl)| *sl)
                .map(|(sf, sl)| (sf.as_str(), *sl))
                .or_else(|| {
                    taint_sources
                        .iter()
                        .filter(|(sf, sl)| sf == file && *sl < *line)
                        .max_by_key(|(_, sl)| *sl)
                        .map(|(sf, sl)| (sf.as_str(), *sl))
                })
                .or_else(|| {
                    taint_sources
                        .iter()
                        .find(|(sf, _)| sf == file)
                        .map(|(sf, sl)| (sf.as_str(), *sl))
                })
        });
        // Every path-derived source recorded for this sink genuinely reached
        // it via a real FlowPath — including cross-file ones. `path_derived_source`
        // above is filtered to the same file as the sink (a reviewer-friendly
        // "nearest in this file" pick for `source_desc`); it must not gate
        // which sources earn a `taint_source` finding, or a source that
        // reaches a sink in a *different* file (e.g. a cross-file call chain)
        // is silently dropped (P1 review-fix F1). Union in the whole set.
        if let Some(set) = sink_to_path_sources.get(&(file.clone(), *line)) {
            for (sf, sl) in set {
                sources_with_emitted_sinks.insert((sf.clone(), *sl));
                let evidence = evidence_by_lines
                    .get(&(sf.clone(), *sl, file.clone(), *line))
                    .cloned()
                    .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
                keep_best_evidence(&mut evidence_for_sources, (sf.clone(), *sl), evidence);
            }
        }

        // Record the exact source location chosen for this sink finding:
        // the path-derived source unconditionally if available, otherwise
        // the fallback source only if it is itself a member of
        // `taint_sources` (the ipc/nearest-in-file fallbacks all resolve to
        // members of `taint_sources`, but guard explicitly per spec).
        match path_derived_source {
            Some((sf, sl)) => {
                sources_with_emitted_sinks.insert((sf.to_string(), sl));
            }
            None => {
                if let Some((sf, sl)) = source_location {
                    if taint_sources.iter().any(|(f, l)| f == sf && *l == sl) {
                        sources_with_emitted_sinks.insert((sf.to_string(), sl));
                        keep_best_evidence(
                            &mut evidence_for_sources,
                            (sf.to_string(), sl),
                            EvidencePath::unlabeled_for("taint"),
                        );
                    }
                }
            }
        }

        let source_desc = source_location
            .map(|(sf, sl)| {
                if sf == file.as_str() {
                    format!("line {}", sl)
                } else {
                    format!("{}:{}", sf, sl)
                }
            })
            .unwrap_or_else(|| "diff lines".to_string());

        // Build a Chain diagram for this sink finding.
        // If we have an identified source location, use it as the Source node;
        // otherwise fall back to a sink-only diagram (Source == Sink as the taint origin).
        let sink_text = ctx
            .files
            .get(file.as_str())
            .map(|p| source_line_text(&p.source, *line))
            .unwrap_or_default();
        let diagram = if let Some((src_file, src_line)) = source_location {
            let src_text = ctx
                .files
                .get(src_file)
                .map(|p| source_line_text(&p.source, src_line))
                .unwrap_or_default();
            build_taint_chain_diagram(src_file, src_line, &src_text, file, *line, &sink_text, &[])
        } else {
            // No upstream source resolved; emit a single-step chain with sink only.
            // Use the sink as both source and sink nodes to satisfy the Chain contract.
            build_taint_chain_diagram(file, *line, &sink_text, file, *line, &sink_text, &[])
        };

        // #636: a sink whose argument shape defuses it is reported at `suggestion`.
        let severity = ctx.files.get(file.as_str()).map_or("warning", |parsed| {
            let is_extra = |t: &str| extra_sinks.iter().any(|s| matches_sink(t, s));
            sink_classifier
                .line_strength(file, parsed, *line, &is_extra)
                .severity()
        });
        let mut finding = SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: severity.to_string(),
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
            diagrams: vec![],
        };
        finding.diagrams.push(diagram);
        let finding_evidence = source_location
            .and_then(|(source_file, source_line)| {
                evidence_by_lines.get(&(source_file.to_string(), source_line, file.clone(), *line))
            })
            .or_else(|| sink_to_path_evidence.get(&(file.clone(), *line)))
            .cloned()
            .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
        result.findings.push(finding);
        result.evidence.push(Some(finding_evidence));
    }

    // Bash-specific: detect unquoted variable expansions on tainted lines.
    // In shell, unquoted $VAR in command arguments is a word-splitting /
    // injection vector. Runs BEFORE the gated source-emission loop below
    // (P1 review-fix F2): an unquoted-expansion finding is itself a
    // sink-style finding, so — like a sink finding — it must license its
    // reaching source into `sources_with_emitted_sinks` before that loop
    // decides which `taint_source` findings to emit. Without this, a bash
    // flow whose only sink-style finding is an unquoted expansion would
    // license no source and silently drop that source's `taint_source`
    // finding (a regression vs. the pre-Change-2 behavior of emitting every
    // source unconditionally).
    let unquoted = detect_unquoted_expansions(ctx.files, &all_tainted);
    for (file, line, var_name) in &unquoted {
        // Avoid duplicate findings if line is already flagged as a sink
        if !sink_lines.contains(&(file.clone(), *line)) {
            sink_lines.insert((file.clone(), *line));

            // License the nearest preceding same-file taint source (the
            // member of `taint_sources` with the same file and the largest
            // line <= this finding's line, if any) as having reached an
            // emitted sink-style finding.
            let unquoted_source = taint_sources
                .iter()
                .filter(|(sf, sl)| sf == file && *sl <= *line)
                .max_by_key(|(_, sl)| *sl)
                .cloned();
            if let Some((sf, sl)) = &unquoted_source {
                sources_with_emitted_sinks.insert((sf.clone(), *sl));
                let evidence = evidence_by_lines
                    .get(&(sf.clone(), *sl, file.clone(), *line))
                    .cloned()
                    .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
                keep_best_evidence(&mut evidence_for_sources, (sf.clone(), *sl), evidence);
            }

            let finding_evidence = unquoted_source
                .as_ref()
                .and_then(|(sf, sl)| evidence_by_lines.get(&(sf.clone(), *sl, file.clone(), *line)))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
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
                diagrams: vec![],
            });
            result.evidence.push(Some(finding_evidence));
        }
    }

    // Emit findings for each taint source that reaches an emitted sink-style
    // finding (P1 Change 2). Runs AFTER sink emission AND the
    // unquoted-expansion block above so `sources_with_emitted_sinks` is
    // fully populated; a source that reaches nothing contributes no finding.
    // Applies uniformly to diff-seeded, explicit `--taint-source`, IPC, and
    // framework sources alike, since all of them flow through the shared
    // `taint_sources` collection.
    for (file, line) in &taint_sources {
        if !sources_with_emitted_sinks.contains(&(file.clone(), *line)) {
            continue;
        }
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
            diagrams: vec![],
        });
        result.evidence.push(Some(
            evidence_for_sources
                .get(&(file.clone(), *line))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint")),
        ));
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
            function_start_line: 3,
            line: 3,
            path: AccessPath::simple(var_name),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Def,
        };
        let to = VarLocation {
            file: "main.go".to_string(),
            function: "handler".to_string(),
            function_start_line: 3,
            line,
            path: AccessPath::simple(var_name),
            start_byte: 0,
            end_byte: 0,
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

    #[test]
    fn build_taint_chain_diagram_collapses_self_loop() {
        let g = build_taint_chain_diagram(
            "foo.c",
            42,
            "x = sink_call(y)",
            "foo.c",
            42,
            "x = sink_call(y)",
            &[],
        );
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 0);
        assert!(matches!(g.nodes[0].kind, NodeKind::Sink));
    }

    #[test]
    fn build_taint_chain_diagram_distinct_source_sink() {
        let g = build_taint_chain_diagram(
            "foo.c",
            10,
            "x = read_input()",
            "foo.c",
            42,
            "system(x)",
            &[],
        );
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        assert!(matches!(g.nodes[0].kind, NodeKind::Source));
        assert!(matches!(g.nodes[1].kind, NodeKind::Sink));
    }
}
