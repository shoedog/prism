"""Concrete model drivers + a fake (spec §3). Command builders are unit-tested on the
ARGV they assemble (the live subprocess call is exercised only in an integration run).
prism ON = MCP config passed; OFF = omitted. Mirrors tier_a/sut.py's subprocess style."""
from __future__ import annotations
import json
import os
import subprocess
import tempfile
import time
from dataclasses import dataclass, replace
from pathlib import Path
from .model import Variant, ArmOutput
from .citations import parse_citations
from .parse import parse_codex_jsonl, parse_claude_stream_json
from .llm import cli_model_flag
from .classify import classify_tools
from adoption.env import build_isolated_config
from adoption.codex_env import build_isolated_codex_home


class ArmRunError(Exception):
    """Structured exception raised when a runner subprocess fails.

    Carries the full context needed for failure persistence:
      argv       — the exact command list that was run
      returncode — the process exit code
      stderr     — the full stderr (uncapped; callers may cap on serialisation)
      stdout     — whatever partial stdout was captured (may be empty)
    """

    def __init__(self, *, argv: list[str], returncode: int, stderr: str, stdout: str):
        self.argv = argv
        self.returncode = returncode
        self.stderr = stderr
        self.stdout = stdout
        super().__init__(
            f"arm exited {returncode}: {stderr[:400]}"
        )


def prism_mcp_args(repo_root: str, *, no_cache: bool = False,
                   cache_dir: str | None = None) -> list[str]:
    """Build the prism-mcp server arg list for a given repo root.

    Used at BOTH the claude MCP-config site and the codex -c mcp_servers.prism.args site
    so both runners honour the same no_cache/cache_dir.

    F2 (harness hardening): cache_dir lets a live run pin ONE explicit nav-cache directory
    that is shared between the prewarm (`prism nav repo-map --cache-dir <dir>`) and the
    agent's prism-mcp (`--cache-dir <dir>`), so both provably hit the same
    `<dir>/prism/nav/<hash>` cache instead of relying on the default OS cache dir.
    no_cache and cache_dir are mutually exclusive (prism-mcp's --no-cache conflicts with
    --cache-dir); no_cache wins when both are given.
    """
    args = ["--repo", repo_root]
    if no_cache:
        args.append("--no-cache")
    elif cache_dir:
        args += ["--cache-dir", os.path.abspath(cache_dir)]  # agents spawn prism-mcp from the session cwd; keep it absolute
    return args


def _reset_clean(root: "Path | str") -> None:
    """Revert the checkout at *root* to HEAD: tracked edits (reset --hard) + untracked files (clean -fd)."""
    root = str(root)
    subprocess.run(["git", "-C", root, "reset", "--hard", "-q"], check=True)
    subprocess.run(["git", "-C", root, "clean", "-fdq"], check=True)


@dataclass
class IsolatedArmResult:
    """Result from run_arm_isolated: the arm output + isolation metadata."""
    out: ArmOutput
    cache_mode: str        # "no-cache" | "cached"
    mcp_args: list[str]   # the prism MCP args used (for assertion in tests)
    prewarm: dict | None = None  # pre-warm telemetry dict; None when prewarm=False


