"""Build an ISOLATED CODEX_HOME directory.

Two modes controlled by include_skill_and_mcp:

  True  (default) — prism-ON arm:
    copies prism-code-navigation skill, writes config.toml with [mcp_servers.prism],
    seeds auth.json.  CODEX_HOME alone excludes ~/.codex and ~/knowledge-ref skills
    (verified live: codex loads only the repo's prism-code-navigation, no fallback).

  False — prism-OFF arm (status-quo baseline):
    NO skill copy, NO [mcp_servers.prism] section. A MINIMAL config.toml is still
    written carrying ONLY the reasoning-exposure keys (auditability parity); auth is
    seeded (CODEX_HOME override loses auth otherwise).
    This gives a clean, isolated home that is parity with the on-arm EXCEPT for the
    prism bundle — so the only difference between arms is prism itself.

Both modes write the top-level reasoning-exposure keys (hide_agent_reasoning=false,
show_raw_agent_reasoning=true) so the model's raw reasoning is surfaced in the event
stream for after-the-fact auditing (codex hides agent reasoning by default).

The caller is responsible for:
  - Setting CODEX_HOME=<returned path> in subprocess env.
  - Passing -C <repo_dir> to set the working directory.
"""
from __future__ import annotations
import atexit, json, os, shutil, tempfile


def build_isolated_codex_home(
    *,
    skill_src: str,
    mcp_repo: str,
    prism_mcp_bin: str,
    root: str | None = None,
    auth_src: str = "~/.codex/auth.json",
    include_skill_and_mcp: bool = True,
    cache_dir: str | None = None,
) -> str:
    """Create an isolated CODEX_HOME and return its path.

    Parameters
    ----------
    skill_src:              Path to the prism-code-navigation skill directory (with SKILL.md).
    mcp_repo:               --repo argument for prism-mcp (the target repo to analyse).
    prism_mcp_bin:          Absolute path to the prism-mcp binary.
    root:                   If given, use as the CODEX_HOME base (not cleaned up); otherwise
                            creates a tempdir that is atexit-cleaned.
    auth_src:               Path to the source auth.json (~ is expanded).
    include_skill_and_mcp:  True  → prism-ON home (skill + config.toml with [mcp_servers.prism]).
                            False → status-quo OFF home (auth only; no skill, no MCP config).
    cache_dir:              F2 (harness hardening) — when given, appended to the prism-mcp
                            args as `--cache-dir <cache_dir>` so codex's prism-mcp reads
                            from the SAME nav cache the harness's prewarm wrote to (instead
                            of both independently falling back to the default OS cache
                            dir).  Ignored when include_skill_and_mcp=False (no MCP server
                            is configured for the OFF arm at all).
    """
    _created = root is None
    home = root or tempfile.mkdtemp(prefix="tc-codex-home-")
    if _created:
        atexit.register(shutil.rmtree, home, True)

    # Ensure the CODEX_HOME root directory exists.
    os.makedirs(home, exist_ok=True)

    # --- 1. auth.json (must be 0600) — seeded in both modes ---
    auth_path = os.path.expanduser(auth_src)
    if os.path.exists(auth_path):
        auth_dst = os.path.join(home, "auth.json")
        shutil.copy2(auth_path, auth_dst)
        os.chmod(auth_dst, 0o600)

    # --- 2. config.toml — written in BOTH modes ---
    # Using TOML literal construction — no external TOML library needed for writing.
    # Top-level keys MUST precede any [table] header (TOML rule), so the reasoning
    # keys come first; the on-arm appends the [mcp_servers.prism] section after them.
    # Reasoning exposure surfaces the model's raw reasoning in the event stream so the
    # transcript is auditable after the fact (codex hides agent reasoning by default).
    cfg_lines = [
        "# Isolated CODEX_HOME — managed by eval/adoption/codex_env.py\n",
        "# DO NOT ADD skill_dirs or other keys that inject user-global skills.\n",
        "\n",
        "# Reasoning exposure (auditability): show raw model reasoning in the stream.\n",
        "hide_agent_reasoning = false\n",
        "show_raw_agent_reasoning = true\n",
    ]
    if include_skill_and_mcp:
        # The [mcp_servers.prism] section registers prism as the only MCP server.
        # Deliberately omitting: skill_dirs, model, approvals_reviewer, projects, notify.
        mcp_args = ["--repo", mcp_repo]
        if cache_dir:
            # F2: explicit --cache-dir so this codex's prism-mcp shares the harness
            # prewarm's cache directory instead of the default OS cache dir.
            mcp_args += ["--cache-dir", cache_dir]
        cfg_lines += [
            "\n",
            "[mcp_servers.prism]\n",
            f'command = "{prism_mcp_bin}"\n',
            f"args = {json.dumps(mcp_args)}\n",
            # F3: generous startup/tool timeouts. The isolated CODEX_HOME does NOT inherit
            # the user's ~/.codex/config.toml, so codex would otherwise use its short
            # DEFAULT startup timeout (~10s) — even a warm ruff init could race it, and any
            # cold CPG build fails invisibly (codex silently falls back to grepping).
            "startup_timeout_sec = 600\n",
            "tool_timeout_sec = 600\n",
        ]
    with open(os.path.join(home, "config.toml"), "w") as f:
        f.writelines(cfg_lines)

    if include_skill_and_mcp:
        # --- 3. skills/prism-code-navigation/ (copy from repo) ---
        skills_dir = os.path.join(home, "skills")
        os.makedirs(skills_dir, exist_ok=True)
        dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
        if os.path.exists(dst):
            shutil.rmtree(dst)
        shutil.copytree(skill_src, dst)
    # else: status-quo baseline — auth + minimal config.toml (reasoning only), no skill, no MCP.

    return home
