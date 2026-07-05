# Harness hardening report — Tier-C Part-C (F1-F4)

Branch `tc-harness-hardening`, base `0dee22f`. Scope: `eval/tier_c/`, `eval/adoption/`,
`eval/tests/` only — no Rust source touched. Commits:

- `732fcd3` — implementation (arm_runner.py + codex_env.py + cli.py)
- `20e9133` — tests (new file + extensions to two existing test files)

## F1 — matched-binary preflight

- `eval/tier_c/arm_runner.py:189-266` — new `PreflightError` + `resolve_matched_binaries(build_dir=None)`.
  Resolution order: explicit `build_dir` → `$PRISM_BUILD_DIR` → parent dir of `$PRISM_BIN`/`$PRISM_MCP_BIN`
  → `<repo>/target/release`. Raises `PreflightError` (never returns a "matched: False") when:
  (a) `$PRISM_BIN` and `$PRISM_MCP_BIN` resolve to **different** parent directories (the exact
  independent-resolution skew vector), or (b) either binary is missing under the resolved dir.
  Returns `{build_dir, prism_bin, prism_mcp_bin, prism_stat:{mtime,size}, prism_mcp_stat:{mtime,size}, matched: True}`.
- `eval/tier_c/cli.py:1037-1047` — `_run_partc_live` calls this once, before `load_issues`/the
  manifest write/`Checkout`, when `skip_binary_preflight=False`; on `PreflightError` it sets
  `failed_stage="preflight"` and re-raises into the existing `except (ArmRunError, Exception)`
  handler, so `status.json` (`failed_stage: "preflight"`) is written exactly like an off/on-arm failure.
- `eval/tier_c/cli.py:1052-1057` — the result is recorded in `manifest.json` as `binary_preflight`
  (mtimes/sizes) and `prism_cache_dir`.
