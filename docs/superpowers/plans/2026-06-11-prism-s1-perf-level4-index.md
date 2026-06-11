# Prism S1 — Perf Hardening + Level-4 Index Inversion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the two profiled CPG-build hotspots (repeated `all_functions()` tree-sitter
queries; per-call Level-4 repo rescans) and parallelize file parsing, with zero behavior
change (spec §2 contract; the only behavior commit, B2, is deferred and NOT in this plan).

**Architecture:** Slice A adds an eager `FunctionTable` to `ParsedFile` and reimplements
`all_functions()` over it. Slice B1 refactors the legacy Level-4 text scanner into a shared
per-line core and builds a `field → file → targets` index once per build. Slice C swaps the
framework cell to `OnceLock` (Sync investigation), parallelizes parsing with rayon (C1), and
— only if `ParsedFile: Sync` — parallelizes per-file extraction with serial assembly (C2).

**Tech Stack:** Rust, tree-sitter 0.25, rayon (new dep), `/usr/bin/time -l` + `sample` for
acceptance profiling.

**Source spec:** `docs/superpowers/specs/2026-06-11-prism-s1-perf-level4-index-design.md`
(rev 2.1). Review record: `docs/prism-query-layer/s1-spec-review-MCP-2026-06-11.md`.

**Recurring gate (every task):** `cargo fmt --check && cargo test` green;
`cargo test --test cli_nav_compat` byte-identical; `cargo test --test algo_taint_cve` green.
`cargo build --features mcp` must compile at Tasks 3, 8, 10. No `CACHE_VERSION` change
anywhere.

**File map:**
- Modify: `src/ast.rs` (FunctionInfo, table build, all_functions over table, legacy refactor,
  candidate enumeration)
- Modify: `src/cpg/build.rs:340-360` (Step 5b param lookup)
- Modify: `src/call_graph.rs:312-360` (Level-4 index build + lookup)
- Modify: `src/repo_loader.rs` (C1 walk/par split)
- Modify: `Cargo.toml` (rayon; `[[test]]` for loader parity if new file)
- Create: `scripts/bench-ladder.sh`
- Tests: `#[cfg(test)]` mods in `src/ast.rs` + `src/call_graph.rs`;
  `tests/navigation/loader_test.rs`; new `tests/infra/parallel_equality_test.rs`

---

### Task 0: Branch

- [ ] **Step 0.1:** `git checkout -b s1-perf-level4` from up-to-date `main`. Run the
  recurring gate once to confirm a green baseline; record `cargo test` summary counts in the
  PR notes.

---

### Task 1: `FunctionInfo` + eager table in `ParsedFile::parse` (Slice A)

**Files:** Modify `src/ast.rs` (struct at ~line 45, `parse()` at 63-90, new methods near
`all_functions()` at 153); tests in `src/ast.rs` `#[cfg(test)]` mod.

- [ ] **Step 1.1: Write the failing test** (in the existing `#[cfg(test)]` mod at the bottom
  of `src/ast.rs`; create one if absent):

```rust
#[test]
fn function_table_captures_named_and_anonymous_in_query_order() {
    // JS: named fn + anonymous callback lambda (function_name() returns None for the latter)
    let src = "function alpha(a, b) { return a; }\nitems.forEach((x) => { use(x); });\n";
    let pf = ParsedFile::parse("t.js", src, Language::JavaScript).unwrap();
    let table = pf.functions();
    // Full captured sequence preserved, query order, including unnamed entries (spec §3 BLOCKER 1)
    let direct = pf.all_functions();
    assert_eq!(table.len(), direct.len());
    for (info, node) in table.iter().zip(direct.iter()) {
        assert_eq!(info.start_byte, node.start_byte());
        assert_eq!(info.end_byte, node.end_byte());
        assert_eq!(info.kind_id, node.kind_id());
    }
    assert_eq!(table[0].name.as_deref(), Some("alpha"));
    assert_eq!(table[0].param_names, vec!["a".to_string(), "b".to_string()]);
    assert!(table.iter().any(|f| f.name.is_none())); // the arrow callback
}

#[test]
fn function_table_rust_and_same_named_functions() {
    let src = "fn f(x: u32) -> u32 { x }\nmod a { pub fn f(y: u32) -> u32 { y } }\n";
    let pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
    let named: Vec<_> = pf.functions().iter().filter(|f| f.name.as_deref() == Some("f")).collect();
    assert_eq!(named.len(), 2); // both kept, query order — no dedup/last-writer-wins
    assert_eq!(named[0].param_names, vec!["x".to_string()]);
}
```

- [ ] **Step 1.2:** Run `cargo test --lib function_table_ -- --nocapture`.
  Expected: FAIL — `functions()` not defined / `FunctionInfo` unknown.

- [ ] **Step 1.3: Implement.** In `src/ast.rs`:

```rust
/// One function definition captured at parse time (spec §3). Plain owned data:
/// the Sync-friendly, S2-ready seam. `name == None` for anonymous functions
/// (JS/TS callback lambdas). Sequence preserves the capture order of the
/// dual-path collection (query when compiled, manual walk otherwise).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub name: Option<String>,
    pub kind_id: u16,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize, // 1-indexed
    pub end_line: usize,   // 1-indexed
    pub param_names: Vec<String>,
}
```

Add field `functions: Vec<FunctionInfo>` to `ParsedFile` (after `framework`). In `parse()`
(line 80-89), construct with `functions: Vec::new()`, then immediately populate via a
two-phase init (name inference reads parent nodes, so this requires the finished tree):

