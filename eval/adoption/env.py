"""Build an ISOLATED claude config home.

Two modes controlled by include_skill_and_mcp:

  True  (default) — prism-ON arm:
    copies the prism-code-navigation skill, writes mcp.json, allows mcp__prism.
    This is the realistic-but-controlled deployment env (spec §Deployment): no
    SessionStart hooks, no other skills (the prior run leaked superpowers:* — a confound).

  False — prism-OFF arm (status-quo baseline):
    NO skill copy, NO mcp.json (mcp_cfg=""), NO mcp__prism in allow.
    Allow = [Read, Grep, Glob, Bash]; deny = [Write, Edit]; hooks = {}.
    Creds are still seeded (CLAUDE_CONFIG_DIR override loses auth otherwise).
    This gives a clean, isolated baseline that is parity with the on-arm EXCEPT
    for the prism bundle — so the only difference between arms is prism itself.
"""
from __future__ import annotations
import atexit, json, os, shutil, tempfile
from dataclasses import dataclass

@dataclass(frozen=True)
class IsolatedConfig:
    config_dir: str   # value for CLAUDE_CONFIG_DIR
    mcp_cfg: str      # path to the --mcp-config json (empty string when include_skill_and_mcp=False)

def build_isolated_config(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                          root: str | None = None,
                          credentials_src: str = "~/.claude/.credentials.json",
                          include_skill_and_mcp: bool = True) -> IsolatedConfig:
    """Build an isolated CLAUDE_CONFIG_DIR.

    Parameters
    ----------
    skill_src:              Path to the prism-code-navigation skill directory.
    mcp_repo:               --repo arg for prism-mcp (used only when include_skill_and_mcp=True).
    prism_mcp_bin:          Path to the prism-mcp binary (used only when include_skill_and_mcp=True).
    root:                   Base directory; created as a tempdir and atexit-cleaned when None.
    credentials_src:        Source path for .credentials.json (seeded in both modes).
    include_skill_and_mcp:  True → prism-ON config (skill + mcp.json + mcp__prism allow).
                            False → status-quo OFF config (no skill, no mcp.json, no mcp__prism).
    """
    _created = root is None
    base = root or tempfile.mkdtemp(prefix="tc-adopt-cfg-")
    if _created:
        atexit.register(shutil.rmtree, base, True)
    cfg_dir = os.path.join(base, "config")
    skills_dir = os.path.join(cfg_dir, "skills")
    os.makedirs(skills_dir, exist_ok=True)

    if include_skill_and_mcp:
        # Copy the prism-code-navigation skill into the isolated config.
        dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
        if os.path.exists(dst):
            shutil.rmtree(dst)
        shutil.copytree(skill_src, dst)
        # settings: NO hooks; permit read/nav tools + prism, DENY Write/Edit.
        # Faithful (a prism user approves the tools) AND safe (eval cannot modify the target repo).
        allow_list = ["Read", "Grep", "Glob", "Bash", "mcp__prism"]
    else:
        # Status-quo baseline: no skill, no mcp__prism — ONLY read/nav tools allowed.
        # Write/Edit denied so the model must write specs inline (parity with prism-ON arm).
        allow_list = ["Read", "Grep", "Glob", "Bash"]

    with open(os.path.join(cfg_dir, "settings.json"), "w") as f:
        json.dump({"hooks": {},
                   "permissions": {"allow": allow_list,
                                   "deny": ["Write", "Edit"]}}, f)

    # Seed auth: overriding CLAUDE_CONFIG_DIR loses ~/.claude credentials, so claude returns
    # "Not logged in". Copy creds in. SECRET — temp dir only, NEVER commit.
    cred = os.path.expanduser(credentials_src)
    if os.path.exists(cred):
        dst_cred = os.path.join(cfg_dir, ".credentials.json")
        shutil.copy2(cred, dst_cred)
        os.chmod(dst_cred, 0o600)

    if include_skill_and_mcp:
        mcp_cfg = os.path.join(base, "mcp.json")
        with open(mcp_cfg, "w") as f:
            json.dump({"mcpServers": {"prism": {"command": prism_mcp_bin,
                                                "args": ["--repo", mcp_repo]}}}, f)
    else:
        mcp_cfg = ""

    return IsolatedConfig(config_dir=cfg_dir, mcp_cfg=mcp_cfg)