- `eval/tier_c/cli.py:79-83` — new `--prism-build-dir` CLI flag on `run-partc`.
- `eval/tier_c/cli.py:150-157` — the `--live` path always passes `skip_binary_preflight=False`
  (no user-facing escape hatch for F1 — it's meant to be unmissable).

**Deviation from spec's "simpler acceptable version":** the spec's fallback text says resolve
from "the `$PRISM_BIN` parent"; I additionally fall back to the `$PRISM_MCP_BIN` parent if only
that one is set (symmetric, not asked for but harmless/more robust).

## F2 — shared `--cache-dir` through prewarm + both agent MCP configs

- `eval/tier_c/arm_runner.py:41-59` — `prism_mcp_args(repo_root, *, no_cache=False, cache_dir=None)`:
  appends `--cache-dir <dir>` only when `cache_dir` is truthy AND `no_cache` is False (prism-mcp's
  `--no-cache` and `--cache-dir` are mutually exclusive per its clap `conflicts_with`; `no_cache` wins).
- `eval/tier_c/arm_runner.py:180-236` (`_prewarm_cpg`) and `:317-325` (`_prism_mcp_config`) both
  gained a `cache_dir` kwarg threaded into `prism_mcp_args`/the argv.
- `eval/tier_c/arm_runner.py:398-405` (`build_codex_cmd`) also threads `cache_dir` (inline
  `-c mcp_servers.prism.args` path — not actually exercised by `CodexRunner` today since it always
  forces `prism=False` on that call and uses `CODEX_HOME` instead for the real on-arm, but kept in
  sync with `prism_mcp_args` for correctness/symmetry).
- `eval/adoption/codex_env.py:29-37,80-98` — `build_isolated_codex_home(..., cache_dir=None)`
  appends `--cache-dir` to `mcp_servers.prism.args` when given; ignored when `include_skill_and_mcp=False`
  (off-arm has no MCP section at all).
- `ClaudeRunner`/`CodexRunner` (`arm_runner.py:468-478`, `572-581`) gained a `cache_dir` constructor
  param threaded to `_prism_mcp_config`/`build_isolated_codex_home` respectively.
- `eval/tier_c/cli.py:1025-1027` — `_run_partc_live` auto-derives `cache_dir = <run_dir>/prism-cache`
  when not explicitly passed (one shared cache base per run), then threads it into `_write_partc_manifest`,
  `_LivePartCComps`, and (inside that class) both `ClaudeRunner`/`CodexRunner` construction and every
  `run_arm_isolated` call (`cli.py:733-746, 764-784`).
- **`no_cache=False` on the on-arm prewarm+agent**: confirmed already true (`run_off_arm`/`run_on_arm`
  both call with `no_cache=False`); unchanged.

**Not touched (in scope for `eval/adoption/` but out of scope per spec's file list):**
`adoption/env.py`'s `build_isolated_config` (the claude-side twin) also writes an `mcp.json`, but
tracing its only caller (`arm_runner.ClaudeRunner._arm_config_dir`) shows that config's `mcp_cfg`
is never actually used — `ClaudeRunner.run` gets its real MCP endpoint from `_prism_mcp_config`
instead. Since it's dead for this pipeline and the spec's F2 list doesn't name it, I left it alone.

## F3 — MCP timeouts

- `eval/adoption/codex_env.py:80-98` — on-arm `config.toml` now also writes
  `startup_timeout_sec = 600` and `tool_timeout_sec = 600` under `[mcp_servers.prism]`.
- `eval/tier_c/arm_runner.py:524-527` — `ClaudeRunner.run` sets `MCP_TIMEOUT=600000` and
  `MCP_TOOL_TIMEOUT=600000` (ms) in the subprocess env, only inside the `if variant.prism:` branch
  (off-arm has no MCP server, so nothing to time out).

## F4 — per-cell warm-initialize gate

- `eval/tier_c/arm_runner.py:329-425` — new `warm_gate_check(repo_root, *, cache_dir=None,
  prism_mcp_bin=None, timeout_s=15.0)`: spawns `prism-mcp --repo <repo> [--cache-dir <dir>]` via
  `subprocess.Popen`, runs the real MCP handshake over stdio (`initialize` →
  `notifications/initialized` → `tools/list`, matching `src/mcp/transport.rs`'s lifecycle gate —
  `tools/list` before a completed handshake is rejected server-side), and returns a telemetry dict
  `{ok, wall_s, tools_count, argv, error}` bounded by one overall `timeout_s` deadline via a reader
  thread + `queue.Queue`. Never raises.
- `eval/tier_c/arm_runner.py:118-134` — wired into `run_arm_isolated`, **inside** the existing
  `if prewarm and variant.prism:` guard, immediately after `_prewarm_cpg` and before `runner.run`.
  On gate failure it raises `ArmRunError` (with an attributable
  `"prism-mcp not warm: init took Xs / tools=N (...)"` message) — `runner.run` is then never called.
  New params: `cache_dir=None`, `skip_warm_gate=True` (default **skip**), `warm_gate_timeout_s=15.0`.
- Because the gate lives inside `run_arm_isolated` (not a separate step in `cli.py`), a gate failure
  on the on-arm is caught by `_run_partc_live`'s **existing** `except ArmRunError as e:` on-arm
  handler with zero new cli.py failure-handling code — it's classified as `failed_stage="on"`,
  `<base>.on.meta.json` is written with the exception info, and `status.json` records the error.
  On success, the gate's telemetry is folded into `prewarm_telemetry["warm_gate"]` (alongside the
  existing `argv/returncode/wall_s`), which is exactly what already gets persisted to
  `<base>.on.prewarm.json` — satisfying "record cache dir, prewarm argv+rc+wall, warm-smoke wall,
  tools count" with no new artifact file.
- `eval/tier_c/cli.py:80-93` — new `--skip-warm-gate` (store_true) and `--warm-gate-timeout-s`
  (float, default 15.0) CLI flags; threaded through `_LivePartCComps` → `run_arm_isolated`.

**Deviation from spec:** the spec's F4 wording ("Record in the cell meta: effective cache dir...")
suggested a dedicated audit artifact; I instead reused the existing `<base>.on.prewarm.json`
mechanism since it already carries the shape needed and is already tested/wired. No new file format.

## Preserving the prism-OFF arm

`run_off_arm` still calls `run_arm_isolated(..., prewarm=False)` and its `variant.prism=False` — the
`if prewarm and variant.prism:` guard means `_prewarm_cpg`/`warm_gate_check` never run for the off
arm regardless of `skip_warm_gate`; `cache_dir` is threaded into `ClaudeRunner(cache_dir=...)` /
`CodexRunner(cache_dir=...)` for code symmetry but is provably inert there — `_prism_mcp_config`/
`build_isolated_codex_home`'s MCP-args branches are gated on `variant.prism`/`include_skill_and_mcp`,
neither of which the off-arm ever sets. Covered by
`test_run_arm_isolated_gate_skipped_for_prism_off_variant`.

## Backward-compat defaults (test-safety design decision)

`run_arm_isolated(..., skip_warm_gate=True, ...)`, `_run_partc_live(..., skip_binary_preflight=True,
skip_warm_gate=True, ...)`, and `_LivePartCComps.__init__(..., skip_warm_gate=True, ...)` all default
to **skipping** the new hardening at the function level. This is deliberate: dozens of pre-existing
unit tests call these directly (mocking `subprocess.run`, not `subprocess.Popen`) and would otherwise
try to spawn a real `prism-mcp`/hit a real filesystem preflight. The **CLI** (`run-partc --live`)
always overrides these explicitly — `skip_binary_preflight=False` unconditionally, `skip_warm_gate=
args.skip_warm_gate` (argparse default `False`, i.e. gate ON) — so real runs are hardened by default;
only direct/test callers that predate this work see the old (unhardened) behavior unless they opt in.

## Tests

`eval/tests/test_tc_harness_hardening.py` (29 tests, new file):
- F2 arg assembly: `prism_mcp_args` includes/omits `--cache-dir`, `no_cache` wins over `cache_dir`;
  `_prewarm_cpg`/`_prism_mcp_config`/`build_codex_cmd` all thread `cache_dir` into their real argv/JSON.
- F3: `ClaudeRunner` sets `MCP_TIMEOUT`/`MCP_TOOL_TIMEOUT` for prism-ON, omits them for prism-OFF.
- F1: `resolve_matched_binaries` — success (stat sizes/mtimes correct), missing-prism/missing-prism-mcp
  raise `PreflightError` with the binary name in the message, env-var skew raises, `PRISM_BUILD_DIR`
  and `$PRISM_BIN`-parent fallback both resolve correctly; plus a real-binary sanity check skipped
  unless `PRISM_BIN`/`PRISM_MCP_BIN` point at files on disk (verified green with
  `PRISM_BIN=/Users/wesleyjinks/code/slicing/target/release/prism PRISM_MCP_BIN=.../prism-mcp`).
- F4: `warm_gate_check` against a mocked `subprocess.Popen` (never a real `prism-mcp`) — good
  handshake → `ok=True`/`tools_count`; empty `tools/list` → `ok=False`; a slow fake reader with
  `timeout_s=0.05` → fails fast (`wall_s < 2.0`) with a timeout error, not a hang; spawn `OSError` →
  `ok=False` with "spawn failed"; malformed (non-JSON) first line → reported via the dict, never raises.
- F4 wiring: `run_arm_isolated` with a mocked `warm_gate_check` — gate failure raises `ArmRunError`
  and `runner.run` is provably never called; gate success lets `runner.run` through and the result
  carries `prewarm["warm_gate"]`; default (`skip_warm_gate` unset) never calls `warm_gate_check` at
  all; prism-OFF variant never triggers it even with `skip_warm_gate=False`.
- CLI wiring: `run-partc --live --prism-build-dir ... --skip-warm-gate --warm-gate-timeout-s 5`
  threads exactly those values into `_run_partc_live` (mocked, no live components); the default
  (no `--skip-warm-gate`) passes `skip_warm_gate=False`; a preflight failure (empty `--prism-build-dir`)
  writes `status.json` with `failed_stage: "preflight"` and never opens `Checkout`.

`eval/adoption/tests/unit/test_codex_env.py` (+4 tests): `--cache-dir` present/absent in
`mcp_servers.prism.args`; `startup_timeout_sec`/`tool_timeout_sec` == 600; off-arm config ignores
`cache_dir` entirely (no `mcp_servers`/`cache-dir`/the cache path string anywhere in its `config.toml`).

`eval/tests/test_tc_partc_audit.py` (2 signatures loosened to `**kwargs`, no new tests): two
pre-existing fakes for `run_arm_isolated`/`_prewarm_cpg` had fixed keyword-only signatures that broke
once `_LivePartCComps` started passing `cache_dir`/`skip_warm_gate`/`warm_gate_timeout_s` — widened
to tolerate any extra kwargs, no assertions changed.

## Verification

- `cd eval && uv run pytest -q --ignore=adoption` → **584 passed, 2 skipped** (pre-change baseline:
  556 passed, 1 skipped — net +28 new tests, 1 additional skip is the new real-binary sanity check).
- `cd eval && uv run pytest -q adoption/tests/unit` → **44 passed** (unaffected by these changes;
  the live-deepeval `adoption/tests/test_prism_adoption.py` was correctly left unexercised — it needs
  real API keys/`claude` calls and is out of scope for this verification).
- Confirmed the real-binary-gated tests go green with
  `PRISM_BIN=/Users/wesleyjinks/code/slicing/target/release/prism
  PRISM_MCP_BIN=/Users/wesleyjinks/code/slicing/target/release/prism-mcp`.
- No live cell was run (per instructions — expensive; the controller re-smokes separately).
- `python3 -m py_compile` on all three changed implementation files: clean.

## Not done / explicitly out of scope

- No `adoption/env.py` (`build_isolated_config`) changes — see F2 note above (dead code path for
  this pipeline).
- No live `run-partc --live` smoke test performed here.
- No Rust source touched.
