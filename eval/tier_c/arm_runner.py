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


def _prism_mcp_config(repo_root: str) -> str:
    """Write a per-checkout claude MCP config pointing prism-mcp at THIS repo_root (the pinned
    worktree). Mirrors CodexRunner's per-checkout --repo. Returns the temp config path."""
    cfg = {"mcpServers": {"prism": {"command": "prism-mcp", "args": ["--repo", repo_root]}}}
    fd, path = tempfile.mkstemp(prefix="tc-mcp-", suffix=".json")
    with os.fdopen(fd, "w") as f:
        json.dump(cfg, f)
    return path

def build_codex_cmd(variant: Variant, *, repo: str) -> list[str]:
    # codex MCP is inline `-c mcp_servers.prism...`; OFF omits it. `-` reads prompt from stdin.
    cmd = ["codex", "exec", "-m", variant.model, "-C", repo, "-s", "workspace-write", "-"]
    if variant.prism:
        cmd[6:6] = ["-c", "mcp_servers.prism.command=prism-mcp",
                    "-c", f'mcp_servers.prism.args=["--repo","{repo}"]']
    return cmd

def build_claude_cmd(variant: Variant, *, mcp_cfg: str) -> list[str]:
    cmd = ["claude", "-p", "--output-format", "json", "--model", variant.model]
    if variant.prism:
        cmd += ["--mcp-config", mcp_cfg, "--strict-mcp-config"]
    return cmd

_TIMEOUT = 1800  # 30 min per arm call

class ClaudeRunner:
    """ArmRunner via `claude -p --output-format json`. prism ON = --mcp-config.

    mcp_cfg: optional static config path override.  When None (default), a per-checkout
    config pointing at repo_root is built on each run() call (mirrors CodexRunner's
    per-checkout --repo).  Pass an explicit path only for testing or special overrides.
    """
    def __init__(self, mcp_cfg: str | None = None):
        self.mcp_cfg = mcp_cfg          # optional static override; per-checkout config is default
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cfg = self.mcp_cfg if self.mcp_cfg else (_prism_mcp_config(repo_root) if variant.prism else "")
        cmd = build_claude_cmd(variant, mcp_cfg=cfg) + [prompt]
        t0 = time.monotonic()
        proc = subprocess.run(cmd, capture_output=True, text=True, cwd=repo_root, timeout=_TIMEOUT)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"arm exited {proc.returncode}: {(proc.stderr or '').strip()[:400]}")
        r = parse_claude_json(proc.stdout)
        return ArmOutput(variant=variant, text=r.text, citations=parse_citations(r.text),
                         tokens=r.output_tokens, tool_calls=r.tool_calls, wall_s=time.monotonic() - t0,
                         used_prism=variant.prism and r.tool_calls > 0)

class CodexRunner:
    """ArmRunner via `codex exec --json` (prompt on stdin). prism ON = inline -c mcp_servers."""
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        cmd = build_codex_cmd(variant, repo=repo_root)
        cmd = ["codex", "exec", "--json"] + cmd[2:]  # codex exec --json ... (robust vs index drift)
        t0 = time.monotonic()
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              cwd=repo_root, timeout=_TIMEOUT)
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
