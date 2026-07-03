//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgContext};
use crate::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use crate::languages::Language;
use crate::output::mermaid::safe_node_id;
use crate::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceFinding, SliceGraph, SliceResult,
    SlicingAlgorithm,
};
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

pub const PY_CWE79_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "mark_safe",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Markup",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "markupsafe.Markup",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "format_html",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "render_template_string",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
];

pub const PY_CWE89_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "execute",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "executemany",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "raw",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const PY_CWE918_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "requests.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "requests.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib.request.urlopen",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib.request.Request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "urllib3.PoolManager.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "httpx.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.put",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.delete",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.patch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.head",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.options",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "aiohttp.ClientSession.request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
];

pub const PY_CWE502_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "pickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "pickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cPickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cPickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cloudpickle.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "cloudpickle.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "yaml.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "jsonpickle.decode",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "marshal.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "marshal.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dill.loads",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dill.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

fn js_spawn_uses_shell_true(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .is_ok_and(|text| text.contains("shell") && text.contains("true"))
}

fn js_yaml_load_uses_unsafe_schema(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .map_or(true, |text| !js_yaml_load_text_uses_safe_schema(text))
}

fn js_yaml_load_text_uses_safe_schema(text: &str) -> bool {
    let yaml_receiver = js_yaml_load_receiver(text);
    js_text_top_level_call_args(text)
        .get(1)
        .is_some_and(|arg| js_yaml_schema_arg_is_safe(arg, yaml_receiver))
}

fn js_yaml_load_receiver(text: &str) -> Option<&str> {
    let callee = text.split_once('(')?.0.trim();
    callee.strip_suffix(".load")
}

fn js_yaml_schema_arg_is_safe(arg: &str, yaml_receiver: Option<&str>) -> bool {
    let arg = arg.trim();
    js_yaml_schema_expr_is_exact_safe(arg, yaml_receiver)
        || js_trusted_object_property_value_text(arg, "schema")
            .is_some_and(|value| js_yaml_schema_expr_is_exact_safe(value, yaml_receiver))
}

fn js_yaml_schema_expr_is_exact_safe(expr: &str, yaml_receiver: Option<&str>) -> bool {
    let expr = expr.trim().trim_end_matches(';').trim();
    let Some(yaml_receiver) = yaml_receiver else {
        return false;
    };
    ["SAFE_SCHEMA", "FAILSAFE_SCHEMA", "JSON_SCHEMA"]
        .iter()
        .any(|schema| expr == format!("{yaml_receiver}.{schema}"))
}

fn js_yaml_load_call_uses_unsafe_schema(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    call_arg_node(call, 1).is_none_or(|arg| !js_yaml_schema_arg_node_is_safe(parsed, call, &arg))
}

fn js_yaml_schema_arg_node_is_safe(parsed: &ParsedFile, call: &Node<'_>, arg: &Node<'_>) -> bool {
    let text = parsed.node_text(arg).trim();
    js_yaml_schema_expr_text_is_safe(parsed, call, text)
        || js_trusted_object_property_value_text(text, "schema")
            .is_some_and(|value| js_yaml_schema_expr_text_is_safe(parsed, call, value))
}

fn js_yaml_schema_expr_text_is_safe(parsed: &ParsedFile, call: &Node<'_>, expr: &str) -> bool {
    let expr = expr.trim().trim_end_matches(';').trim();
    for schema in ["SAFE_SCHEMA", "FAILSAFE_SCHEMA", "JSON_SCHEMA"] {
        if expr == schema
            && js_ts_identifier_binds_imported_member_at_call(parsed, call, expr, "js-yaml", schema)
        {
            return true;
        }
        if let Some(receiver) = expr.strip_suffix(schema).and_then(|p| p.strip_suffix('.')) {
            if !receiver.is_empty()
                && js_ts_identifier_binds_module_at_call(parsed, call, receiver, "js-yaml")
            {
                return true;
            }
        }
    }
    false
}

fn js_trusted_object_property_value_text<'a>(object_text: &'a str, key: &str) -> Option<&'a str> {
    let object_text = object_text.trim();
    let inner = object_text.strip_prefix('{')?.strip_suffix('}')?;
    let mut value = None;
    for prop in js_split_top_level_commas(inner) {
        if prop.is_empty() {
            continue;
        }
        if prop.trim_start().starts_with("...") {
            return None;
        }
        let Some(colon) = js_find_top_level_colon(prop) else {
            continue;
        };
        let prop_key_text = prop[..colon].trim();
        if prop_key_text.starts_with('[') {
            return None;
        }
        let prop_key = prop_key_text.trim_matches(['"', '\'', '`']);
        if prop_key == key {
            if value.is_some() {
                return None;
            }
            value = Some(prop[colon + 1..].trim());
        }
    }
    value
}

fn js_text_top_level_call_args(text: &str) -> Vec<&str> {
    let Some(open) = text.find('(') else {
        return Vec::new();
    };
    let body = &text[open + 1..];
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut args = Vec::new();

    for (idx, ch) in body.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                args.push(body[start..idx].trim());
                return args;
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(body[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    args
}

fn js_split_top_level_commas(text: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut quote = None;
    let mut escape = false;
    let mut parts = Vec::new();

    for (idx, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

fn js_find_top_level_colon(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escape = false;
    for (idx, ch) in text.char_indices() {
        if let Some(q) = quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

fn js_sql_call_is_not_parametrized(call: &CallSite) -> bool {
    call.call_node
        .utf8_text(call.source.as_bytes())
        .map_or(true, |text| {
            !(text.contains("bind") || text.contains("parameters"))
        })
}

pub const JS_CWE79_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "insertAdjacentHTML",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "dangerouslySetInnerHTML",
        category: SanitizerCategory::Xss,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE89_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "query",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_sql_call_is_not_parametrized),
    },
    SinkPattern {
        call_path: "execute",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_sql_call_is_not_parametrized),
    },
    SinkPattern {
        call_path: "raw",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "literal",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$where",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$queryRawUnsafe",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "$executeRawUnsafe",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE918_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "fetch",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "get",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "post",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "request",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "axios",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "got",
        category: SanitizerCategory::Ssrf,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE502_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "unserialize",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "yaml.load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_yaml_load_uses_unsafe_schema),
    },
    SinkPattern {
        call_path: "load",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: Some(js_yaml_load_uses_unsafe_schema),
    },
    SinkPattern {
        call_path: "deserialize",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "eval",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Function",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInNewContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInThisContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "runInContext",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "Script",
        category: SanitizerCategory::Deserialization,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
];

pub const JS_CWE78_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "execSync",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "spawn",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: Some(js_spawn_uses_shell_true),
    },
    SinkPattern {
        call_path: "spawnSync",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: Some(js_spawn_uses_shell_true),
    },
];

pub const JS_CWE22_SINKS: &[SinkPattern] = &[
    SinkPattern {
        call_path: "readFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "readFileSync",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "writeFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "writeFileSync",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "createReadStream",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "createWriteStream",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "unlink",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "rm",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "rename",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "sendFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "download",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
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

fn js_ts_alias_def(
    line: usize,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
) -> JsTsAliasDef {
    JsTsAliasDef {
        line,
        start_byte,
        scope,
        byte_scope,
        kills: Vec::new(),
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

/// Compute the textual call path for a call expression. For `r.URL.Query()`,
/// returns `Some("r.URL.Query")`; for Python `request.args.get()`, returns
/// `Some("request.args.get")`.
fn call_path_text(parsed: &ParsedFile, call_node: &Node<'_>) -> Option<String> {
    let func = call_node.child_by_field_name("function")?;
    Some(parsed.node_text(&func).to_string())
}

fn go_call_path_text(parsed: &ParsedFile, call_node: &Node<'_>) -> Option<String> {
    call_path_text(parsed, call_node)
}

fn is_js_ts_language(language: Language) -> bool {
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

/// Walk `root` collecting every call node for the file's language.
fn collect_calls<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if parsed.language.is_call_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls(parsed, child, out);
    }
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

fn detect_go_framework_sources(file_path: &str, parsed: &ParsedFile, sources: &mut Vec<TaintSeed>) {
    let spec = match parsed.framework() {
        Some(s) => s,
        None => return,
    };
    let target_types = framework_request_types(spec.name);
    if target_types.is_empty() {
        return;
    }

    for func in parsed.all_functions() {
        let param_names = collect_request_param_names(parsed, &func, target_types);
        if param_names.is_empty() {
            continue;
        }
        let mut calls = Vec::new();
        collect_go_calls(func, &mut calls);

        for source_pat in spec.sources {
            let concrete_paths: Vec<String> = if framework_prefixes(spec.name)
                .iter()
                .any(|p| source_pat.call_path.starts_with(p))
            {
                param_names
                    .iter()
                    .map(|n| substitute_prefix(source_pat.call_path, n, spec.name))
                    .collect()
            } else {
                vec![source_pat.call_path.to_string()]
            };

            for call in &calls {
                let actual = match go_call_path_text(parsed, call) {
                    Some(s) => s,
                    None => continue,
                };
                if concrete_paths.contains(&actual) {
                    sources.push(TaintSeed::line(
                        file_path.to_string(),
                        call.start_position().row + 1,
                    ));
                }
            }
        }
    }
}

fn detect_js_ts_framework_sources(
    file_path: &str,
    parsed: &ParsedFile,
    sources: &mut Vec<TaintSeed>,
) {
    let framework = match parsed.framework() {
        Some(spec) => spec.name,
        None => return,
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return;
    }
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);

    for func in parsed.all_functions() {
        let line = func.start_position().row + 1;
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }

        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }

        for param in &source_params {
            sources.push(TaintSeed::target(
                file_path.to_string(),
                line,
                AccessPath::simple(param.as_str()),
            ));
        }

        let assignment_sources =
            js_ts_request_data_assignment_sources(parsed, &func, framework, &source_params);
        for source in assignment_sources {
            sources.push(TaintSeed::target_scoped(
                file_path.to_string(),
                source.line,
                source.target,
                source.start_byte,
                source.scope,
                source.byte_scope,
            ));
        }
    }
}

#[derive(Clone)]
struct JsTsParam {
    name: String,
    has_nest_source_decorator: bool,
}

fn js_ts_function_params(parsed: &ParsedFile, func: &Node<'_>) -> Vec<JsTsParam> {
    let params = match find_js_ts_parameters_node(*func) {
        Some(n) => n,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut cursor = params.walk();
    for child in params.named_children(&mut cursor) {
        if let Some(name) = js_ts_param_name(parsed, &child) {
            let has_nest_source_decorator = js_ts_param_has_nest_source_decorator(parsed, &child);
            out.push(JsTsParam {
                name,
                has_nest_source_decorator,
            });
        }
    }
    out
}

fn find_js_ts_parameters_node(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(params) = node.child_by_field_name("parameters") {
        return Some(params);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "formal_parameters");
    found
}

fn js_ts_param_name(parsed: &ParsedFile, param: &Node<'_>) -> Option<String> {
    for field in ["pattern", "name", "parameter", "left"] {
        if let Some(node) = param.child_by_field_name(field) {
            if let Some(name) = js_ts_first_identifier(parsed, node) {
                return Some(name);
            }
        }
    }
    if parsed.language.is_identifier_node(param.kind()) {
        return Some(parsed.node_text(param).to_string());
    }
    let mut names = Vec::new();
    collect_identifier_names(parsed, *param, &mut names);
    names.into_iter().find(|name| {
        !matches!(
            name.as_str(),
            "Body"
                | "Query"
                | "Param"
                | "Headers"
                | "Req"
                | "Request"
                | "Get"
                | "Post"
                | "Put"
                | "Patch"
                | "Delete"
                | "All"
        )
    })
}

fn js_ts_first_identifier(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    if parsed.language.is_identifier_node(node.kind()) {
        return Some(parsed.node_text(&node).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = js_ts_first_identifier(parsed, child) {
            return Some(name);
        }
    }
    None
}

fn collect_identifier_names(parsed: &ParsedFile, node: Node<'_>, out: &mut Vec<String>) {
    if parsed.language.is_identifier_node(node.kind()) {
        out.push(parsed.node_text(&node).to_string());
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_identifier_names(parsed, child, out);
    }
}

fn js_ts_framework_source_params(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    params: &[JsTsParam],
    framework_receivers: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match framework {
        "nestjs" => {
            if !js_ts_has_route_decorator(parsed, func) {
                return out;
            }
            for param in params {
                if param.has_nest_source_decorator {
                    out.insert(param.name.clone());
                }
            }
        }
        "fastify" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if let Some(first) = params.first() {
                if matches!(first.name.as_str(), "request" | "req") {
                    out.insert(first.name.clone());
                }
            }
        }
        "express" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if params.len() >= 2
                && matches!(params[0].name.as_str(), "req" | "request")
                && matches!(params[1].name.as_str(), "res" | "response")
            {
                out.insert(params[0].name.clone());
            }
        }
        "koa" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if let Some(first) = params.first() {
                if first.name == "ctx" || first.name == "context" {
                    out.insert(first.name.clone());
                }
            }
        }
        _ => {}
    }
    out
}

fn js_ts_framework_receiver_names(parsed: &ParsedFile, framework: &str) -> BTreeSet<String> {
    let imports = parsed.extract_imports();
    let mut receivers = BTreeSet::new();
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    for assignment in assignments {
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if lhs.kind() != "identifier" {
            continue;
        }
        if js_ts_expr_constructs_framework_receiver(parsed, rhs, framework, &imports) {
            receivers.insert(parsed.node_text(&lhs).to_string());
        }
    }
    receivers
}

fn js_ts_is_framework_route_handler(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
) -> bool {
    let imports = parsed.extract_imports();
    let mut current = func.parent();
    while let Some(node) = current {
        if parsed.language.is_call_node(node.kind())
            && js_ts_call_args_contain_function(parsed, &node, func.id())
            && js_ts_route_call_matches_framework(
                parsed,
                &node,
                framework,
                framework_receivers,
                &imports,
            )
        {
            return true;
        }
        current = node.parent();
    }

    let binding_names = js_ts_function_binding_names(parsed, func);
    !binding_names.is_empty()
        && js_ts_registered_route_references_handler(
            parsed,
            framework,
            framework_receivers,
            &imports,
            &binding_names,
        )
}

fn js_ts_function_binding_names(parsed: &ParsedFile, func: &Node<'_>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Some(name) = parsed.language.function_name(func) {
        if parsed.language.is_identifier_node(name.kind())
            || matches!(
                name.kind(),
                "property_identifier" | "shorthand_property_identifier" | "field_identifier"
            )
        {
            names.insert(parsed.node_text(&name).to_string());
        }
    }
    names
}

fn js_ts_registered_route_references_handler(
    parsed: &ParsedFile,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
    binding_names: &BTreeSet<String>,
) -> bool {
    let mut calls = Vec::new();
    collect_js_ts_call_nodes(parsed.tree.root_node(), parsed, &mut calls);
    calls.iter().any(|call| {
        js_ts_route_call_matches_framework(parsed, call, framework, framework_receivers, imports)
            && js_ts_call_args_reference_handler_name(parsed, call, binding_names)
    })
}

fn collect_js_ts_call_nodes<'a>(node: Node<'a>, parsed: &ParsedFile, out: &mut Vec<Node<'a>>) {
    if parsed.language.is_call_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_call_nodes(child, parsed, out);
    }
}

fn js_ts_call_args_reference_handler_name(
    parsed: &ParsedFile,
    call: &Node<'_>,
    binding_names: &BTreeSet<String>,
) -> bool {
    let Some(args) = parsed.language.call_arguments(call) else {
        return false;
    };
    js_ts_node_references_handler_name(parsed, args, binding_names)
}

fn js_ts_node_references_handler_name(
    parsed: &ParsedFile,
    node: Node<'_>,
    binding_names: &BTreeSet<String>,
) -> bool {
    if parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    if parsed.language.is_identifier_node(node.kind())
        && binding_names.contains(parsed.node_text(&node))
    {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| js_ts_node_references_handler_name(parsed, child, binding_names));
    found
}

fn js_ts_call_args_contain_function(parsed: &ParsedFile, call: &Node<'_>, func_id: usize) -> bool {
    let Some(args) = parsed.language.call_arguments(call) else {
        return false;
    };
    js_ts_node_contains_function_without_intervening_function(parsed, args, func_id)
}

fn js_ts_node_contains_function_without_intervening_function(
    parsed: &ParsedFile,
    node: Node<'_>,
    func_id: usize,
) -> bool {
    if node.id() == func_id {
        return true;
    }
    if parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(|child| {
        js_ts_node_contains_function_without_intervening_function(parsed, child, func_id)
    });
    found
}

fn js_ts_route_call_matches_framework(
    parsed: &ParsedFile,
    call: &Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let method = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    if !js_ts_framework_route_method(framework, method) {
        return false;
    }
    let Some(receiver) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_receiver_expr_is_framework_instance(
        parsed,
        receiver,
        framework,
        framework_receivers,
        imports,
    ) || js_ts_receiver_expr_is_route_builder(
        parsed,
        receiver,
        framework,
        framework_receivers,
        imports,
    )
}

fn js_ts_framework_route_method(framework: &str, method: &str) -> bool {
    match framework {
        "express" => matches!(
            method,
            "get" | "post" | "put" | "patch" | "delete" | "all" | "use"
        ),
        "fastify" => matches!(
            method,
            "get" | "post" | "put" | "patch" | "delete" | "options" | "head" | "all" | "route"
        ),
        "koa" => matches!(
            method,
            "use" | "get" | "post" | "put" | "patch" | "delete" | "all"
        ),
        _ => false,
    }
}

fn js_ts_receiver_expr_is_framework_instance(
    parsed: &ParsedFile,
    receiver: Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let receiver = unwrap_parenthesized(receiver);
    if receiver.kind() == "identifier" {
        return framework_receivers.contains(parsed.node_text(&receiver));
    }
    js_ts_expr_constructs_framework_receiver(parsed, receiver, framework, imports)
}

fn js_ts_receiver_expr_is_route_builder(
    parsed: &ParsedFile,
    receiver: Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let receiver = unwrap_parenthesized(receiver);
    if !parsed.language.is_call_node(receiver.kind()) {
        return false;
    }
    let Some(function) = receiver.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    if parsed.node_text(&property) != "route" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_receiver_expr_is_framework_instance(
        parsed,
        object,
        framework,
        framework_receivers,
        imports,
    )
}

fn js_ts_expr_constructs_framework_receiver(
    parsed: &ParsedFile,
    expr: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if !matches!(expr.kind(), "call_expression" | "new_expression") {
        return false;
    }
    let Some(callee) = js_ts_call_or_new_callee(expr) else {
        return false;
    };
    js_ts_callee_constructs_framework_receiver(parsed, callee, framework, imports)
}

fn js_ts_call_or_new_callee(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("function")
        .or_else(|| node.child_by_field_name("constructor"))
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.named_child(0))
}

fn js_ts_callee_constructs_framework_receiver(
    parsed: &ParsedFile,
    callee: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let callee = unwrap_parenthesized(callee);
    if callee.kind() == "identifier" {
        let local = parsed.node_text(&callee);
        return imports
            .get(local)
            .is_some_and(|module| js_ts_module_matches_framework(module, framework));
    }

    if parsed.language.is_call_node(callee.kind()) {
        return js_ts_require_call_module(parsed, &callee)
            .is_some_and(|module| js_ts_module_matches_framework(&module, framework));
    }

    if callee.kind() == "member_expression" {
        let Some(property) = callee.child_by_field_name("property") else {
            return false;
        };
        let Some(object) = callee.child_by_field_name("object") else {
            return false;
        };
        let property_text = parsed.node_text(&property);
        return (framework == "express" && property_text == "Router")
            && js_ts_expr_resolves_to_framework_module(parsed, object, framework, imports);
    }

    false
}

fn js_ts_expr_resolves_to_framework_module(
    parsed: &ParsedFile,
    expr: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() == "identifier" {
        let local = parsed.node_text(&expr);
        return imports
            .get(local)
            .is_some_and(|module| js_ts_module_matches_framework(module, framework));
    }
    if parsed.language.is_call_node(expr.kind()) {
        return js_ts_require_call_module(parsed, &expr)
            .is_some_and(|module| js_ts_module_matches_framework(&module, framework));
    }
    false
}

fn js_ts_module_matches_framework(module: &str, framework: &str) -> bool {
    match framework {
        "express" => module == "express",
        "fastify" => module == "fastify",
        "koa" => matches!(module, "koa" | "@koa/router" | "koa-router"),
        _ => false,
    }
}

fn js_ts_param_has_nest_source_decorator(parsed: &ParsedFile, param: &Node<'_>) -> bool {
    js_ts_node_has_direct_decorator_named(
        parsed,
        *param,
        &["Body", "Query", "Param", "Headers", "Req", "Request"],
    )
}

fn js_ts_has_route_decorator(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    let route_names = ["Get", "Post", "Put", "Patch", "Delete", "All"];
    js_ts_node_has_direct_decorator_named(parsed, *func, &route_names)
        || js_ts_function_header_has_decorator_named(parsed, func, &route_names)
}

fn js_ts_node_has_direct_decorator_named(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &[&str],
) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" && js_ts_decorator_name_is_one_of(parsed, child, names) {
            return true;
        }
    }
    false
}

fn js_ts_decorator_name_is_one_of(
    parsed: &ParsedFile,
    decorator: Node<'_>,
    names: &[&str],
) -> bool {
    js_ts_decorator_name(parsed, decorator)
        .is_some_and(|decorator_name| names.iter().any(|name| decorator_name == *name))
}

fn js_ts_function_header_has_decorator_named(
    parsed: &ParsedFile,
    func: &Node<'_>,
    names: &[&str],
) -> bool {
    let start_line = func.start_position().row + 1;
    let params_line = find_js_ts_parameters_node(*func)
        .map(|params| params.start_position().row + 1)
        .unwrap_or(start_line);
    let window_start = start_line.saturating_sub(5).max(1);
    for line in window_start..=params_line {
        if names.iter().any(|name| {
            let decorator = format!("@{}", name);
            parsed.line_has_code_text(line, &decorator)
        }) {
            return true;
        }
    }
    false
}

