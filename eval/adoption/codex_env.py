"""Build an ISOLATED CODEX_HOME directory so codex uses ONLY the repo's
prism-code-navigation skill + the prism MCP server (and NOT any user-global skills
from ~/.codex/skills/ or knowledge-ref/).

The isolation contract:
  - auth.json copied from auth_src, chmod 0600
  - config.toml has ONLY [mcp_servers.prism]; no skill_dirs, no extra keys that
    would inject user skills.  Use `--ignore-user-config` on every codex exec call
    so ~/.codex/config.toml is not loaded.  Auth still reads from CODEX_HOME.
  - skills/prism-code-navigation/ copied from skill_src (the repo's deployed skill).

The caller is responsible for:
  - Setting CODEX_HOME=<returned path> in subprocess env.
  - Passing --ignore-user-config to `codex exec` to suppress the user config.
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
) -> str:
    """Create an isolated CODEX_HOME and return its path.

    Parameters
    ----------
    skill_src:     Path to the prism-code-navigation skill directory (with SKILL.md).
    mcp_repo:      --repo argument for prism-mcp (the target repo to analyse).
    prism_mcp_bin: Absolute path to the prism-mcp binary.
    root:          If given, use as the CODEX_HOME base (not cleaned up); otherwise
                   creates a tempdir that is atexit-cleaned.
    auth_src:      Path to the source auth.json (~ is expanded).
    """
    _created = root is None
    home = root or tempfile.mkdtemp(prefix="tc-codex-home-")
    if _created:
        atexit.register(shutil.rmtree, home, True)

    # Ensure the CODEX_HOME root directory exists.
    os.makedirs(home, exist_ok=True)

    # --- 1. auth.json (must be 0600) ---
    auth_path = os.path.expanduser(auth_src)
    if os.path.exists(auth_path):
        auth_dst = os.path.join(home, "auth.json")
        shutil.copy2(auth_path, auth_dst)
        os.chmod(auth_dst, 0o600)

    # --- 2. config.toml (ONLY prism MCP; no skill_dirs key) ---
    # Using TOML literal construction — no external TOML library needed for writing.
    # The [mcp_servers.prism] section registers prism as the only MCP server.
    # Deliberately omitting: skill_dirs, model, approvals_reviewer, projects, notify.
    # We pass --ignore-user-config on every exec call to prevent ~/.codex/config.toml
    # from adding back the user's global skills or other settings.
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

    return home
