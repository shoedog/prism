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


def prism_mcp_args(repo_root: str, *, no_cache: bool = False) -> list[str]:
    """Build the prism-mcp server arg list for a given repo root.

    Used at BOTH the claude MCP-config site and the codex -c mcp_servers.prism.args site
    so both runners honour the same no_cache flag.
    """
    args = ["--repo", repo_root]
    if no_cache:
        args.append("--no-cache")
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


def run_arm_isolated(
    runner,
    *,
    checkout,
    variant: Variant,
    stage: str = "spec",
    prompt: str = "",
    no_cache: bool = True,
    prewarm: bool = False,
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

    Returns IsolatedArmResult so callers can assert on cache_mode and mcp_args.
    """
    root = str(checkout.root)
    mcp_args = prism_mcp_args(root, no_cache=no_cache)
    cache_mode = "no-cache" if no_cache else "cached"

    _reset_clean(root)
    try:
        if prewarm and variant.prism:
            _prewarm_cpg(root)
        out = runner.run(variant, stage, prompt, root)
    finally:
        _reset_clean(root)

    return IsolatedArmResult(out=out, cache_mode=cache_mode, mcp_args=mcp_args)


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


def _prewarm_cpg(root: str) -> None:
    """Build the prism nav CPG cache for `root` so the arm's prism-mcp starts warm.
    (Cold build can exceed claude's MCP handshake timeout on large repos.)"""
    try:
        subprocess.run([_prism_bin(), "nav", "repo-map", "--repo", root],
                       capture_output=True, timeout=900)
    except Exception:
        pass  # best-effort; if warming fails the arm still runs (just slower / may miss prism)

def _prism_mcp_config(repo_root: str, *, no_cache: bool = False) -> str:
    """Write a per-checkout claude MCP config pointing prism-mcp at THIS repo_root (the pinned
    worktree). Mirrors CodexRunner's per-checkout --repo. Returns the temp config path."""
    cfg = {"mcpServers": {"prism": {"command": _prism_mcp_bin(),
                                    "args": prism_mcp_args(repo_root, no_cache=no_cache)}}}
    fd, path = tempfile.mkstemp(prefix="tc-mcp-", suffix=".json")
    with os.fdopen(fd, "w") as f:
        json.dump(cfg, f)
    return path

def build_codex_cmd(variant: Variant, *, repo: str, no_cache: bool = False) -> list[str]:
    # codex MCP is inline `-c mcp_servers.prism...`; OFF omits it. `-` reads prompt from stdin.
    cmd = ["codex", "exec", "-m", cli_model_flag(variant.model), "-C", repo, "-s", "workspace-write", "-"]
    if variant.prism:
        mcp_args_json = json.dumps(prism_mcp_args(repo, no_cache=no_cache))
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

    For prism-ON arms, CLAUDE_CONFIG_DIR is set to a lazily-built, cached, repo-independent
    config dir containing the prism-code-navigation skill, a settings.json with
    permissions.allow including mcp__prism, and seeded credentials.  The per-checkout
    --mcp-config (with --no-cache) is kept separately for the actual MCP server endpoint.
    """
    def __init__(self, mcp_cfg: str | None = None, lsp_deny_dir: str | None = None,
                 no_cache: bool = False):
        self.mcp_cfg = mcp_cfg          # optional static override; per-checkout config is default
        self.lsp_deny_dir = lsp_deny_dir
        self.no_cache = no_cache
        self._cfg_dir: str | None = None  # lazily built, cached across run() calls

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
            )
            self._cfg_dir = cfg.config_dir
        return self._cfg_dir

    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cfg = self.mcp_cfg if self.mcp_cfg else (
            _prism_mcp_config(repo_root, no_cache=self.no_cache) if variant.prism else "")
        cmd = build_claude_cmd(variant, mcp_cfg=cfg) + [prompt]
        t0 = time.monotonic()
        env = dict(os.environ)
        if variant.prism:
            env["CLAUDE_CONFIG_DIR"] = self._arm_config_dir()
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=repo_root, timeout=_TIMEOUT, env=env)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"arm exited {proc.returncode}: {(proc.stderr or '').strip()[:400]}")
        r = parse_claude_stream_json(proc.stdout)
        flags = classify_tools(r.commands)
        prism_calls = r.prism_calls
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=prism_calls > 0,
                         prism_calls=prism_calls, dose=r.dose,
                         low_dose=prism_calls > 0 and prism_calls <= 1,
                         commands=r.commands, in_tokens=r.input_tokens, cost_usd=r.cost_usd,
                         **flags)

class CodexRunner:
    """ArmRunner via `codex exec --json` (prompt on stdin).

    prism ON:  CODEX_HOME is set to an isolated home built by build_isolated_codex_home
               (skill + MCP config.toml for this repo_root + auth).  Inline -c mcp_servers.prism.*
               args are NOT injected (the MCP server is already defined in CODEX_HOME/config.toml).
               A fresh CODEX_HOME is created per run() call because mcp_repo (= repo_root) varies.
    prism OFF: CODEX_HOME is not set; no inline -c mcp_servers.prism.* args.

    used_prism is set from prism_calls > 0 (real mcp_tool_call events with server=='prism').
    lsp_deny_dir: when set, prepended to PATH for lsp=False variants (deny-shim enforcement).
    no_cache: when True, pass --no-cache to prism-mcp so stale CPGs cannot survive arm resets.
    """
    def __init__(self, lsp_deny_dir: str | None = None, no_cache: bool = False):
        self.lsp_deny_dir = lsp_deny_dir
        self.no_cache = no_cache

    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        # For prism-ON arms: build the base command WITHOUT inline -c mcp_servers.prism.* args
        # (CODEX_HOME/config.toml defines the server); for prism-OFF arms: no prism args at all.
        # We pass a prism=False variant to build_codex_cmd when using CODEX_HOME so the function
        # does not inject the inline -c flags (CODEX_HOME is the MCP source of truth).
        if variant.prism:
            base_cmd = build_codex_cmd(replace(variant, prism=False),
                                       repo=repo_root, no_cache=self.no_cache)
        else:
            base_cmd = build_codex_cmd(variant, repo=repo_root, no_cache=self.no_cache)
        cmd = ["codex", "exec", "--json"] + base_cmd[2:]  # codex exec --json ... (robust vs index drift)
        t0 = time.monotonic()
        env = dict(os.environ)
        if variant.prism:
            env["CODEX_HOME"] = build_isolated_codex_home(
                skill_src=_skill_src(),
                mcp_repo=repo_root,
                prism_mcp_bin=_prism_mcp_bin(),
            )
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              cwd=repo_root, timeout=_TIMEOUT, env=env)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"arm exited {proc.returncode}: {(proc.stderr or '').strip()[:400]}")
        r = parse_codex_jsonl(proc.stdout)
        flags = classify_tools(r.commands)
        prism_calls = r.prism_calls
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=prism_calls > 0,
                         prism_calls=prism_calls, dose=r.dose,
                         low_dose=prism_calls > 0 and prism_calls <= 1,
                         commands=r.commands, in_tokens=r.input_tokens, cost_usd=r.cost_usd,
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