def run_arm_isolated(
    runner,
    *,
    checkout,
    variant: Variant,
    stage: str = "spec",
    prompt: str = "",
    no_cache: bool = True,
    prewarm: bool = False,
    cache_dir: str | None = None,
    skip_warm_gate: bool = True,
    warm_gate_timeout_s: float = 15.0,
) -> IsolatedArmResult:
    """Run *runner* inside an isolated, immutable checkout.

    Isolation contract:
    - BEFORE: reset --hard + clean -fd (clean slate, no prior arm pollution).
    - AFTER (in finally): reset --hard + clean -fd (revert any mutations the arm made).
    - prism-mcp is launched with --no-cache (when no_cache=True) so a stale CPG cannot
      survive across arms.
    - prewarm=True + variant.prism=True: runs `prism nav repo-map` AFTER the before-reset
      and BEFORE runner.run so the arm's prism-mcp starts from a warm cache (avoids
      cold-CPG-build exceeding claude's MCP handshake timeout on large repos).  No reset
      is performed between the warm and the arm (which would invalidate the warm cache).
    - cache_dir (F2): threaded into both the prewarm and mcp_args telemetry so the
      prewarm and the agent's prism-mcp provably share one cache directory.
    - skip_warm_gate=False (F4): immediately after a successful prewarm (still inside the
      prewarm+variant.prism guard), spawns a throwaway prism-mcp against the SAME repo +
      cache_dir and runs a JSON-RPC initialize/tools-list handshake (`warm_gate_check`).
      If the handshake doesn't complete in time or returns no tools, this raises
      ArmRunError BEFORE runner.run is ever called — a 0-dose arm (agent silently ran
      without prism) is worse than a skipped/failed cell.  Default True (skip) so callers
      that don't opt in (and existing tests, which mock subprocess.run, not Popen) are
      unaffected; the live Part-C CLI passes skip_warm_gate=False by default.

    Returns IsolatedArmResult so callers can assert on cache_mode and mcp_args.
    """
    root = str(checkout.root)
    # Normalize at the callee too (PR #172 review): a RELATIVE cache_dir from any caller must not
    # be resolved differently by the prewarm (harness cwd), the agent MCP config (absolutized) and
    # the warm gate (now spawned from the checkout cwd, like the agents spawn prism-mcp).
    if cache_dir:
        cache_dir = os.path.abspath(cache_dir)
    mcp_args = prism_mcp_args(root, no_cache=no_cache, cache_dir=cache_dir)
    cache_mode = "no-cache" if no_cache else "cached"

    _reset_clean(root)
    prewarm_telemetry: dict | None = None
    try:
        if prewarm and variant.prism:
            prewarm_telemetry = _prewarm_cpg(root, cache_dir=cache_dir)
            if not skip_warm_gate:
                gate = warm_gate_check(root, cache_dir=cache_dir,
                                       timeout_s=warm_gate_timeout_s)
                prewarm_telemetry["warm_gate"] = gate
                if not gate["ok"]:
                    raise ArmRunError(
                        argv=gate.get("argv", []),
                        returncode=-1,
                        stderr=(
                            "prism-mcp not warm: init took "
                            f"{gate.get('wall_s', 0.0):.1f}s / tools={gate.get('tools_count', 0)}"
                            + (f" ({gate['error']})" if gate.get("error") else "")
                        ),
                        stdout="",
                    )
        out = runner.run(variant, stage, prompt, root)
    finally:
        _reset_clean(root)

    return IsolatedArmResult(out=out, cache_mode=cache_mode, mcp_args=mcp_args,
                             prewarm=prewarm_telemetry)


def _prism_mcp_bin() -> str:
    """Resolve the prism-mcp server binary so prism-ON arms can actually launch it.
    Order: $PRISM_MCP_BIN -> on PATH -> this repo's target/release/prism-mcp -> bare name.
    (prism-mcp is typically NOT on PATH; it's built at <prism-repo>/target/release/prism-mcp,
    and this harness lives at <prism-repo>/eval/tier_c — so resolve it explicitly, else the
    MCP server fails to start and prism-ON silently degrades to no-prism.)"""
    import shutil
    env = os.environ.get("PRISM_MCP_BIN")
    if env:
        return env
    found = shutil.which("prism-mcp")
    if found:
        return found
    cand = os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                     "target", "release", "prism-mcp"))
    return cand if os.path.exists(cand) else "prism-mcp"


def _prism_bin() -> str:
    """Resolve the prism CLI binary (not prism-mcp) for pre-warming the nav CPG cache.
    Order: $PRISM_BIN -> on PATH -> this repo's target/release/prism -> bare name."""
    import shutil
    env = os.environ.get("PRISM_BIN")
    if env:
        return env
    found = shutil.which("prism")
    if found:
        return found
    cand = os.path.normpath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                     "target", "release", "prism"))
    return cand if os.path.exists(cand) else "prism"