fn js_ts_decorator_name(parsed: &ParsedFile, decorator: Node<'_>) -> Option<String> {
    let target = js_ts_first_decorator_target(parsed, decorator)?;
    let target = unwrap_parenthesized(target);
    let target = if parsed.language.is_call_node(target.kind()) {
        js_ts_call_or_new_callee(target)?
    } else {
        target
    };
    let target = unwrap_parenthesized(target);
    let name_node = if target.kind() == "member_expression" {
        target.child_by_field_name("property")?
    } else {
        target
    };
    if !parsed.language.is_identifier_node(name_node.kind()) {
        return None;
    }
    Some(
        parsed
            .node_text(&name_node)
            .trim_start_matches('@')
            .to_string(),
    )
}

fn js_ts_first_decorator_target<'a>(parsed: &ParsedFile, decorator: Node<'a>) -> Option<Node<'a>> {
    let mut cursor = decorator.walk();
    let target = decorator.named_children(&mut cursor).find(|child| {
        parsed.language.is_call_node(child.kind())
            || child.kind() == "member_expression"
            || parsed.language.is_identifier_node(child.kind())
    });
    target
}

fn js_ts_request_data_assignment_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> BTreeSet<JsTsRequestDataSource> {
    let mut out = BTreeSet::new();
    let func_start_line = func.start_position().row + 1;
    let mut request_alias_defs = BTreeMap::new();
    for param in source_params {
        js_ts_add_alias_def(
            &mut request_alias_defs,
            param.clone(),
            js_ts_alias_def(func_start_line, None, None, None),
        );
    }
    let mut koa_request_object_alias_defs = BTreeMap::new();
    collect_js_ts_request_assignments(
        parsed,
        *func,
        func.id(),
        framework,
        &mut request_alias_defs,
        &mut koa_request_object_alias_defs,
        &mut out,
    );
    collect_js_ts_request_alias_source_uses(
        parsed,
        *func,
        framework,
        &request_alias_defs,
        &koa_request_object_alias_defs,
        &mut out,
    );
    out
}

fn js_ts_add_alias_def(defs: &mut JsTsAliasDefs, alias: String, def: JsTsAliasDef) -> bool {
    let entries = defs.entry(alias).or_default();
    if entries
        .iter()
        .any(|existing| existing.same_binding_as(&def))
    {
        return false;
    }
    entries.push(def);
    true
}

fn js_ts_merge_alias_defs(into: &mut JsTsAliasDefs, from: &JsTsAliasDefs) {
    for (alias, from_defs) in from {
        let entries = into.entry(alias.clone()).or_default();
        for from_def in from_defs {
            if let Some(existing) = entries
                .iter_mut()
                .find(|existing| existing.same_binding_as(from_def))
            {
                for kill in &from_def.kills {
                    if !existing.kills.contains(kill) {
                        existing.kills.push(*kill);
                    }
                }
            } else {
                entries.push(from_def.clone());
            }
        }
    }
}

fn js_ts_active_alias_names_at_range(
    defs: &JsTsAliasDefs,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> BTreeSet<String> {
    defs.iter()
        .filter_map(|(alias, alias_defs)| {
            alias_defs
                .iter()
                .any(|def| def.visible_range(line, start_byte, end_byte))
                .then(|| alias.clone())
        })
        .collect()
}

fn js_ts_kill_alias_defs_at(
    defs: &mut JsTsAliasDefs,
    alias: &str,
    line: usize,
    byte: usize,
    byte_scope: Option<(usize, usize)>,
) {
    let kill = JsTsAliasKill {
        line,
        byte,
        byte_scope,
    };
    if let Some(alias_defs) = defs.get_mut(alias) {
        for def in alias_defs {
            if def.visible_range(line, byte, byte) && !def.kills.contains(&kill) {
                def.kills.push(kill);
            }
        }
    }
}

fn js_ts_kill_simple_lhs_alias_defs_at(
    parsed: &ParsedFile,
    lhs: &Node<'_>,
    binding_node: &Node<'_>,
    line: usize,
    root_func_id: usize,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
) -> BTreeSet<String> {
    let byte_scope = js_ts_assignment_kill_byte_scope(parsed, binding_node, root_func_id);
    let mut aliases = BTreeSet::new();
    collect_js_ts_lhs_alias_identifiers(parsed, lhs, &mut aliases);
    for alias in &aliases {
        if alias == "_" {
            continue;
        }
        js_ts_kill_alias_defs_at(
            request_alias_defs,
            alias,
            line,
            lhs.start_byte(),
            byte_scope,
        );
        js_ts_kill_alias_defs_at(
            koa_request_object_alias_defs,
            alias,
            line,
            lhs.start_byte(),
            byte_scope,
        );
    }
    aliases
}

fn js_ts_assignment_kill_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        return js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id);
    }
    js_ts_conditional_execution_byte_scope(parsed, binding_node, root_func_id)
        .or_else(|| js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id))
}

fn js_ts_conditional_execution_byte_scope(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let mut child = *node;
    let mut current = child.parent();
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return None;
        }
        if matches!(
            parent.kind(),
            "catch_clause" | "else_clause" | "finally_clause" | "switch_case" | "switch_default"
        ) {
            return Some((parent.start_byte(), parent.end_byte()));
        }
        if js_ts_is_conditional_execution_boundary(parsed, &parent) {
            return Some((child.start_byte(), child.end_byte()));
        }
        child = parent;
        current = parent.parent();
    }
    None
}

fn js_ts_is_conditional_execution_boundary(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let kind = node.kind();
    matches!(
        kind,
        "conditional_expression"
            | "do_statement"
            | "for_await_statement"
            | "for_in_statement"
            | "for_of_statement"
            | "for_statement"
            | "if_statement"
            | "switch_statement"
            | "ternary_expression"
            | "try_statement"
            | "while_statement"
    ) || (kind == "binary_expression" && {
        let text = parsed.node_text(node);
        text.contains("&&") || text.contains("||")
    })
}

fn collect_js_ts_lhs_alias_identifiers(
    parsed: &ParsedFile,
    node: &Node<'_>,
    out: &mut BTreeSet<String>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            out.insert(parsed.node_text(node).to_string());
        }
        "assignment_pattern"
        | "object_assignment_pattern"
        | "parenthesized_expression"
        | "rest_pattern" => {
            if let Some(child) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_lhs_alias_identifiers(parsed, &value, out);
            } else {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "shorthand_property_identifier_pattern" {
                        collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
                    }
                }
            }
        }
        "member_expression" | "subscript_expression" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
            }
        }
    }
}

fn js_ts_shrink_request_sources_after_unconditional_kill(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
    aliases: &BTreeSet<String>,
    line: usize,
    kill_byte: usize,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    if aliases.is_empty()
        || js_ts_conditional_execution_byte_scope(parsed, binding_node, root_func_id).is_some()
    {
        return;
    }

    let mut removals = Vec::new();
    let mut replacements = Vec::new();
    for source in out.iter() {
        if !source.target.fields.is_empty()
            || !aliases.contains(&source.target.base)
            || source.line > line
            || !js_ts_line_in_scope(source.scope, line)
        {
            continue;
        }

        let source_start = source
            .start_byte
            .or_else(|| source.byte_scope.map(|(start, _)| start))
            .unwrap_or(0);
        if source.line == line && source_start >= kill_byte {
            continue;
        }

        removals.push(source.clone());
        if source.line == line {
            if let Some(byte_scope) =
                js_ts_intersect_byte_scopes(source.byte_scope, Some((source_start, kill_byte)))
            {
                let mut replacement = source.clone();
                replacement.byte_scope = Some(byte_scope);
                replacements.push(replacement);
            }
            continue;
        }

        let scope_start = source.scope.map(|(start, _)| start).unwrap_or(source.line);
        if let Some(scope_end) = line.checked_sub(1) {
            if scope_start <= scope_end {
                let mut replacement = source.clone();
                replacement.scope = Some((scope_start, scope_end));
                replacements.push(replacement);
            }
        }
    }

    for source in removals {
        out.remove(&source);
    }
    for source in replacements {
        out.insert(source);
    }
}

fn js_ts_seed_scope_contains(seed: &TaintSeed, line: usize) -> bool {
    seed.scope
        .map(|(start, end)| start <= line && line <= end)
        .unwrap_or(true)
}

fn js_ts_seed_scope_contains_range(
    parsed: &ParsedFile,
    seed: &TaintSeed,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    js_ts_seed_scope_contains(seed, line)
        && (seed.line < line
            || seed
                .start_byte
                .map(|seed_start| seed_start <= start_byte)
                .unwrap_or(true))
        && seed
            .byte_scope
            .map(|(start, end)| start <= start_byte && end_byte <= end)
            .unwrap_or(true)
        && seed.start_byte.is_none_or(|seed_start| {
            !js_ts_ranges_are_in_sibling_control_flow_branches(
                parsed.tree.root_node(),
                seed_start,
                seed_start,
                start_byte,
                end_byte,
            )
        })
}

fn js_ts_line_in_scope(scope: Option<(usize, usize)>, line: usize) -> bool {
    scope
        .map(|(start, end)| start <= line && line <= end)
        .unwrap_or(true)
}

fn js_ts_request_source(
    line: usize,
    target: AccessPath,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
) -> JsTsRequestDataSource {
    JsTsRequestDataSource {
        line,
        target,
        start_byte,
        scope,
        byte_scope,
    }
}

fn collect_js_ts_request_assignments(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return;
    }

    if node.kind() == "variable_declarator" {
        if let (Some(lhs), Some(rhs)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) {
            collect_js_ts_request_assignment_targets(
                parsed,
                lhs,
                rhs,
                node,
                root_func_id,
                framework,
                request_alias_defs,
                koa_request_object_alias_defs,
                out,
            );
        }
        return;
    }

    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_js_ts_request_assignment_targets(
                parsed,
                lhs,
                rhs,
                node,
                root_func_id,
                framework,
                request_alias_defs,
                koa_request_object_alias_defs,
                out,
            );
        }
        return;
    }

    if matches!(node.kind(), "if_statement" | "if_expression") {
        collect_js_ts_request_assignments_in_if(
            parsed,
            node,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_request_assignments(
            parsed,
            child,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
    }
}

fn collect_js_ts_request_assignments_in_if(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    let consequence = node.child_by_field_name("consequence");
    let alternative = node.child_by_field_name("alternative");
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if consequence.is_some_and(|branch| branch.id() == child.id())
            || alternative.is_some_and(|branch| branch.id() == child.id())
        {
            continue;
        }
        collect_js_ts_request_assignments(
            parsed,
            child,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
    }

    let base_request_alias_defs = request_alias_defs.clone();
    let base_koa_request_object_alias_defs = koa_request_object_alias_defs.clone();
    let mut branch_request_alias_defs = Vec::new();
    let mut branch_koa_request_object_alias_defs = Vec::new();
    for branch in [consequence, alternative].into_iter().flatten() {
        let mut request_defs = base_request_alias_defs.clone();
        let mut koa_defs = base_koa_request_object_alias_defs.clone();
        let branch_exits = js_ts_node_definitely_exits(branch);
        collect_js_ts_request_assignments(
            parsed,
            branch,
            root_func_id,
            framework,
            &mut request_defs,
            &mut koa_defs,
            out,
        );
        collect_js_ts_request_alias_source_uses(
            parsed,
            branch,
            framework,
            &request_defs,
            &koa_defs,
            out,
        );
        if branch_exits {
            branch_request_alias_defs.push(js_ts_branch_scoped_alias_delta(
                &request_defs,
                &base_request_alias_defs,
                branch,
            ));
            branch_koa_request_object_alias_defs.push(js_ts_branch_scoped_alias_delta(
                &koa_defs,
                &base_koa_request_object_alias_defs,
                branch,
            ));
        } else {
            branch_request_alias_defs.push(request_defs);
            branch_koa_request_object_alias_defs.push(koa_defs);
        }
    }

    for branch_defs in &branch_request_alias_defs {
        js_ts_merge_alias_defs(request_alias_defs, branch_defs);
    }
    for branch_defs in &branch_koa_request_object_alias_defs {
        js_ts_merge_alias_defs(koa_request_object_alias_defs, branch_defs);
    }
}

fn js_ts_branch_scoped_alias_delta(
    branch_defs: &JsTsAliasDefs,
    base_defs: &JsTsAliasDefs,
    branch: Node<'_>,
) -> JsTsAliasDefs {
    let branch_line_scope = Some((
        branch.start_position().row + 1,
        branch.end_position().row + 1,
    ));
    let branch_byte_scope = Some((branch.start_byte(), branch.end_byte()));
    let mut out = BTreeMap::new();
    for (alias, defs) in branch_defs {
        for def in defs {
            if base_defs
                .get(alias)
                .is_some_and(|base| base.iter().any(|base_def| base_def.same_binding_as(def)))
            {
                continue;
            }
            let mut scoped = def.clone();
            scoped.scope = js_ts_intersect_line_scopes(scoped.scope, branch_line_scope);
            scoped.byte_scope = js_ts_intersect_byte_scopes(scoped.byte_scope, branch_byte_scope);
            js_ts_add_alias_def(&mut out, alias.clone(), scoped);
        }
    }
    out
}

fn js_ts_intersect_line_scopes(
    left: Option<(usize, usize)>,
    right: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    match (left, right) {
        (Some((left_start, left_end)), Some((right_start, right_end))) => {
            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            (start <= end).then_some((start, end))
        }
        (Some(scope), None) | (None, Some(scope)) => Some(scope),
        (None, None) => None,
    }
}

fn js_ts_intersect_byte_scopes(
    left: Option<(usize, usize)>,
    right: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    match (left, right) {
        (Some((left_start, left_end)), Some((right_start, right_end))) => {
            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            (start <= end).then_some((start, end))
        }
        (Some(scope), None) | (None, Some(scope)) => Some(scope),
        (None, None) => None,
    }
}

fn js_ts_node_definitely_exits(node: Node<'_>) -> bool {
    match node.kind() {
        "return_statement" | "throw_statement" => true,
        "else_clause" | "statement_block" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(js_ts_node_definitely_exits),
        "if_statement" | "if_expression" => {
            let Some(consequence) = node.child_by_field_name("consequence") else {
                return false;
            };
            let Some(alternative) = node.child_by_field_name("alternative") else {
                return false;
            };
            js_ts_node_definitely_exits(consequence) && js_ts_node_definitely_exits(alternative)
        }
        _ => false,
    }
}

fn collect_js_ts_request_assignment_targets(
    parsed: &ParsedFile,
    lhs: Node<'_>,
    rhs: Node<'_>,
    binding_node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    let target_line = lhs.start_position().row + 1;
    let binding_scope = js_ts_assignment_effective_line_scope(parsed, &binding_node, root_func_id);
    let binding_byte_scope =
        js_ts_assignment_effective_byte_scope(parsed, &binding_node, root_func_id);
    let alias_def = js_ts_alias_def(
        target_line,
        Some(lhs.start_byte()),
        binding_scope,
        binding_byte_scope,
    );
    let request_aliases = js_ts_active_alias_names_at_range(
        request_alias_defs,
        target_line,
        rhs.start_byte(),
        rhs.end_byte(),
    );
    let koa_request_object_aliases = js_ts_active_alias_names_at_range(
        koa_request_object_alias_defs,
        target_line,
        rhs.start_byte(),
        rhs.end_byte(),
    );
    if js_ts_rhs_is_bare_alias(parsed, &rhs, &request_aliases) {
        let collected_request_data = collect_js_ts_request_destructure_targets(
            parsed,
            lhs,
            framework,
            target_line,
            binding_scope,
            binding_byte_scope,
            out,
        );
        let collected_koa_request_alias = framework == "koa"
            && collect_js_ts_koa_request_object_alias_destructure_targets(
                parsed,
                lhs,
                target_line,
                binding_scope,
                binding_byte_scope,
                koa_request_object_alias_defs,
                out,
                true,
            );
        if collected_request_data || collected_koa_request_alias {
            return;
        }
        if let Some(alias) = js_ts_simple_lhs_identifier(parsed, &lhs) {
            if alias != "_" {
                js_ts_add_alias_def(request_alias_defs, alias, alias_def);
            }
            return;
        }
    }
    if framework == "koa"
        && (js_ts_rhs_is_bare_alias(parsed, &rhs, &koa_request_object_aliases)
            || js_ts_rhs_is_koa_request_object_alias(parsed, &rhs, &request_aliases))
    {
        if collect_js_ts_request_object_destructure_targets(
            parsed,
            lhs,
            target_line,
            binding_scope,
            binding_byte_scope,
            out,
        ) {
            return;
        }
        if let Some(alias) = js_ts_simple_lhs_identifier(parsed, &lhs) {
            if alias != "_" {
                js_ts_add_alias_def(koa_request_object_alias_defs, alias, alias_def);
            }
        }
        return;
    }
    if !node_contains_js_ts_source_access_with_request_object_aliases(
        parsed,
        rhs,
        framework,
        &request_aliases,
        &koa_request_object_aliases,
    ) {
        let killed_aliases = js_ts_kill_simple_lhs_alias_defs_at(
            parsed,
            &lhs,
            &binding_node,
            target_line,
            root_func_id,
            request_alias_defs,
            koa_request_object_alias_defs,
        );
        js_ts_shrink_request_sources_after_unconditional_kill(
            parsed,
            &binding_node,
            root_func_id,
            &killed_aliases,
            target_line,
            lhs.start_byte(),
            out,
        );
        return;
    }
    collect_js_ts_lhs_targets(
        parsed,
        lhs,
        target_line,
        binding_scope,
        binding_byte_scope,
        out,
    );
}

fn collect_js_ts_lhs_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                out.insert(js_ts_request_source(
                    line,
                    AccessPath::simple(name),
                    Some(node.start_byte()),
                    scope,
                    byte_scope,
                ));
            }
        }
        "member_expression" => {
            out.insert(js_ts_request_source(
                line,
                AccessPath::from_expr(parsed.node_text(&node)),
                Some(node.start_byte()),
                None,
                None,
            ));
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_targets(parsed, left, line, scope, byte_scope, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_targets(parsed, child, line, scope, byte_scope, out);
            }
        }
    }
}

fn collect_js_ts_request_alias_source_uses(
    parsed: &ParsedFile,
    func: Node<'_>,
    framework: &str,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    for (alias, alias_defs) in request_alias_defs {
        collect_js_ts_request_alias_source_uses_for_alias(
            parsed,
            func,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            alias,
            alias_defs,
            false,
            out,
        );
    }
    for (alias, alias_defs) in koa_request_object_alias_defs {
        collect_js_ts_request_alias_source_uses_for_alias(
            parsed,
            func,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            alias,
            alias_defs,
            true,
            out,
        );
    }
}

fn collect_js_ts_request_alias_source_uses_for_alias(
    parsed: &ParsedFile,
    func: Node<'_>,
    framework: &str,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
    alias: &str,
    alias_defs: &[JsTsAliasDef],
    is_koa_request_object_alias: bool,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    for def in alias_defs {
        let refs = parsed.find_variable_references_scoped(&func, alias, def.line);
        for ref_line in refs {
            if ref_line < def.line || !def.visible_on(ref_line) {
                continue;
            }
            let source_ranges = js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                ref_line,
                framework,
                alias,
                is_koa_request_object_alias,
                request_alias_defs,
                koa_request_object_alias_defs,
            );
            if source_ranges.is_empty() {
                continue;
            }
            out.insert(js_ts_request_source(
                ref_line,
                AccessPath::simple(alias.to_string()),
                None,
                def.scope,
                def.byte_scope,
            ));
        }
    }
}

fn collect_js_ts_request_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    framework: &str,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                let field = parsed.node_text(&child);
                if js_ts_request_field_allowed(framework, field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(child.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                let Some(left) = child
                    .child_by_field_name("left")
                    .or_else(|| child.named_child(0))
                else {
                    continue;
                };
                if !matches!(
                    left.kind(),
                    "identifier" | "shorthand_property_identifier_pattern"
                ) {
                    continue;
                }
                let field = parsed.node_text(&left);
                if js_ts_request_field_allowed(framework, field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(left.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if !js_ts_request_field_allowed(framework, &field) {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
                }
            }
            "object_pattern" => {
                collect_js_ts_request_destructure_targets(
                    parsed, child, framework, line, scope, byte_scope, out,
                );
            }
            _ => {}
        }
    }
    out.len() > before
}