```rust
        let mut pf = Self {
            path: path.to_string(),
            source: source.to_string(),
            tree,
            language,
            parse_error_count,
            parse_node_count,
            line_offsets,
            framework: std::cell::OnceCell::new(),
            functions: Vec::new(),
        };
        pf.functions = pf.build_function_table();
        Ok(pf)
```

```rust
    /// Build the eager function table via the existing dual-path collection
    /// (compiled query, else manual walk) — spec §3 / review MINOR 8.
    fn build_function_table(&self) -> Vec<FunctionInfo> {
        self.all_functions_via_tree()
            .into_iter()
            .map(|node| FunctionInfo {
                name: self
                    .language
                    .function_name(&node)
                    .map(|n| self.node_text(&n).to_string()),
                kind_id: node.kind_id(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                param_names: self.function_parameter_names(&node),
            })
            .collect()
    }

    pub fn functions(&self) -> &[FunctionInfo] {
        &self.functions
    }
```

Rename the body of the current `all_functions()` (lines 153-179) to
`fn all_functions_via_tree(&self) -> Vec<Node<'_>>` (private; same code). Leave
`all_functions()` itself delegating to `all_functions_via_tree()` for now — Task 2 rewires it.

- [ ] **Step 1.4:** `cargo test --lib function_table_` → PASS. Full recurring gate → green
  (table is additive; nothing consumes it yet).

- [ ] **Step 1.5:** Commit: `feat(ast): eager FunctionTable on ParsedFile (S1 slice A, part 1)`

---

### Task 2: `all_functions()` over the table + reconstruction fallback

**Files:** Modify `src/ast.rs`; tests in same `#[cfg(test)]` mod.

- [ ] **Step 2.1: Write the failing tests:**

```rust
#[test]
fn all_functions_reconstructed_matches_direct_query_per_language() {
    // (source, language) per priority language; each ≥2 functions, JS includes an anonymous fn
    let cases: Vec<(&str, Language, &str)> = vec![
        ("fn a() {}\nfn b(x: u32) { let _ = x; }", Language::Rust, "t.rs"),
        ("func A() {}\nfunc B(x int) { _ = x }", Language::Go, "t.go"),
        ("def a():\n    pass\n\ndef b(x):\n    return x\n", Language::Python, "t.py"),
        ("function a() {}\nitems.map((x) => x + 1);", Language::JavaScript, "t.js"),
        ("void a(void) {}\nint b(int x) { return x; }", Language::C, "t.c"),
        ("class K { void a() {} };\nint b() { return 0; }", Language::Cpp, "t.cpp"),
    ];
    for (src, lang, path) in cases {
        let pf = ParsedFile::parse(path, src, lang).unwrap();
        let direct = pf.all_functions_via_tree();
        let reconstructed = pf.all_functions();
        assert_eq!(direct.len(), reconstructed.len(), "{path}");
        for (d, r) in direct.iter().zip(reconstructed.iter()) {
            assert_eq!((d.kind_id(), d.start_byte(), d.end_byte()),
                       (r.kind_id(), r.start_byte(), r.end_byte()), "{path}");
        }
    }
}

#[test]
fn all_functions_falls_back_to_direct_query_on_reconstruction_miss() {
    let src = "fn a() {}\nfn b() {}\n";
    let mut pf = ParsedFile::parse("t.rs", src, Language::Rust).unwrap();
    pf.functions[0].kind_id = u16::MAX; // synthetic corruption: no node can match
    let (nodes, used_fallback) = pf.all_functions_inner();
    assert!(used_fallback); // the flag is the r2-BLOCKER-1 drift detector
    assert_eq!(nodes.len(), 2); // full sequence via fallback — never silently skipped
}
```

And extend `all_functions_reconstructed_matches_direct_query_per_language` (Step 2.1) with
the zero-fallback assertion per language:

```rust
        let (reconstructed, used_fallback) = pf.all_functions_inner();
        assert!(!used_fallback, "{path}: reconstruction must not fall back (spec §3)");
```

- [ ] **Step 2.2:** `cargo test --lib all_functions_` → FAIL (reconstruction not implemented;
  fallback test fails because corruption is ignored).

- [ ] **Step 2.3: Implement.** Replace `all_functions()`:

```rust
    /// Function nodes, reconstructed from the eager table. On any reconstruction
    /// miss, falls back to the direct dual-path collection for the WHOLE file —
    /// never a partial sequence (spec §3, r1 MAJOR 2). The bool is the
    /// fallback-fire flag (spec §3, r2-BLOCKER 1): in-module tests assert it is
    /// false per language so grammar drift cannot silently route a language to
    /// the slow path.
    pub fn all_functions(&self) -> Vec<Node<'_>> {
        self.all_functions_inner().0
    }

    pub(crate) fn all_functions_inner(&self) -> (Vec<Node<'_>>, bool) {
        let mut out = Vec::with_capacity(self.functions.len());
        for info in &self.functions {
            match self.reconstruct_function_node(info) {
                Some(node) => out.push(node),
                None => return (self.all_functions_via_tree(), true),
            }
        }
        (out, false)
    }

    fn reconstruct_function_node(&self, info: &FunctionInfo) -> Option<Node<'_>> {
        let mut node = self
            .tree
            .root_node()
            .descendant_for_byte_range(info.start_byte, info.end_byte)?;
        // descendant_for_byte_range returns the DEEPEST node spanning the range,
        // so recovery walks UP through same-span ancestors (spec §3, r2-BLOCKER 1
        // — a walk-down can never reach a same-span ancestor).
        loop {
            if node.start_byte() == info.start_byte
                && node.end_byte() == info.end_byte
                && node.kind_id() == info.kind_id
            {
                return Some(node);
            }
            match node.parent() {
                Some(p) if p.start_byte() == info.start_byte && p.end_byte() == info.end_byte => {
                    node = p
                }
                _ => return None,
            }
        }
    }
```