class PreflightError(Exception):
    """F1: raised by resolve_matched_binaries when prism + prism-mcp cannot be matched to
    ONE build. A prior full run was silently voided because independently-resolved
    binaries skewed across builds and the CPG cache invalidated between prewarm and the
    agent's prism-mcp — this is meant to be impossible to miss."""


def resolve_matched_binaries(build_dir: str | None = None) -> dict:
    """F1 matched-binary preflight: resolve `prism` + `prism-mcp` from ONE build directory
    and assert they actually exist there, so the harness cannot silently prewarm with one
    build's `prism` while the agent launches a DIFFERENT build's `prism-mcp` (the CPG nav
    cache is invalidated across binaries built from different trees, so a skew here
    silently degrades prism-ON arms back to a cold-or-never-warm MCP handshake).

    Resolution order for the build dir (first hit wins):
      1. the explicit *build_dir* argument (e.g. CLI --prism-build-dir)
      2. $PRISM_BUILD_DIR
      3. the parent directory of $PRISM_BIN, if set
      4. this repo's target/release

    Skew guard: if BOTH $PRISM_BIN and $PRISM_MCP_BIN are set, they must resolve to the
    SAME parent directory — that's the exact independent-resolution skew vector this
    preflight exists to kill. Mismatched parents raise PreflightError even before checking
    existence under the computed build_dir.

    Returns a dict (suitable for embedding in the run manifest):
      {"build_dir": str, "prism_bin": str, "prism_mcp_bin": str,
       "prism_stat": {"mtime": float, "size": int},
       "prism_mcp_stat": {"mtime": float, "size": int}, "matched": True}

    Raises PreflightError (never returns a "matched": False dict) when either binary is
    missing under the resolved build_dir, or the env-var skew guard trips — the run must
    FAIL LOUD, not degrade silently.
    """
    env_prism = os.environ.get("PRISM_BIN")
    env_mcp = os.environ.get("PRISM_MCP_BIN")

    if env_prism and env_mcp:
        p1 = os.path.realpath(os.path.dirname(os.path.abspath(env_prism)))
        p2 = os.path.realpath(os.path.dirname(os.path.abspath(env_mcp)))
        if p1 != p2:
            raise PreflightError(
                f"matched-binary preflight FAILED: $PRISM_BIN ({env_prism!r}) and "
                f"$PRISM_MCP_BIN ({env_mcp!r}) resolve to DIFFERENT build directories "
                f"({p1!r} vs {p2!r}). Both binaries must come from ONE build — set "
                f"PRISM_BUILD_DIR (or unset one override) so prewarm and the agent's "
                f"prism-mcp share the same CPG nav cache."
            )

    resolved_build_dir = build_dir or os.environ.get("PRISM_BUILD_DIR")
    if resolved_build_dir is None and env_prism:
        resolved_build_dir = os.path.dirname(os.path.abspath(env_prism))
    if resolved_build_dir is None and env_mcp:
        resolved_build_dir = os.path.dirname(os.path.abspath(env_mcp))
    if resolved_build_dir is None:
        resolved_build_dir = os.path.normpath(
            os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..",
                         "target", "release"))

    prism_bin = env_prism or os.path.join(resolved_build_dir, "prism")
    prism_mcp_bin = env_mcp or os.path.join(resolved_build_dir, "prism-mcp")

    for label, path in (("prism", prism_bin), ("prism-mcp", prism_mcp_bin)):
        if not os.path.exists(path):
            raise PreflightError(
                f"matched-binary preflight FAILED: {label} binary not found at {path!r} "
                f"(resolved build_dir={resolved_build_dir!r}). Build both from the SAME "
                f"tree with `cargo build --release --bin prism --bin prism-mcp --features mcp` "
                f"before running a live Part-C cell."
            )

    def _stat(p: str) -> dict:
        st = os.stat(p)
        return {"mtime": st.st_mtime, "size": st.st_size}

    return {
        "build_dir": resolved_build_dir,
        "prism_bin": prism_bin,
        "prism_mcp_bin": prism_mcp_bin,
        "prism_stat": _stat(prism_bin),
        "prism_mcp_stat": _stat(prism_mcp_bin),
        "matched": True,
    }


