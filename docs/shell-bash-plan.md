# Shell / Bash Language Support — Analysis & Plan

**Status:** Analysis complete, scaffolding ready
**Date:** 2026-04-02
**Priority:** Should-have (firmware review, strong security use case)
**Estimated effort:** 1–2 weeks

---

## 1. Why Shell Is High-Value for Security Analysis

Shell scripts are the #1 source of command injection vulnerabilities in
firmware. The failure mode is trivial to introduce and catastrophic:

```bash
# Vulnerable: unquoted variable in command position
FILE=$1
cat $FILE              # Path traversal: $1 = "/etc/passwd"
rm -rf $DIR/$FILE      # If $FILE = "../../", deletes the filesystem

# Vulnerable: variable in eval/backtick
INPUT=$(curl -s "$URL")
eval "$INPUT"          # Remote code execution

# Safe equivalent
cat -- "$FILE"
```

Unlike compiled languages where injection requires specific API misuse, in
shell scripts **every unquoted variable expansion in command position is a
potential injection vector**. This makes taint analysis exceptionally valuable.

### Where shell scripts appear in firmware

| Location | Purpose | Risk |
|----------|---------|------|
| `/etc/init.d/*` | Service startup/shutdown | Privilege escalation via env vars |
| `sysupgrade`, `fw_update` | Firmware update scripts | Bricking, persistent compromise |
| `/usr/lib/opkg/*` | Package manager hooks | Supply chain injection |
| Factory provisioning | Initial device setup | Credential leakage, backdoors |
| Build scripts | CI/CD, image creation | Build system compromise |
| Cron jobs | Periodic maintenance | Persistent unauthorized access |

---

## 2. tree-sitter-bash Grammar Analysis

**Crate:** `tree-sitter-bash` (0.25.1) — mature, actively maintained.

### Key node types for Prism

| Node type | Prism mapping | Example |
|-----------|--------------|---------|
| `function_definition` | Function node | `foo() { ... }` |
| `command` | Statement (Call) | `cat file.txt` |
| `variable_assignment` | Statement (Assignment) | `X=value` |
| `if_statement` | Statement (Branch) | `if [ ... ]; then ... fi` |
| `while_statement` | Statement (Loop) | `while ...; do ... done` |
| `for_statement` | Statement (Loop) | `for x in ...; do ... done` |
| `case_statement` | Statement (Branch) | `case $x in ... esac` |
| `pipeline` | Statement (Call chain) | `cmd1 \| cmd2 \| cmd3` |
| `command_substitution` | Expression (taint propagation) | `$(command)` or `` `command` `` |
| `variable_name` | Variable reference | `$VAR`, `${VAR}`, `"$VAR"` |
| `string` / `raw_string` | Literal | `"quoted"`, `'raw'` |
| `subshell` | Statement (scope boundary) | `(commands)` |
| `heredoc_body` | Literal (may contain expansions) | `<<EOF ... EOF` |
| `test_command` | Expression (condition) | `[ -f "$FILE" ]`, `[[ ... ]]` |

### Shell-specific parsing challenges

1. **Word splitting:** `$VAR` without quotes undergoes word splitting and glob
   expansion. `"$VAR"` does not. This distinction is security-critical but
   syntactic — tree-sitter captures it via the `string` vs bare `word` context.

2. **Variable scoping:** Shell has no lexical scope. Variables are global by
   default; `local` in functions is bash-specific. All variables in a script
   share one namespace.

3. **Command substitution nesting:** `$(cmd1 $(cmd2))` — taint flows inward
   through nested substitutions.

4. **Pipelines:** `cmd1 | cmd2` — stdout of cmd1 becomes stdin of cmd2. Taint
   propagates left to right through pipes.

5. **Here-documents:** `<<EOF ... $VAR ... EOF` — variable expansion inside
   heredocs. Taint flows through heredoc bodies.

---

## 3. Algorithm Mapping

### 3.1 TaintSlice — Command injection detection

**Sources (user-controlled input):**

| Source | Pattern | Risk level |
|--------|---------|------------|
| Positional params | `$1` .. `$9`, `$@`, `$*`, `${!i}` | High — direct user input |
| `read` builtin | `read VAR`, `read -r LINE` | High — stdin/pipe input |
| `curl`/`wget` output | `$(curl ...)`, `$(wget ...)` | High — network input |
| Environment vars | `$INPUT`, `$USER_DATA`, `$QUERY` | Medium — depends on caller |
| Command substitution | `$(cat file)`, `` `cmd` `` | Medium — file/command output |

**Sinks (dangerous operations):**

