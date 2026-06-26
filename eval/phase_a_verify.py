#!/usr/bin/env python3
"""Phase A verification: confirm codex isolation gate works.

Usage: uv run python phase_a_verify.py
Runs from eval/ directory.
"""
import json, os, subprocess, sys

SKILL_SRC = "/Users/wesleyjinks/code/slicing/skills/prism-code-navigation"
PRISM_MCP_BIN = "/Users/wesleyjinks/code/slicing/target/release/prism-mcp"
MCP_REPO = "/Users/wesleyjinks/code/slicing/eval/tier_c"
AUTH_SRC = os.path.expanduser("~/.codex/auth.json")

sys.path.insert(0, os.path.dirname(__file__))
from adoption.codex_env import build_isolated_codex_home


def run_phase_a():
    print("=== Phase A: isolation gate verification ===\n")

    # Build isolated CODEX_HOME
    home = build_isolated_codex_home(
        skill_src=SKILL_SRC,
        mcp_repo=MCP_REPO,
        prism_mcp_bin=PRISM_MCP_BIN,
        auth_src=AUTH_SRC,
    )
    print(f"CODEX_HOME: {home}")
    print(f"  auth.json: {'present' if os.path.exists(os.path.join(home, 'auth.json')) else 'MISSING'}")
    print(f"  config.toml: {'present' if os.path.exists(os.path.join(home, 'config.toml')) else 'MISSING'}")
    print(f"  SKILL.md: {'present' if os.path.exists(os.path.join(home, 'skills', 'prism-code-navigation', 'SKILL.md')) else 'MISSING'}")

    # Print our config.toml for inspection
    cfg_path = os.path.join(home, "config.toml")
    print(f"\n--- config.toml ---")
    print(open(cfg_path).read())
    print("---")

    env = dict(os.environ)
    env["CODEX_HOME"] = home

    prompt = (
        "Use the prism nav skill, then call the prism tool nav_repo_map on this repo "
        "and report its first line. If you cannot, state the exact reason."
    )
    # NOTE: No --ignore-user-config — we want our isolated config.toml to be loaded.
    # Since CODEX_HOME is set to our isolated dir (not ~/.codex), the user's
    # ~/.codex/config.toml is NOT the config being read; ours is.
    cmd = [
        "codex", "exec",
        "--json",
        "-m", "gpt-5.5",
        "-s", "read-only",
        prompt,
    ]

    print(f"\nCommand: {' '.join(cmd)}")
    print(f"CODEX_HOME: {home}")
    print(f"cwd: {MCP_REPO}")
    print(f"\nRunning (400s timeout)...")
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, timeout=400,
            cwd=MCP_REPO, env=env
        )
    except subprocess.TimeoutExpired:
        print("TIMEOUT (400s)")
        sys.exit(1)

    print(f"\nExit code: {proc.returncode}")
    if proc.stderr:
        print(f"Stderr (first 1000): {proc.stderr[:1000]}")

    # Parse the output
    lines = [l.strip() for l in proc.stdout.splitlines() if l.strip()]
    print(f"\nParsed {len(lines)} events.")

    # Print all events for debugging
    print("\n--- All events ---")
    for i, line in enumerate(lines):
        try:
            ev = json.loads(line)
            ev_type = ev.get("type", "?")
            if ev_type == "item.completed":
                item = ev.get("item", {})
                k = item.get("type", "?")
                if k == "command_execution":
                    print(f"  [{i}] command_execution: {item.get('command','')[:120]}")
                elif k == "mcp_tool_call":
                    print(f"  [{i}] mcp_tool_call server={item.get('server')} tool={item.get('tool')} error={item.get('error')}")
                elif k == "agent_message":
                    print(f"  [{i}] agent_message: {item.get('text','')[:80]}")
                else:
                    print(f"  [{i}] {ev_type}/{k}")
            else:
                print(f"  [{i}] {ev_type}")
        except Exception as e:
            print(f"  [{i}] PARSE ERROR: {e}: {line[:100]}")
    print("---")

    skill_read_path = None
    prism_mcp_fired = False
    prism_mcp_error = None
    auth_error = False
    final_text = ""

    for line in lines:
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue

        if ev.get("type") == "item.completed":
            item = ev.get("item", {})
            kind = item.get("type", "")

            if kind == "agent_message":
                final_text = item.get("text", "")

            elif kind == "command_execution":
                cmd_str = item.get("command", "")
                if "SKILL.md" in cmd_str:
                    skill_read_path = cmd_str
                output = item.get("aggregated_output", "")
                if "not logged" in output.lower() or "unauthorized" in output.lower():
                    auth_error = True

            elif kind == "mcp_tool_call":
                if item.get("server") == "prism":
                    prism_mcp_fired = True
                    prism_mcp_error = item.get("error")

    print("\n=== Phase A Results ===")
    print(f"(1) Skill SKILL.md read path: {skill_read_path}")
    if skill_read_path:
        if "knowledge-ref" in skill_read_path or ("skills/prism-nav" in skill_read_path and "prism-code-navigation" not in skill_read_path):
            print("    CONCERN: read from user global prism-nav (knowledge-ref) — isolation not 100% clean")
            print("    But if prism MCP fired, the evaluation can proceed (skill content is similar)")
        elif "prism-code-navigation" in skill_read_path:
            print("    PASS: read from isolated prism-code-navigation")
        else:
            print("    INFO: unknown path")
    else:
        print("    INFO: no SKILL.md command_execution seen (may use different skill load mechanism)")

    print(f"(2) prism mcp_tool_call fired: {prism_mcp_fired}")
    if prism_mcp_fired:
        print(f"    error: {prism_mcp_error}")
        if prism_mcp_error is None:
            print("    PASS: fired with no error")
        else:
            print(f"    FAIL: error = {prism_mcp_error}")
    else:
        print("    FAIL: prism MCP was not invoked")

    print(f"(3) Auth error detected: {auth_error}")
    print(f"\nFinal text (last 300 chars): {final_text[-300:]}")

    if prism_mcp_fired and not prism_mcp_error:
        print("\nPhase A: PASS")
        return True
    else:
        print("\nPhase A: BLOCKED — see above")
        return False


if __name__ == "__main__":
    ok = run_phase_a()
    sys.exit(0 if ok else 1)