def _prewarm_cpg(root: str, *, cache_dir: str | None = None) -> dict:
    """Build the prism nav CPG cache for `root` so the arm's prism-mcp starts warm.
    (Cold build can exceed claude's MCP handshake timeout on large repos.)

    F2: cache_dir, when given, is passed through as `--cache-dir` so the prewarm writes
    to the SAME cache directory the agent's prism-mcp will read from (instead of both
    independently falling back to the default OS cache dir).

    Returns a telemetry dict:
      {"argv": [...], "returncode": int, "stdout": str, "stderr": str,
       "wall_s": float, "exception": str|None, "cache_dir": str|None}
    Never raises — best-effort; if warming fails the arm still runs.
    """
    # `--cache-dir` is a GLOBAL `prism nav` flag and MUST precede the `repo-map`
    # subcommand — `nav repo-map --repo R --cache-dir D` errors with "unexpected
    # argument '--cache-dir'" (clap), silently defeating the prewarm (best-effort,
    # never raises) so prism-mcp then cold-builds and the warm gate trips.
    argv = [_prism_bin(), "nav"]
    if cache_dir:
        argv += ["--cache-dir", os.path.abspath(cache_dir)]
    argv += ["repo-map", "--repo", root]
    t0 = time.monotonic()
    try:
        r = subprocess.run(argv, capture_output=True, timeout=900)
        wall_s = time.monotonic() - t0
        stdout = (r.stdout.decode("utf-8", errors="replace") if isinstance(r.stdout, bytes)
                  else (r.stdout or ""))
        stderr = (r.stderr.decode("utf-8", errors="replace") if isinstance(r.stderr, bytes)
                  else (r.stderr or ""))
        return {
            "argv": argv,
            "returncode": r.returncode,
            "stdout": stdout[:4000],
            "stderr": stderr[:4000],
            "wall_s": wall_s,
            "exception": None,
            "cache_dir": cache_dir,
        }
    except Exception as e:
        wall_s = time.monotonic() - t0
        return {
            "argv": argv,
            "returncode": -1,
            "stdout": "",
            "stderr": "",
            "wall_s": wall_s,
            "exception": f"{type(e).__name__}: {e}",
            "cache_dir": cache_dir,
        }

def _prism_mcp_config(repo_root: str, *, no_cache: bool = False,
                      cache_dir: str | None = None) -> str:
    """Write a per-checkout claude MCP config pointing prism-mcp at THIS repo_root (the pinned
    worktree). Mirrors CodexRunner's per-checkout --repo. Returns the temp config path."""
    cfg = {"mcpServers": {"prism": {"command": _prism_mcp_bin(),
                                    "args": prism_mcp_args(repo_root, no_cache=no_cache,
                                                           cache_dir=cache_dir)}}}
    fd, path = tempfile.mkstemp(prefix="tc-mcp-", suffix=".json")
    with os.fdopen(fd, "w") as f:
        json.dump(cfg, f)
    return path