| Sink | Pattern | Vulnerability |
|------|---------|--------------|
| `eval` | `eval "$VAR"` | Arbitrary code execution |
| Backtick / `$(...)` in command | `` `$VAR` ``, `$($VAR)` | Code execution via expansion |
| `exec` | `exec $CMD` | Process replacement |
| `source` / `.` | `source "$FILE"` | Code inclusion |
| Unquoted in command position | `cat $FILE`, `rm $PATH` | Argument injection, path traversal |
| `xargs` | `echo $INPUT \| xargs rm` | Argument injection |
| `find -exec` | `find . -name "$PAT" -exec ...` | Glob injection |
| `awk`/`sed` with variable | `awk "$PATTERN"` | Code injection in awk |
| `su`/`sudo` | `sudo $CMD` | Privilege escalation |
| `chmod`/`chown` | `chmod $MODE $FILE` | Permission manipulation |
| SQL in shell | `sqlite3 db "SELECT $INPUT"` | SQL injection |

**The killer detection:** Unquoted variable expansion in command arguments.
This is the single most common shell vulnerability class:
```bash
# TAINTED: $1 flows unquoted to rm
rm -rf /tmp/$1       # rm -rf /tmp/../../etc = disaster

# SAFE: quoted
rm -rf "/tmp/$1"     # Treated as single argument
```

Tree-sitter distinguishes quoted (`string` node containing `variable_name`)
from unquoted (bare `word` or `simple_expansion` in command arguments). Prism
can detect this syntactically.

### 3.2 ProvenanceSlice — Origin classification

| Origin | Pattern | Classification |
|--------|---------|----------------|
| `$1`–`$9`, `$@`, `$*` | Positional parameters | ScriptArgs |
| `read` | `read VAR` | UserInput |
| `$(curl/wget ...)` | Network fetch | NetworkInput |
| `$ENVVAR` | Environment variable | EnvVar |
| `source`/`.` file | Sourced config | Config |
| `$(cat file)` | File read | FileInput |
| Hardcoded string | `'literal'` | Constant |

### 3.3 AbsenceSlice — Cleanup pairs

| Open | Close | Pattern |
|------|-------|---------|
| `trap '' SIGINT` | Trap restored or script exits | Signal handler cleanup |
| `mktemp` | `rm "$tmpfile"` | Temp file cleanup |
| `mount` | `umount` | Filesystem mount |
| `pushd` | `popd` | Directory stack |
| `exec 3>file` | `exec 3>&-` | File descriptor |
| Lock file creation | Lock file removal | Process locking |

### 3.4 QuantumSlice — Background processes

| Pattern | Detection |
|---------|-----------|
| `cmd &` | Background job |
| `wait` | Synchronization point |
| `(subshell) &` | Background subshell |
| `coproc` | Coprocess |
| `nohup` | Detached process |

### 3.5 MembraneSlice — Script interface

Shell scripts have implicit interfaces:
- **Inputs:** Positional parameters, environment variables, stdin
- **Outputs:** stdout, stderr, exit code, file writes
- **Callers:** Scripts that `source` or invoke this script

When a diff touches a script, MembraneSlice identifies:
1. Which input variables flow into the changed region
2. Which downstream scripts source/call this script
3. Whether exit codes changed (callers checking `$?`)

---

## 4. Implementation Plan

### Step 1: Language scaffolding (2–3 days)
- Add `Language::Bash` to enum with `.sh`, `.bash` extensions
- Add `tree-sitter-bash` to Cargo.toml
- Implement all language methods:
  - `function_node_types()` → `["function_definition"]`
  - `is_control_flow_node()` → `if_statement`, `while_statement`,
    `for_statement`, `case_statement`
  - `assignment_node_types()` → `["variable_assignment"]`
  - `is_call_node()` → `["command"]`
  - `call_function_name()` → first word of command
- Basic parsing + algorithm smoke tests

### Step 2: Taint analysis patterns (3–4 days)
- Source patterns: `$1`–`$9`, `read`, `curl`, `wget`, env vars
- Sink patterns: `eval`, `exec`, `source`, unquoted variable detection
- **Unquoted variable detection:** AST check — is `variable_name` node inside a
  `string` node (quoted → safe) or bare in a `command` (unquoted → taint sink)?
- Integration tests with shell injection fixtures

### Step 3: Remaining algorithms (2–3 days)
- ProvenanceSlice: origin classification
- AbsenceSlice: cleanup pairs (trap, mktemp/rm, mount/umount)
- QuantumSlice: background process detection
- Integration tests

### Step 4: Firmware-specific patterns (1–2 days)
- OpenWrt init script patterns (`/etc/init.d/`)
- Firmware update script patterns (`sysupgrade`)
- Package manager hook patterns (`opkg`)

---

## 5. Estimated LOC

| Component | Estimated LOC |
|-----------|--------------|
| Language enum + node mappings | ~80 |
| Taint sources/sinks | ~60 |
| Provenance/absence/quantum patterns | ~40 |
| Test fixtures (.sh files + .diff files) | ~80 |
| Integration tests | ~100 |
| **Total** | **~360** |

This is a single PR. Shell is structurally simpler than C/C++ (no structs,
no pointers, no headers) — the main complexity is the taint sink patterns.

---

## 6. Scaffolding Files

Created alongside this plan:
- `tests/fixtures/bash/` — sample `.sh` files for test development