fn collect_js_ts_koa_request_object_alias_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
    allow_alias_defs: bool,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = koa_request_object_alias_defs.len();
    let source_before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                if allow_alias_defs && parsed.node_text(&child) == "request" {
                    js_ts_add_alias_def(
                        koa_request_object_alias_defs,
                        "request".to_string(),
                        js_ts_alias_def(line, Some(child.start_byte()), scope, byte_scope),
                    );
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if field != "request" {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    if value.kind() == "object_pattern" {
                        collect_js_ts_request_object_destructure_targets(
                            parsed, value, line, scope, byte_scope, out,
                        );
                    } else {
                        if allow_alias_defs {
                            collect_js_ts_lhs_simple_alias_defs(
                                parsed,
                                value,
                                line,
                                scope,
                                byte_scope,
                                koa_request_object_alias_defs,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
    koa_request_object_alias_defs.len() > before || out.len() > source_before
}

fn collect_js_ts_lhs_simple_alias_defs(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    defs: &mut JsTsAliasDefs,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                js_ts_add_alias_def(
                    defs,
                    name.to_string(),
                    js_ts_alias_def(line, Some(node.start_byte()), scope, byte_scope),
                );
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_simple_alias_defs(parsed, left, line, scope, byte_scope, defs);
            }
        }
        _ => {}
    }
}

fn collect_js_ts_request_object_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                let field = parsed.node_text(&child);
                if JS_TS_REQUEST_DATA_FIELDS.contains(&field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(child.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                let Some(left) = child
                    .child_by_field_name("left")
                    .or_else(|| child.named_child(0))
                else {
                    continue;
                };
                if !matches!(
                    left.kind(),
                    "identifier" | "shorthand_property_identifier_pattern"
                ) {
                    continue;
                }
                let field = parsed.node_text(&left);
                if JS_TS_REQUEST_DATA_FIELDS.contains(&field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(left.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if !JS_TS_REQUEST_DATA_FIELDS.contains(&field.as_str()) {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
                }
            }
            "object_pattern" => {
                collect_js_ts_request_object_destructure_targets(
                    parsed, child, line, scope, byte_scope, out,
                );
            }
            _ => {}
        }
    }
    out.len() > before
}

fn node_contains_js_ts_source_access(
    parsed: &ParsedFile,
    node: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> bool {
    let text = parsed.node_text(&node);
    if source_params
        .iter()
        .any(|param| js_ts_source_access_text_matches(text, framework, param))
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_source_access(parsed, child, framework, source_params) {
            return true;
        }
    }
    false
}

fn node_contains_js_ts_source_access_with_request_object_aliases(
    parsed: &ParsedFile,
    node: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
    request_object_aliases: &BTreeSet<String>,
) -> bool {
    node_contains_js_ts_source_access(parsed, node, framework, source_params)
        || (framework == "koa"
            && node_contains_js_ts_request_object_source_access(
                parsed,
                node,
                request_object_aliases,
            ))
}

fn node_contains_js_ts_source_access_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> bool {
    if !node_contains_line(&node, line) {
        return false;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let start_line = node.start_position().row + 1;
    if start_line == line {
        let text = parsed.node_text(&node);
        if source_params
            .iter()
            .any(|param| js_ts_source_access_text_matches(text, framework, param))
        {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_source_access_on_line(
            parsed,
            child,
            root_func_id,
            line,
            framework,
            source_params,
        ) {
            return true;
        }
    }
    false
}

fn node_contains_js_ts_source_access_on_line_with_request_object_aliases(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    source_params: &BTreeSet<String>,
    request_object_aliases: &BTreeSet<String>,
) -> bool {
    node_contains_js_ts_source_access_on_line(
        parsed,
        node,
        root_func_id,
        line,
        framework,
        source_params,
    ) || (framework == "koa"
        && node_contains_js_ts_request_object_source_access_on_line(
            parsed,
            node,
            root_func_id,
            line,
            request_object_aliases,
        ))
}

fn node_contains_js_ts_request_object_source_access(
    parsed: &ParsedFile,
    node: Node<'_>,
    aliases: &BTreeSet<String>,
) -> bool {
    let text = parsed.node_text(&node);
    if aliases.iter().any(|alias| {
        JS_TS_REQUEST_DATA_FIELDS
            .iter()
            .any(|field| js_ts_field_access_text_matches(text, alias, field))
    }) {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_request_object_source_access(parsed, child, aliases) {
            return true;
        }
    }
    false
}

fn node_contains_js_ts_request_object_source_access_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    aliases: &BTreeSet<String>,
) -> bool {
    if !node_contains_line(&node, line) {
        return false;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let start_line = node.start_position().row + 1;
    if start_line == line {
        let text = parsed.node_text(&node);
        if aliases.iter().any(|alias| {
            JS_TS_REQUEST_DATA_FIELDS
                .iter()
                .any(|field| js_ts_field_access_text_matches(text, alias, field))
        }) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_js_ts_request_object_source_access_on_line(
            parsed,
            child,
            root_func_id,
            line,
            aliases,
        ) {
            return true;
        }
    }
    false
}

const JS_TS_REQUEST_DATA_FIELDS: &[&str] = &[
    "body",
    "query",
    "params",
    "headers",
    "cookies",
    "files",
    "url",
    "path",
    "originalUrl",
    "host",
    "hostname",
];

const JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS: &[&str] = &[
    "query",
    "params",
    "headers",
    "cookies",
    "files",
    "url",
    "path",
    "originalUrl",
    "host",
    "hostname",
];

fn js_ts_request_field_allowed(framework: &str, field: &str) -> bool {
    match framework {
        "fastify" | "express" => JS_TS_REQUEST_DATA_FIELDS.contains(&field),
        "koa" => JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS.contains(&field),
        _ => false,
    }
}

fn js_ts_request_source_identifier(text: &str) -> bool {
    JS_TS_REQUEST_DATA_FIELDS.contains(&text) || text == "request"
}

fn js_ts_koa_request_base_texts(param: &str) -> [String; 6] {
    [
        format!("{}.request", param),
        format!("{}?.request", param),
        format!("{}[\"request\"]", param),
        format!("{}['request']", param),
        format!("{}?.[\"request\"]", param),
        format!("{}?.['request']", param),
    ]
}

fn js_ts_koa_request_field_access_text_matches(text: &str, param: &str, field: &str) -> bool {
    js_ts_koa_request_base_texts(param)
        .iter()
        .any(|base| js_ts_field_access_text_matches(text, base, field))
}

fn js_ts_normalized_property_key(parsed: &ParsedFile, key: &Node<'_>) -> Option<String> {
    let text = parsed.node_text(key).trim();
    let text = if let Some(inner) = text.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let inner = inner.trim();
        inner
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .or_else(|| inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?
    } else {
        text
    };
    let text = text
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(text);
    (!text.is_empty()).then(|| text.to_string())
}

fn js_ts_exact_field_access_text_matches(text: &str, base: &str, field: &str) -> bool {
    let text = text.trim();
    if js_ts_exact_field_access_compact_text_matches(text, base, field) {
        return true;
    }
    if !text.chars().any(char::is_whitespace) {
        return false;
    }
    let compact = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    js_ts_exact_field_access_compact_text_matches(&compact, base, field)
}

fn js_ts_exact_field_access_compact_text_matches(text: &str, base: &str, field: &str) -> bool {
    [
        format!("{}.{}", base, field),
        format!("{}[\"{}\"]", base, field),
        format!("{}['{}']", base, field),
        format!("{}?.{}", base, field),
        format!("{}?.[\"{}\"]", base, field),
        format!("{}?.['{}']", base, field),
    ]
    .iter()
    .any(|candidate| text == candidate)
}

fn js_ts_field_access_text_matches(text: &str, base: &str, field: &str) -> bool {
    let text = text.trim();
    if js_ts_field_access_compact_text_matches(text, base, field) {
        return true;
    }
    if !text.chars().any(char::is_whitespace) {
        return false;
    }
    let compact = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    js_ts_field_access_compact_text_matches(&compact, base, field)
}

fn js_ts_field_access_compact_text_matches(text: &str, base: &str, field: &str) -> bool {
    let prefixes = [
        format!("{}.{}", base, field),
        format!("{}[\"{}\"]", base, field),
        format!("{}['{}']", base, field),
        format!("{}?.{}", base, field),
        format!("{}?.[\"{}\"]", base, field),
        format!("{}?.['{}']", base, field),
    ];
    prefixes.iter().any(|prefix| {
        text == prefix
            || text.starts_with(&format!("{}.", prefix))
            || text.starts_with(&format!("{}[", prefix))
            || text.starts_with(&format!("{}?.", prefix))
            || text.starts_with(&format!("{}?.[", prefix))
    })
}

fn js_ts_source_access_text_matches(text: &str, framework: &str, param: &str) -> bool {
    let text = text.trim();
    match framework {
        "nestjs" => {
            text == param
                || text.starts_with(&format!("{}.", param))
                || text.starts_with(&format!("{}[", param))
        }
        "fastify" | "express" => JS_TS_REQUEST_DATA_FIELDS
            .iter()
            .any(|field| js_ts_field_access_text_matches(text, param, field)),
        "koa" => {
            JS_TS_KOA_DIRECT_REQUEST_DATA_FIELDS
                .iter()
                .any(|field| js_ts_field_access_text_matches(text, param, field))
                || JS_TS_REQUEST_DATA_FIELDS
                    .iter()
                    .any(|field| js_ts_koa_request_field_access_text_matches(text, param, field))
        }
        _ => false,
    }
}

fn js_ts_framework_source_access_ranges_by_line(
    files: &BTreeMap<String, ParsedFile>,
    source_lines: &BTreeSet<(String, usize)>,
) -> BTreeMap<(String, usize), Vec<(usize, usize)>> {
    let mut out = BTreeMap::new();
    for (file, line) in source_lines {
        let Some(parsed) = files.get(file) else {
            continue;
        };
        if !is_js_ts_language(parsed.language) {
            continue;
        }
        let ranges = js_ts_request_source_access_ranges_for_line(parsed, *line);
        if !ranges.is_empty() {
            out.insert((file.clone(), *line), ranges);
        }
    }
    out
}

fn js_ts_request_source_access_ranges_for_line(
    parsed: &ParsedFile,
    line: usize,
) -> Vec<(usize, usize)> {
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return Vec::new();
    };
    if !matches!(framework, "fastify" | "express" | "koa") {
        return Vec::new();
    }

    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    let mut ranges = Vec::new();
    for func in parsed.all_functions() {
        if !node_contains_line(&func, line) {
            continue;
        }
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }
        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }

        let func_start_line = func.start_position().row + 1;
        let mut request_alias_defs = BTreeMap::new();
        for param in &source_params {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                param.clone(),
                js_ts_alias_def(func_start_line, None, None, None),
            );
        }
        let mut koa_request_object_alias_defs = BTreeMap::new();
        let mut assignment_sources = BTreeSet::new();
        collect_js_ts_request_assignments(
            parsed,
            func,
            func.id(),
            framework,
            &mut request_alias_defs,
            &mut koa_request_object_alias_defs,
            &mut assignment_sources,
        );
        for alias in request_alias_defs.keys() {
            ranges.extend(js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                line,
                framework,
                alias,
                false,
                &request_alias_defs,
                &koa_request_object_alias_defs,
            ));
        }
        for alias in koa_request_object_alias_defs.keys() {
            ranges.extend(js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                line,
                framework,
                alias,
                true,
                &request_alias_defs,
                &koa_request_object_alias_defs,
            ));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn js_ts_request_source_access_ranges_for_alias_on_line(
    parsed: &ParsedFile,
    line: usize,
    alias: &str,
) -> Vec<(usize, usize)> {
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return Vec::new();
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    for func in parsed.all_functions() {
        if !node_contains_line(&func, line) {
            continue;
        }
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }
        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }
        let func_start_line = func.start_position().row + 1;
        let mut request_alias_defs = BTreeMap::new();
        for param in &source_params {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                param.clone(),
                js_ts_alias_def(func_start_line, None, None, None),
            );
        }
        let mut koa_request_object_alias_defs = BTreeMap::new();
        let mut assignment_sources = BTreeSet::new();
        collect_js_ts_request_assignments(
            parsed,
            func,
            func.id(),
            framework,
            &mut request_alias_defs,
            &mut koa_request_object_alias_defs,
            &mut assignment_sources,
        );
        let is_koa_request_object_alias = framework == "koa"
            && koa_request_object_alias_defs
                .get(alias)
                .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)));
        let alias_ranges = js_ts_source_access_ranges_for_alias_from_defs(
            parsed,
            func,
            line,
            framework,
            alias,
            is_koa_request_object_alias,
            &request_alias_defs,
            &koa_request_object_alias_defs,
        );
        ranges.extend(alias_ranges);
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn js_ts_alias_def_can_reach_range(
    parsed: &ParsedFile,
    def: &JsTsAliasDef,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    def.start_byte.is_none_or(|def_start| {
        !js_ts_ranges_are_in_sibling_control_flow_branches(
            parsed.tree.root_node(),
            def_start,
            def_start,
            start_byte,
            end_byte,
        )
    })
}

fn js_ts_ranges_are_in_sibling_control_flow_branches(
    node: Node<'_>,
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    if !node_contains_range(&node, left_start, left_end)
        || !node_contains_range(&node, right_start, right_end)
    {
        return false;
    }
    if matches!(
        node.kind(),
        "if_statement" | "if_expression" | "conditional_expression" | "ternary_expression"
    ) {
        let left_branch = js_ts_conditional_branch_containing_range(&node, left_start, left_end);
        let right_branch = js_ts_conditional_branch_containing_range(&node, right_start, right_end);
        if left_branch.is_some() && right_branch.is_some() && left_branch != right_branch {
            return true;
        }
    }
    if node.kind() == "switch_statement" {
        let left_case = js_ts_switch_case_containing_range(node, left_start, left_end);
        let right_case = js_ts_switch_case_containing_range(node, right_start, right_end);
        if let (Some(left_case), Some(right_case)) = (left_case, right_case) {
            if left_case.id() != right_case.id()
                && !js_ts_switch_cases_can_fall_through(node, left_case, right_case)
            {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if js_ts_ranges_are_in_sibling_control_flow_branches(
            child,
            left_start,
            left_end,
            right_start,
            right_end,
        ) {
            return true;
        }
    }
    false
}

fn js_ts_conditional_branch_containing_range(
    node: &Node<'_>,
    start_byte: usize,
    end_byte: usize,
) -> Option<&'static str> {
    let consequence = node.child_by_field_name("consequence");
    if consequence.is_some_and(|branch| node_contains_range(&branch, start_byte, end_byte)) {
        return Some("consequence");
    }
    let alternative = node.child_by_field_name("alternative");
    if alternative.is_some_and(|branch| node_contains_range(&branch, start_byte, end_byte)) {
        return Some("alternative");
    }
    None
}

fn js_ts_switch_case_containing_range<'a>(
    node: Node<'a>,
    start_byte: usize,
    end_byte: usize,
) -> Option<Node<'a>> {
    if matches!(node.kind(), "switch_case" | "switch_default")
        && node_contains_range(&node, start_byte, end_byte)
    {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !node_contains_range(&child, start_byte, end_byte) {
            continue;
        }
        if let Some(case_node) = js_ts_switch_case_containing_range(child, start_byte, end_byte) {
            return Some(case_node);
        }
    }
    None
}

fn js_ts_switch_cases_can_fall_through(
    switch_node: Node<'_>,
    left_case: Node<'_>,
    right_case: Node<'_>,
) -> bool {
    if left_case.start_byte() >= right_case.start_byte() {
        return false;
    }

    let mut cases = Vec::new();
    collect_js_ts_switch_cases(switch_node, &mut cases);
    let Some(left_index) = cases
        .iter()
        .position(|case_node| case_node.id() == left_case.id())
    else {
        return false;
    };
    let Some(right_index) = cases
        .iter()
        .position(|case_node| case_node.id() == right_case.id())
    else {
        return false;
    };
    if left_index >= right_index {
        return false;
    }

    cases[left_index..right_index]
        .iter()
        .all(|case_node| js_ts_switch_case_can_fall_through(*case_node))
}

fn collect_js_ts_switch_cases<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if matches!(node.kind(), "switch_case" | "switch_default") {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_switch_cases(child, out);
    }
}

fn js_ts_switch_case_can_fall_through(case_node: Node<'_>) -> bool {
    !case_node
        .named_child(case_node.named_child_count().saturating_sub(1))
        .is_some_and(js_ts_node_stops_switch_fallthrough)
}

fn js_ts_node_stops_switch_fallthrough(node: Node<'_>) -> bool {
    match node.kind() {
        "break_statement" | "continue_statement" | "return_statement" | "throw_statement" => true,
        "statement_block" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(js_ts_node_stops_switch_fallthrough),
        "if_statement" | "if_expression" => {
            let Some(consequence) = node.child_by_field_name("consequence") else {
                return false;
            };
            let Some(alternative) = node.child_by_field_name("alternative") else {
                return false;
            };
            js_ts_node_stops_switch_fallthrough(consequence)
                && js_ts_node_stops_switch_fallthrough(alternative)
        }
        _ => false,
    }
}

fn js_ts_source_access_ranges_for_alias_from_defs(
    parsed: &ParsedFile,
    func: Node<'_>,
    line: usize,
    framework: &str,
    alias: &str,
    is_koa_request_object_alias: bool,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
) -> Vec<(usize, usize)> {
    let mut request_aliases = BTreeSet::new();
    if request_alias_defs
        .get(alias)
        .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)))
    {
        request_aliases.insert(alias.to_string());
    }
    let mut koa_request_object_aliases = BTreeSet::new();
    if is_koa_request_object_alias
        && koa_request_object_alias_defs
            .get(alias)
            .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)))
    {
        koa_request_object_aliases.insert(alias.to_string());
    }
    if request_aliases.is_empty() && koa_request_object_aliases.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    collect_js_ts_source_access_ranges_on_line(
        parsed,
        func,
        func.id(),
        line,
        framework,
        &request_aliases,
        &koa_request_object_aliases,
        &mut ranges,
    );
    ranges.retain(|(start, end)| {
        request_alias_defs.get(alias).is_some_and(|defs| {
            defs.iter().any(|def| {
                def.visible_range(line, *start, *end)
                    && js_ts_alias_def_can_reach_range(parsed, def, *start, *end)
            })
        }) || (is_koa_request_object_alias
            && koa_request_object_alias_defs
                .get(alias)
                .is_some_and(|defs| {
                    defs.iter().any(|def| {
                        def.visible_range(line, *start, *end)
                            && js_ts_alias_def_can_reach_range(parsed, def, *start, *end)
                    })
                }))
    });
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn collect_js_ts_source_access_ranges_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    request_aliases: &BTreeSet<String>,
    koa_request_object_aliases: &BTreeSet<String>,
    out: &mut Vec<(usize, usize)>,
) {
    if !node_contains_line(&node, line) {
        return;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return;
    }

    let start_line = node.start_position().row + 1;
    let is_source_access = if start_line == line {
        let text = parsed.node_text(&node);
        request_aliases
            .iter()
            .any(|alias| js_ts_source_access_text_matches(text, framework, alias))
            || (framework == "koa"
                && koa_request_object_aliases.iter().any(|alias| {
                    JS_TS_REQUEST_DATA_FIELDS
                        .iter()
                        .any(|field| js_ts_field_access_text_matches(text, alias, field))
                }))
    } else {
        false
    };
    if is_source_access && js_ts_source_access_range_node_kind(node.kind()) {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }

    let before_children = out.len();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_source_access_ranges_on_line(
            parsed,
            child,
            root_func_id,
            line,
            framework,
            request_aliases,
            koa_request_object_aliases,
            out,
        );
    }
    if out.len() > before_children {
        return;
    }

    if start_line == line {
        if is_source_access {
            out.push((node.start_byte(), node.end_byte()));
        }
    }
}

fn js_ts_source_access_range_node_kind(kind: &str) -> bool {
    matches!(kind, "member_expression" | "subscript_expression")
}

fn js_ts_framework_source_target_ranges_by_line(
    files: &BTreeMap<String, ParsedFile>,
    framework_sources: &[TaintSeed],
) -> BTreeMap<(String, usize), Vec<(usize, usize)>> {
    let mut targets_by_line = BTreeMap::<(String, usize), BTreeSet<String>>::new();
    for source in framework_sources {
        let Some(target) = source.target.as_ref() else {
            continue;
        };
        let Some(parsed) = files.get(&source.file) else {
            continue;
        };
        if !is_js_ts_language(parsed.language) {
            continue;
        }
        targets_by_line
            .entry((source.file.clone(), source.line))
            .or_default()
            .insert(target.base.clone());
    }

    let mut out = BTreeMap::new();
    for ((file, line), targets) in targets_by_line {
        let Some(parsed) = files.get(&file) else {
            continue;
        };
        let ranges = js_ts_lhs_binding_ranges_for_line(parsed, line, &targets);
        if !ranges.is_empty() {
            out.insert((file, line), ranges);
        }
    }
    out
}

fn js_ts_lhs_binding_ranges_for_line(
    parsed: &ParsedFile,
    line: usize,
    targets: &BTreeSet<String>,
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    collect_js_ts_lhs_binding_ranges_on_line(
        parsed,
        parsed.tree.root_node(),
        line,
        targets,
        &mut ranges,
    );
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn collect_js_ts_lhs_binding_ranges_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    targets: &BTreeSet<String>,
    ranges: &mut Vec<(usize, usize)>,
) {
    if !node_contains_line(&node, line) {
        return;
    }
    if node.kind() == "variable_declarator" {
        if let Some(lhs) = node.child_by_field_name("name") {
            collect_js_ts_lhs_identifier_ranges(parsed, lhs, targets, ranges);
        }
        return;
    }
    if parsed.language.is_assignment_node(node.kind()) {
        if let Some(lhs) = parsed.language.assignment_target(&node) {
            collect_js_ts_lhs_identifier_ranges(parsed, lhs, targets, ranges);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_lhs_binding_ranges_on_line(parsed, child, line, targets, ranges);
    }
}

fn collect_js_ts_lhs_identifier_ranges(
    parsed: &ParsedFile,
    node: Node<'_>,
    targets: &BTreeSet<String>,
    ranges: &mut Vec<(usize, usize)>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if targets.contains(parsed.node_text(&node)) {
                ranges.push((node.start_byte(), node.end_byte()));
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_identifier_ranges(parsed, left, targets, ranges);
            }
        }
        "member_expression" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_identifier_ranges(parsed, child, targets, ranges);
            }
        }
    }
}