def warm_gate_check(repo_root: str, *, cache_dir: str | None = None,
                    prism_mcp_bin: str | None = None,
                    timeout_s: float = 15.0) -> dict:
    """F4 warm-initialize GATE: spawn a throwaway `prism-mcp --repo repo_root --eager
    [--cache-dir cache_dir]`, run the JSON-RPC handshake (initialize ->
    notifications/initialized -> tools/list) over stdio, and assert it completes within
    *timeout_s* AND tools/list returns a non-empty tool set.

    `--eager` is deliberate: lazy initialization answers without an index, so it cannot
    prove the shared cache is warm. This is the loud-failure guarantee: if the arm's
    prism-mcp would NOT actually start warm (cold CPG build racing the host's MCP timeout, a stale/mismatched cache, a
    crashed server, ...), this catches it BEFORE the agent is launched — a 0-dose arm
    (agent silently ran with no prism tools available) is worse than a failed cell.

    Never raises; always returns a telemetry dict:
      {"ok": bool, "wall_s": float, "tools_count": int, "argv": [...], "error": str|None}
    "ok" is True iff the handshake completed and tools/list returned >=1 tool.
    """
    import queue
    import threading

    bin_path = prism_mcp_bin or _prism_mcp_bin()
    # absolute repo_root: the gate is spawned with cwd=repo_root (mirrors the agents), so a relative
    # root would make prism-mcp see `<root>/<root>` (PR #172 re-review).
    repo_root = os.path.abspath(repo_root)
    argv = [bin_path, "--repo", repo_root, "--eager"]
    if cache_dir:
        # absolute: this process is spawned with cwd=repo_root (mirrors the agents); a relative
        # path would resolve inside the checkout and miss the prewarmed cache (fail loud, but wrongly).
        argv += ["--cache-dir", os.path.abspath(cache_dir)]

    t0 = time.monotonic()
    try:
        # cwd=repo_root mirrors how the agents spawn prism-mcp (session cwd = the checkout), so a
        # cwd-relative --cache-dir that would miss in the agent's session ALSO misses here and the
        # gate fails loud instead of passing from the harness cwd (2026-08-21 slate void).
        proc = subprocess.Popen(
            argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, bufsize=1, cwd=repo_root,
        )
    except Exception as e:
        return {"ok": False, "wall_s": time.monotonic() - t0, "tools_count": 0,
               "argv": argv, "error": f"spawn failed: {type(e).__name__}: {e}"}

    lines: "queue.Queue[str]" = queue.Queue()

    def _reader() -> None:
        try:
            for line in proc.stdout:
                lines.put(line)
        except Exception:
            pass

    threading.Thread(target=_reader, daemon=True).start()

    def _remaining() -> float:
        return max(0.0, timeout_s - (time.monotonic() - t0))

    try:
        init_req = {
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "tc-warm-gate", "version": "1"},
            },
        }
        proc.stdin.write(json.dumps(init_req) + "\n")
        proc.stdin.flush()

        init_line = lines.get(timeout=_remaining())
        init_resp = json.loads(init_line)
        if "result" not in init_resp:
            raise RuntimeError(f"initialize did not return a result: {init_line[:200]!r}")

        notif = {"jsonrpc": "2.0", "method": "notifications/initialized"}
        proc.stdin.write(json.dumps(notif) + "\n")
        proc.stdin.flush()

        tools_req = {"jsonrpc": "2.0", "id": 2, "method": "tools/list"}
        proc.stdin.write(json.dumps(tools_req) + "\n")
        proc.stdin.flush()

        tools_line = lines.get(timeout=_remaining())
        tools_resp = json.loads(tools_line)
        tools = (tools_resp.get("result") or {}).get("tools", [])
        wall_s = time.monotonic() - t0
        ok = len(tools) > 0
        return {
            "ok": ok, "wall_s": wall_s, "tools_count": len(tools), "argv": argv,
            "error": None if ok else "tools/list returned an empty tool set",
        }
    except queue.Empty:
        return {"ok": False, "wall_s": time.monotonic() - t0, "tools_count": 0,
               "argv": argv,
               "error": f"timed out after {timeout_s}s waiting for the prism-mcp handshake"}
    except Exception as e:
        return {"ok": False, "wall_s": time.monotonic() - t0, "tools_count": 0,
               "argv": argv, "error": f"{type(e).__name__}: {e}"}
    finally:
        try:
            proc.kill()
            proc.wait(timeout=5)
        except Exception:
            pass


def build_codex_cmd(variant: Variant, *, repo: str, no_cache: bool = False,
                    cache_dir: str | None = None) -> list[str]:
    # codex MCP is inline `-c mcp_servers.prism...`; OFF omits it. `-` reads prompt from stdin.
    cmd = ["codex", "exec", "-m", cli_model_flag(variant.model), "-C", repo, "-s", "workspace-write", "-"]
    if variant.prism:
        mcp_args_json = json.dumps(prism_mcp_args(repo, no_cache=no_cache, cache_dir=cache_dir))
        cmd[6:6] = ["-c", f"mcp_servers.prism.command={_prism_mcp_bin()}",
                    "-c", f"mcp_servers.prism.args={mcp_args_json}"]
    return cmd