- [ ] **Step 2.4:** `cargo test --lib all_functions_` → PASS. **Full recurring gate** → green
  (this rewires all 28 call sites through reconstruction; goldens prove invisibility).

- [ ] **Step 2.5:** Commit: `feat(ast): all_functions() served from FunctionTable with whole-file fallback (S1 slice A, part 2)`

---

### Task 3: Step 5b param lookup via the table

**Files:** Modify `src/cpg/build.rs:347-361`; test in `src/cpg/tests.rs`.

- [ ] **Step 3.1: Write the failing test** (in `src/cpg/tests.rs`; build a two-file fixture
  through the existing test helpers used by neighboring tests in that file):

```rust
#[test]
fn step5b_param_binding_first_wins_parity() {
    // callee file defines two same-named fns; Step 5b must bind args to the FIRST
    // (tree-order) match — pinned-until-S2 (spec §3). Caller passes `tainted`.
    let callee_src = "def f(p):\n    return p\n\nclass K:\n    def f(q):\n        return q\n";
    let caller_src = "from m import f\n\ndef call():\n    tainted = source()\n    f(tainted)\n";
    let files = parse_fixture_files(&[("m.py", callee_src), ("c.py", caller_src)]);
    let ctx = CpgContext::build(&files, None);
    // arg→param edge must target p (first f), at m.py line 1 — and NOT q
    assert!(has_dataflow_edge(&ctx.cpg, ("c.py", 5, "tainted"), ("m.py", 1, "p")));
    assert!(!has_dataflow_edge(&ctx.cpg, ("c.py", 5, "tainted"), ("m.py", 4, "q")));
}
```

(Reuse the file-map + edge-assertion helpers already present in `src/cpg/tests.rs` — e.g. the
patterns used by `test_taint_trace_records_boundary_at_param_def`; if no edge-assertion
helper exists, add a local `has_dataflow_edge` that scans `ctx.cpg` Variable nodes/edges.)

- [ ] **Step 3.2:** Run it — expected: **PASS already** (it pins current behavior). This is
  the parity baseline; commit it before touching Step 5b so the change is bisect-proof.
  Commit: `test(cpg): pin Step 5b first-match param binding (pinned-until-S2)`

- [ ] **Step 3.3: Implement.** In `src/cpg/build.rs`, replace the lookup block (~lines
  347-361):

```rust
                    let param_names = {
                        match callee_parsed
                            .functions()
                            .iter()
                            .find(|f| f.name.as_deref() == Some(callee_id.name.as_str()))
                        {
                            Some(f) => f.param_names.clone(),
                            None => continue,
                        }
                    };
```

(Deletes the `all_functions()` + `function_name` + `node_text` per-callee re-query. Unnamed
entries can never match — identical to today's `function_name()==None` failing the
comparison.)

- [ ] **Step 3.4:** Full recurring gate + `cargo build --features mcp` → green; parity test
  still green.

- [ ] **Step 3.5:** Commit: `perf(cpg): Step 5b param lookup via FunctionTable (S1 slice A, part 3)`

---

### Task 4: Bench script + Slice-A acceptance evidence

**Files:** Create `scripts/bench-ladder.sh` (executable).

- [ ] **Step 4.1:** Create the script per spec §6 (adapted from the session prototype):

```bash
#!/bin/bash
# Prism scale-ladder benchmark (spec §6). Usage:
#   scripts/bench-ladder.sh [--cache-dir DIR] [--timeout SECS] [name:path ...]
# Defaults: fresh temp cache dir; 2400s; pinned list (prism,tokio,hugo,django,rust-analyzer)
# expected as sibling checkouts under ~/code / ~/code/bench-repos. Emits one markdown row
# per repo: repo | loc | files | cold_s | maxrss_mb | cache_mb | warm_s | status
set -u
command -v timeout >/dev/null || { echo "needs GNU timeout (brew install coreutils)" >&2; exit 2; }
/usr/bin/time -l true 2>/dev/null || { echo "needs BSD /usr/bin/time -l (macOS)" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRISM="$ROOT/target/release/prism"
BENCH_REPOS="${PRISM_BENCH_REPOS:-$HOME/code/bench-repos}"
CACHE_BASE="$(mktemp -d /tmp/prism-bench-cache.XXXXXX)"; TMO=2400; REPOS=()
while [ $# -gt 0 ]; do case "$1" in
  --cache-dir) CACHE_BASE="$2"; shift 2;; --timeout) TMO="$2"; shift 2;;
  *) REPOS+=("$1"); shift;; esac; done
if [ ${#REPOS[@]} -eq 0 ]; then REPOS=(
  "prism:$ROOT" "tokio:$BENCH_REPOS/tokio" "hugo:$BENCH_REPOS/hugo"
  "django:$BENCH_REPOS/django" "rust-analyzer:$BENCH_REPOS/rust-analyzer" ); fi
echo "repo | loc | files | cold_s | maxrss_mb | cache_mb | warm_s | status"
EXT=('*.rs' '*.go' '*.py' '*.js' '*.jsx' '*.ts' '*.tsx' '*.c' '*.cc' '*.cpp' '*.h' '*.hpp' '*.java' '*.lua' '*.tf' '*.sh' '*.bash')
for spec in "${REPOS[@]}"; do
  name="${spec%%:*}"; repo="${spec#*:}"; cdir="$CACHE_BASE/$name"; mkdir -p "$cdir"
  if [ ! -d "$repo" ]; then
    echo "$name | - | - | - | - | - | - | missing ($repo)"; continue
  fi
  loc=$(cd "$repo" && git ls-files -- "${EXT[@]}" 2>/dev/null \
    | grep -vE '^(vendor|node_modules|dist|build|target)/' | tr '\n' '\0' \
    | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
  files=$(cd "$repo" && git ls-files -- "${EXT[@]}" 2>/dev/null \
    | grep -cvE '^(vendor|node_modules|dist|build|target)/')
  t0=$(date +%s)
  /usr/bin/time -l timeout "$TMO" "$PRISM" nav --cache-dir "$cdir" repo-map \
    --repo "$repo" --format json >/dev/null 2>"/tmp/bench-$name.time"
  st=$?; t1=$(date +%s); cold=$((t1 - t0))
  rss=$(awk '/maximum resident set size/{printf "%.0f", $1/1048576}' "/tmp/bench-$name.time")
  if [ $st -eq 124 ]; then
    echo "$name | $loc | $files | TIMEOUT>${TMO}s | ${rss:-?} | - | - | timeout"; continue
  elif [ $st -ne 0 ]; then
    echo "$name | $loc | $files | $cold | ${rss:-?} | - | - | exit$st"; continue
  fi
  cmb=$(du -sm "$cdir" 2>/dev/null | awk '{print $1}')
  w0=$(python3 -c 'import time; print(time.time())')
  timeout 300 "$PRISM" nav --cache-dir "$cdir" repo-map --repo "$repo" --format json >/dev/null 2>&1
  w1=$(python3 -c 'import time; print(time.time())')
  warm=$(python3 -c "print(f'{$w1 - $w0:.2f}')")
  echo "$name | $loc | $files | $cold | ${rss:-?} | $cmb | $warm | ok"
done
```

