"""Concrete model drivers + a fake (spec §3). Command builders are unit-tested on the
ARGV they assemble (the live subprocess call is exercised only in an integration run).
prism ON = MCP config passed; OFF = omitted. Mirrors tier_a/sut.py's subprocess style."""
from __future__ import annotations
import json
import os
import subprocess
import tempfile
import time
from .model import Variant, ArmOutput
from .citations import parse_citations
from .parse import parse_claude_json, parse_codex_jsonl
from .llm import cli_model_flag


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

def _prism_mcp_config(repo_root: str) -> str:
    """Write a per-checkout claude MCP config pointing prism-mcp at THIS repo_root (the pinned
    worktree). Mirrors CodexRunner's per-checkout --repo. Returns the temp config path."""
    cfg = {"mcpServers": {"prism": {"command": _prism_mcp_bin(), "args": ["--repo", repo_root]}}}
    fd, path = tempfile.mkstemp(prefix="tc-mcp-", suffix=".json")
    with os.fdopen(fd, "w") as f:
        json.dump(cfg, f)
    return path

def build_codex_cmd(variant: Variant, *, repo: str) -> list[str]:
    # codex MCP is inline `-c mcp_servers.prism...`; OFF omits it. `-` reads prompt from stdin.
    cmd = ["codex", "exec", "-m", cli_model_flag(variant.model), "-C", repo, "-s", "workspace-write", "-"]
    if variant.prism:
        cmd[6:6] = ["-c", f"mcp_servers.prism.command={_prism_mcp_bin()}",
                    "-c", f'mcp_servers.prism.args=["--repo","{repo}"]']
    return cmd

def build_claude_cmd(variant: Variant, *, mcp_cfg: str) -> list[str]:
    cmd = ["claude", "-p", "--output-format", "json", "--model", cli_model_flag(variant.model)]
    if variant.prism:
        cmd += ["--mcp-config", mcp_cfg, "--strict-mcp-config"]
    return cmd

_TIMEOUT = 1800  # 30 min per arm call

class ClaudeRunner:
    """ArmRunner via `claude -p --output-format json`. prism ON = --mcp-config.

    mcp_cfg: optional static config path override.  When None (default), a per-checkout
    config pointing at repo_root is built on each run() call (mirrors CodexRunner's
    per-checkout --repo).  Pass an explicit path only for testing or special overrides.
    lsp_deny_dir: when set, prepended to PATH for lsp=False variants (deny-shim enforcement).
    """
    def __init__(self, mcp_cfg: str | None = None, lsp_deny_dir: str | None = None):
        self.mcp_cfg = mcp_cfg          # optional static override; per-checkout config is default
        self.lsp_deny_dir = lsp_deny_dir
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cfg = self.mcp_cfg if self.mcp_cfg else (_prism_mcp_config(repo_root) if variant.prism else "")
        cmd = build_claude_cmd(variant, mcp_cfg=cfg) + [prompt]
        t0 = time.monotonic()
        env = dict(os.environ)
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=repo_root, timeout=_TIMEOUT, env=env)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"arm exited {proc.returncode}: {(proc.stderr or '').strip()[:400]}")
        r = parse_claude_json(proc.stdout)
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=variant.prism and r.tool_calls > 0)

class CodexRunner:
    """ArmRunner via `codex exec --json` (prompt on stdin). prism ON = inline -c mcp_servers.
    lsp_deny_dir: when set, prepended to PATH for lsp=False variants (deny-shim enforcement).
    """
    def __init__(self, lsp_deny_dir: str | None = None):
        self.lsp_deny_dir = lsp_deny_dir
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cmd = build_codex_cmd(variant, repo=repo_root)
        cmd = ["codex", "exec", "--json"] + cmd[2:]  # codex exec --json ... (robust vs index drift)
        t0 = time.monotonic()
        env = dict(os.environ)
        if not variant.lsp and self.lsp_deny_dir:
            env["PATH"] = self.lsp_deny_dir + os.pathsep + env["PATH"]
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              cwd=repo_root, timeout=_TIMEOUT, env=env)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"arm exited {proc.returncode}: {(proc.stderr or '').strip()[:400]}")
        r = parse_codex_jsonl(proc.stdout)
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=variant.prism and r.tool_calls > 0)

class FakeArmRunner:
    """Deterministic runner keyed by variant.id -> canned text (spec §6 fakes-drive-tests)."""
    def __init__(self, by_id: dict[str, str]):
        self._by_id = by_id
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        text = self._by_id.get(variant.id, "")
        return ArmOutput(variant=variant, text=text, citations=parse_citations(text),
                         tokens=len(text.split()), tool_calls=0, wall_s=0.0,
                         used_prism="prism" in text.lower() if variant.prism else False)

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