def build_claude_cmd(variant: Variant, *, mcp_cfg: str) -> list[str]:
    cmd = ["claude", "-p", "--output-format", "stream-json", "--verbose",
           "--model", cli_model_flag(variant.model)]
    if variant.prism:
        cmd += ["--mcp-config", mcp_cfg, "--strict-mcp-config"]
    return cmd

_TIMEOUT = 1800  # 30 min per arm call

def _skill_src() -> str:
    """Absolute path to the prism-code-navigation skill directory in this repo."""
    return os.path.normpath(os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "..", "skills", "prism-code-navigation"))


class ClaudeRunner:
    """ArmRunner via `claude -p --output-format stream-json`. prism ON = --mcp-config.

    Uses parse_claude_stream_json so real mcp__prism__* tool calls are visible and
    counted; used_prism is set from prism_calls > 0 (not a variant.prism heuristic).

    mcp_cfg: optional static config path override.  When None (default), a per-checkout
    config pointing at repo_root is built on each run() call (mirrors CodexRunner's
    per-checkout --repo).  Pass an explicit path only for testing or special overrides.
    lsp_deny_dir: when set, prepended to PATH for lsp=False variants (deny-shim enforcement).
    no_cache: when True, pass --no-cache to prism-mcp so stale CPGs cannot survive arm resets.
    cache_dir (F2): explicit --cache-dir threaded to the per-checkout prism-mcp config so it
    reads from the SAME nav cache the harness's prewarm wrote to. Ignored when no_cache=True.

    CLAUDE_CONFIG_DIR isolation (both arms):
      - prism-ON:  _arm_config_dir() — skill + mcp__prism allow + seeded creds.
      - prism-OFF: _status_quo_config_dir() — no skill, no mcp__prism, deny Write/Edit,
                   seeded creds.  The ONLY difference between arms is the prism bundle.
    The per-checkout --mcp-config (with --no-cache) is kept separately for the prism-ON
    arm's actual MCP server endpoint.
    """
    def __init__(self, mcp_cfg: str | None = None, lsp_deny_dir: str | None = None,
                 no_cache: bool = False, cache_dir: str | None = None):
        self.mcp_cfg = mcp_cfg          # optional static override; per-checkout config is default
        self.lsp_deny_dir = lsp_deny_dir
        self.no_cache = no_cache
        self.cache_dir = cache_dir
        self._cfg_dir: str | None = None         # lazily built, cached: prism-ON config
        self._sq_cfg_dir: str | None = None      # lazily built, cached: status-quo OFF config

    def _arm_config_dir(self) -> str:
        """Return a cached, repo-independent CLAUDE_CONFIG_DIR with the prism skill + allow-list.

        Built once on the first prism-ON call and reused for all subsequent runs.
        Uses mcp_repo="." as a placeholder — the config_dir is repo-independent (skill + perms
        + creds only); the actual per-checkout MCP server is supplied via --mcp-config.
        """
        if self._cfg_dir is None:
            cfg = build_isolated_config(
                skill_src=_skill_src(),
                mcp_repo=".",
                prism_mcp_bin=_prism_mcp_bin(),
                include_skill_and_mcp=True,
            )
            self._cfg_dir = cfg.config_dir
        return self._cfg_dir

    def _status_quo_config_dir(self) -> str:
        """Return a cached, repo-independent CLAUDE_CONFIG_DIR for the prism-OFF baseline.

        Contains ONLY: settings.json with deny=[Write,Edit], allow=[Read,Grep,Glob,Bash]
        (no mcp__prism), hooks={}, empty skills dir, and seeded credentials.
        The only difference between this dir and _arm_config_dir is the prism bundle.
        """
        if self._sq_cfg_dir is None:
            cfg = build_isolated_config(
                skill_src=_skill_src(),
                mcp_repo=".",
                prism_mcp_bin=_prism_mcp_bin(),
                include_skill_and_mcp=False,
            )
            self._sq_cfg_dir = cfg.config_dir
        return self._sq_cfg_dir

    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cfg = self.mcp_cfg if self.mcp_cfg else (
            _prism_mcp_config(repo_root, no_cache=self.no_cache, cache_dir=self.cache_dir)
            if variant.prism else "")
        cmd = build_claude_cmd(variant, mcp_cfg=cfg) + [prompt]
        t0 = time.monotonic()
        env = dict(os.environ)
        # Both arms run in isolated CLAUDE_CONFIG_DIRs so neither inherits user hooks/skills.
        # The only difference is the prism bundle (skill + mcp__prism allow).
        if variant.prism:
            env["CLAUDE_CONFIG_DIR"] = self._arm_config_dir()
            # F3: generous MCP handshake/tool timeouts (ms) so a cold-CPG-build prism-mcp
            # is never killed by claude's default MCP timeout before it can respond.
            env["MCP_TIMEOUT"] = "600000"
            env["MCP_TOOL_TIMEOUT"] = "600000"
        else:
            env["CLAUDE_CONFIG_DIR"] = self._status_quo_config_dir()
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=repo_root, timeout=_TIMEOUT, env=env)
        stderr_text = (proc.stderr or "").strip()
        stdout_text = proc.stdout or ""
        if proc.returncode != 0 or not stdout_text.strip():
            raise ArmRunError(
                argv=list(cmd),
                returncode=proc.returncode,
                stderr=stderr_text,
                stdout=stdout_text,
            )
        r = parse_claude_stream_json(stdout_text)
        flags = classify_tools(r.commands)
        prism_calls = r.prism_calls
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=prism_calls > 0,
                         prism_calls=prism_calls, dose=r.dose,
                         low_dose=prism_calls > 0 and prism_calls <= 1,
                         commands=r.commands, in_tokens=r.input_tokens, cost_usd=r.cost_usd,
                         raw_stdout=stdout_text,
                         argv=list(cmd),
                         returncode=proc.returncode,
                         stderr=stderr_text,
                         cwd=repo_root,
                         **flags)

