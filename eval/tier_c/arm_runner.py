"""Concrete model drivers + a fake (spec §3). Command builders are unit-tested on the
ARGV they assemble (the live subprocess call is exercised only in an integration run).
prism ON = MCP config passed; OFF = omitted. Mirrors tier_a/sut.py's subprocess style."""
from __future__ import annotations
from .model import Variant, ArmOutput
from .citations import parse_citations

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

class FakeArmRunner:
    """Deterministic runner keyed by variant.id -> canned text (spec §6 fakes-drive-tests)."""
    def __init__(self, by_id: dict[str, str]):
        self._by_id = by_id
    def run(self, variant: Variant, stage: str, prompt: str, repo_root: str) -> ArmOutput:
        text = self._by_id.get(variant.id, "")
        return ArmOutput(variant=variant, text=text, citations=parse_citations(text),
                         tokens=len(text.split()), tool_calls=0, wall_s=0.0,
                         used_prism="prism" in text.lower() if variant.prism else False)
