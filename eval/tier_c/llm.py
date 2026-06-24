"""Single-shot model-call seam for judges (Phase-1c). NO MCP/tools — a judge must not use prism.
Opus→claude -p --output-format json; gpt→codex exec --json. Returns the model's text answer."""
from __future__ import annotations
import subprocess
from .parse import parse_claude_json, parse_codex_jsonl

_TIMEOUT = 600

# Variant.model -> (cli, cli-model-flag). Verify flag values live (claude alias 'opus'; codex 'gpt-5.5').
MODEL_CLI = {
    "opus-4.8": ("claude", "opus"),
    "gpt-5.5": ("codex", "gpt-5.5"),
}

def live_ask(model: str, prompt: str) -> str:
    cli, flag = MODEL_CLI[model]
    if cli == "claude":
        cmd = ["claude", "-p", "--output-format", "json", "--model", flag, prompt]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=_TIMEOUT)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"claude judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
        return parse_claude_json(proc.stdout).text
    cmd = ["codex", "exec", "--json", "-m", flag, "-s", "read-only", "-"]
    proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True, timeout=_TIMEOUT)
    if proc.returncode != 0 or not proc.stdout.strip():
        raise RuntimeError(f"codex judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
    return parse_codex_jsonl(proc.stdout).text