- [ ] **Step 4.2:** `chmod +x scripts/bench-ladder.sh`; smoke it on prism only:
  `cargo build --release && scripts/bench-ladder.sh prism:$PWD` → one well-formed row.

- [ ] **Step 4.3: Slice-A acceptance (spec §7 row A).** Cold tokio build under `sample`:
  start `target/release/prism nav --cache-dir $(mktemp -d) repo-map --repo ~/code/bench-repos/tokio --format json`
  in background; `sample $(pgrep -x prism | head -1) 10 -file /tmp/a-profile.txt`; assert
  `grep -c "all_functions\|ts_query" /tmp/a-profile.txt` frames appear in <1% of samples
  (compare the per-symbol sample counts against the total at the file top). Record the
  numbers + the bench row deltas in the PR description.

- [ ] **Step 4.4:** Commit: `chore(bench): committed scale-ladder script + slice-A acceptance evidence`

---

### Task 5: B1 part 1 — quirk fixtures, then legacy core extraction

**Files:** Modify `src/ast.rs:3333-3398`; tests in `src/ast.rs` `#[cfg(test)]` mod.

The fixtures come FIRST and pin the legacy core's behavior **directly** (they are the
quirk-retirement set of spec §4 B2; the shared-core refactor makes the differential test in
Task 6 primarily an enumeration-completeness proof, so core behavior must be pinned here).

- [ ] **Step 5.1: Write the pinning tests (against the EXISTING function; must pass before
  any refactor):**

```rust
fn fns(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn level4_legacy_quirk_arrow_anywhere_priority() {
    // ->field ANYWHERE in the line beats a closer .field (spec §4 quirk 1):
    // the .cb assignment to `f` is dropped because ->cb is found first.
    let src = "s.cb = f; t->cb = g;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
    assert_eq!(got, vec!["g".to_string()]);
}

#[test]
fn level4_legacy_quirk_prefix_consumption() {
    // ->cbx matches find("->cb"); scan position advances past it, and the
    // REAL earlier .cb assignment is never revisited (spec §4 quirk 2).
    let src = "s.cb = f; t->cbx = g;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["f", "g"]));
    assert_eq!(got, Vec::<String>::new());
}

#[test]
fn level4_legacy_rhs_rules() {
    // single-= anchor (== rejected), &-strip, stop at ; , } ) and whitespace,
    // known_fns filter, per-file BTreeSet dedup+sort.
    let src = "a.cb == f;\nb.cb = &handler;\nc.cb = handler, x;\nd.cb = unknown_fn;\ne.cb = handler;\n";
    let got = resolve_struct_field_assignment(src, "cb", &fns(&["handler", "f"]));
    assert_eq!(got, vec!["handler".to_string()]); // deduped, == rejected, unknown filtered
}

#[test]
fn level4_legacy_designated_initializer_multi_field() {
    let src = "static struct ops o = { .open = do_open, .close = do_close };\n";
    assert_eq!(resolve_struct_field_assignment(src, "open", &fns(&["do_open", "do_close"])),
               vec!["do_open".to_string()]);
    assert_eq!(resolve_struct_field_assignment(src, "close", &fns(&["do_open", "do_close"])),
               vec!["do_close".to_string()]);
}
```

- [ ] **Step 5.2:** `cargo test --lib level4_legacy_` → **PASS against current code** (these
  pin, not change). If any fails, STOP: the quirk model is wrong — re-read the legacy
  function and fix the TEST to match actual behavior, then update spec §4's quirk list to
  match reality before proceeding.

- [ ] **Step 5.3:** Commit: `test(ast): pin Level-4 legacy scanner quirks (B2 retirement set)`

- [ ] **Step 5.4: Refactor to a shared per-line core** (no behavior change). In `src/ast.rs`:

```rust
/// The legacy per-line, per-field matcher — extracted verbatim from
/// resolve_struct_field_assignment so the Level-4 index (call_graph) and the
/// legacy oracle share ONE core. Quirks (arrow-anywhere priority,
/// prefix-consumption, single-= anchor, RHS stop set, &-strip) are pinned by
/// the level4_legacy_* tests and are CONTRACT until B2 retires them.
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
```