fn detect_python_framework_sources(
    file_path: &str,
    parsed: &ParsedFile,
    sources: &mut Vec<TaintSeed>,
) {
    let pydantic_models = collect_python_pydantic_models(parsed);
    let flask_receivers = crate::frameworks::python::flask::route_receivers(parsed);
    // Compute FastAPI route receivers once per file rather than per function;
    // the AST walk is O(tree_size) and `function_has_route_decorator_with_receivers`
    // re-uses the result for each handler check.
    let fastapi_receivers = crate::frameworks::python::fastapi::route_receivers(parsed);
    for func in parsed.all_functions() {
        if python_is_inner_decorated_function(&func) {
            continue;
        }
        let line = func.start_position().row + 1;
        let params = python_function_params(parsed, &func);
        let has_request_param = params.iter().any(|param| param.name == "request");
        let django_request_data = if has_request_param {
            python_django_request_data_sources(parsed, &func)
        } else {
            PythonDjangoRequestDataSources::default()
        };
        let has_django_import_context =
            parsed.source.contains("django") || parsed.source.contains("rest_framework");
        let is_fastapi_route =
            crate::frameworks::python::fastapi::function_has_route_decorator_with_receivers(
                parsed,
                &func,
                &fastapi_receivers,
            );
        let is_flask_route =
            crate::frameworks::python::flask::function_has_route_decorator_with_receivers(
                parsed,
                &func,
                &flask_receivers,
            );
        let is_drf_or_django_view = has_request_param
            && (has_django_import_context
                || (django_request_data.has_access
                    && python_looks_like_standalone_django_view(parsed, &func)));

        if is_fastapi_route {
            for param in &params {
                if param.name == "self" {
                    continue;
                }
                let annotation = param.annotation.as_deref().unwrap_or("");
                if annotation.contains("Request")
                    || annotation.contains("Query")
                    || annotation.contains("Path")
                    || annotation.contains("Body")
                    || annotation.contains("Header")
                    || annotation.contains("Form")
                    || annotation.contains("File")
                    || pydantic_models.contains(annotation)
                {
                    sources.push(TaintSeed::target(
                        file_path.to_string(),
                        line,
                        AccessPath::simple(param.name.as_str()),
                    ));
                }
            }
        } else if is_drf_or_django_view {
            for param in &params {
                if param.name == "request" {
                    sources.push(TaintSeed::target(
                        file_path.to_string(),
                        line,
                        AccessPath::simple("request"),
                    ));
                }
            }
            for (source_line, target) in django_request_data.targets {
                sources.push(TaintSeed::target(
                    file_path.to_string(),
                    source_line,
                    target,
                ));
            }
        } else if is_flask_route {
            let flask_request_data = python_flask_request_data_sources(parsed, &func);
            for source_line in flask_request_data.lines {
                sources.push(TaintSeed::line(file_path.to_string(), source_line));
            }
            for (source_line, target) in flask_request_data.targets {
                sources.push(TaintSeed::target(
                    file_path.to_string(),
                    source_line,
                    target,
                ));
            }
        }
    }
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

fn python_django_request_data_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
) -> PythonDjangoRequestDataSources {
    let mut sources = PythonDjangoRequestDataSources::default();
    collect_django_request_data_sources(parsed, *func, &mut sources);
    sources
}

fn python_flask_request_data_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
) -> PythonFlaskRequestDataSources {
    let mut sources = PythonFlaskRequestDataSources::default();
    collect_flask_request_data_sources(parsed, *func, &mut sources);
    sources
}

fn collect_flask_request_data_sources(
    parsed: &ParsedFile,
    node: Node<'_>,
    sources: &mut PythonFlaskRequestDataSources,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_flask_request_assignment_targets(parsed, lhs, rhs, sources);
        }
        return;
    }

    if let Some((target, value)) = python_named_expression_parts(parsed, node) {
        if node_contains_flask_request_data_access(parsed, value) {
            collect_flask_request_targets(parsed, target, node.start_position().row + 1, sources);
        }
        return;
    }

    if parsed.language.is_call_node(node.kind()) {
        if let Some(path) = call_path_text(parsed, &node) {
            if python_is_flask_request_source_call(path.trim()) {
                sources.lines.insert(node.start_position().row + 1);
                return;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_flask_request_data_sources(parsed, child, sources);
    }
}

fn collect_flask_request_assignment_targets(
    parsed: &ParsedFile,
    lhs: Node<'_>,
    rhs: Node<'_>,
    sources: &mut PythonFlaskRequestDataSources,
) {
    let lhs_items = python_assignment_items(lhs);
    let rhs_items = python_assignment_items(rhs);
    if lhs_items.len() == rhs_items.len() && lhs_items.len() > 1 {
        for (lhs_item, rhs_item) in lhs_items.into_iter().zip(rhs_items) {
            if node_contains_flask_request_data_access(parsed, rhs_item) {
                collect_flask_request_targets(
                    parsed,
                    lhs_item,
                    rhs_item.start_position().row + 1,
                    sources,
                );
            }
        }
        return;
    }

    if node_contains_flask_request_data_access(parsed, rhs) {
        collect_flask_request_targets(parsed, lhs, rhs.start_position().row + 1, sources);
    }
}

fn collect_flask_request_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    sources: &mut PythonFlaskRequestDataSources,
) {
    match node.kind() {
        "identifier" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                sources.targets.insert((line, AccessPath::simple(name)));
            }
        }
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "tuple"
        | "list"
        | "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_flask_request_targets(parsed, child, line, sources);
            }
        }
        _ => {}
    }
}

fn python_assignment_items(node: Node<'_>) -> Vec<Node<'_>> {
    let node = unwrap_python_parenthesized_expression(node);
    if !matches!(
        node.kind(),
        "pattern_list" | "expression_list" | "tuple" | "list" | "tuple_pattern" | "list_pattern"
    ) {
        return vec![node];
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn unwrap_python_parenthesized_expression(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        let mut cursor = node.walk();
        let next = node.named_children(&mut cursor).next();
        match next {
            Some(child) => node = child,
            None => return node,
        }
    }
    node
}

fn python_named_expression_parts<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
) -> Option<(Node<'a>, Node<'a>)> {
    if parsed.language != Language::Python
        || !matches!(node.kind(), "named_expression" | "assignment_expression")
    {
        return None;
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'a>> = node.named_children(&mut cursor).collect();
    let target = children.first().copied()?;
    let value = children.last().copied()?;
    if target.id() == value.id() {
        return None;
    }
    Some((target, value))
}

fn collect_django_request_data_sources(
    parsed: &ParsedFile,
    node: Node<'_>,
    sources: &mut PythonDjangoRequestDataSources,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            if node_contains_django_request_data_access(parsed, rhs) {
                sources.has_access = true;
                collect_bare_identifier_targets(
                    parsed,
                    lhs,
                    node.start_position().row + 1,
                    sources,
                );
            }
        }
    }

    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_django_request_data_access(text.trim()) {
            sources.has_access = true;
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_django_request_data_sources(parsed, child, sources);
    }
}

fn node_contains_django_request_data_access(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_django_request_data_access(text.trim()) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_django_request_data_access(parsed, child) {
            return true;
        }
    }
    false
}

fn node_contains_flask_request_data_access(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if parsed.language.is_call_node(node.kind()) {
        if let Some(path) = call_path_text(parsed, &node) {
            if python_is_flask_request_source_call(path.trim()) {
                return true;
            }
        }
    }
    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_flask_request_data_access(text.trim()) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_flask_request_data_access(parsed, child) {
            return true;
        }
    }
    false
}

fn collect_bare_identifier_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    sources: &mut PythonDjangoRequestDataSources,
) {
    match node.kind() {
        "identifier" => {
            sources
                .targets
                .insert((line, AccessPath::simple(parsed.node_text(&node))));
        }
        "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_bare_identifier_targets(parsed, child, line, sources);
            }
        }
        _ => {}
    }
}

fn python_is_django_request_data_access(path: &str) -> bool {
    const ACCESSORS: &[&str] = &["GET", "POST", "FILES", "COOKIES", "META", "body", "method"];
    ACCESSORS.iter().any(|accessor| {
        let prefix = format!("request.{accessor}");
        path == prefix || path.starts_with(&format!("{prefix}."))
    })
}

fn python_is_flask_request_source_call(path: &str) -> bool {
    python_is_flask_request_data_access(path)
        || matches!(path, "request.get_json" | "request.get_data")
}

fn python_is_flask_request_data_access(path: &str) -> bool {
    const ACCESSORS: &[&str] = &[
        "args", "form", "values", "cookies", "headers", "files", "json", "data",
    ];
    ACCESSORS.iter().any(|accessor| {
        let prefix = format!("request.{accessor}");
        path == prefix || path.starts_with(&format!("{prefix}."))
    })
}

fn python_looks_like_standalone_django_view(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    parsed.path.ends_with("views.py")
        || parsed
            .language
            .function_name(func)
            .map(|name| parsed.node_text(&name).contains("view"))
            .unwrap_or(false)
}

fn python_is_inner_decorated_function(func: &Node<'_>) -> bool {
    func.kind() == "function_definition"
        && func
            .parent()
            .is_some_and(|parent| parent.kind() == "decorated_definition")
}

#[derive(Debug)]
struct PythonParam {
    name: String,
    annotation: Option<String>,
}

fn python_function_params(parsed: &ParsedFile, func: &Node<'_>) -> Vec<PythonParam> {
    let mut out = Vec::new();
    let function_node = if func.kind() == "decorated_definition" {
        let mut cursor = func.walk();
        let found = func
            .children(&mut cursor)
            .find(|child| child.kind() == "function_definition")
            .unwrap_or(*func);
        found
    } else {
        *func
    };
    let params = match function_node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return out,
    };
    let mut cursor = params.walk();
    for child in params.named_children(&mut cursor) {
        let text = parsed.node_text(&child).trim();
        if text.is_empty() || text == "/" || text == "*" {
            continue;
        }
        let text = text.trim_start_matches('*');
        let (name_part, rest) = text
            .split_once(':')
            .map(|(n, r)| (n.trim(), Some(r.trim())))
            .unwrap_or((text.trim(), None));
        let name = name_part
            .split('=')
            .next()
            .unwrap_or(name_part)
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let annotation = rest.map(|r| {
            r.split('=')
                .next()
                .unwrap_or(r)
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        });
        out.push(PythonParam { name, annotation });
    }
    out
}

fn collect_python_pydantic_models(parsed: &ParsedFile) -> BTreeSet<String> {
    let mut models = BTreeSet::new();
    for line in parsed.source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("class ") || !trimmed.contains("BaseModel") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = rest
                .split(['(', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() {
                models.insert(name);
            }
        }
    }
    models
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

fn js_ts_variable_reference_ranges_scoped(
    parsed: &ParsedFile,
    func: Node<'_>,
    var_name: &str,
    def_line: usize,
) -> Vec<(usize, usize, usize)> {
    let scoped_lines = parsed.find_variable_references_scoped(&func, var_name, def_line);
    let mut refs = Vec::new();
    collect_js_ts_identifier_reference_ranges(parsed, func, var_name, &scoped_lines, &mut refs);
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn collect_js_ts_identifier_reference_ranges(
    parsed: &ParsedFile,
    node: Node<'_>,
    var_name: &str,
    scoped_lines: &BTreeSet<usize>,
    out: &mut Vec<(usize, usize, usize)>,
) {
    let line = node.start_position().row + 1;
    if scoped_lines.contains(&line)
        && node.kind() == "identifier"
        && parsed.node_text(&node) == var_name
    {
        out.push((line, node.start_byte(), node.end_byte()));
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_identifier_reference_ranges(parsed, child, var_name, scoped_lines, out);
    }
}

fn synthesize_js_ts_assignment_alias_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
) {
    if !is_js_ts_language(ctx.parsed.language) {
        return;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(ctx.func, ctx.parsed, &mut assignments);
    assignments.sort_by_key(|node| node.start_byte());

    let mut request_alias_defs = BTreeMap::new();
    let mut koa_request_object_alias_defs = BTreeMap::new();
    let js_ts_request_source = js_ts_request_source_seed_framework_and_params(ctx);
    let mut alias_defs = BTreeMap::new();
    if js_ts_request_source.is_none() {
        js_ts_add_alias_def(
            &mut alias_defs,
            ctx.target.base.clone(),
            js_ts_alias_def(
                ctx.seed.line,
                ctx.seed.start_byte,
                ctx.seed.scope,
                ctx.seed.byte_scope,
            ),
        );
    }
    if let Some((framework, _)) = &js_ts_request_source {
        if *framework == "koa" && js_ts_seed_is_koa_request_object_alias(ctx) {
            js_ts_add_alias_def(
                &mut koa_request_object_alias_defs,
                ctx.target.base.clone(),
                js_ts_alias_def(
                    ctx.seed.line,
                    ctx.seed.start_byte,
                    ctx.seed.scope,
                    ctx.seed.byte_scope,
                ),
            );
        } else {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                ctx.target.base.clone(),
                js_ts_alias_def(
                    ctx.seed.line,
                    ctx.seed.start_byte,
                    ctx.seed.scope,
                    ctx.seed.byte_scope,
                ),
            );
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for assignment in &assignments {
            let assignment_line = assignment.start_position().row + 1;
            if assignment_line < ctx.seed.line
                || (assignment_line == ctx.seed.line
                    && ctx
                        .seed
                        .start_byte
                        .is_some_and(|seed_start| assignment.start_byte() <= seed_start))
            {
                continue;
            }
            if !js_ts_seed_scope_contains(ctx.seed, assignment_line) {
                continue;
            }
            if js_ts_enclosing_function_id(ctx.parsed, assignment) != Some(ctx.func.id()) {
                continue;
            }
            if let Some(cfg_set) = ctx.reachable {
                if !reference_line_cfg_reachable(
                    ctx.parsed,
                    &ctx.func,
                    &ctx.seed.file,
                    assignment_line,
                    cfg_set,
                ) {
                    continue;
                }
            }
            let Some((lhs, rhs)) = js_ts_assignment_target_and_value(ctx.parsed, assignment) else {
                continue;
            };
            let alias_scope =
                js_ts_assignment_effective_line_scope(ctx.parsed, assignment, ctx.func.id());
            let alias_byte_scope =
                js_ts_assignment_effective_byte_scope(ctx.parsed, assignment, ctx.func.id());
            let alias_def = js_ts_alias_def(
                assignment_line,
                Some(lhs.start_byte()),
                alias_scope,
                alias_byte_scope,
            );
            if let Some((framework, source_params)) = &js_ts_request_source {
                let request_aliases = js_ts_active_alias_names_at_range(
                    &request_alias_defs,
                    assignment_line,
                    rhs.start_byte(),
                    rhs.end_byte(),
                )
                .into_iter()
                .chain(source_params.iter().cloned())
                .collect::<BTreeSet<_>>();
                let koa_request_object_aliases = js_ts_active_alias_names_at_range(
                    &koa_request_object_alias_defs,
                    assignment_line,
                    rhs.start_byte(),
                    rhs.end_byte(),
                );
                if js_ts_rhs_is_bare_alias(ctx.parsed, &rhs, &request_aliases) {
                    if let Some(alias) = js_ts_simple_lhs_identifier(ctx.parsed, &lhs) {
                        if alias != "_"
                            && js_ts_add_alias_def(
                                &mut request_alias_defs,
                                alias.clone(),
                                alias_def,
                            )
                        {
                            add_js_ts_request_alias_reference_edges(
                                ctx,
                                edges,
                                &alias,
                                assignment_line,
                                alias_scope,
                                framework,
                                &source_params
                                    .iter()
                                    .cloned()
                                    .chain(request_aliases)
                                    .chain(std::iter::once(alias.clone()))
                                    .collect(),
                                &koa_request_object_aliases,
                            );
                            changed = true;
                        }
                    }
                    continue;
                }
                if *framework == "koa"
                    && (js_ts_rhs_is_bare_alias(ctx.parsed, &rhs, &koa_request_object_aliases)
                        || js_ts_rhs_is_koa_request_object_alias(
                            ctx.parsed,
                            &rhs,
                            &request_aliases,
                        ))
                {
                    if let Some(alias) = js_ts_simple_lhs_identifier(ctx.parsed, &lhs) {
                        if alias != "_"
                            && js_ts_add_alias_def(
                                &mut koa_request_object_alias_defs,
                                alias.clone(),
                                alias_def,
                            )
                        {
                            add_js_ts_request_alias_reference_edges(
                                ctx,
                                edges,
                                &alias,
                                assignment_line,
                                alias_scope,
                                framework,
                                &request_aliases,
                                &source_params
                                    .iter()
                                    .cloned()
                                    .chain(koa_request_object_aliases)
                                    .chain(std::iter::once(alias.clone()))
                                    .collect(),
                            );
                            changed = true;
                        }
                    }
                    continue;
                }
                if node_contains_js_ts_source_access_with_request_object_aliases(
                    ctx.parsed,
                    rhs,
                    framework,
                    &request_aliases,
                    &koa_request_object_aliases,
                ) {
                    continue;
                }
                js_ts_kill_simple_lhs_alias_defs_at(
                    ctx.parsed,
                    &lhs,
                    assignment,
                    assignment_line,
                    ctx.func.id(),
                    &mut request_alias_defs,
                    &mut koa_request_object_alias_defs,
                );
            }
            let rhs_uses_alias = alias_defs.iter().any(|(alias, alias_defs)| {
                alias_defs.iter().any(|alias_def| {
                    alias_def.visible_range(assignment_line, rhs.start_byte(), rhs.end_byte())
                        && node_contains_identifier(ctx.parsed, &rhs, alias)
                })
            });
            if !rhs_uses_alias {
                continue;
            }
            if add_js_ts_tainted_assignment_aliases(
                ctx,
                edges,
                &lhs,
                assignment,
                assignment_line,
                &mut alias_defs,
            ) {
                changed = true;
            }
        }
    }
}

fn add_js_ts_tainted_assignment_aliases(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
    lhs: &Node<'_>,
    binding_node: &Node<'_>,
    assignment_line: usize,
    alias_defs: &mut JsTsAliasDefs,
) -> bool {
    let mut added = false;
    let alias_scope =
        js_ts_assignment_effective_line_scope(ctx.parsed, binding_node, ctx.func.id());
    let alias_byte_scope =
        js_ts_assignment_effective_byte_scope(ctx.parsed, binding_node, ctx.func.id());
    for alias in assignment_lhs_identifiers(ctx.parsed, lhs) {
        if alias == "_" {
            continue;
        }
        let alias_def = js_ts_alias_def(
            assignment_line,
            Some(lhs.start_byte()),
            alias_scope,
            alias_byte_scope,
        );
        if !js_ts_add_alias_def(alias_defs, alias.clone(), alias_def.clone()) {
            continue;
        }
        added = true;
        if !edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file
                && edge.to.line == assignment_line
                && edge.to.var_name() == alias
        }) {
            edges.push(FlowEdge {
                from: ctx.from.clone(),
                to: VarLocation {
                    file: ctx.seed.file.clone(),
                    function: ctx.func_name.to_string(),
                    function_start_line: ctx.func.start_position().row + 1,
                    line: assignment_line,
                    path: AccessPath::simple(alias.clone()),
                    start_byte: 0,
                    end_byte: 0,
                    kind: VarAccessKind::Def,
                },
            });
        }
        let refs = ctx
            .parsed
            .find_variable_references_scoped(&ctx.func, &alias, assignment_line);
        for ref_line in refs {
            if ref_line <= assignment_line {
                continue;
            }
            if !js_ts_seed_scope_contains(ctx.seed, ref_line) || !alias_def.visible_on(ref_line) {
                continue;
            }
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
            if edges.iter().any(|edge| {
                edge.to.file == ctx.seed.file
                    && edge.to.line == ref_line
                    && edge.to.var_name() == alias
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
                    path: AccessPath::simple(alias.clone()),
                    start_byte: 0,
                    end_byte: 0,
                    kind: VarAccessKind::Use,
                },
            });
        }
    }
    added
}

fn add_js_ts_request_alias_reference_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
    alias: &str,
    assignment_line: usize,
    alias_scope: Option<(usize, usize)>,
    framework: &str,
    request_aliases: &BTreeSet<String>,
    koa_request_object_aliases: &BTreeSet<String>,
) {
    let refs = ctx
        .parsed
        .find_variable_references_scoped(&ctx.func, alias, assignment_line);
    for ref_line in refs {
        if ref_line < assignment_line {
            continue;
        }
        if !js_ts_seed_scope_contains(ctx.seed, ref_line)
            || !js_ts_line_in_scope(alias_scope, ref_line)
        {
            continue;
        }
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
        if !node_contains_js_ts_source_access_on_line_with_request_object_aliases(
            ctx.parsed,
            ctx.func,
            ctx.func.id(),
            ref_line,
            framework,
            request_aliases,
            koa_request_object_aliases,
        ) {
            continue;
        }
        let source_ranges =
            js_ts_request_source_access_ranges_for_alias_on_line(ctx.parsed, ref_line, alias);
        if source_ranges.is_empty() {
            continue;
        }
        if edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file && edge.to.line == ref_line && edge.to.var_name() == alias
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
                path: AccessPath::simple(alias.to_string()),
                start_byte: 0,
                end_byte: 0,
                kind: VarAccessKind::Use,
            },
        });
    }
}

fn js_ts_request_source_seed_framework_and_params(
    ctx: &TargetSeedSynthesisContext<'_>,
) -> Option<(&'static str, BTreeSet<String>)> {
    if !ctx.target.is_simple() {
        return None;
    }
    let framework = ctx.parsed.framework()?.name;
    if !matches!(framework, "fastify" | "express" | "koa") {
        return None;
    }
    let params = js_ts_function_params(ctx.parsed, &ctx.func);
    if params.is_empty() {
        return None;
    }
    let framework_receivers = js_ts_framework_receiver_names(ctx.parsed, framework);
    let source_params = js_ts_framework_source_params(
        ctx.parsed,
        &ctx.func,
        framework,
        &params,
        &framework_receivers,
    );
    if source_params.contains(&ctx.target.base)
        && ctx.seed.line == ctx.func.start_position().row + 1
    {
        return Some((framework, source_params));
    }
    let target_aliases = std::iter::once(ctx.target.base.clone()).collect::<BTreeSet<_>>();
    if framework == "koa"
        && node_contains_js_ts_request_object_source_access_on_line(
            ctx.parsed,
            ctx.func,
            ctx.func.id(),
            ctx.seed.line,
            &target_aliases,
        )
    {
        return Some((framework, source_params));
    }
    if !node_contains_js_ts_source_access_on_line(
        ctx.parsed,
        ctx.func,
        ctx.func.id(),
        ctx.seed.line,
        framework,
        &target_aliases,
    ) {
        return None;
    }
    let source_aliases = source_params
        .iter()
        .cloned()
        .chain(target_aliases)
        .collect::<BTreeSet<_>>();
    Some((framework, source_aliases))
}

fn js_ts_seed_is_koa_request_object_alias(ctx: &TargetSeedSynthesisContext<'_>) -> bool {
    if ctx.parsed.framework().map(|spec| spec.name) != Some("koa") {
        return false;
    }
    let target_aliases = std::iter::once(ctx.target.base.clone()).collect::<BTreeSet<_>>();
    node_contains_js_ts_request_object_source_access_on_line(
        ctx.parsed,
        ctx.func,
        ctx.func.id(),
        ctx.seed.line,
        &target_aliases,
    )
}

fn js_ts_rhs_is_bare_alias(
    parsed: &ParsedFile,
    rhs: &Node<'_>,
    aliases: &BTreeSet<String>,
) -> bool {
    let rhs = unwrap_js_ts_alias_rhs(*rhs);
    rhs.kind() == "identifier" && aliases.contains(parsed.node_text(&rhs))
}

