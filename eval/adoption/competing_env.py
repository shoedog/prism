# eval/adoption/competing_env.py
"""COMPETING claude env: the REAL skill set (superpowers plugins + user skills + the tuned
prism-code-navigation) with the SessionStart memory hook STRIPPED — tests the tuned skill under
realistic competition, no prism hint. Pairs with env.py's isolated builder.

What is and is NOT copied from the real ~/.claude:
  COPIED:   settings.json (hooks stripped, prism plugins excluded — see I3),
            skills/ (all real competing skills + the tuned skill),
            plugins/ (symlinked — superpowers etc. unchanged),
            .credentials.json (auth; temp dir, chmod 0600).
  EXCLUDED: memories*.sqlite and AGENTS.md — these carry the memory-based prism hint
            and must be absent for a clean no-hint control.
  MUTATED:  settings.hooks → {} (I1: no SessionStart memory injection),
            settings.permissions.deny ⊇ {Write, Edit} (I1: repo safety — eval cannot
            modify the target repo),
            settings.enabledPlugins: prism@* entries removed (I3: the prism plugin bundles
            prism-code-navigation; keeping it would shadow the tuned competing skill)."""
from __future__ import annotations
import atexit, json, os, shutil, tempfile
from .env import IsolatedConfig   # reuse the (config_dir, mcp_cfg) dataclass

def build_competing_config(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                           root: str | None = None, real_home: str = "~/.claude",
                           credentials_src: str = "~/.claude/.credentials.json") -> IsolatedConfig:
    base = root or tempfile.mkdtemp(prefix="tc-compete-cfg-")
    if root is None:
        atexit.register(shutil.rmtree, base, True)
    cfg_dir = os.path.join(base, "config"); os.makedirs(cfg_dir, exist_ok=True)
    real = os.path.expanduser(real_home)

    # plugins: symlink the real (big) dir so superpowers etc. load unchanged
    rp = os.path.join(real, "plugins")
    if os.path.exists(rp):
        os.symlink(rp, os.path.join(cfg_dir, "plugins"))

    # skills: COPY real skills (so we can add the tuned one alongside) + add the tuned skill
    skills_dir = os.path.join(cfg_dir, "skills"); os.makedirs(skills_dir, exist_ok=True)
    rs = os.path.join(real, "skills")
    if os.path.isdir(rs):
        for name in os.listdir(rs):
            src = os.path.join(rs, name)
            if os.path.isdir(src):
                shutil.copytree(src, os.path.join(skills_dir, name), symlinks=True)
    tuned_dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(tuned_dst):
        shutil.rmtree(tuned_dst)
    shutil.copytree(skill_src, tuned_dst)

    # settings: copy real, then apply safety/control mutations
    settings = {}
    rsj = os.path.join(real, "settings.json")
    if os.path.exists(rsj):
        settings = json.load(open(rsj))

    # Control 1 (no-hint): strip memory-injection hooks (no SessionStart)
    settings["hooks"] = {}

    # I1 (repo safety): force Write + Edit denied so the eval cannot modify the target repo.
    # Mirror env.py's isolated builder which always denies Write/Edit.
    perms = settings.setdefault("permissions", {})
    existing_deny = set(perms.get("deny", []))
    perms["deny"] = sorted(existing_deny | {"Write", "Edit"})

    # I3 (no-shadow): remove prism@* plugin entries so the plugin's bundled prism-code-navigation
    # doesn't shadow the injected tuned competing skill.  Non-prism plugins survive.
    if "enabledPlugins" in settings:
        settings["enabledPlugins"] = {
            k: v for k, v in settings["enabledPlugins"].items()
            if "prism" not in k
        }

    with open(os.path.join(cfg_dir, "settings.json"), "w") as f:
        json.dump(settings, f)

    # creds (secret; temp dir only, chmod 0600)
    cred = os.path.expanduser(credentials_src)
    if os.path.exists(cred):
        dst = os.path.join(cfg_dir, ".credentials.json")
        shutil.copy2(cred, dst); os.chmod(dst, 0o600)

    # prism MCP via --mcp-config (same as isolated)
    mcp_cfg = os.path.join(base, "mcp.json")
    with open(mcp_cfg, "w") as f:
        json.dump({"mcpServers": {"prism": {"command": prism_mcp_bin,
                                            "args": ["--repo", mcp_repo]}}}, f)
    return IsolatedConfig(config_dir=cfg_dir, mcp_cfg=mcp_cfg)
