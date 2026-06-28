"""Single-shot model-call seam for judges (Phase-1c). NO MCP/tools — a judge must not use prism.
Opus→claude -p --output-format json; gpt→codex exec --json. Returns the model's text answer."""
from __future__ import annotations
import subprocess
from .parse import parse_claude_json, parse_codex_jsonl

_TIMEOUT = 600

# Variant.model -> (cli, cli-model-flag). Verify flag values live (claude alias 'opus'; codex 'gpt-5.5').
# SINGLE SOURCE OF TRUTH for the CLI model flag — both the judges (live_ask) AND the arm runners
# (arm_runner.build_*_cmd) MUST map through this, or they pass an invalid flag (e.g. claude rejects
# "opus-4.8", needs the "opus" alias — caught by the 2026-06-24 live smoke).
MODEL_CLI = {
    "opus-4.8": ("claude", "opus"),
    "gpt-5.5": ("codex", "gpt-5.5"),
}

def cli_model_flag(model: str) -> str:
    """The CLI `--model`/`-m` flag value for a Variant.model. Used by judges AND arm runners."""
    return MODEL_CLI[model][1]

def live_ask(model: str, prompt: str) -> str:
    cli, flag = MODEL_CLI[model]
    if cli == "claude":
        # --strict-mcp-config: ignore the user's DEFAULT MCP servers (the prism-dev plugin).
        # Without it, each judge `claude -p` launches prism-mcp, which eagerly builds the CPG
        # before answering — a ~3s relevance call balloons to minutes (one citation observed
        # hanging >4min). Judges are designed tool-free (judges_live.py); this enforces it.
        # No --mcp-config is passed, so strict mode yields ZERO MCP servers. Prompt stays the
        # trailing positional (the flag is boolean).
        cmd = ["claude", "-p", "--output-format", "json", "--model", flag,
               "--strict-mcp-config", prompt]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"claude judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
        try:
            return parse_claude_json(proc.stdout).text
        except ValueError as e:
            raise RuntimeError(f"claude judge output unparseable: {e}") from e
    cmd = ["codex", "exec", "--json", "-m", flag, "-s", "read-only", "-"]
    proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True, timeout=_TIMEOUT)
    if proc.returncode != 0 or not proc.stdout.strip():
        raise RuntimeError(f"codex judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
    try:
        return parse_codex_jsonl(proc.stdout).text
    except ValueError as e:
        raise RuntimeError(f"codex judge output unparseable: {e}") from e
