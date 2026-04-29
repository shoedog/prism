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
}

impl TaintSeed {
    fn line(file: String, line: usize) -> Self {
        Self {
            file,
            line,
            target: None,
        }
    }

    fn target(file: String, line: usize, target: AccessPath) -> Self {
        Self {
            file,
            line,
            target: Some(target),
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

    for func in parsed.all_functions() {
        let line = func.start_position().row + 1;
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }

        let source_params = js_ts_framework_source_params(parsed, &func, framework, &params);
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
        for (source_line, target) in assignment_sources {
            sources.push(TaintSeed::target(
                file_path.to_string(),
                source_line,
                target,
            ));
        }
    }
}

#[derive(Clone)]
struct JsTsParam {
    name: String,
    text: String,
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
            out.push(JsTsParam {
                name,
                text: parsed.node_text(&child).to_string(),
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
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match framework {
        "nestjs" => {
            if !js_ts_has_route_decorator(parsed, func) {
                return out;
            }
            for param in params {
                if js_ts_param_has_nest_source_decorator(&param.text) {
                    out.insert(param.name.clone());
                }
            }
        }
        "fastify" => {
            if let Some(first) = params.first() {
                if matches!(first.name.as_str(), "request" | "req") {
                    out.insert(first.name.clone());
                }
            }
        }
        "express" => {
            if params.len() >= 2
                && matches!(params[0].name.as_str(), "req" | "request")
                && matches!(params[1].name.as_str(), "res" | "response")
            {
                out.insert(params[0].name.clone());
            }
        }
        "koa" => {
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

fn js_ts_param_has_nest_source_decorator(text: &str) -> bool {
    ["@Body", "@Query", "@Param", "@Headers", "@Req", "@Request"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn js_ts_has_route_decorator(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    let mut current = Some(*func);
    while let Some(node) = current {
        let text = parsed.node_text(&node);
        if [
            "@Get",
            "@Post",
            "@Put",
            "@Patch",
            "@Delete",
            "@All",
            "@Controller",
        ]
        .iter()
        .any(|needle| text.contains(needle))
        {
            return true;
        }
        current = node.parent();
    }
    false
}

fn js_ts_request_data_assignment_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> BTreeSet<(usize, AccessPath)> {
    let mut out = BTreeSet::new();
    collect_js_ts_request_assignments(parsed, *func, framework, source_params, &mut out);
    out
}

fn collect_js_ts_request_assignments(
    parsed: &ParsedFile,
    node: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
    out: &mut BTreeSet<(usize, AccessPath)>,
) {
    if node.kind() == "variable_declarator" {
        if let (Some(lhs), Some(rhs)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) {
            collect_js_ts_request_assignment_targets(
                parsed,
                lhs,
                rhs,
                framework,
                source_params,
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
                framework,
                source_params,
                out,
            );
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_request_assignments(parsed, child, framework, source_params, out);
    }
}

fn collect_js_ts_request_assignment_targets(
    parsed: &ParsedFile,
    lhs: Node<'_>,
    rhs: Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
    out: &mut BTreeSet<(usize, AccessPath)>,
) {
    if !node_contains_js_ts_source_access(parsed, rhs, framework, source_params) {
        return;
    }
    let line = rhs.start_position().row + 1;
    collect_js_ts_lhs_targets(parsed, lhs, line, out);
}

fn collect_js_ts_lhs_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    out: &mut BTreeSet<(usize, AccessPath)>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                out.insert((line, AccessPath::simple(name)));
            }
        }
        "member_expression" => {
            out.insert((line, AccessPath::from_expr(parsed.node_text(&node))));
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_targets(parsed, child, line, out);
            }
        }
    }
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

fn js_ts_source_access_text_matches(text: &str, framework: &str, param: &str) -> bool {
    let text = text.trim();
    match framework {
        "nestjs" => {
            text == param
                || text.starts_with(&format!("{}.", param))
                || text.starts_with(&format!("{}[", param))
        }
        "fastify" => ["body", "query", "params", "headers"].iter().any(|field| {
            text == format!("{}.{}", param, field)
                || text.starts_with(&format!("{}.{}.", param, field))
                || text.starts_with(&format!("{}.{}[", param, field))
        }),
        "express" => ["body", "query", "params", "headers", "cookies"]
            .iter()
            .any(|field| {
                text == format!("{}.{}", param, field)
                    || text.starts_with(&format!("{}.{}.", param, field))
                    || text.starts_with(&format!("{}.{}[", param, field))
            }),
        "koa" => {
            ["query", "params", "headers", "cookies"]
                .iter()
                .any(|field| {
                    text == format!("{}.{}", param, field)
                        || text.starts_with(&format!("{}.{}.", param, field))
                        || text.starts_with(&format!("{}.{}[", param, field))
                })
                || ["body", "headers"].iter().any(|field| {
                    text == format!("{}.request.{}", param, field)
                        || text.starts_with(&format!("{}.request.{}.", param, field))
                        || text.starts_with(&format!("{}.request.{}[", param, field))
                })
        }
        _ => false,
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
            line: seed.line,
            path: target.clone(),
            kind: VarAccessKind::Def,
        };
        let mut edges = Vec::new();
        let reachable_dfg = ctx.cpg.dfg_forward_reachable(&from);
        for target_loc in reachable_dfg {
            if target_loc.file == seed.file
                && target_loc.function == func_name
                && target_loc.line <= seed.line
            {
                continue;
            }
            if let Some(cfg_set) = &reachable {
                if target_loc.file == seed.file
                    && target_loc.function == func_name
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
    let refs =
        ctx.parsed
            .find_variable_references_scoped(&ctx.func, &ctx.target.base, ctx.seed.line);
    for ref_line in refs {
        if ref_line <= ctx.seed.line {
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
                && edge.to.var_name() == ctx.target.base
        }) {
            continue;
        }
        edges.push(FlowEdge {
            from: ctx.from.clone(),
            to: VarLocation {
                file: ctx.seed.file.clone(),
                function: ctx.func_name.to_string(),
                line: ref_line,
                path: AccessPath::simple(ctx.target.base.clone()),
                kind: VarAccessKind::Use,
            },
        });
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
    for assignment in assignments {
        let assignment_line = assignment.start_position().row + 1;
        if assignment_line <= ctx.seed.line {
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
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(ctx.parsed, &assignment) else {
            continue;
        };
        if !node_contains_identifier(ctx.parsed, &rhs, &ctx.target.base) {
            continue;
        }
        for alias in assignment_lhs_identifiers(ctx.parsed, &lhs) {
            if alias == "_" {
                continue;
            }
            let refs =
                ctx.parsed
                    .find_variable_references_scoped(&ctx.func, &alias, assignment_line);
            for ref_line in refs {
                if ref_line <= assignment_line {
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
                edges.push(FlowEdge {
                    from: ctx.from.clone(),
                    to: VarLocation {
                        file: ctx.seed.file.clone(),
                        function: ctx.func_name.to_string(),
                        line: ref_line,
                        path: AccessPath::simple(alias.clone()),
                        kind: VarAccessKind::Use,
                    },
                });
            }
        }
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
        return js_ts_ssrf_sink_call_path_matches(parsed, actual, sink_pat.call_path);
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

fn js_ts_ssrf_sink_call_path_matches(parsed: &ParsedFile, actual: &str, expected: &str) -> bool {
    if call_path_matches(parsed, actual, expected)
        && !matches!(expected, "get" | "post" | "request")
    {
        return true;
    }
    if !matches!(expected, "get" | "post" | "request") {
        return false;
    }
    let Some((receiver, method)) = actual.rsplit_once('.') else {
        return false;
    };
    if method != expected {
        return false;
    }
    let receiver_tail = receiver.rsplit('.').next().unwrap_or(receiver);
    matches!(
        receiver_tail,
        "axios" | "got" | "superagent" | "http" | "https" | "undici" | "nodeFetch"
    )
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
            if arg_node_taints_match(parsed, attr, line, p) {
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
            let value_line = v.start_position().row + 1;
            if arg_node_taints_match(parsed, &v, value_line, path) {
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
    let call_line = call.start_position().row + 1;
    (1..=3).any(|arg_idx| match call_arg_node(call, arg_idx) {
        Some(arg) if arg_idx == 1 && js_ts_exec_file_arg_is_argv_or_callback(&arg) => false,
        Some(arg) if js_ts_exec_file_arg_is_callback(&arg) => false,
        Some(arg) => js_ts_node_has_unsafe_shell_option(parsed, &arg, call_line),
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
    before_line: usize,
) -> bool {
    let node = unwrap_parenthesized(*node);
    let text = parsed.node_text(&node);
    if text.trim_start().starts_with('{') && js_ts_object_text_has_unsafe_shell_option(text) {
        return true;
    }
    if node.kind() != "identifier" {
        return true;
    }
    js_ts_identifier_bound_to_unsafe_shell_options(parsed, parsed.node_text(&node), before_line)
}

fn js_ts_identifier_bound_to_unsafe_shell_options(
    parsed: &ParsedFile,
    var_name: &str,
    before_line: usize,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_inspectable_safe_options = false;
    for assignment in assignments {
        if assignment.start_position().row + 1 >= before_line {
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
    }
    !saw_inspectable_safe_options
        || js_ts_shell_options_have_unsafe_mutation(parsed, var_name, before_line)
}

fn js_ts_shell_options_have_unsafe_mutation(
    parsed: &ParsedFile,
    var_name: &str,
    before_line: usize,
) -> bool {
    let receiver_names = js_ts_collection_aliases_before(parsed, var_name, before_line);
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);
    for assignment in assignments {
        if assignment.start_position().row + 1 >= before_line {
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
        if call.start_position().row + 1 >= before_line {
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
        && !call_path_matches(parsed, &actual, "load")
    {
        return false;
    }
    let text = parsed.node_text(call);
    js_yaml_load_text_uses_safe_schema(text)
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
    if node.kind() == "variable_declarator" {
        return Some((
            node.child_by_field_name("name")?,
            node.child_by_field_name("value")?,
        ));
    }
    Some((
        parsed.language.assignment_target(node)?,
        parsed.language.assignment_value(node)?,
    ))
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
        || !js_ts_allowlist_receiver_is_trusted(
            parsed,
            receiver,
            path,
            condition.start_position().row + 1,
        )
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
        || !call_arg_node(&call, 0).is_some_and(|arg| {
            js_ts_path_prefix_arg_is_trusted(parsed, &arg, path, condition.start_position().row + 1)
        })
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
    guard_line: usize,
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
    js_ts_identifier_bound_to_literal_collection(parsed, receiver, guard_line)
}

fn js_ts_identifier_bound_to_literal_collection(
    parsed: &ParsedFile,
    var_name: &str,
    before_line: usize,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_literal_collection = false;
    for assignment in assignments {
        if assignment.start_position().row + 1 >= before_line {
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
        } else {
            return false;
        }
    }
    saw_literal_collection && !js_ts_collection_has_untrusted_update(parsed, var_name, before_line)
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
    before_line: usize,
) -> bool {
    let receiver_names = js_ts_collection_aliases_before(parsed, var_name, before_line);
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in calls {
        if call.start_position().row + 1 >= before_line {
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
        if assignment.start_position().row + 1 >= before_line {
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

fn js_ts_collection_aliases_before(
    parsed: &ParsedFile,
    var_name: &str,
    before_line: usize,
) -> BTreeSet<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut aliases = BTreeSet::new();
    aliases.insert(var_name.to_string());

    let mut changed = true;
    while changed {
        changed = false;
        for assignment in &assignments {
            if assignment.start_position().row + 1 >= before_line {
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
    guard_line: usize,
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
    js_ts_identifier_literal_string_before(parsed, name, guard_line)
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
    before_line: usize,
) -> Option<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut literal = None;
    for assignment in assignments {
        if assignment.start_position().row + 1 >= before_line {
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
            if sink_call_has_tainted_arg_in_path(parsed, call, pat, path)
                && flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pat, call)
            {
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
    if !matches!(parsed.language, Language::Go | Language::Python)
        && !is_js_ts_language(parsed.language)
    {
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
    // Lines whose identifiers are recognized framework SOURCE calls (e.g.
    // `r.URL.Query()`, `c.Query()`, `mux.Vars()`). These overlap textually with
    // the cross-language flat sink registry — `Query` is in SINK_PATTERNS as a
    // generic `sql.Query` substring matcher — so without this set, a tainted
    // source line would double-fire as a sink. Used during sink evaluation to
    // suppress flat substring matches on lines positively identified as sources.
    let framework_source_set: BTreeSet<(String, usize)> = framework_sources
        .iter()
        .filter(|s| s.target.is_none())
        .map(|s| (s.file.clone(), s.line))
        .collect();

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

                // Structured sinks (Go Phase 1 plus Python Phase 2).
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