fn js_ts_rhs_is_koa_request_object_alias(
    parsed: &ParsedFile,
    rhs: &Node<'_>,
    context_aliases: &BTreeSet<String>,
) -> bool {
    let rhs = unwrap_js_ts_alias_rhs(*rhs);
    let text = parsed.node_text(&rhs).trim();
    context_aliases
        .iter()
        .any(|alias| js_ts_exact_field_access_text_matches(text, alias, "request"))
}

fn unwrap_js_ts_alias_rhs(mut node: Node<'_>) -> Node<'_> {
    loop {
        let unwrapped = unwrap_parenthesized(node);
        if unwrapped.id() != node.id() {
            node = unwrapped;
            continue;
        }

        let next = match node.kind() {
            "as_expression" | "satisfies_expression" => node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(0)),
            "non_null_expression" => node
                .child_by_field_name("argument")
                .or_else(|| node.named_child(0)),
            "type_assertion" => node
                .child_by_field_name("expression")
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(node.named_child_count().saturating_sub(1))),
            _ => None,
        };

        match next {
            Some(next) if next.id() != node.id() => node = next,
            _ => return node,
        }
    }
}

fn js_ts_simple_lhs_identifier(parsed: &ParsedFile, lhs: &Node<'_>) -> Option<String> {
    (lhs.kind() == "identifier").then(|| parsed.node_text(lhs).to_string())
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
enum SinkMatchOutcome {
    Match(&'static SinkPattern),
    SemanticallyExcluded,
    NoMatch,
}

fn call_arg_node<'a>(call: &Node<'a>, arg_idx: usize) -> Option<Node<'a>> {
    let arguments = call.child_by_field_name("arguments").or_else(|| {
        let mut cursor = call.walk();
        let found = call
            .named_children(&mut cursor)
            .find(|child| child.kind() == "arguments");
        found
    })?;
    let mut cursor = arguments.walk();
    let arg = arguments.named_children(&mut cursor).nth(arg_idx);
    arg
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
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathSanitizerKind {
    Clean,
    Rel,
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

fn go_path_traversal_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: Option<&FlowPath>,
    sink_line: usize,
    sink_pat: Option<&'static SinkPattern>,
    sink_call: Option<&Node<'_>>,
) -> bool {
    if parsed.language != Language::Go || !cpg.has_cfg_edges() {
        return false;
    }

    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };

    for binding in collect_path_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if let Some(p) = path {
            if !path_targets_var_at_line(parsed, p, sink_line, &binding.result_var) {
                continue;
            }
        }
        if let (Some(call), Some(pat)) = (sink_call, sink_pat) {
            if !sink_call_uses_var_in_tainted_arg(parsed, call, pat, &binding.result_var) {
                continue;
            }
        } else if let Some(pat) = sink_pat {
            if !line_has_matching_sink_call_using_var(
                parsed,
                sink_line,
                pat,
                &binding.result_var,
                path,
            ) {
                continue;
            }
        } else if path.is_none() {
            continue;
        }

        if guard_safely_controls_sink(parsed, cpg, &func_node, &binding, sink_line) {
            return true;
        }
    }

    false
}

fn collect_path_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<PathSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_assignments(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let lhs = match parsed.language.assignment_target(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let rhs = match parsed.language.assignment_value(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let lhs_items = assignment_lhs_identifiers(parsed, &lhs);
        if lhs_items.is_empty() {
            continue;
        }
        let rhs_items = assignment_rhs_expressions(&rhs);

        for (idx, expr) in rhs_items.iter().enumerate() {
            if expr.kind() != "call_expression" {
                continue;
            }
            let kind = match go_call_path_text(parsed, expr).as_deref() {
                Some("filepath.Clean") => PathSanitizerKind::Clean,
                Some("filepath.Rel") => PathSanitizerKind::Rel,
                _ => continue,
            };
            let result_var = match lhs_items.get(idx).or_else(|| {
                if rhs_items.len() == 1 {
                    lhs_items.first()
                } else {
                    None
                }
            }) {
                Some(name) if name != "_" => name.clone(),
                _ => continue,
            };
            bindings.push(PathSanitizerBinding {
                kind,
                result_var,
                call_line: expr.start_position().row + 1,
            });
        }
    }
    bindings
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

fn line_has_matching_sink_call_using_var(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    var_name: &str,
    path: Option<&FlowPath>,
) -> bool {
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
        if actual != sink_pat.call_path || !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
        }
        if let Some(p) = path {
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, p) {
                continue;
            }
        }
        if sink_call_uses_var_in_tainted_arg(parsed, call, sink_pat, var_name) {
            return true;
        }
    }
    false
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

fn python_xss_cleansed_for_sink(
    parsed: &ParsedFile,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if parsed.language != Language::Python || sink_pat.category != SanitizerCategory::Xss {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_safe_format_html_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.result_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.result_var) {
            continue;
        }
        return true;
    }
    false
}

fn collect_safe_format_html_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<SafeFormatHtmlBinding> {
    let mut assignments = Vec::new();
    collect_assignments(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let lhs = match parsed.language.assignment_target(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let rhs = match parsed.language.assignment_value(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let lhs_items = assignment_lhs_identifiers(parsed, &lhs);
        if lhs_items.is_empty() {
            continue;
        }
        let rhs_items = if call_path_text(parsed, &rhs).is_some() {
            vec![rhs]
        } else {
            assignment_rhs_expressions(&rhs)
        };

        for (idx, expr) in rhs_items.iter().enumerate() {
            let actual = match call_path_text(parsed, expr) {
                Some(s) => s,
                None => continue,
            };
            if !call_path_matches(parsed, &actual, "format_html") {
                continue;
            }
            if call_literal_arg(parsed, expr, 0).is_none() {
                continue;
            }
            let result_var = match lhs_items.get(idx).or_else(|| {
                if rhs_items.len() == 1 {
                    lhs_items.first()
                } else {
                    None
                }
            }) {
                Some(name) if name != "_" => name.clone(),
                _ => continue,
            };
            bindings.push(SafeFormatHtmlBinding {
                result_var,
                call_line: expr.start_position().row + 1,
            });
        }
    }
    bindings
}

fn guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    func_node: &Node<'_>,
    binding: &PathSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_guard_control(parsed, &condition, binding) {
            Some(c) => c,
            None => continue,
        };
        let consequence = match guard.child_by_field_name("consequence") {
            Some(n) => n,
            None => continue,
        };
        let consequence_entry = match first_statement_line(parsed, &consequence) {
            Some(line) => line,
            None => continue,
        };

        match control {
            GuardControl::RejectBranch => {
                if !block_ends_with_return(parsed, &consequence) {
                    continue;
                }
                let safe_entry = match safe_successor_line(cpg, parsed, &guard, consequence_entry) {
                    Some(line) => line,
                    None => continue,
                };
                if cfg_line_reaches(cpg, &parsed.path, safe_entry, sink_line)
                    && !cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
            GuardControl::AllowBranch => {
                if node_contains_line(&consequence, sink_line)
                    && cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
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

fn classify_guard_control(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &PathSanitizerBinding,
) -> Option<GuardControl> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if condition_text.contains("&&") {
        return None;
    }
    if condition_text.contains("||") {
        return if binding.kind == PathSanitizerKind::Rel
            && contains_positive_hasprefix_call(parsed, &condition, &binding.result_var, false)
        {
            Some(GuardControl::RejectBranch)
        } else {
            None
        };
    }
    if is_negated_hasprefix_condition(parsed, &condition, &binding.result_var) {
        return match binding.kind {
            PathSanitizerKind::Clean => Some(GuardControl::RejectBranch),
            PathSanitizerKind::Rel => Some(GuardControl::AllowBranch),
        };
    }
    if is_bare_hasprefix_condition(parsed, &condition, &binding.result_var) {
        return match binding.kind {
            PathSanitizerKind::Clean => Some(GuardControl::AllowBranch),
            PathSanitizerKind::Rel => Some(GuardControl::RejectBranch),
        };
    }
    None
}

fn unwrap_parenthesized(mut node: Node<'_>) -> Node<'_> {
    loop {
        if node.kind() != "parenthesized_expression" {
            return node;
        }
        node = match node.named_child(0) {
            Some(child) => child,
            None => return node,
        };
    }
}

fn is_negated_hasprefix_condition(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    var_name: &str,
) -> bool {
    let condition_text = parsed.node_text(condition).trim();
    if condition.kind() != "unary_expression" || !condition_text.starts_with('!') {
        return false;
    }
    if let Some(child) = condition.named_child(0) {
        let child = unwrap_parenthesized(child);
        is_bare_hasprefix_condition(parsed, &child, var_name)
    } else {
        false
    }
}

fn is_bare_hasprefix_condition(parsed: &ParsedFile, condition: &Node<'_>, var_name: &str) -> bool {
    let condition = unwrap_parenthesized(*condition);
    is_hasprefix_call_for_var(parsed, &condition, var_name)
}

fn contains_positive_hasprefix_call(
    parsed: &ParsedFile,
    node: &Node<'_>,
    var_name: &str,
    negated: bool,
) -> bool {
    let node_text = parsed.node_text(node).trim();
    let next_negated = negated || (node.kind() == "unary_expression" && node_text.starts_with('!'));
    if node.kind() == "call_expression"
        && !next_negated
        && is_hasprefix_call_for_var(parsed, node, var_name)
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if contains_positive_hasprefix_call(parsed, &child, var_name, next_negated) {
            return true;
        }
    }
    false
}

fn is_hasprefix_call_for_var(parsed: &ParsedFile, node: &Node<'_>, var_name: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    if go_call_path_text(parsed, node).as_deref() != Some("strings.HasPrefix") {
        return false;
    }
    let arguments = match node.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    let first_arg = match arguments.named_children(&mut cursor).next() {
        Some(n) => unwrap_parenthesized(n),
        None => return false,
    };
    first_arg.kind() == "identifier" && parsed.node_text(&first_arg) == var_name
}

fn first_statement_line(parsed: &ParsedFile, node: &Node<'_>) -> Option<usize> {
    if parsed.language.is_statement_node(node.kind()) {
        return Some(node.start_position().row + 1);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
fn line_matches_structured_sink(
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

fn js_ts_sink_call_has_inline_framework_source_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
) -> bool {
    if !is_js_ts_language(parsed.language) {
        return false;
    }
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return false;
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return false;
    }
    let Some(func) = parsed.enclosing_function(call.start_position().row + 1) else {
        return false;
    };
    let params = js_ts_function_params(parsed, &func);
    if params.is_empty() {
        return false;
    }
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    let source_params =
        js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
    if source_params.is_empty() {
        return false;
    }
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if sink_pat.tainted_arg_indices.contains(&idx)
            && node_contains_js_ts_source_access(parsed, arg, framework, &source_params)
        {
            return true;
        }
    }
    false
}

fn js_ts_inline_framework_source_yaml_sink_lines(parsed: &ParsedFile) -> BTreeSet<usize> {
    let mut lines = BTreeSet::new();
    if !is_js_ts_language(parsed.language) {
        return lines;
    }
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in &calls {
        let actual = match call_path_text(parsed, call) {
            Some(actual) => actual,
            None => continue,
        };
        for pat in JS_CWE502_SINKS {
            if !matches!(pat.call_path, "yaml.load" | "load") {
                continue;
            }
            if !sink_call_path_matches(parsed, call, &actual, pat)
                || !call_passes_sink_semantics(parsed, call, pat)
            {
                continue;
            }
            if js_ts_sink_call_has_inline_framework_source_arg(parsed, call, pat) {
                lines.insert(call.start_position().row + 1);
                break;
            }
        }
    }
    lines
}

fn call_path_matches(parsed: &ParsedFile, actual: &str, expected: &str) -> bool {
    actual == expected
        || (parsed.language == Language::Python
            && !expected.contains('.')
            && actual
                .rsplit('.')
                .next()
                .is_some_and(|tail| tail == expected))
        || (is_js_ts_language(parsed.language)
            && !expected.contains('.')
            && actual
                .rsplit('.')
                .next()
                .is_some_and(|tail| tail == expected))
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

fn js_ts_js_yaml_bare_load_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
) -> bool {
    if actual.contains('.') {
        return false;
    }
    js_ts_identifier_binds_imported_member_at_call(parsed, call, actual, "js-yaml", "load")
}

fn js_ts_identifier_binds_imported_member_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    js_ts_imported_member_binding_visible_at_call(
        parsed,
        call,
        local_name,
        module_name,
        imported_member,
    ) && !js_ts_identifier_has_local_shadow_before_call(
        parsed,
        call,
        local_name,
        module_name,
        Some(imported_member),
    )
}

fn js_ts_identifier_binds_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    js_ts_module_binding_visible_at_call(parsed, call, local_name, module_name)
        && !js_ts_identifier_has_local_shadow_before_call(
            parsed,
            call,
            local_name,
            module_name,
            None,
        )
}

fn js_ts_identifier_has_local_shadow_before_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: Option<&str>,
) -> bool {
    let call_line = call.start_position().row + 1;
    let Some(func) = parsed.enclosing_function(call_line) else {
        return false;
    };
    if js_ts_function_parameter_binds_name(parsed, &func, local_name) {
        return true;
    }
    if js_ts_function_declaration_shadows_call(parsed, &func, call, local_name) {
        return true;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    for assignment in assignments {
        let is_function_scoped_var = js_ts_binding_is_function_scoped_var(&assignment);
        let binding_starts_after_call = assignment.start_byte() >= call.start_byte();
        if binding_starts_after_call && !is_function_scoped_var {
            continue;
        }
        if !js_ts_binding_scope_reaches_call(parsed, func.id(), call, &assignment) {
            continue;
        }
        let Some(lhs) = js_ts_assignment_target(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == local_name)
        {
            continue;
        }
        if !binding_starts_after_call {
            if let Some(rhs) = js_ts_assignment_value(parsed, &assignment) {
                if js_ts_assignment_imports_allowed_binding(
                    parsed,
                    &lhs,
                    &rhs,
                    local_name,
                    module_name,
                    imported_member,
                ) {
                    continue;
                }
            }
        }
        return true;
    }
    false
}

fn js_ts_binding_scope_reaches_call(
    parsed: &ParsedFile,
    root_func_id: usize,
    call: &Node<'_>,
    binding_node: &Node<'_>,
) -> bool {
    let binding_scope_id = if js_ts_binding_is_function_scoped_var(binding_node) {
        js_ts_nearest_function_scope_id(parsed, binding_node, root_func_id)
    } else {
        js_ts_nearest_scope_block_id(parsed, binding_node, root_func_id)
    };
    let Some(binding_scope_id) = binding_scope_id else {
        return false;
    };
    js_ts_scope_chain_contains(call, root_func_id, binding_scope_id)
}

fn js_ts_binding_is_function_scoped_var(binding_node: &Node<'_>) -> bool {
    if binding_node.kind() != "variable_declarator" {
        return false;
    }
    binding_node
        .parent()
        .is_some_and(|parent| parent.kind() == "variable_declaration")
}

fn js_ts_binding_scope_line_range(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    js_ts_binding_scope_node(parsed, binding_node, root_func_id)
        .map(|scope| (scope.start_position().row + 1, scope.end_position().row + 1))
}

fn js_ts_binding_scope_byte_range(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    js_ts_binding_scope_node(parsed, binding_node, root_func_id)
        .map(|scope| (scope.start_byte(), scope.end_byte()))
}

fn js_ts_binding_scope_node<'a>(
    parsed: &ParsedFile,
    binding_node: &Node<'a>,
    root_func_id: usize,
) -> Option<Node<'a>> {
    let function_scoped = js_ts_binding_is_function_scoped_var(binding_node);
    let mut current = Some(*binding_node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(parent);
        }
        if function_scoped {
            if parent.id() != binding_node.id()
                && parsed
                    .language
                    .function_node_types()
                    .contains(&parent.kind())
            {
                return Some(parent);
            }
        } else if js_ts_is_lexical_scope_boundary(parsed, parent.kind()) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn js_ts_assignment_target_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        js_ts_binding_scope_line_range(parsed, binding_node, root_func_id)
    } else {
        js_ts_assignment_existing_binding_scope(parsed, binding_node, root_func_id)
    }
}

fn js_ts_assignment_target_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        js_ts_binding_scope_byte_range(parsed, binding_node, root_func_id)
    } else {
        js_ts_assignment_existing_binding_byte_scope(parsed, binding_node, root_func_id)
    }
}

fn js_ts_assignment_effective_line_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let binding_scope = js_ts_assignment_target_scope(parsed, binding_node, root_func_id);
    let exiting_branch_scope =
        js_ts_enclosing_definitely_exiting_branch(binding_node, root_func_id).map(|branch| {
            (
                branch.start_position().row + 1,
                branch.end_position().row + 1,
            )
        });
    js_ts_intersect_line_scopes(binding_scope, exiting_branch_scope)
}

fn js_ts_assignment_effective_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let binding_scope = js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id);
    let exiting_branch_scope =
        js_ts_enclosing_definitely_exiting_branch(binding_node, root_func_id)
            .map(|branch| (branch.start_byte(), branch.end_byte()));
    js_ts_intersect_byte_scopes(binding_scope, exiting_branch_scope)
}

fn js_ts_enclosing_definitely_exiting_branch<'a>(
    node: &Node<'a>,
    root_func_id: usize,
) -> Option<Node<'a>> {
    let mut child = *node;
    let mut current = child.parent();
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return None;
        }
        if matches!(parent.kind(), "if_statement" | "if_expression") {
            let consequence = parent.child_by_field_name("consequence");
            let alternative = parent.child_by_field_name("alternative");
            if (consequence.is_some_and(|branch| branch.id() == child.id())
                || alternative.is_some_and(|branch| branch.id() == child.id()))
                && js_ts_node_definitely_exits(child)
            {
                return Some(child);
            }
        }
        child = parent;
        current = parent.parent();
    }
    None
}

fn js_ts_assignment_existing_binding_scope(
    parsed: &ParsedFile,
    assignment_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let (lhs, _) = js_ts_assignment_target_and_value(parsed, assignment_node)?;
    let aliases = js_ts_lhs_binding_names(parsed, &lhs);
    if aliases.is_empty() {
        return None;
    }
    let func = parsed.enclosing_function(assignment_node.start_position().row + 1)?;
    if func.id() != root_func_id {
        return None;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    assignments
        .into_iter()
        .filter(|candidate| {
            candidate.kind() == "variable_declarator"
                && candidate.start_byte() < assignment_node.start_byte()
                && js_ts_binding_scope_reaches_call(
                    parsed,
                    root_func_id,
                    assignment_node,
                    candidate,
                )
                && js_ts_assignment_target(parsed, candidate).is_some_and(|candidate_lhs| {
                    js_ts_lhs_binding_names(parsed, &candidate_lhs)
                        .iter()
                        .any(|name| aliases.contains(name))
                })
        })
        .max_by_key(|candidate| candidate.start_byte())
        .and_then(|candidate| js_ts_binding_scope_line_range(parsed, &candidate, root_func_id))
}

fn js_ts_assignment_existing_binding_byte_scope(
    parsed: &ParsedFile,
    assignment_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let (lhs, _) = js_ts_assignment_target_and_value(parsed, assignment_node)?;
    let aliases = js_ts_lhs_binding_names(parsed, &lhs);
    if aliases.is_empty() {
        return None;
    }
    let func = parsed.enclosing_function(assignment_node.start_position().row + 1)?;
    if func.id() != root_func_id {
        return None;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(func, parsed, &mut assignments);
    assignments
        .into_iter()
        .filter(|candidate| {
            candidate.kind() == "variable_declarator"
                && candidate.start_byte() < assignment_node.start_byte()
                && js_ts_binding_scope_reaches_call(
                    parsed,
                    root_func_id,
                    assignment_node,
                    candidate,
                )
                && js_ts_assignment_target(parsed, candidate).is_some_and(|candidate_lhs| {
                    js_ts_lhs_binding_names(parsed, &candidate_lhs)
                        .iter()
                        .any(|name| aliases.contains(name))
                })
        })
        .max_by_key(|candidate| candidate.start_byte())
        .and_then(|candidate| js_ts_binding_scope_byte_range(parsed, &candidate, root_func_id))
}

fn js_ts_lhs_binding_names(parsed: &ParsedFile, lhs: &Node<'_>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_js_ts_lhs_alias_identifiers(parsed, lhs, &mut names);
    names.remove("_");
    names
}

fn js_ts_is_lexical_scope_boundary(parsed: &ParsedFile, kind: &str) -> bool {
    parsed.language.is_scope_block(kind)
        || matches!(
            kind,
            "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "for_await_statement"
                | "switch_statement"
                | "switch_body"
                | "catch_clause"
        )
}

fn js_ts_function_declaration_shadows_call(
    parsed: &ParsedFile,
    func: &Node<'_>,
    call: &Node<'_>,
    local_name: &str,
) -> bool {
    js_ts_scope_has_function_declaration_shadow(parsed, *func, func.id(), call, local_name)
}

fn js_ts_scope_has_function_declaration_shadow(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    call: &Node<'_>,
    local_name: &str,
) -> bool {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if parsed
            .language
            .function_node_types()
            .contains(&child.kind())
        {
            if matches!(
                child.kind(),
                "function_declaration" | "generator_function_declaration"
            ) && js_ts_binding_scope_reaches_call(parsed, root_func_id, call, &child)
                && parsed
                    .language
                    .function_name(&child)
                    .is_some_and(|name| parsed.node_text(&name) == local_name)
            {
                return true;
            }
            // Function declarations introduce the binding; their bodies are a
            // nested scope and should not contribute shadows to the caller.
            continue;
        }
        if js_ts_scope_has_function_declaration_shadow(
            parsed,
            child,
            root_func_id,
            call,
            local_name,
        ) {
            return true;
        }
    }
    false
}

fn js_ts_nearest_scope_block_id(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<usize> {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(root_func_id);
        }
        if js_ts_is_lexical_scope_boundary(parsed, parent.kind()) {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}

fn js_ts_nearest_function_scope_id(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<usize> {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return Some(root_func_id);
        }
        if parent.id() != node.id()
            && parsed
                .language
                .function_node_types()
                .contains(&parent.kind())
        {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}

fn js_ts_scope_chain_contains(
    node: &Node<'_>,
    root_func_id: usize,
    target_scope_id: usize,
) -> bool {
    let mut current = Some(*node);
    while let Some(parent) = current {
        if parent.id() == target_scope_id {
            return true;
        }
        if parent.id() == root_func_id {
            return target_scope_id == root_func_id;
        }
        current = parent.parent();
    }
    false
}

fn js_ts_function_parameter_binds_name(
    parsed: &ParsedFile,
    func: &Node<'_>,
    local_name: &str,
) -> bool {
    let Some(params) = func.child_by_field_name("parameters") else {
        return false;
    };
    let mut ids = Vec::new();
    collect_nodes_of_kind(params, "identifier", &mut ids);
    ids.iter().any(|id| parsed.node_text(id) == local_name)
}

fn js_ts_assignment_imports_allowed_binding(
    parsed: &ParsedFile,
    lhs: &Node<'_>,
    rhs: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: Option<&str>,
) -> bool {
    match imported_member {
        Some(member) => {
            if js_ts_require_call_module(parsed, rhs).is_some_and(|module| module == module_name) {
                return js_ts_pattern_binds_member(parsed, lhs, member, local_name);
            }
            lhs.kind() == "identifier"
                && parsed.node_text(lhs) == local_name
                && js_ts_require_member_expression(parsed, rhs).is_some_and(
                    |(module, actual_member)| module == module_name && actual_member == member,
                )
        }
        None => {
            lhs.kind() == "identifier"
                && parsed.node_text(lhs) == local_name
                && js_ts_require_call_module(parsed, rhs)
                    .is_some_and(|module| module == module_name)
        }
    }
}

fn js_ts_imported_member_binding_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "import_statement"
            && js_ts_import_statement_binds_member(
                parsed,
                &child,
                local_name,
                module_name,
                imported_member,
            )
        {
            return true;
        }
    }

    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(root, parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && js_ts_require_declarator_binds_member(
                parsed,
                decl,
                local_name,
                module_name,
                imported_member,
            )
            && js_ts_import_declarator_visible_at_call(parsed, call, decl)
    })
}