Rewrite `resolve_struct_field_assignment` as the thin wrapper (same signature, same
prefilter):

```rust
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
```

Note the `==` check folded into one condition — verify it is logically identical to the
original two sequential `continue`s (it is: `starts_with("==") ⇒ starts_with("=")`).

- [ ] **Step 5.5:** `cargo test --lib level4_legacy_` → PASS (pins prove the refactor is
  invisible). Full recurring gate → green.

- [ ] **Step 5.6:** Commit: `refactor(ast): extract Level-4 per-line core (shared by oracle + index)`

---

### Task 6: B1 part 2 — candidate enumeration, index build, lookup swap, differential oracle

**Files:** Modify `src/ast.rs` (enumeration fn), `src/call_graph.rs:312-360` (index build +
lookup); tests in both `#[cfg(test)]` mods.

- [ ] **Step 6.1: Write the failing enumeration test:**

```rust
#[test]
fn candidate_fields_are_maximal_post_accessor_identifiers() {
    let got = candidate_fields_on_line("s.cb = f; t->cbx = g; obj.data->next = h; x = 3.14;");
    let want: BTreeSet<String> =
        ["cb", "cbx", "data", "next", "14"].iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want); // "14" is harmless noise: digits are identifier chars,
                           // and no callee filter ever queries it
}

#[test]
fn candidate_fields_use_the_pinned_unicode_predicate() {
    // spec §4 r2-4: same predicate as the Level-4 call-site filter
    // (char::is_alphanumeric || '_'), Unicode-aware — `café` is ONE identifier.
    let got = candidate_fields_on_line("obj.café = handler; p->x_1 = g;");
    let want: BTreeSet<String> = ["café", "x_1"].iter().map(|s| s.to_string()).collect();
    assert_eq!(got, want);
}
```

- [ ] **Step 6.2:** FAIL (function absent). **Implement** in `src/ast.rs`:

```rust
/// Every maximal identifier immediately preceded by `->` or `.` on the line,
/// under the PINNED predicate (spec §4, r2-4): char::is_alphanumeric(c) || c == '_'
/// — the same Unicode-aware class the Level-4 call-site filter applies to callee
/// names — scanned over RAW source lines including comments and string literals
/// (B1 reproduces legacy text-scan semantics; B2 retires that).
/// Completeness argument: a field can only produce a target when the accessor
/// occurrence is followed (after optional whitespace) by `=`, which terminates
/// the identifier run — so every PRODUCTIVE field is a maximal run here. Prefix
/// occurrences (`->cbx` while querying `cb`) never produce targets; their
/// consumption side-effects are reproduced by running the legacy core.
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
```

- [ ] **Step 6.3:** Test PASS. Commit: `feat(ast): Level-4 candidate-field enumeration`

- [ ] **Step 6.4: Index build + lookup swap.** In `src/call_graph.rs`, immediately before the
  Level-4 loop (line ~312), build the index; then replace the inner all-files scan
  (lines ~338-357) with a lookup:

```rust
        // Level-4 index (S1/B1): field -> file -> targets, built ONCE per build.
        // Reuses the legacy per-line core, so per-(field,file) results are
        // byte-identical to resolve_struct_field_assignment by construction.
        type Level4Index = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;
        let mut level4_index: Level4Index = BTreeMap::new();
        for (path, parsed) in files {
            let mut per_field: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for line in parsed.source.lines() {
                let trimmed = line.trim();
                for field in crate::ast::candidate_fields_on_line(trimmed) {
                    let targets = per_field.entry(field.clone()).or_default();
                    crate::ast::line_field_targets(trimmed, &field, &known_fn_names, targets);
                }
            }
            for (field, targets) in per_field {
                if !targets.is_empty() {
                    level4_index
                        .entry(field)
                        .or_default()
                        .insert(path.clone(), targets);
                }
            }
        }
```

Replace the per-call inner loop (keep every existing outer filter untouched):

```rust
                // Search the prebuilt index for assignments to this field name
                let field_name = &site.callee_name;
                if let Some(by_file) = level4_index.get(field_name) {
                    for targets in by_file.values() {
                        for target in targets {
                            level4_sites.push((
                                caller_id.clone(),
                                CallSite {
                                    caller: caller_id.clone(),
                                    callee_name: target.clone(),
                                    line: site.line,
                                    qualifier: None,
                                },
                            ));
                        }
                    }
                }
```

(Emission-order argument: legacy iterated ALL `files` in BTreeMap order, emitting each file's
sorted targets; the index's `by_file` is the SUBSET of files with non-empty targets, in the
same relative order, and empty files emitted nothing — identical push sequence.)

- [ ] **Step 6.5: Write the differential oracle test** (in `src/call_graph.rs` tests; spec §4
  MAJOR 4 — index-independent universe, both directions):

