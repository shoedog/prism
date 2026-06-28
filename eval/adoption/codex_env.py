"""Build an ISOLATED CODEX_HOME directory.

Two modes controlled by include_skill_and_mcp:

  True  (default) — prism-ON arm:
    copies prism-code-navigation skill, writes config.toml with [mcp_servers.prism],
    seeds auth.json.  CODEX_HOME alone excludes ~/.codex and ~/knowledge-ref skills
    (verified live: codex loads only the repo's prism-code-navigation, no fallback).

  False — prism-OFF arm (status-quo baseline):
    NO skill copy, NO [mcp_servers.prism] in config.toml (config.toml omitted entirely).
    Auth is still seeded (CODEX_HOME override loses auth otherwise).
    This gives a clean, isolated home that is parity with the on-arm EXCEPT for the
    prism bundle — so the only difference between arms is prism itself.

The caller is responsible for:
  - Setting CODEX_HOME=<returned path> in subprocess env.
  - Passing -C <repo_dir> to set the working directory.
"""
from __future__ import annotations
import atexit, os, shutil, tempfile


def build_isolated_codex_home(
    *,
    skill_src: str,
    mcp_repo: str,
    prism_mcp_bin: str,
    root: str | None = None,
    auth_src: str = "~/.codex/auth.json",
    include_skill_and_mcp: bool = True,
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

    if include_skill_and_mcp:
        # --- 2. config.toml (ONLY prism MCP; no skill_dirs) ---
        # Using TOML literal construction — no external TOML library needed for writing.
        # The [mcp_servers.prism] section registers prism as the only MCP server.
        # Deliberately omitting: skill_dirs, model, approvals_reviewer, projects, notify.
        cfg_lines = [
            "# Isolated CODEX_HOME — managed by eval/adoption/codex_env.py\n",
            "# DO NOT ADD skill_dirs or other keys that inject user-global skills.\n",
            "\n",
            "[mcp_servers.prism]\n",
            f'command = "{prism_mcp_bin}"\n',
            f'args = ["--repo", "{mcp_repo}"]\n',
        ]
        with open(os.path.join(home, "config.toml"), "w") as f:
            f.writelines(cfg_lines)

        # --- 3. skills/prism-code-navigation/ (copy from repo) ---
        skills_dir = os.path.join(home, "skills")
        os.makedirs(skills_dir, exist_ok=True)
        dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
        if os.path.exists(dst):
            shutil.rmtree(dst)
        shutil.copytree(skill_src, dst)
    # else: status-quo baseline — auth only, no config.toml, no skills dir.

    return home