fn js_ts_import_declarator_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    declarator: &Node<'_>,
) -> bool {
    js_ts_assignment_visible_before_context(parsed, call, declarator)
}

fn js_ts_module_binding_visible_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "import_statement"
            && js_ts_import_statement_binds_module(parsed, &child, local_name, module_name)
        {
            return true;
        }
    }

    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(root, parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && js_ts_require_declarator_binds_module(parsed, decl, local_name, module_name)
            && js_ts_assignment_visible_before_context(parsed, call, decl)
    })
}

fn js_ts_assignment_visible_before_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    assignment: &Node<'_>,
) -> bool {
    if assignment.start_byte() < context.start_byte() {
        return js_ts_binding_visible_at_context(parsed, context, assignment);
    }

    // Top-level CommonJS bindings declared after a route callback are still
    // visible when that callback executes after module initialization.
    if js_ts_enclosing_function_id(parsed, assignment).is_none()
        && js_ts_enclosing_function_id(parsed, context).is_some()
    {
        return js_ts_binding_visible_at_context(parsed, context, assignment);
    }

    false
}

fn js_ts_binding_visible_at_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    binding_node: &Node<'_>,
) -> bool {
    let root_id = parsed.tree.root_node().id();
    let context_func_id = js_ts_enclosing_function_id(parsed, context);
    let binding_func_id = js_ts_enclosing_function_id(parsed, binding_node);

    match (binding_func_id, context_func_id) {
        (Some(binding_func_id), Some(context_func_id)) if binding_func_id == context_func_id => {
            js_ts_binding_scope_reaches_call(parsed, context_func_id, context, binding_node)
        }
        (Some(_), _) => false,
        (None, _) => js_ts_binding_scope_reaches_call(parsed, root_id, context, binding_node),
    }
}

fn js_ts_enclosing_function_id(parsed: &ParsedFile, node: &Node<'_>) -> Option<usize> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parsed
            .language
            .function_node_types()
            .contains(&parent.kind())
        {
            return Some(parent.id());
        }
        current = parent.parent();
    }
    None
}

fn js_ts_import_statement_binds_member(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let Some(source) = node.child_by_field_name("source") else {
        return false;
    };
    let source_text = parsed
        .node_text(&source)
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    if source_text != module_name {
        return false;
    }
    let mut specs = Vec::new();
    collect_nodes_of_kind(*node, "import_specifier", &mut specs);
    for spec in specs {
        let Some(name) = spec.child_by_field_name("name") else {
            continue;
        };
        if parsed.node_text(&name) != imported_member {
            continue;
        }
        let local = spec
            .child_by_field_name("alias")
            .map(|alias| parsed.node_text(&alias))
            .unwrap_or_else(|| parsed.node_text(&name));
        if local == local_name {
            return true;
        }
    }
    false
}

fn js_ts_import_statement_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let Some(source) = node.child_by_field_name("source") else {
        return false;
    };
    let source_text = parsed
        .node_text(&source)
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    if source_text != module_name {
        return false;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_clause" => {
                if js_ts_import_clause_binds_module(parsed, &child, local_name) {
                    return true;
                }
            }
            "identifier" => {
                if parsed.node_text(&child) == local_name {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn js_ts_import_clause_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if parsed.node_text(&child) == local_name {
                    return true;
                }
            }
            "namespace_import" => {
                if child
                    .child_by_field_name("name")
                    .is_some_and(|name| parsed.node_text(&name) == local_name)
                {
                    return true;
                }
                let mut inner = child.walk();
                if child
                    .children(&mut inner)
                    .any(|n| n.kind() == "identifier" && parsed.node_text(&n) == local_name)
                {
                    return true;
                }
            }
            // Named imports bind members, not the module namespace object.
            "named_imports" => {}
            _ => {}
        }
    }
    false
}

fn js_ts_require_declarator_binds_member(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let Some(value) = node.child_by_field_name("value") else {
        return false;
    };
    if js_ts_require_call_module(parsed, &value).is_some_and(|module| module == module_name) {
        return js_ts_pattern_binds_member(parsed, &name, imported_member, local_name);
    }
    if name.kind() == "identifier" && parsed.node_text(&name) == local_name {
        return js_ts_require_member_expression(parsed, &value)
            .is_some_and(|(module, member)| module == module_name && member == imported_member);
    }
    false
}

fn js_ts_require_declarator_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let Some(value) = node.child_by_field_name("value") else {
        return false;
    };
    name.kind() == "identifier"
        && parsed.node_text(&name) == local_name
        && js_ts_require_call_module(parsed, &value).is_some_and(|module| module == module_name)
}

fn js_ts_pattern_binds_member(
    parsed: &ParsedFile,
    pattern: &Node<'_>,
    imported_member: &str,
    local_name: &str,
) -> bool {
    let mut cursor = pattern.walk();
    for child in pattern.children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                if imported_member == local_name && parsed.node_text(&child) == imported_member {
                    return true;
                }
            }
            "pair_pattern" => {
                let key_matches = child
                    .child_by_field_name("key")
                    .is_some_and(|key| parsed.node_text(&key) == imported_member);
                let value_matches = child.child_by_field_name("value").is_some_and(|value| {
                    if value.kind() == "identifier" {
                        parsed.node_text(&value) == local_name
                    } else if value.kind() == "object_pattern" || value.kind() == "array_pattern" {
                        js_ts_pattern_binds_member(parsed, &value, imported_member, local_name)
                    } else {
                        false
                    }
                });
                if key_matches && value_matches {
                    return true;
                }
            }
            "object_pattern" | "array_pattern" => {
                if js_ts_pattern_binds_member(parsed, &child, imported_member, local_name) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn js_ts_require_member_expression(
    parsed: &ParsedFile,
    node: &Node<'_>,
) -> Option<(String, String)> {
    let node = unwrap_parenthesized(*node);
    if node.kind() != "member_expression" {
        return None;
    }
    let object = node.child_by_field_name("object")?;
    let property = node.child_by_field_name("property")?;
    let module = js_ts_require_call_module(parsed, &object)?;
    let member = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        .to_string();
    Some((module, member))
}

fn js_ts_require_call_module(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let node = unwrap_parenthesized(*node);
    if !parsed.language.is_call_node(node.kind()) {
        return None;
    }
    let function = parsed.language.call_function_name(&node)?;
    if parsed.node_text(&function) != "require" {
        return None;
    }
    let arg = call_arg_node(&node, 0)?;
    js_ts_literal_string_value(parsed, &arg)
}

fn js_ts_ssrf_sink_call_path_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
    expected: &str,
) -> bool {
    match expected {
        "fetch" => js_ts_ssrf_fetch_call_matches(parsed, call, actual),
        "axios" => js_ts_ssrf_direct_module_call_matches(parsed, call, &["axios"]),
        "got" => js_ts_ssrf_direct_module_call_matches(parsed, call, &["got"]),
        "get" | "post" | "request" => js_ts_ssrf_method_call_matches(parsed, call, expected),
        _ => call_path_matches(parsed, actual, expected),
    }
}

fn js_ts_ssrf_fetch_call_matches(parsed: &ParsedFile, call: &Node<'_>, actual: &str) -> bool {
    if matches!(actual, "fetch" | "globalThis.fetch" | "window.fetch") {
        return true;
    }
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let local = parsed.node_text(&function);
        return js_ts_identifier_binds_any_module_at_call(
            parsed,
            call,
            local,
            &["node-fetch", "cross-fetch", "isomorphic-fetch"],
        ) || js_ts_identifier_binds_any_imported_member_at_call(
            parsed,
            call,
            local,
            &["undici"],
            "fetch",
        );
    }
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    if parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        != "fetch"
    {
        return false;
    }
    function
        .child_by_field_name("object")
        .is_some_and(|object| {
            js_ts_expr_binds_any_module_at_call(parsed, call, object, &["undici"])
        })
}

fn js_ts_ssrf_direct_module_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    modules: &[&str],
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    js_ts_expr_binds_any_module_at_call(parsed, call, function, modules)
}

fn js_ts_ssrf_method_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expected_method: &str,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let local = parsed.node_text(&function);
        return js_ts_identifier_binds_any_imported_member_at_call(
            parsed,
            call,
            local,
            js_ts_ssrf_method_modules(expected_method),
            expected_method,
        );
    }
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let method = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    if method != expected_method {
        return false;
    }
    let Some(receiver) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_expr_binds_any_module_at_call(
        parsed,
        call,
        receiver,
        js_ts_ssrf_method_modules(expected_method),
    ) || js_ts_expr_is_ssrf_client_factory_call(parsed, call, receiver)
        || js_ts_expr_is_ssrf_factory_client_binding(parsed, call, receiver)
}

fn js_ts_ssrf_method_modules(method: &str) -> &'static [&'static str] {
    match method {
        "get" | "post" | "request" => &[
            "axios",
            "got",
            "superagent",
            "http",
            "node:http",
            "https",
            "node:https",
            "undici",
        ],
        _ => &[],
    }
}

fn js_ts_identifier_binds_any_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    modules: &[&str],
) -> bool {
    modules
        .iter()
        .any(|module| js_ts_identifier_binds_module_at_call(parsed, call, local_name, module))
}

fn js_ts_identifier_binds_any_imported_member_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    modules: &[&str],
    imported_member: &str,
) -> bool {
    modules.iter().any(|module| {
        js_ts_identifier_binds_imported_member_at_call(
            parsed,
            call,
            local_name,
            module,
            imported_member,
        )
    })
}

fn js_ts_expr_binds_any_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expr: Node<'_>,
    modules: &[&str],
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() == "identifier" {
        return js_ts_identifier_binds_any_module_at_call(
            parsed,
            call,
            parsed.node_text(&expr),
            modules,
        );
    }
    if let Some(module) = js_ts_require_call_module(parsed, &expr) {
        return modules.iter().any(|expected| module == *expected);
    }
    false
}

fn js_ts_expr_is_ssrf_client_factory_call(
    parsed: &ParsedFile,
    context_call: &Node<'_>,
    expr: Node<'_>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if !parsed.language.is_call_node(expr.kind()) {
        return false;
    }
    let Some(function) = expr.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let factory = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    match factory {
        "create" => js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["axios"]),
        "extend" => js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["got"]),
        "agent" => {
            js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["superagent"])
        }
        _ => false,
    }
}

fn js_ts_expr_is_ssrf_factory_client_binding(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expr: Node<'_>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() != "identifier" {
        return false;
    }
    let local_name = parsed.node_text(&expr);
    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && decl.child_by_field_name("name").is_some_and(|name| {
                name.kind() == "identifier" && parsed.node_text(&name) == local_name
            })
            && js_ts_assignment_visible_before_context(parsed, call, decl)
            && decl
                .child_by_field_name("value")
                .is_some_and(|value| js_ts_expr_is_ssrf_client_factory_call(parsed, call, value))
    })
}

fn python_is_aiohttp_request_call(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let imports = parsed.extract_imports();
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let name = parsed.node_text(&function);
        return name == "request" && python_imports_resolve_to_module(&imports, name, "aiohttp");
    }
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != "request" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    python_expression_resolves_to_module(parsed, &imports, object, "aiohttp")
}

fn python_is_aiohttp_client_session_method_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expected_method: &str,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != expected_method {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let object = unwrap_parenthesized(object);
    if python_is_aiohttp_client_session_constructor_call(parsed, object) {
        return true;
    }
    if object.kind() != "identifier" {
        return false;
    }
    let receiver = parsed.node_text(&object);
    python_aiohttp_client_session_vars(parsed, call.start_position().row + 1).contains(receiver)
}

fn python_is_urllib3_pool_manager_request_call(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != "request" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let object = unwrap_parenthesized(object);
    if python_is_urllib3_pool_manager_constructor_call(parsed, object) {
        return true;
    }
    if object.kind() != "identifier" {
        return false;
    }
    let receiver = parsed.node_text(&object);
    python_urllib3_pool_manager_vars(parsed, call.start_position().row + 1).contains(receiver)
}

fn python_aiohttp_client_session_vars(parsed: &ParsedFile, sink_line: usize) -> BTreeSet<String> {
    python_constructor_receiver_vars(parsed, sink_line, "aiohttp", "ClientSession", true)
}

fn python_urllib3_pool_manager_vars(parsed: &ParsedFile, sink_line: usize) -> BTreeSet<String> {
    python_constructor_receiver_vars(parsed, sink_line, "urllib3", "PoolManager", false)
}

fn python_constructor_receiver_vars(
    parsed: &ParsedFile,
    sink_line: usize,
    module_name: &str,
    constructor_name: &str,
    include_with_aliases: bool,
) -> BTreeSet<String> {
    let Some(func_node) = parsed.enclosing_function(sink_line) else {
        return BTreeSet::new();
    };
    let imports = parsed.extract_imports();
    let mut names = BTreeSet::new();

    let mut assignments = Vec::new();
    collect_assignments(func_node, parsed, &mut assignments);
    for assignment in assignments {
        if assignment.start_position().row + 1 > sink_line {
            continue;
        }
        let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&assignment),
            parsed.language.assignment_value(&assignment),
        ) else {
            continue;
        };
        let lhs_items = python_assignment_items(lhs);
        let rhs_items = python_assignment_items(rhs);
        if lhs_items.len() == rhs_items.len() && lhs_items.len() > 1 {
            for (lhs_item, rhs_item) in lhs_items.into_iter().zip(rhs_items) {
                if python_is_constructor_call_from_module(
                    parsed,
                    &imports,
                    rhs_item,
                    module_name,
                    constructor_name,
                ) {
                    collect_bare_identifier_name(parsed, lhs_item, &mut names);
                }
            }
        } else if python_is_constructor_call_from_module(
            parsed,
            &imports,
            rhs,
            module_name,
            constructor_name,
        ) {
            collect_bare_identifier_name(parsed, lhs, &mut names);
        }
    }

    if include_with_aliases {
        collect_constructor_with_aliases_from_with(
            parsed,
            &imports,
            func_node,
            sink_line,
            module_name,
            constructor_name,
            &mut names,
        );
    }
    names
}

fn collect_bare_identifier_name(parsed: &ParsedFile, node: Node<'_>, names: &mut BTreeSet<String>) {
    let node = unwrap_parenthesized(node);
    if node.kind() == "identifier" {
        names.insert(parsed.node_text(&node).to_string());
    }
}

fn collect_constructor_with_aliases_from_with(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    sink_line: usize,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    if node.start_position().row + 1 > sink_line {
        return;
    }
    if node.kind() == "with_statement" {
        collect_constructor_as_pattern_aliases(
            parsed,
            imports,
            node,
            module_name,
            constructor_name,
            names,
        );
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_constructor_with_aliases_from_with(
            parsed,
            imports,
            child,
            sink_line,
            module_name,
            constructor_name,
            names,
        );
    }
}

fn collect_constructor_as_pattern_aliases(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    with_node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    let header_end = with_node
        .child_by_field_name("body")
        .map(|body| body.start_byte())
        .unwrap_or(with_node.end_byte());
    collect_constructor_as_patterns_before_body(
        parsed,
        imports,
        with_node,
        header_end,
        module_name,
        constructor_name,
        names,
    );
}

fn collect_constructor_as_patterns_before_body(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    header_end: usize,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    if node.start_byte() >= header_end {
        return;
    }
    if node.kind() == "as_pattern" {
        if let Some(alias) =
            constructor_as_pattern_alias(parsed, imports, node, module_name, constructor_name)
        {
            names.insert(alias);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_constructor_as_patterns_before_body(
            parsed,
            imports,
            child,
            header_end,
            module_name,
            constructor_name,
            names,
        );
    }
}

fn constructor_as_pattern_alias(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    as_pattern: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> Option<String> {
    let mut alias = None;
    let mut has_constructor = false;
    let mut cursor = as_pattern.walk();
    for child in as_pattern.children(&mut cursor) {
        if child.kind() == "as_pattern_target" {
            let name = parsed.node_text(&child).trim();
            if is_python_identifier(name) {
                alias = Some(name.to_string());
            }
            continue;
        }
        if node_contains_constructor_call_from_module(
            parsed,
            imports,
            child,
            module_name,
            constructor_name,
        ) {
            has_constructor = true;
        }
    }
    has_constructor.then_some(alias).flatten()
}

fn node_contains_constructor_call_from_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> bool {
    if python_is_constructor_call_from_module(parsed, imports, node, module_name, constructor_name)
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_constructor_call_from_module(
            parsed,
            imports,
            child,
            module_name,
            constructor_name,
        ) {
            return true;
        }
    }
    false
}

fn python_is_aiohttp_client_session_constructor_call(parsed: &ParsedFile, node: Node<'_>) -> bool {
    let imports = parsed.extract_imports();
    python_is_constructor_call_from_module(parsed, &imports, node, "aiohttp", "ClientSession")
}

fn python_is_urllib3_pool_manager_constructor_call(parsed: &ParsedFile, node: Node<'_>) -> bool {
    let imports = parsed.extract_imports();
    python_is_constructor_call_from_module(parsed, &imports, node, "urllib3", "PoolManager")
}

fn python_is_constructor_call_from_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> bool {
    let node = unwrap_parenthesized(node);
    if node.kind() != "call" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let callee = parsed.node_text(&function).trim();
    let (namespace, basename) = match callee.rsplit_once('.') {
        Some((ns, name)) => (Some(ns.trim()), name.trim()),
        None => (None, callee),
    };
    if basename != constructor_name {
        return false;
    }
    match namespace {
        Some(ns) => python_expression_text_resolves_to_module(imports, ns, module_name),
        None => python_imports_resolve_to_module(imports, basename, module_name),
    }
}

fn is_python_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn python_expression_resolves_to_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
) -> bool {
    let node = unwrap_parenthesized(node);
    if node.kind() != "identifier" && node.kind() != "attribute" {
        return false;
    }
    python_expression_text_resolves_to_module(imports, parsed.node_text(&node).trim(), module_name)
}

fn python_expression_text_resolves_to_module(
    imports: &BTreeMap<String, String>,
    text: &str,
    module_name: &str,
) -> bool {
    let head = text.split('.').next().unwrap_or(text);
    python_imports_resolve_to_module(imports, head, module_name)
}

fn python_imports_resolve_to_module(
    imports: &BTreeMap<String, String>,
    name: &str,
    module_name: &str,
) -> bool {
    imports
        .get(name)
        .is_some_and(|module| python_module_matches(module, module_name))
}

fn python_module_matches(module: &str, expected: &str) -> bool {
    module == expected || module.starts_with(&format!("{}.", expected))
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

fn js_ts_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    if let Some(outcome) = js_ts_dangerously_set_inner_html_outcome(parsed, line, path) {
        return outcome;
    }

    let mut any_call_path_match = false;
    for pat in JS_CWE79_SINKS
        .iter()
        .chain(JS_CWE89_SINKS.iter())
        .chain(JS_CWE918_SINKS.iter())
        .chain(JS_CWE502_SINKS.iter())
        .chain(JS_CWE78_SINKS.iter())
        .chain(JS_CWE22_SINKS.iter())
    {
        if pat.call_path == "dangerouslySetInnerHTML" {
            continue;
        }
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    if any_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

fn js_ts_dangerously_set_inner_html_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> Option<SinkMatchOutcome> {
    let pat = JS_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "dangerouslySetInnerHTML")?;
    let mut attrs = Vec::new();
    collect_nodes_of_kind(parsed.tree.root_node(), "jsx_attribute", &mut attrs);
    let mut had_attr = false;
    for attr in &attrs {
        if !node_contains_line(attr, line) {
            continue;
        }
        let text = parsed.node_text(attr);
        if !text.contains("dangerouslySetInnerHTML") || !text.contains("__html") {
            continue;
        }
        had_attr = true;
        if let Some(p) = path {
            if arg_node_taints_match(parsed, attr, p) {
                return Some(SinkMatchOutcome::Match(pat));
            }
        } else {
            return Some(SinkMatchOutcome::Match(pat));
        }
    }
    if had_attr {
        Some(SinkMatchOutcome::SemanticallyExcluded)
    } else {
        None
    }
}

fn collect_nodes_of_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_nodes_of_kind(child, kind, out);
    }
}