```rust
#[test]
fn level4_index_matches_legacy_oracle_over_full_universe() {
    // Corpus: every fixture-ish source we can cheaply assemble + prism's own sources.
    let mut sources: Vec<(String, String)> = vec![
        ("quirks.c".into(),
         "s.cb = f; t->cb = g;\ns.cb = f; t->cbx = g;\nstatic struct ops o = { .open = do_open, .close = do_close };\na.cb == nope;\nb.cb = &handler;\n".into()),
    ];
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src_dir).unwrap().flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            sources.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
        }
    }
    let known: BTreeSet<String> =
        ["f", "g", "do_open", "do_close", "handler", "build", "new", "slice", "run"]
            .iter().map(|s| s.to_string()).collect();

    // Universe: ALL post-accessor identifiers across the corpus (index-independent,
    // superset of every Phase-3-queryable field) + explicit negatives.
    let mut universe: BTreeSet<String> =
        ["no_such_field", "cb", "cbx", "open", "close"].iter().map(|s| s.to_string()).collect();
    for (_, src) in &sources {
        for line in src.lines() {
            universe.extend(crate::ast::candidate_fields_on_line(line.trim()));
        }
    }

    // Build the index exactly as CallGraph::build does.
    let mut index: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    for (path, src) in &sources {
        let mut per_field: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for line in src.lines() {
            let trimmed = line.trim();
            for field in crate::ast::candidate_fields_on_line(trimmed) {
                let t = per_field.entry(field.clone()).or_default();
                crate::ast::line_field_targets(trimmed, &field, &known, t);
            }
        }
        for (field, t) in per_field {
            if !t.is_empty() {
                index.entry(field).or_default().insert(path.clone(), t);
            }
        }
    }

    // Pairing rule (spec §4, r2-5) — two halves; never universe × all-files × full-scan
    // (that would relocate the removed hotspot into the test suite).
    // Half 1 — EXCESS: every (field, file) the index claims must equal the legacy scan.
    for (field, by_file) in &index {
        for (path, targets) in by_file {
            let src = &sources.iter().find(|(p, _)| p == path).unwrap().1;
            let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
            let got: Vec<String> = targets.iter().cloned().collect();
            assert_eq!(got, legacy, "excess: field={field} file={path}");
        }
    }
    // Half 2 — MISSES: per universe field, legacy-scan ONLY files containing the
    // ->field / .field substring (the legacy has_field check hoisted to file level —
    // provably outcome-preserving), and assert the index agrees (absent key == empty).
    for field in &universe {
        let arrow = format!("->{field}");
        let dot = format!(".{field}");
        for (path, src) in &sources {
            if !(src.contains(&arrow) || src.contains(&dot)) {
                continue; // legacy provably returns empty; index has no entry by construction
            }
            let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
            let from_index: Vec<String> = index
                .get(field)
                .and_then(|m| m.get(path))
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            assert_eq!(from_index, legacy, "miss: field={field} file={path}");
        }
    }
}
```

- [ ] **Step 6.6:** `cargo test level4` → all PASS. **Full recurring gate** → green
  (byte-identity: same emission sequence, goldens unchanged).

- [ ] **Step 6.7: B1 acceptance (spec §7 row B1, r2-6b).** Cold hugo build under `sample`
  (same procedure as Step 4.3); gate: the **legacy symbol `resolve_struct_field_assignment`
  at ≈0% of samples** (it has no production caller post-B1). `line_field_targets` is a
  distinct symbol and MAY legitimately appear during the one-time index build — it is
  excluded from the gate (record its share as context). Re-run `scripts/bench-ladder.sh`
  (default list) — expect django and rust-analyzer to complete (the motivating TIMEOUTs).
  Record rows in PR.

- [ ] **Step 6.8:** Commit: `perf(callgraph): Level-4 inverted index — O(repo) once vs O(calls×files) (S1/B1)`

---

### Task 7: Slice C investigation — `Sync` + `OnceLock` swap

**Files:** Modify `src/ast.rs:57-58,88` (cell type), possibly `src/mcp/session.rs:25`
(clippy allow); test in `src/ast.rs`.

- [ ] **Step 7.1: Write the failing assertion test:**

```rust
#[test]
fn parsed_file_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ParsedFile>(); // C2 gate (spec §5)
    assert_send_sync::<crate::repo_loader::LoadedRepo>();
}
```

