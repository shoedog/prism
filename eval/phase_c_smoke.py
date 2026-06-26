#!/usr/bin/env python3
"""Phase C smoke: run ONE codex probe end-to-end through run_trial.

Probe: callers-count-claims  (nav_callers expected)
Model: gpt-5.5
Prints: skill_loaded + prism_calls + one-line answer.
DO NOT run all 12×5 — this is the 1-probe smoke only.

Usage: uv run python phase_c_smoke.py
"""
import os, sys

SKILL_SRC = "/Users/wesleyjinks/code/slicing/skills/prism-code-navigation"
PRISM_MCP_BIN = "/Users/wesleyjinks/code/slicing/target/release/prism-mcp"
EVAL_ROOT = "/Users/wesleyjinks/code/slicing/eval"
RESULTS_ROOT = "/Users/wesleyjinks/code/slicing/eval/adoption/results"
AUTH_SRC = os.path.expanduser("~/.codex/auth.json")

sys.path.insert(0, EVAL_ROOT)

from adoption.goldens import load_probes
from adoption.env import build_isolated_config
from adoption.runner import run_trial

# Load probe
probes = {p.id: p for p in load_probes()}
probe = probes["callers-count-claims"]
print(f"Probe: {probe.id}")
print(f"Prompt: {probe.prompt[:120]}...")
print(f"Repo: {probe.repo}")
print(f"Expected tools: {probe.expected_tools}")
print()

# Build a minimal claude cfg (unused for codex path but required by run_trial signature)
# We use a dummy path since the codex path doesn't read cfg.config_dir or cfg.mcp_cfg
from adoption.env import IsolatedConfig
dummy_cfg = IsolatedConfig(config_dir="/tmp/dummy", mcp_cfg="/tmp/dummy.json")

# Read skill bytes for cache key
skill_bytes = open(os.path.join(SKILL_SRC, "SKILL.md"), "rb").read()

print("Running codex trial (model=gpt-5.5, trial=0)...")
traj = run_trial(
    probe, trial=0,
    cfg=dummy_cfg,
    eval_root=EVAL_ROOT,
    results_root=RESULTS_ROOT,
    skill_bytes=skill_bytes,
    model="gpt-5.5",
    skill_src=SKILL_SRC,
    prism_mcp_bin=PRISM_MCP_BIN,
)

print()
print("=== Smoke result ===")
print(f"skill_loaded:  {traj.loaded_prism_skill()}")
print(f"prism_calls:   {traj.prism_nav_calls()}")
print(f"answer (1 line): {traj.final_text.splitlines()[0] if traj.final_text else '(empty)'}")
print(f"all tool_calls: {[(n, list(d.keys())) for n,d in traj.tool_calls]}")
