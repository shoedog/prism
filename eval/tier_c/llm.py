"""Single-shot model-call seam for judges (Phase-1c). NO MCP/tools — a judge must not use prism.
Opus→claude -p --output-format json; gpt→codex exec --json. Returns the model's text answer."""
from __future__ import annotations
import subprocess
import tempfile
from .parse import parse_claude_json, parse_codex_jsonl

_TIMEOUT = 600

# Variant.model -> (cli, cli-model-flag). Verify flag values live (claude alias 'opus'; codex 'gpt-5.5').
# SINGLE SOURCE OF TRUTH for the CLI model flag — both the judges (live_ask) AND the arm runners
# (arm_runner.build_*_cmd) MUST map through this, or they pass an invalid flag (e.g. claude rejects
# "opus-4.8", needs the "opus" alias — caught by the 2026-06-24 live smoke).
MODEL_CLI = {
    "opus-4.8": ("claude", "opus"),
    "sonnet-4.6": ("claude", "sonnet"),
    "haiku-4.5": ("claude", "haiku"),
    "gpt-5.5": ("codex", "gpt-5.5"),
    "gpt-5.5-xhigh": ("codex", "gpt-5.5"),
}

# Models that require a reasoning-effort flag: model-name -> effort level.
_MODEL_EFFORT: dict[str, str] = {
    "gpt-5.5-xhigh": "xhigh",
}

# The relevance/rank oracle model. Sonnet (not opus): owner testing found the smaller
# models follow the "answer YES or NO and nothing else" instruction reliably, whereas
# opus intermittently went AGENTIC on the judge prompt — using built-in tools to
# investigate the repo, taking minutes and returning prose (mis-parsed as not-YES,
# corrupting the score). Sonnet is also faster + cheaper. Swap to "haiku-4.5" for max
# speed/cost. (Arms still run on their own model; this is only the scoring oracle.)
JUDGE_MODEL = "sonnet-4.6"

# Ensemble tiebreaker: consulted ONLY when the two sonnet judges disagree (see ensemble.py).
# Opus is authoritative-but-pricier, so it adjudicates contested calls, not every call.
JUDGE_TIEBREAKER = "opus-4.8"

# A judge is a pure text relevance call — it must NEVER use tools. --strict-mcp-config
# drops MCP, but the default config still exposes the built-in tools; disallow them all
# so the judge cannot go agentic regardless of model. Belt-and-suspenders with JUDGE_MODEL.
_JUDGE_DISALLOWED_TOOLS = [
    "Bash", "Read", "Grep", "Glob", "Edit", "Write", "MultiEdit", "NotebookEdit",
    "WebFetch", "WebSearch", "Task", "TodoWrite", "BashOutput", "KillShell",
    "SlashCommand", "ExitPlanMode",
]

def cli_model_flag(model: str) -> str:
    """The CLI `--model`/`-m` flag value for a Variant.model. Used by judges AND arm runners."""
    return MODEL_CLI[model][1]

def live_ask(model: str, prompt: str) -> str:
    cli, flag = MODEL_CLI[model]
    if cli == "claude":
        # Judge isolation (two independent failure modes, both observed live):
        #  1. --strict-mcp-config: ignore the user's DEFAULT MCP servers (the prism-dev plugin).
        #     Without it, each judge `claude -p` launches prism-mcp, which eagerly builds the CPG
        #     before answering — a relevance call balloons to minutes (no --mcp-config => 0 servers).
        #  2. --disallowed-tools <all>: a judge must be tool-free, but the default config still
        #     exposes built-in tools after MCP is dropped. Opus intermittently went AGENTIC
        #     (used Read/Grep to investigate, ~5min, returned prose not YES/NO). Disallow them all.
        # The prompt goes on STDIN (not a positional) so the variadic --disallowed-tools cannot
        # swallow it. See also JUDGE_MODEL (sonnet) — the model-level half of the same fix.
        cmd = ["claude", "-p", "--output-format", "json", "--model", flag,
               "--strict-mcp-config", "--disallowed-tools", *_JUDGE_DISALLOWED_TOOLS]
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True, timeout=_TIMEOUT)
        if proc.returncode != 0 or not proc.stdout.strip():
            raise RuntimeError(f"claude judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
        try:
            return parse_claude_json(proc.stdout).text
        except ValueError as e:
            raise RuntimeError(f"claude judge output unparseable: {e}") from e
    effort = _MODEL_EFFORT.get(model)
    cmd = ["codex", "exec", "--json", "--skip-git-repo-check", "-m", flag, "-s", "read-only"]
    if effort:
        cmd += ["-c", f"model_reasoning_effort={effort}"]
    cmd += ["-"]
    # Tool-free: run from an empty cwd so the judge cannot explore a repo (it must answer
    # from the inlined code, like the claude judge). Without this codex read files for minutes.
    with tempfile.TemporaryDirectory(prefix="tc-codex-judge-") as _cwd:
        proc = subprocess.run(cmd, input=prompt, capture_output=True, text=True,
                              timeout=_TIMEOUT, cwd=_cwd)
    if proc.returncode != 0 or not proc.stdout.strip():
        raise RuntimeError(f"codex judge exited {proc.returncode}: {(proc.stderr or '').strip()[:300]}")
    try:
        return parse_codex_jsonl(proc.stdout).text
    except ValueError as e:
        raise RuntimeError(f"codex judge output unparseable: {e}") from e