- [ ] **Step 7.2:** `cargo test --lib parsed_file_is_send` → expected: **COMPILE FAIL**
  citing `std::cell::OnceCell<...> cannot be shared between threads` (this is the
  investigation's documented finding; `tree_sitter::Tree` is Send+Sync in 0.25).

- [ ] **Step 7.3: Implement:** change `src/ast.rs:58` to
  `pub framework: std::sync::OnceLock<Option<&'static crate::frameworks::FrameworkSpec>>,`
  and `:88` to `framework: std::sync::OnceLock::new(),` (`get_or_init` exists on `OnceLock`;
  the `framework()` body at :95-99 is unchanged). If the assertion STILL fails, some other
  field is `!Sync`: record the exact compiler error in the plan-execution notes, mark **C2
  descoped** per spec §5, delete the assertion test, and skip Task 9 — Task 8 (C1) proceeds
  regardless.

- [ ] **Step 7.4:** Test compiles + passes → C2 is GO. Optionally remove
  `#[allow(clippy::arc_with_non_send_sync)]` in `src/mcp/session.rs` and update its comment
  (hygiene, only if `cargo clippy --features mcp` stays clean). Full recurring gate → green.

- [ ] **Step 7.5:** Commit: `chore(ast): framework cell → OnceLock; ParsedFile is Send+Sync (C investigation: GO for C2)`
  (or `...(C investigation: C2 descoped — <reason>)`).

---

### Task 8: C1 — parallel parsing in `repo_loader`

**Files:** Modify `Cargo.toml` (add `rayon = "1"` to `[dependencies]`),
`src/repo_loader.rs:39-217`; tests in `tests/navigation/loader_test.rs`.

- [ ] **Step 8.1: Write the failing parity test** (in `tests/navigation/loader_test.rs`,
  alongside the existing loader tests):

```rust
#[test]
fn parallel_loader_is_element_for_element_identical_to_serial_reference() {
    // Mixed tree: parsed files, unsupported ext, non-UTF8 with unsupported ext
    // (classification must stay NotUtf8? No: unsupported ext + non-UTF8 → the
    // serial order is read→utf8→language, so it must stay NotUtf8 — spec §5 MAJOR 5),
    // oversized file, hidden dir, and a parse-degraded file.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("sub")).unwrap();
    std::fs::write(p.join("a.py"), "def a():\n    return 1\n").unwrap();
    std::fs::write(p.join("sub/b.rs"), "fn b() {}\n").unwrap();
    std::fs::write(p.join("notes.txt"), "hello").unwrap(); // Unsupported
    std::fs::write(p.join("bad.xyz"), [0xFFu8, 0xFE, 0x00]).unwrap(); // NotUtf8 (read happens before ext check)
    std::fs::write(p.join("big.py"), "x".repeat(3 * 1024 * 1024)).unwrap(); // TooLarge
    let repo = load_repo(p).unwrap();
    let reference = load_repo_serial_reference(p).unwrap(); // test-only serial twin
    assert_eq!(repo.files.keys().collect::<Vec<_>>(), reference.files.keys().collect::<Vec<_>>());
    assert_eq!(repo.file_hashes, reference.file_hashes);
    let skips = |r: &LoadedRepo| r.skipped.iter().map(|s| (s.path.clone(), format!("{:?}", s.reason))).collect::<Vec<_>>();
    assert_eq!(skips(&repo), skips(&reference)); // element-for-element, ORDER included
}
```

- [ ] **Step 8.2:** FAIL (`load_repo_serial_reference` absent). **Implement** in
  `src/repo_loader.rs`:

  1. Introduce `enum WalkItem { Skip(SkippedFile), Candidate { rel: String, source: String, language: Language } }`.
  2. Refactor `walk()` so that, instead of parsing inline, it performs **exactly today's
     sequence** — builtin-dir skip → symlink → hidden-dir → metadata → size → `fs::read` →
     UTF-8 → `Language::from_path` — and pushes `WalkItem`s in encounter order. Every skip
     reason lands at its current position; a passing file becomes a `Candidate` carrying the
     already-read source.
  3. `load_repo` becomes: walk → `let outcomes: Vec<_> = candidates.into_par_iter().map(|c| { let parsed = ParsedFile::parse(&c.rel, &c.source, c.language); let hash = format!("{:x}", Sha256::digest(c.source.as_bytes())); (parsed, hash) }).collect();`
     (rayon's indexed `collect` preserves order) → merge by iterating `WalkItem`s in order:
     `Skip` → push to `skipped`; `Candidate` → take the next outcome; apply today's exact
     post-parse logic (Err → `ParseFailed`; `error_rate() > SEVERE_PARSE_ERROR_RATE` →
     `ParseFailed`; else insert into `files` + `file_hashes`).
  4. Add `pub(crate) fn load_repo_serial_reference(root: &Path) -> Result<LoadedRepo>`
     (`#[cfg(test)]`-gated or `pub(crate)`): same pipeline with a plain serial `.map()` —
     the parity twin. Keep it trivially small by sharing the walk + merge code; only the
     par/serial map differs.
  5. `use rayon::prelude::*;` and add `rayon = "1"` to `Cargo.toml` `[dependencies]`.

- [ ] **Step 8.3:** Parity test PASS; full recurring gate + `--features mcp` build → green.

- [ ] **Step 8.4:** Commit: `perf(loader): parallel file parsing with element-identical merge (S1/C1)`

---

### Task 9: C2 — parallel per-file extraction, serial assembly *(SKIP if Task 7 descoped C2)*

**Files:** Modify `src/call_graph.rs` (Phases 1-2 per-file loops), `src/data_flow.rs:156-`
(per-file loop); Create `tests/infra/parallel_equality_test.rs`; Modify `Cargo.toml`
(`[[test]] name = "infra_parallel_equality" path = "tests/infra/parallel_equality_test.rs"`).

- [ ] **Step 9.1: Write the failing exact-order equality test:**

```rust
// tests/infra/parallel_equality_test.rs
// Serial-vs-parallel CPG equality IN INSERTION ORDER (spec §2a): node and edge
// vectors must match element-for-element, not as sorted sets — cache bytes
// serialize insertion order and there is no CACHE_VERSION bump in S1.
use prism::cpg::CpgContext;
use prism::repo_loader::load_repo;

#[test]
fn cpg_build_is_identical_under_parallel_extraction() {
    let repo = load_repo(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    let ctx1 = CpgContext::build(&repo.files, None);
    let ctx2 = CpgContext::build(&repo.files, None);
    let dump = |c: &CpgContext| {
        let nodes: Vec<String> = c.cpg.node_indices().map(|i| format!("{:?}", c.cpg.node(i))).collect();
        let edges: Vec<String> = c.cpg.edge_dump(); // see Step 9.2 — ordered (src, dst, kind) triples
        (nodes, edges)
    };
    assert_eq!(dump(&ctx1), dump(&ctx2)); // build-to-build determinism incl. par scheduling
}
```

(Plus a fixture-repo variant using the tempdir fixture pattern from
`tests/navigation/loader_test.rs`, asserting equality between a build with
`RAYON_NUM_THREADS=1` and the default — the env-var run is wired in Step 9.3's runner note.)

- [ ] **Step 9.2: Implement.**
  - Add `pub fn edge_dump(&self) -> Vec<String>` to `CodePropertyGraph` (`src/cpg/query.rs`
    or `types.rs` impl block): `self.graph.edge_references().map(|e| format!("{:?}->{:?}:{:?}", e.source().index(), e.target().index(), e.weight())).collect()` — insertion-ordered by petgraph.
  - `src/call_graph.rs` Phase 1 (~line 67-100) and Phase 2 (~line 145-190): convert
    `for (path, parsed) in files { …push… }` into
    `let per_file: Vec<_> = files.par_iter().map(|(path, parsed)| { …collect this file's items into a local Vec… }).collect();`
    followed by a **serial** flatten-in-order into the existing structures. The local-Vec
    contents and their order must be exactly what the serial loop pushed for that file.
  - `src/data_flow.rs` `build_from_refs` (~line 156): same pattern — par-map each file to its
    local `(defs, uses, edges, alias)` collections, then serial merge in file order into the
    BTreeMaps/Vec (entry-extend in the same sequence the serial code used).
  - **Assembly (`cpg/build.rs`) stays serial — do not touch it** (§2a).

- [ ] **Step 9.2b: Byte-level cache parity (spec §2a, r2-3).** Add to the same test file:

```rust
#[test]
fn cache_blob_bytes_identical_serial_vs_parallel() {
    // The cache serializes CPG + CallGraph + DataFlowGraph vectors in insertion
    // order (no CACHE_VERSION bump in S1) — byte equality is the strongest §2a proof.
    let repo = load_repo(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    // Build + store twice through the public cached-build path (RAYON pool is
    // process-global, so cross-thread-count comparison runs via the env-var matrix
    // below; within one process, two builds prove scheduling determinism).
    prism::navigation::NavigationIndex::build_cached_under(&repo, d1.path());
    prism::navigation::NavigationIndex::build_cached_under(&repo, d2.path());
    fn find_bin(d: &std::path::Path) -> std::path::PathBuf {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                return find_bin(&p); // cache layout: <base>/<repo-hash>/cpg-cache.bin
            }
            if p.file_name().and_then(|n| n.to_str()) == Some("cpg-cache.bin") {
                return p;
            }
        }
        panic!("cpg-cache.bin not found under {}", d.display());
    }
    let b1 = std::fs::read(find_bin(d1.path())).unwrap();
    let b2 = std::fs::read(find_bin(d2.path())).unwrap();
    assert_eq!(b1, b2);
}
```

  And in the test-runner matrix: run the whole `infra_parallel_equality` target twice —
  default threads and `RAYON_NUM_THREADS=1` — then compare one `cpg-cache.bin` produced
  under each setting for byte equality (a tiny shell step in the task notes, since the env
  var is process-global):
  `RAYON_NUM_THREADS=1 cargo test --test infra_parallel_equality && cargo test --test infra_parallel_equality`.

- [ ] **Step 9.3:** Run the equality tests under both thread settings as above — green under
  both proves scheduling-independence. Full recurring gate → green. C acceptance (spec §7
  row C, r2-6a): **full-command** `time` user/wall ratio ≥ 1.5 on a cold hugo build (the
  per-phase ratio is unobservable from `/usr/bin/time`; serial phases dilute the number —
  record the ratio with that caveat).

- [ ] **Step 9.4:** Commit: `perf(cpg): parallel per-file extraction, serial assembly (S1/C2)`

---

### Task 10: Final acceptance + handoff

- [ ] **Step 10.1:** Full gate sweep: `cargo fmt --check && cargo test && cargo test --features mcp && cargo test --test cli_nav_compat && cargo test --test algo_taint_cve` — all green.
- [ ] **Step 10.2:** `cargo build --release && scripts/bench-ladder.sh` (default pinned
  list). Paste the before/after table into the PR description next to the baseline
  (prism 29s / tokio 89s / hugo 469s / django TIMEOUT / rust-analyzer TIMEOUT). Verify warm
  column within 10% of baseline (spec §7 warm-parity report-out; explain if not).
- [ ] **Step 10.3:** Profile evidence: attach the three `sample` summaries (A: tokio, B1:
  hugo, C: user/wall ratio) with the <1% sample-share numbers.
- [ ] **Step 10.4:** Update `docs/superpowers/specs/…s1….md` status line with outcomes
  (C2 GO/descoped; acceptance numbers). Record deferred work in the followups doc, carrying
  two priced-in risks **verbatim** (spec r2-13): (1) B2 is "scheduled only once the Tier-A
  harness is live" — B1 relocates the quirky scanner somewhere *less visible* than today's
  hot loop, so the trigger must not silently slip; (2) lifting §2a's insertion-order cap on
  parallel assembly is **S2-adjacent `NodeIndex`-identity work, not a C2 option**. Also:
  call-site migration off `Node`, C2-if-descoped.
- [ ] **Step 10.5:** Squash to a docs+feat pair (force-push, bisect hygiene) → dual code
  review (codex gpt-5.5 + Claude reviewer per spec §9; reviewers get prism MCP) → merge.

## Self-Review

**Spec coverage:** §3 → Tasks 1-3 (BLOCKER 1: Option<String>+kind_id in Task 1; MAJOR 2
fallback in Task 2; MINOR 8 dual-path via `all_functions_via_tree`; first-wins pin in Task
3; MINOR 9 warm parity in Tasks 4/10). §4 B1 → Tasks 5-6 (MAJOR 3 quirks pinned first;
MAJOR 4 universe both directions; MINOR 10 rationale lives in spec; B2 absent by design).
§5 → Tasks 7-9 (MAJOR 5 classification/order in Task 8's walk contract; §2a exact-order in
Task 9; descope path in Task 7). §6 → Task 4 (MINOR 12 contract + pinned list). §7 → Tasks
4.3/6.7/9.3/10 (MAJOR 7 objective gates). §2a no-CACHE_VERSION → no serialized-shape change
in any task. **Type consistency:** `FunctionInfo` fields used in Tasks 2-3 match Task 1;
`line_field_targets`/`candidate_fields_on_line` signatures match between Tasks 5-6;
`WalkItem`/`load_repo_serial_reference` confined to Task 8. **Placeholders:** none — every
code step carries the code; Step 9.2's per-file conversions specify the exact mechanical
pattern and the invariant (local-Vec contents == serial push sequence) the implementor must
satisfy.