fn python_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    if let Some(outcome) = python_render_template_string_outcome(parsed, line, path) {
        return outcome;
    }

    let mut any_call_path_match = false;
    for pat in PY_CWE79_SINKS
        .iter()
        .chain(PY_CWE89_SINKS.iter())
        .chain(PY_CWE918_SINKS.iter())
        .chain(PY_CWE502_SINKS.iter())
    {
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    if any_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

fn python_render_template_string_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> Option<SinkMatchOutcome> {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let pat = PY_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "render_template_string")?;
    let mut had_call = false;
    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = call_path_text(parsed, call)?;
        if !call_path_matches(parsed, &actual, "render_template_string") {
            continue;
        }
        had_call = true;
        if let Some(p) = path {
            if arg_is_tainted_in_path(parsed, call, 0, p) {
                return Some(SinkMatchOutcome::Match(pat));
            }
        }
        let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
        let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
        if unsafe_vars.is_empty() && !autoescape_disabled {
            continue;
        }
        if path.is_none() {
            return Some(SinkMatchOutcome::Match(pat));
        }
        if python_render_tainted_context_matches(
            parsed,
            call,
            path?,
            &unsafe_vars,
            autoescape_disabled,
        ) {
            return Some(SinkMatchOutcome::Match(pat));
        }
    }
    if had_call {
        Some(SinkMatchOutcome::SemanticallyExcluded)
    } else {
        None
    }
}

fn python_render_unsafe_template_vars(parsed: &ParsedFile, call: &Node<'_>) -> BTreeSet<String> {
    let template = match call_literal_arg(parsed, call, 0) {
        Some(s) => s,
        None => return BTreeSet::new(),
    };
    let mut vars = BTreeSet::new();
    for part in template.split("{{").skip(1) {
        let expr = part.split("}}").next().unwrap_or(part);
        if !expr.contains("| safe") && !expr.contains("|safe") {
            continue;
        }
        let name = expr
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() {
            vars.insert(name.to_string());
        }
    }
    vars
}

fn python_render_autoescape_disabled(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    call_literal_arg(parsed, call, 0).is_some_and(|s| {
        let compact: String = s
            .to_ascii_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        compact.contains("{%autoescapefalse%}")
    })
}

fn python_render_tainted_context_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    path: &FlowPath,
    unsafe_vars: &BTreeSet<String>,
    autoescape_disabled: bool,
) -> bool {
    let args = match call.child_by_field_name("arguments") {
        Some(a) => a,
        None => return false,
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        if child.kind() != "keyword_argument" {
            continue;
        }
        let key = match child.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).to_string(),
            None => continue,
        };
        if !autoescape_disabled && !unsafe_vars.contains(&key) {
            continue;
        }
        let value = child
            .child_by_field_name("value")
            .or_else(|| child.named_child(1));
        if let Some(v) = value {
            if arg_node_taints_match(parsed, &v, path) {
                return true;
            }
        }
    }
    false
}

fn python_sink_with_inline_flask_source(
    parsed: &ParsedFile,
    line: usize,
) -> Option<&'static SinkPattern> {
    if parsed.language != Language::Python {
        return None;
    }

    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let render_pat = PY_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "render_template_string");

    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = match call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if call_path_matches(parsed, &actual, "render_template_string") {
            if let Some(pat) = render_pat {
                if python_render_inline_flask_source_matches(parsed, call) {
                    return Some(pat);
                }
            }
            continue;
        }

        for pat in PY_CWE79_SINKS
            .iter()
            .chain(PY_CWE89_SINKS.iter())
            .chain(PY_CWE918_SINKS.iter())
            .chain(PY_CWE502_SINKS.iter())
        {
            if !sink_call_path_matches(parsed, call, &actual, pat) {
                continue;
            }
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            if pat.category == SanitizerCategory::Sqli
                && python_sql_call_is_parametrized(parsed, call)
            {
                continue;
            }
            if pat.category == SanitizerCategory::Deserialization
                && python_yaml_load_uses_safe_loader(parsed, call)
            {
                continue;
            }
            if pat.tainted_arg_indices.iter().any(|&idx| {
                call_arg_node(call, idx)
                    .is_some_and(|arg| node_contains_flask_request_data_access(parsed, arg))
            }) {
                return Some(pat);
            }
        }
    }
    None
}

fn python_render_inline_flask_source_matches(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
    let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
    if unsafe_vars.is_empty() && !autoescape_disabled {
        return false;
    }

    let args = match call.child_by_field_name("arguments") {
        Some(a) => a,
        None => return false,
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        if child.kind() != "keyword_argument" {
            continue;
        }
        let key = match child.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).to_string(),
            None => continue,
        };
        if !autoescape_disabled && !unsafe_vars.contains(&key) {
            continue;
        }
        let value = child
            .child_by_field_name("value")
            .or_else(|| child.named_child(1));
        if value.is_some_and(|v| node_contains_flask_request_data_access(parsed, v)) {
            return true;
        }
    }
    false
}

fn python_sql_call_is_parametrized(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "execute")
        && !call_path_matches(parsed, &actual, "executemany")
    {
        return false;
    }

    if let Some(query) = call_literal_arg(parsed, call, 0) {
        return python_sql_literal_has_placeholder(&query) && call_has_arg_after(call, 0);
    }

    let arg0 = match call_arg_node(call, 0) {
        Some(n) => n,
        None => return false,
    };
    let arg0_text = parsed.node_text(&arg0);
    if !(arg0_text.contains(".bindparams(") || arg0_text.contains(".params(")) {
        return false;
    }
    let Some(query) = first_string_literal_text(parsed, &arg0) else {
        return false;
    };
    arg0_text.contains("text(") && python_sql_literal_has_named_placeholder(&query)
}

fn python_sql_literal_has_placeholder(query: &str) -> bool {
    query.contains("%s") || query.contains('?') || python_sql_literal_has_named_placeholder(query)
}

fn python_sql_literal_has_named_placeholder(query: &str) -> bool {
    let bytes = query.as_bytes();
    bytes.iter().enumerate().any(|(idx, b)| {
        *b == b':'
            && bytes
                .get(idx + 1)
                .is_some_and(|next| (*next as char).is_ascii_alphabetic() || *next == b'_')
    })
}

fn call_has_arg_after(call: &Node<'_>, arg_idx: usize) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    arguments.named_children(&mut cursor).count() > arg_idx + 1
}

fn first_string_literal_text(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let text = parsed.node_text(node).trim();
    if node.kind() == "string"
        || node.kind() == "interpreted_string_literal"
        || node.kind() == "raw_string_literal"
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('r')
        || text.starts_with('R')
        || text.starts_with('u')
        || text.starts_with('U')
        || text.starts_with('b')
        || text.starts_with('B')
    {
        let quote_idx = text.find(['"', '\'']).unwrap_or(0);
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
            .unwrap_or(without_prefix);
        return Some(trimmed.to_string());
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(text) = first_string_literal_text(parsed, &child) {
            return Some(text);
        }
    }
    None
}

fn python_yaml_load_uses_safe_loader(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "yaml.load")
        && !call_path_matches(parsed, &actual, "load")
    {
        return false;
    }
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if idx == 0 {
            continue;
        }
        let text = parsed.node_text(&arg);
        if text.contains("SafeLoader") || text.contains("CSafeLoader") {
            return true;
        }
    }
    false
}

fn js_ts_exec_file_is_literal_binary(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "execFile")
        && !call_path_matches(parsed, &actual, "execFileSync")
    {
        return false;
    }
    let Some(binary) = call_literal_arg(parsed, call, 0) else {
        return false;
    };
    match js_ts_literal_binary_kind(&binary) {
        JsTsLiteralBinaryKind::Shell => return false,
        JsTsLiteralBinaryKind::Interpreter => {
            if !js_ts_exec_file_interpreter_argv_is_inspectably_safe(parsed, call) {
                return false;
            }
        }
        JsTsLiteralBinaryKind::Other => {}
    }
    !js_ts_exec_file_shell_option_is_unsafe(parsed, call)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JsTsLiteralBinaryKind {
    Shell,
    Interpreter,
    Other,
}

fn js_ts_literal_binary_kind(binary: &str) -> JsTsLiteralBinaryKind {
    let basename = binary
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(binary)
        .to_ascii_lowercase();
    if matches!(
        basename.as_str(),
        "sh" | "bash"
            | "dash"
            | "zsh"
            | "ksh"
            | "csh"
            | "fish"
            | "cmd"
            | "cmd.exe"
            | "pwsh"
            | "powershell"
            | "powershell.exe"
    ) {
        return JsTsLiteralBinaryKind::Shell;
    }
    if matches!(
        basename.as_str(),
        "node" | "node.exe" | "python" | "python3" | "python.exe" | "perl" | "ruby" | "php"
    ) {
        return JsTsLiteralBinaryKind::Interpreter;
    }
    JsTsLiteralBinaryKind::Other
}

fn js_ts_exec_file_interpreter_argv_is_inspectably_safe(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> bool {
    let Some(argv) = call_arg_node(call, 1) else {
        return true;
    };
    let argv = unwrap_parenthesized(argv);
    if argv.kind() != "array" {
        return false;
    }
    let mut cursor = argv.walk();
    for arg in argv.named_children(&mut cursor) {
        let Some(value) = js_ts_literal_string_value(parsed, &arg) else {
            return false;
        };
        if js_ts_interpreter_eval_flag(&value) {
            return false;
        }
    }
    true
}

fn js_ts_exec_file_shell_option_is_unsafe(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    (1..=3).any(|arg_idx| match call_arg_node(call, arg_idx) {
        Some(arg) if arg_idx == 1 && js_ts_exec_file_arg_is_argv_or_callback(&arg) => false,
        Some(arg) if js_ts_exec_file_arg_is_callback(&arg) => false,
        Some(arg) => js_ts_node_has_unsafe_shell_option(parsed, &arg, call),
        None => false,
    })
}

fn js_ts_exec_file_arg_is_argv_or_callback(arg: &Node<'_>) -> bool {
    let arg = unwrap_parenthesized(*arg);
    arg.kind() == "array" || js_ts_exec_file_arg_is_callback(&arg)
}

fn js_ts_exec_file_arg_is_callback(arg: &Node<'_>) -> bool {
    let arg = unwrap_parenthesized(*arg);
    matches!(
        arg.kind(),
        "function" | "function_expression" | "arrow_function"
    )
}

fn js_ts_node_has_unsafe_shell_option(
    parsed: &ParsedFile,
    node: &Node<'_>,
    context: &Node<'_>,
) -> bool {
    let node = unwrap_parenthesized(*node);
    let text = parsed.node_text(&node);
    if text.trim_start().starts_with('{') && js_ts_object_text_has_unsafe_shell_option(text) {
        return true;
    }
    if node.kind() != "identifier" {
        return true;
    }
    js_ts_identifier_bound_to_unsafe_shell_options(parsed, parsed.node_text(&node), context)
}

fn js_ts_identifier_bound_to_unsafe_shell_options(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_inspectable_safe_options = false;
    let mut saw_module_lifetime_binding = false;
    for assignment in assignments {
        if !js_ts_assignment_visible_before_context(parsed, context, &assignment) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == var_name)
        {
            continue;
        }
        let rhs_text = parsed.node_text(&rhs);
        if !rhs_text.trim_start().starts_with('{') {
            return true;
        }
        if js_ts_object_text_has_unsafe_shell_option(rhs_text) {
            return true;
        }
        saw_inspectable_safe_options = true;
        if js_ts_enclosing_function_id(parsed, &assignment).is_none() {
            saw_module_lifetime_binding = true;
        }
    }
    !saw_inspectable_safe_options
        || js_ts_shell_options_have_unsafe_mutation(
            parsed,
            var_name,
            context,
            saw_module_lifetime_binding,
        )
}

fn js_ts_shell_options_have_unsafe_mutation(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
    include_module_init_after_context: bool,
) -> bool {
    let receiver_names = js_ts_collection_aliases_visible_at_context(
        parsed,
        var_name,
        context,
        include_module_init_after_context,
    );
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);
    for assignment in assignments {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &assignment,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some((lhs, _rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        let lhs_text = parsed.node_text(&lhs);
        if receiver_names.iter().any(|name| {
            lhs_text.starts_with(&format!("{name}.shell"))
                || lhs_text.starts_with(&format!("{name}["))
        }) {
            return true;
        }
    }

    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in calls {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &call,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some(actual) = call_path_text(parsed, &call) else {
            continue;
        };
        if actual != "Object.assign" {
            continue;
        }
        let Some(target) = call_arg_node(&call, 0) else {
            continue;
        };
        let target = unwrap_parenthesized(target);
        if target.kind() == "identifier" && receiver_names.contains(parsed.node_text(&target)) {
            return true;
        }
    }

    false
}

fn js_ts_object_text_has_unsafe_shell_option(text: &str) -> bool {
    let Some(inner) = text
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
    else {
        return true;
    };
    for prop in js_split_top_level_commas(inner) {
        let prop = prop.trim();
        if prop.is_empty() {
            continue;
        }
        if prop.starts_with("...") {
            return true;
        }
        let Some(colon) = js_find_top_level_colon(prop) else {
            if js_ts_colonless_object_property_may_define_key(prop, "shell") {
                return true;
            }
            continue;
        };
        let prop_key_text = prop[..colon].trim();
        if prop_key_text.starts_with('[') {
            return true;
        }
        let prop_key = prop_key_text.trim_matches(['"', '\'', '`']);
        if prop_key == "shell" && prop[colon + 1..].trim() != "false" {
            return true;
        }
    }
    false
}

fn js_ts_colonless_object_property_may_define_key(prop: &str, key: &str) -> bool {
    let prop = prop.trim();
    if prop.starts_with('[') || prop.contains('[') {
        return true;
    }
    let name = prop
        .split_once('(')
        .map(|(head, _)| head.trim())
        .unwrap_or(prop)
        .trim_start_matches("async ")
        .trim_start_matches('*')
        .split_whitespace()
        .last()
        .unwrap_or(prop)
        .trim_matches(['"', '\'', '`']);
    name == key
}

fn js_ts_interpreter_eval_flag(flag: &str) -> bool {
    matches!(
        flag,
        "-c" | "-e" | "-p" | "-r" | "--eval" | "--print" | "--command"
    )
}

fn js_ts_literal_string_value(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let node = unwrap_parenthesized(*node);
    let text = parsed.node_text(&node).trim();
    if !(matches!(node.kind(), "string" | "template_string")
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('`'))
    {
        return None;
    }
    if text.starts_with('`') && text.contains("${") {
        return None;
    }
    text.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .or_else(|| text.strip_prefix('`').and_then(|s| s.strip_suffix('`')))
        .map(ToString::to_string)
}

fn js_ts_yaml_load_uses_safe_schema(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "yaml.load")
        && !js_ts_js_yaml_bare_load_call_matches(parsed, call, &actual)
    {
        return false;
    }
    !js_yaml_load_call_uses_unsafe_schema(parsed, call)
}

fn js_ts_sql_call_is_parametrized(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !["query", "execute"]
        .iter()
        .any(|expected| call_path_matches(parsed, &actual, expected))
    {
        return false;
    }
    let text = parsed.node_text(call);
    (text.contains("bind") || text.contains("parameters"))
        && call_literal_arg(parsed, call, 0)
            .as_deref()
            .is_some_and(js_sql_literal_has_placeholder)
}

fn js_sql_literal_has_placeholder(query: &str) -> bool {
    query.contains('?')
        || query.contains("$1")
        || query.contains("@")
        || python_sql_literal_has_named_placeholder(query)
}

fn js_ts_prisma_tagged_template_is_safe(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    (call_path_matches(parsed, &actual, "$queryRaw")
        || call_path_matches(parsed, &actual, "$executeRaw"))
        && parsed.node_text(call).contains('`')
        && !parsed.node_text(call).contains("Prisma.raw")
}

fn js_ts_ssrf_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if !cpg.has_cfg_edges() {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_js_ts_url_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.url_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.url_var) {
            continue;
        }
        if js_ts_url_guard_safely_controls_sink(parsed, cpg, path, &func_node, &binding, sink_line)
        {
            return true;
        }
    }
    false
}

fn js_ts_path_traversal_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if !cpg.has_cfg_edges() {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_js_ts_path_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.result_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.result_var) {
            continue;
        }
        if js_ts_path_guard_safely_controls_sink(parsed, cpg, path, &func_node, &binding, sink_line)
        {
            return true;
        }
    }
    false
}

fn call_or_constructor_path_text(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    if let Some(path) = call_path_text(parsed, node) {
        return Some(path);
    }
    let constructor = node
        .child_by_field_name("constructor")
        .or_else(|| node.child_by_field_name("function"))?;
    Some(parsed.node_text(&constructor).to_string())
}

fn collect_js_ts_url_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<UrlSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let (lhs, rhs) = match js_ts_assignment_target_and_value(parsed, &assignment) {
            Some(parts) => parts,
            None => continue,
        };
        let actual = match call_or_constructor_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !call_path_matches(parsed, &actual, "URL")
            && !call_path_matches(parsed, &actual, "URL.parse")
            && !call_path_matches(parsed, &actual, "url.parse")
        {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) => name.clone(),
            None => continue,
        };
        let url_arg = match call_arg_node(&rhs, 0) {
            Some(n) => unwrap_parenthesized(n),
            None => continue,
        };
        if url_arg.kind() != "identifier" {
            continue;
        }
        bindings.push(UrlSanitizerBinding {
            url_var: parsed.node_text(&url_arg).to_string(),
            result_var,
            call_line: rhs.start_position().row + 1,
        });
    }
    bindings
}

fn collect_js_ts_path_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<JsTsPathSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let (lhs, rhs) = match js_ts_assignment_target_and_value(parsed, &assignment) {
            Some(parts) => parts,
            None => continue,
        };
        let actual = match call_or_constructor_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !js_ts_path_sanitizer_call_path_matches(parsed, &actual) {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) if name != "_" => name.clone(),
            _ => continue,
        };
        bindings.push(JsTsPathSanitizerBinding {
            result_var,
            call_line: rhs.start_position().row + 1,
        });
    }
    bindings
}

fn collect_js_ts_assignment_like_nodes<'a>(
    node: Node<'a>,
    parsed: &ParsedFile,
    out: &mut Vec<Node<'a>>,
) {
    if node.kind() == "variable_declarator" || parsed.language.is_assignment_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_assignment_like_nodes(child, parsed, out);
    }
}

fn js_ts_assignment_target_and_value<'a>(
    parsed: &ParsedFile,
    node: &Node<'a>,
) -> Option<(Node<'a>, Node<'a>)> {
    Some((
        js_ts_assignment_target(parsed, node)?,
        js_ts_assignment_value(parsed, node)?,
    ))
}

fn js_ts_assignment_target<'a>(parsed: &ParsedFile, node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "variable_declarator" {
        return node.child_by_field_name("name");
    }
    parsed.language.assignment_target(node)
}

fn js_ts_assignment_value<'a>(parsed: &ParsedFile, node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "variable_declarator" {
        return node.child_by_field_name("value");
    }
    parsed.language.assignment_value(node)
}

fn js_ts_path_sanitizer_call_path_matches(parsed: &ParsedFile, actual: &str) -> bool {
    [
        "path.resolve",
        "path.normalize",
        "path.relative",
        "fs.realpathSync",
        "fs.promises.realpath",
        "realpath",
        "realpathSync",
    ]
    .iter()
    .any(|expected| call_path_matches(parsed, actual, expected))
}

fn js_ts_url_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    func_node: &Node<'_>,
    binding: &UrlSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_js_ts_url_guard(parsed, &condition, binding, path) {
            Some(c) => c,
            None => continue,
        };
        if js_ts_guard_safely_controls_sink(parsed, cpg, &guard, control, sink_line) {
            return true;
        }
    }
    false
}

fn js_ts_path_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    func_node: &Node<'_>,
    binding: &JsTsPathSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_js_ts_path_guard(parsed, &condition, binding, path) {
            Some(c) => c,
            None => continue,
        };
        if js_ts_guard_safely_controls_sink(parsed, cpg, &guard, control, sink_line) {
            return true;
        }
    }
    false
}

fn classify_js_ts_url_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &UrlSanitizerBinding,
    path: &FlowPath,
) -> Option<GuardControl> {
    let (call, negated) = js_ts_single_guard_call(parsed, condition)?;
    let actual = call_path_text(parsed, &call)?;
    let (receiver, method) = actual.rsplit_once('.')?;
    if !matches!(method, "includes" | "has")
        || !js_ts_allowlist_receiver_is_trusted(parsed, receiver, path, condition)
        || !call_arg_node(&call, 0)
            .is_some_and(|arg| js_ts_node_is_hostname_for_url_binding(parsed, &arg, binding))
    {
        return None;
    }
    if negated {
        Some(GuardControl::RejectBranch)
    } else {
        Some(GuardControl::AllowBranch)
    }
}

fn classify_js_ts_path_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &JsTsPathSanitizerBinding,
    path: &FlowPath,
) -> Option<GuardControl> {
    let (call, negated) = js_ts_single_guard_call(parsed, condition)?;
    let actual = call_path_text(parsed, &call)?;
    let (receiver, method) = actual.rsplit_once('.')?;
    if method != "startsWith"
        || receiver != binding.result_var
        || !call_arg_node(&call, 0)
            .is_some_and(|arg| js_ts_path_prefix_arg_is_trusted(parsed, &arg, path, condition))
    {
        return None;
    }
    if negated {
        Some(GuardControl::RejectBranch)
    } else {
        Some(GuardControl::AllowBranch)
    }
}

fn js_ts_single_guard_call<'a>(
    parsed: &ParsedFile,
    condition: &Node<'a>,
) -> Option<(Node<'a>, bool)> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if condition_text.contains("&&") || condition_text.contains("||") {
        return None;
    }
    if condition.kind() == "unary_expression" && condition_text.trim_start().starts_with('!') {
        let child = unwrap_parenthesized(condition.named_child(0)?);
        return (child.kind() == "call_expression").then_some((child, true));
    }
    (condition.kind() == "call_expression").then_some((condition, false))
}