class CodexRunner:
    """ArmRunner via `codex exec --json` (prompt on stdin).

    prism ON:  CODEX_HOME is set to an isolated home built by build_isolated_codex_home
               (skill + MCP config.toml for this repo_root + auth).  Inline -c mcp_servers.prism.*
               args are NOT injected (the MCP server is already defined in CODEX_HOME/config.toml).
               A fresh CODEX_HOME is created per run() call because mcp_repo (= repo_root) varies.
    prism OFF: CODEX_HOME is set to an isolated status-quo home (auth only; no skill, no MCP
               config.toml).  This prevents the user's ~/.codex skills/hooks from contaminating
               the baseline — the only difference between arms is the prism bundle.

    used_prism is set from prism_calls > 0 (real mcp_tool_call events with server=='prism').
    lsp_deny_dir: when set, prepended to PATH for lsp=False variants (deny-shim enforcement).
    no_cache: when True, pass --no-cache to prism-mcp so stale CPGs cannot survive arm resets.
    cache_dir (F2): explicit --cache-dir threaded into CODEX_HOME/config.toml's
    [mcp_servers.prism].args so codex's prism-mcp reads from the SAME nav cache the
    harness's prewarm wrote to.
    """
    def __init__(self, lsp_deny_dir: str | None = None, no_cache: bool = False,
                 cache_dir: str | None = None):
        self.lsp_deny_dir = lsp_deny_dir
        self.no_cache = no_cache
        self.cache_dir = cache_dir
        self._sq_codex_home: str | None = None   # lazily built, cached: status-quo OFF home

    def _status_quo_codex_home(self) -> str:
        """Return a cached, repo-independent CODEX_HOME for the prism-OFF baseline.

        Contains ONLY auth.json (seeded); no config.toml, no skills.
        The only difference between this home and the prism-ON home is the prism bundle.
        Cached because mcp_repo is irrelevant for the OFF home (no MCP config written).
        """
        if self._sq_codex_home is None:
            self._sq_codex_home = build_isolated_codex_home(
                skill_src=_skill_src(),
                mcp_repo=".",
                prism_mcp_bin=_prism_mcp_bin(),
                include_skill_and_mcp=False,
            )
        return self._sq_codex_home

    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        # For prism-ON arms: build the base command WITHOUT inline -c mcp_servers.prism.* args
        # (CODEX_HOME/config.toml defines the server); for prism-OFF arms: no prism args at all.
        # We pass a prism=False variant to build_codex_cmd when using CODEX_HOME so the function
        # does not inject the inline -c flags (CODEX_HOME is the MCP source of truth).
        if variant.prism:
            base_cmd = build_codex_cmd(replace(variant, prism=False),
                                       repo=repo_root, no_cache=self.no_cache,
                                       cache_dir=self.cache_dir)
        else:
            base_cmd = build_codex_cmd(variant, repo=repo_root, no_cache=self.no_cache,
                                       cache_dir=self.cache_dir)
        cmd = ["codex", "exec", "--json"] + base_cmd[2:]  # codex exec --json ... (robust vs index drift)
        t0 = time.monotonic()
        env = dict(os.environ)
        if variant.prism:
            env["CODEX_HOME"] = build_isolated_codex_home(
                skill_src=_skill_src(),
                mcp_repo=repo_root,
                prism_mcp_bin=_prism_mcp_bin(),
                include_skill_and_mcp=True,
                cache_dir=self.cache_dir,
            )
        else:
            # OFF arm: isolated status-quo home (auth only; no skill, no MCP config).
            # Prevents user's ~/.codex skills/hooks from contaminating the baseline.
            env["CODEX_HOME"] = self._status_quo_codex_home()
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              cwd=repo_root, timeout=_TIMEOUT, env=env)
        stderr_text = (proc.stderr or "").strip()
        stdout_text = proc.stdout or ""
        if proc.returncode != 0 or not stdout_text.strip():
            raise ArmRunError(
                argv=list(cmd),
                returncode=proc.returncode,
                stderr=stderr_text,
                stdout=stdout_text,
            )
        r = parse_codex_jsonl(stdout_text)
        flags = classify_tools(r.commands)
        prism_calls = r.prism_calls
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=prism_calls > 0,
                         prism_calls=prism_calls, dose=r.dose,
                         low_dose=prism_calls > 0 and prism_calls <= 1,
                         commands=r.commands, in_tokens=r.input_tokens, cost_usd=r.cost_usd,
                         raw_stdout=stdout_text,
                         argv=list(cmd),
                         returncode=proc.returncode,
                         stderr=stderr_text,
                         cwd=repo_root,
                         **flags)

class FakeArmRunner:
    """Deterministic runner keyed by variant.id -> canned text (spec §6 fakes-drive-tests).
    prism_calls/dose/low_dose are all zero/empty — this runner never issues real tool calls.
    used_prism = prism_calls > 0 = False (consistent with the real runners' contract)."""
    def __init__(self, by_id: dict[str, str]):
        self._by_id = by_id
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        from .model import Dose
        text = self._by_id.get(variant.id, "")
        return ArmOutput(variant=variant, text=text, citations=parse_citations(text),
                         tokens=len(text.split()), tool_calls=0, wall_s=0.0,
                         used_prism=False,
                         prism_calls=0, dose=Dose(), low_dose=False)

class RoutingArmRunner:
    """Dispatch a variant to its CLI runner by model family (Opus->claude, gpt->codex)."""
    def __init__(self, claude, codex):
        self.claude, self.codex = claude, codex
    def run(self, variant, stage, prompt, repo_root):
        if variant.family == "anthropic":
            runner = self.claude
        elif variant.family == "openai":
            runner = self.codex
        else:
            raise ValueError(
                f"RoutingArmRunner: no CLI registered for family {variant.family!r} "
                f"(model {variant.model!r})")
        return runner.run(variant, stage, prompt, repo_root)