fn js_ts_allowlist_receiver_is_trusted(
    parsed: &ParsedFile,
    receiver: &str,
    path: &FlowPath,
    guard: &Node<'_>,
) -> bool {
    if !receiver.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }
    let lower = receiver.to_ascii_lowercase();
    if [
        "block",
        "deny",
        "forbid",
        "reject",
        "ban",
        "blacklist",
        "disallow",
        "exclude",
        "invalid",
        "unsafe",
        "untrusted",
        "not_allow",
        "notallow",
        "not_safe",
        "notsafe",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    if !["allow", "whitelist", "trusted", "safe"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return false;
    }
    if path
        .edges
        .iter()
        .any(|edge| edge.to.file == parsed.path && edge.to.var_name() == receiver)
    {
        return false;
    }
    js_ts_identifier_bound_to_literal_collection(parsed, receiver, guard)
}

fn js_ts_identifier_bound_to_literal_collection(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_literal_collection = false;
    let mut saw_module_lifetime_binding = false;
    for assignment in assignments {
        if !js_ts_assignment_visible_before_context(parsed, context, &assignment) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == var_name)
        {
            continue;
        }
        if js_ts_node_is_literal_string_collection(parsed, &rhs) {
            saw_literal_collection = true;
            if js_ts_enclosing_function_id(parsed, &assignment).is_none() {
                saw_module_lifetime_binding = true;
            }
        } else {
            return false;
        }
    }
    saw_literal_collection
        && !js_ts_collection_has_untrusted_update(
            parsed,
            var_name,
            context,
            saw_module_lifetime_binding,
        )
}

fn js_ts_node_is_literal_string_collection(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let node = unwrap_parenthesized(*node);
    if node.kind() == "array" {
        let mut cursor = node.walk();
        return node
            .named_children(&mut cursor)
            .all(|child| js_ts_literal_string_value(parsed, &child).is_some());
    }
    let Some(actual) = call_or_constructor_path_text(parsed, &node) else {
        return false;
    };
    if !call_path_matches(parsed, &actual, "Set") {
        return false;
    }
    match call_arg_node(&node, 0) {
        Some(arg) => js_ts_node_is_literal_string_collection(parsed, &arg),
        None => true,
    }
}

fn js_ts_collection_has_untrusted_update(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
    include_module_init_after_context: bool,
) -> bool {
    let receiver_names = js_ts_collection_aliases_visible_at_context(
        parsed,
        var_name,
        context,
        include_module_init_after_context,
    );
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in calls {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &call,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some(actual) = call_path_text(parsed, &call) else {
            continue;
        };
        if actual == "Object.assign" {
            let Some(target) = call_arg_node(&call, 0).map(unwrap_parenthesized) else {
                continue;
            };
            if target.kind() != "identifier" || !receiver_names.contains(parsed.node_text(&target))
            {
                continue;
            }
            let mut source_idx = 1;
            while let Some(source_arg) = call_arg_node(&call, source_idx) {
                if !js_ts_node_is_literal_string_collection(parsed, &source_arg) {
                    return true;
                }
                source_idx += 1;
            }
            continue;
        }
        let Some((receiver, method)) = actual.rsplit_once('.') else {
            continue;
        };
        if !receiver_names.contains(receiver)
            || !matches!(method, "add" | "push" | "unshift" | "splice" | "set")
        {
            continue;
        }
        if !js_ts_call_args_are_literal_strings(parsed, &call) {
            return true;
        }
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);
    for assignment in assignments {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &assignment,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        let lhs_text = parsed.node_text(&lhs);
        if !receiver_names.iter().any(|name| {
            lhs_text.starts_with(&format!("{name}[")) || lhs_text.starts_with(&format!("{name}."))
        }) {
            continue;
        }
        if js_ts_literal_string_value(parsed, &rhs).is_none() {
            return true;
        }
    }

    false
}

fn js_ts_collection_aliases_visible_at_context(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
    include_module_init_after_context: bool,
) -> BTreeSet<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut aliases = BTreeSet::new();
    aliases.insert(var_name.to_string());

    let mut changed = true;
    while changed {
        changed = false;
        for assignment in &assignments {
            if !js_ts_effect_visible_at_context(
                parsed,
                context,
                assignment,
                include_module_init_after_context,
            ) {
                continue;
            }
            let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, assignment) else {
                continue;
            };
            let rhs = unwrap_parenthesized(rhs);
            if rhs.kind() != "identifier" || !aliases.contains(parsed.node_text(&rhs)) {
                continue;
            }
            for lhs_name in assignment_lhs_identifiers(parsed, &lhs) {
                if lhs_name != "_" && aliases.insert(lhs_name) {
                    changed = true;
                }
            }
        }
    }

    aliases
}

fn js_ts_effect_visible_at_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    effect: &Node<'_>,
    include_module_init_after_context: bool,
) -> bool {
    if effect.start_byte() < context.start_byte() {
        return js_ts_effect_scope_reaches_context(parsed, context, effect);
    }

    include_module_init_after_context
        && js_ts_enclosing_function_id(parsed, effect).is_none()
        && js_ts_enclosing_function_id(parsed, context).is_some()
}

fn js_ts_effect_scope_reaches_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    effect: &Node<'_>,
) -> bool {
    let context_func_id = js_ts_enclosing_function_id(parsed, context);
    let effect_func_id = js_ts_enclosing_function_id(parsed, effect);

    match (effect_func_id, context_func_id) {
        (Some(effect_func_id), Some(context_func_id)) => effect_func_id == context_func_id,
        (None, _) => true,
        _ => false,
    }
}

fn js_ts_call_args_are_literal_strings(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(args) = call.child_by_field_name("arguments") else {
        return true;
    };
    let mut cursor = args.walk();
    let all_literal = args
        .named_children(&mut cursor)
        .all(|arg| js_ts_literal_string_value(parsed, &arg).is_some());
    all_literal
}

fn js_ts_node_is_hostname_for_url_binding(
    parsed: &ParsedFile,
    node: &Node<'_>,
    binding: &UrlSanitizerBinding,
) -> bool {
    let node = unwrap_parenthesized(*node);
    parsed.node_text(&node).trim() == format!("{}.hostname", binding.result_var)
}

fn js_ts_path_prefix_arg_is_trusted(
    parsed: &ParsedFile,
    node: &Node<'_>,
    path: &FlowPath,
    guard: &Node<'_>,
) -> bool {
    let node = unwrap_parenthesized(*node);
    if let Some(prefix) = js_ts_literal_string_value(parsed, &node) {
        return js_ts_path_prefix_value_has_boundary(&prefix);
    }
    if node.kind() != "identifier" {
        return false;
    }
    let name = parsed.node_text(&node);
    if path
        .edges
        .iter()
        .any(|edge| edge.to.file == parsed.path && edge.to.var_name() == name)
    {
        return false;
    }
    js_ts_identifier_literal_string_before(parsed, name, guard)
        .is_some_and(|prefix| js_ts_path_prefix_value_has_boundary(&prefix))
}

fn js_ts_path_prefix_value_has_boundary(prefix: &str) -> bool {
    let prefix = prefix.trim();
    if matches!(prefix, "/" | "\\") {
        return false;
    }
    let bytes = prefix.as_bytes();
    if bytes.len() == 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
    {
        return false;
    }
    prefix.ends_with('/') || prefix.ends_with('\\')
}

fn js_ts_identifier_literal_string_before(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> Option<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut literal = None;
    for assignment in assignments {
        if !js_ts_assignment_visible_before_context(parsed, context, &assignment) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == var_name)
        {
            continue;
        }
        literal = Some(js_ts_literal_string_value(parsed, &rhs)?);
    }
    literal
}

fn js_ts_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    guard: &Node<'_>,
    control: GuardControl,
    sink_line: usize,
) -> bool {
    let consequence = match guard.child_by_field_name("consequence") {
        Some(n) => n,
        None => return false,
    };
    let consequence_entry = match first_statement_line(parsed, &consequence) {
        Some(line) => line,
        None => return false,
    };

    match control {
        GuardControl::RejectBranch => {
            if !block_ends_with_return(parsed, &consequence) {
                return false;
            }
            let safe_entry = match safe_successor_line(cpg, parsed, guard, consequence_entry) {
                Some(line) => line,
                None => return false,
            };
            cfg_line_reaches(cpg, &parsed.path, safe_entry, sink_line)
                && !cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
        }
        GuardControl::AllowBranch => {
            node_contains_line(&consequence, sink_line)
                && cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
        }
    }
}

fn python_ssrf_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if !cpg.has_cfg_edges() {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_url_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.url_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.url_var) {
            continue;
        }
        if python_url_guard_safely_controls_sink(parsed, cpg, &func_node, &binding, sink_line) {
            return true;
        }
    }
    false
}

fn collect_url_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<UrlSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_assignments(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let lhs = match parsed.language.assignment_target(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let rhs = match parsed.language.assignment_value(&assignment) {
            Some(n) => n,
            None => continue,
        };
        if rhs.kind() != "call" && rhs.kind() != "call_expression" {
            continue;
        }
        let actual = match call_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !call_path_matches(parsed, &actual, "urlparse")
            && !call_path_matches(parsed, &actual, "urllib.parse.urlparse")
        {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) => name.clone(),
            None => continue,
        };
        let url_arg = match call_arg_node(&rhs, 0) {
            Some(n) => n,
            None => continue,
        };
        if url_arg.kind() != "identifier" {
            continue;
        }
        bindings.push(UrlSanitizerBinding {
            url_var: parsed.node_text(&url_arg).to_string(),
            result_var,
            call_line: rhs.start_position().row + 1,
        });
    }
    bindings
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

fn python_url_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    func_node: &Node<'_>,
    binding: &UrlSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_python_url_guard(parsed, &condition, binding) {
            Some(c) => c,
            None => continue,
        };
        let consequence = match guard.child_by_field_name("consequence") {
            Some(n) => n,
            None => continue,
        };
        let consequence_entry = match first_statement_line(parsed, &consequence) {
            Some(line) => line,
            None => continue,
        };

        match control {
            GuardControl::RejectBranch => {
                if !block_ends_with_return(parsed, &consequence) {
                    continue;
                }
                let safe_entry = match safe_successor_line(cpg, parsed, &guard, consequence_entry) {
                    Some(line) => line,
                    None => continue,
                };
                if cfg_line_reaches(cpg, &parsed.path, safe_entry, sink_line)
                    && !cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
            GuardControl::AllowBranch => {
                if node_contains_line(&consequence, sink_line)
                    && cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
        }
    }

    false
}

fn classify_python_url_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &UrlSanitizerBinding,
) -> Option<GuardControl> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if !python_url_condition_targets_binding(condition_text, binding) {
        return None;
    }
    if condition_text.contains(" not in ") {
        Some(GuardControl::RejectBranch)
    } else if condition_text.contains(" in ") {
        Some(GuardControl::AllowBranch)
    } else {
        None
    }
}

fn python_url_condition_targets_binding(
    condition_text: &str,
    binding: &UrlSanitizerBinding,
) -> bool {
    let parsed_host = format!("{}.hostname", binding.result_var);
    let direct_urlparse = format!("urlparse({}).hostname", binding.url_var);
    let qualified_urlparse = format!("urllib.parse.urlparse({}).hostname", binding.url_var);
    condition_text.contains(&parsed_host)
        || condition_text.contains(&direct_urlparse)
        || condition_text.contains(&qualified_urlparse)
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

fn is_js_yaml_load_sink_pattern(parsed: &ParsedFile, sink_pat: &'static SinkPattern) -> bool {
    is_js_ts_language(parsed.language)
        && sink_pat.category == SanitizerCategory::Deserialization
        && matches!(sink_pat.call_path, "yaml.load" | "load")
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

fn js_ts_safe_structured_sink_call_ranges(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let mut ranges = Vec::new();
    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = match call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        for pat in JS_CWE79_SINKS
            .iter()
            .chain(JS_CWE89_SINKS.iter())
            .chain(JS_CWE918_SINKS.iter())
            .chain(JS_CWE502_SINKS.iter())
            .chain(JS_CWE78_SINKS.iter())
            .chain(JS_CWE22_SINKS.iter())
        {
            if pat.call_path == "dangerouslySetInnerHTML" {
                continue;
            }
            if !sink_call_path_matches(parsed, call, &actual, pat) {
                continue;
            }
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            let tainted_arg = sink_call_has_tainted_arg_in_path(parsed, call, pat, path);
            if !tainted_arg {
                if let Some(function) = call.child_by_field_name("function") {
                    ranges.push((function.start_byte(), function.end_byte()));
                } else {
                    ranges.push((call.start_byte(), call.end_byte()));
                }
                break;
            }
            if flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pat, call) {
                ranges.push((call.start_byte(), call.end_byte()));
                break;
            }
        }

        if js_ts_exec_file_is_literal_binary(parsed, call)
            || js_ts_yaml_load_uses_safe_schema(parsed, call)
            || js_ts_sql_call_is_parametrized(parsed, call)
            || js_ts_prisma_tagged_template_is_safe(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
    }

    let mut attrs = Vec::new();
    collect_nodes_of_kind(parsed.tree.root_node(), "jsx_attribute", &mut attrs);
    for attr in &attrs {
        if node_contains_line(attr, line)
            && parsed.node_text(attr).contains("dangerouslySetInnerHTML")
            && path.cleansed_for.contains(&SanitizerCategory::Xss)
        {
            ranges.push((attr.start_byte(), attr.end_byte()));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn python_safe_structured_sink_call_ranges(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let mut ranges = Vec::new();
    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = match call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        for pat in PY_CWE79_SINKS
            .iter()
            .chain(PY_CWE89_SINKS.iter())
            .chain(PY_CWE918_SINKS.iter())
            .chain(PY_CWE502_SINKS.iter())
        {
            if !sink_call_path_matches(parsed, call, &actual, pat) {
                continue;
            }
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            if sink_call_has_tainted_arg_in_path(parsed, call, pat, path)
                && flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pat, call)
            {
                ranges.push((call.start_byte(), call.end_byte()));
                break;
            }
        }
        if call_path_matches(parsed, &actual, "render_template_string") {
            let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
            let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
            if (unsafe_vars.is_empty() && !autoescape_disabled)
                || !python_render_tainted_context_matches(
                    parsed,
                    call,
                    path,
                    &unsafe_vars,
                    autoescape_disabled,
                )
            {
                ranges.push((call.start_byte(), call.end_byte()));
            }
        }
        if call_path_matches(parsed, &actual, "execute")
            && python_sql_call_is_parametrized(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if call_path_matches(parsed, &actual, "executemany")
            && python_sql_call_is_parametrized(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if python_yaml_load_uses_safe_loader(parsed, call) {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if call_path_matches(parsed, &actual, "format_html")
            && call_literal_arg(parsed, call, 0).is_some()
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn node_in_ranges(node: &Node<'_>, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= node.start_byte() && node.end_byte() <= *end)
}

fn node_contains_node(outer: &Node<'_>, inner: &Node<'_>) -> bool {
    outer.start_byte() <= inner.start_byte() && inner.end_byte() <= outer.end_byte()
}

fn node_contains_range(node: &Node<'_>, start_byte: usize, end_byte: usize) -> bool {
    node.start_byte() <= start_byte && end_byte <= node.end_byte()
}

fn node_contains_any_range(node: &Node<'_>, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| node_contains_range(node, *start, *end))
}

fn call_arguments_node<'a>(call: &Node<'a>) -> Option<Node<'a>> {
    call.child_by_field_name("arguments").or_else(|| {
        let mut cursor = call.walk();
        let found = call
            .named_children(&mut cursor)
            .find(|child| child.kind() == "arguments");
        found
    })
}

fn js_ts_call_has_tainted_or_source_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    let Some(arguments) = call_arguments_node(call) else {
        return false;
    };
    let mut cursor = arguments.walk();
    let has_tainted_arg = arguments.named_children(&mut cursor).any(|arg| {
        arg_node_taints_match(parsed, &arg, path) || node_contains_any_range(&arg, source_ranges)
    });
    has_tainted_arg
}

fn js_ts_enclosing_call_for_function_identifier<'a>(
    parsed: &ParsedFile,
    id: &Node<'a>,
) -> Option<Node<'a>> {
    let mut current = id.parent();
    while let Some(parent) = current {
        if parsed.language.is_call_node(parent.kind()) {
            return parent
                .child_by_field_name("function")
                .filter(|function| node_contains_node(function, id))
                .map(|_| parent);
        }
        current = parent.parent();
    }
    None
}

fn js_ts_enclosing_assignment_for_lhs_identifier<'a>(
    parsed: &ParsedFile,
    id: &Node<'a>,
) -> Option<Node<'a>> {
    let mut current = Some(*id);
    while let Some(node) = current {
        if node.kind() == "variable_declarator" || parsed.language.is_assignment_node(node.kind()) {
            return js_ts_assignment_target(parsed, &node)
                .filter(|lhs| node_contains_node(lhs, id))
                .map(|_| node);
        }
        current = node.parent();
    }
    None
}

fn js_ts_assignment_has_tainted_or_source_rhs(
    parsed: &ParsedFile,
    assignment: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    js_ts_assignment_value(parsed, assignment).is_some_and(|rhs| {
        arg_node_taints_match(parsed, &rhs, path) || node_contains_any_range(&rhs, source_ranges)
    })
}

fn js_ts_flat_sink_identifier_has_tainted_request_source(
    parsed: &ParsedFile,
    id: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    js_ts_enclosing_call_for_function_identifier(parsed, id).is_some_and(|call| {
        js_ts_call_has_tainted_or_source_arg(parsed, &call, path, source_ranges)
    }) || js_ts_enclosing_assignment_for_lhs_identifier(parsed, id).is_some_and(|assignment| {
        js_ts_assignment_has_tainted_or_source_rhs(parsed, &assignment, path, source_ranges)
    })
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
    collect_calls(parsed, func_node, &mut calls);

    for recognizer in crate::sanitizers::active_recognizers() {
        if recognizer.category != category {
            continue;
        }
        // Look for a call to the recognizer's call_path within the function.
        let mut matched = false;
        for call in &calls {
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !call_path_matches(parsed, &actual, recognizer.call_path) {
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

// [->Phase-IP / A2] TEMPORARY layering inversion: reasoning reaches into taint.rs. Relocate
// cleansed_categories_for_source + function_body_cleansed_for into src/sanitizers/ when A2 lands.
// Tracked: docs/superpowers/specs/2026-06-09-prism-tier2-planA-substrate-hardening-design.md §9.
/// A4: the single reasoning-facing cleansing adapter. Returns the sanitizer categories present
/// in the SOURCE FUNCTION BODY (NOT path-proof) as lowercase strings. Gated to Go/Python/JS-TS
/// exactly like apply_cleansers; honest-empty otherwise.
pub(crate) fn cleansed_categories_for_source(
    files: &std::collections::BTreeMap<String, ParsedFile>,
    source: &crate::data_flow::VarLocation,
) -> Vec<String> {
    let parsed = match files.get(&source.file) {
        Some(p) => p,
        None => return Vec::new(),
    };
    if !crate::sanitizers::sanitizer_supported(parsed.language) {
        return Vec::new();
    }

    let mut out = Vec::new();
    let cats: std::collections::BTreeSet<crate::frameworks::SanitizerCategory> =
        crate::sanitizers::active_recognizers()
            .map(|r| r.category)
            .collect();
    for category in cats {
        if function_body_cleansed_for(parsed, source.line, category) {
            out.push(sanitizer_category_str(category).to_string());
        }
    }
    out
}

fn sanitizer_category_str(c: crate::frameworks::SanitizerCategory) -> &'static str {
    use crate::frameworks::SanitizerCategory::*;
    match c {
        Xss => "xss",
        Sqli => "sqli",
        Ssrf => "ssrf",
        Deserialization => "deserialization",
        OsCommand => "os_command",
        PathTraversal => "path_traversal",
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
    let mut paths = ctx.cpg.taint_forward_cfg(&taint_sources);
    synthesize_target_seed_paths(&framework_sources, ctx, &mut paths);

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
                    if !all_sinks.iter().any(|s| matches_sink(text, s)) {
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
                        sink_lines.insert((edge.to.file.clone(), edge.to.line));
                        if let Some(first_edge) = path.edges.first() {
                            sink_to_path_sources
                                .entry((edge.to.file.clone(), edge.to.line))
                                .or_default()
                                .insert((first_edge.from.file.clone(), first_edge.from.line));
                        }
                    }
                }

                // Structured sinks (Go Phase 1 plus Python Phase 2).
                if matches!(outcome, SinkMatchOutcome::Match(_))
                    && !structured_suppressed_by_cleanser
                {
                    sink_lines.insert((edge.to.file.clone(), edge.to.line));
                    if let Some(first_edge) = path.edges.first() {
                        sink_to_path_sources
                            .entry((edge.to.file.clone(), edge.to.line))
                            .or_default()
                            .insert((first_edge.from.file.clone(), first_edge.from.line));
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
                    }
                }
            }
        }

        let source_desc = source_location
            .map(|(_, sl)| format!("line {}", sl))
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

        let mut finding = SliceFinding {
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
            diagrams: vec![],
        };
        finding.diagrams.push(diagram);
        result.findings.push(finding);
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
            if let Some((sf, sl)) = taint_sources
                .iter()
                .filter(|(sf, sl)| sf == file && *sl <= *line)
                .max_by_key(|(_, sl)| *sl)
            {
                sources_with_emitted_sinks.insert((sf.clone(), *sl));
            }

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

    #[test]
    fn test_cleansed_categories_for_source_python_xss() {
        let src = "def f(u):\n    safe = html.escape(u)\n    return safe\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.py".to_string(),
            ParsedFile::parse("t.py", src, Language::Python).unwrap(),
        );
        let source = VarLocation {
            file: "t.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            cats.iter().any(|c| c == "xss"),
            "html.escape => xss: {cats:?}"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_rust_empty() {
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.rs".to_string(),
            ParsedFile::parse(
                "t.rs",
                "fn f(u: &str) -> String { u.to_string() }",
                Language::Rust,
            )
            .unwrap(),
        );
        let s = VarLocation {
            file: "t.rs".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 1,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        assert!(cleansed_categories_for_source(&files, &s).is_empty());
    }
}
