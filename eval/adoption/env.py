"""Build an ISOLATED claude config home containing ONLY the prism skill + prism MCP.
This is the realistic-but-controlled deployment env (spec §Deployment): no SessionStart
hooks, no other skills (the prior run leaked superpowers:* — a confound)."""
from __future__ import annotations
import json, os, shutil, tempfile
from dataclasses import dataclass

@dataclass(frozen=True)
class IsolatedConfig:
    config_dir: str   # value for CLAUDE_CONFIG_DIR
    mcp_cfg: str      # path to the --mcp-config json

def build_isolated_config(*, skill_src: str, mcp_repo: str, prism_mcp_bin: str,
                          root: str | None = None) -> IsolatedConfig:
    base = root or tempfile.mkdtemp(prefix="tc-adopt-cfg-")
    cfg_dir = os.path.join(base, "config")
    skills_dir = os.path.join(cfg_dir, "skills")
    os.makedirs(skills_dir, exist_ok=True)
    dst = os.path.join(skills_dir, os.path.basename(skill_src.rstrip("/")))
    if os.path.exists(dst):
        shutil.rmtree(dst)
    shutil.copytree(skill_src, dst)
    # minimal settings: explicitly NO hooks (empty), so nothing is injected.
    with open(os.path.join(cfg_dir, "settings.json"), "w") as f:
        json.dump({"hooks": {}}, f)
    mcp_cfg = os.path.join(base, "mcp.json")
    with open(mcp_cfg, "w") as f:
        json.dump({"mcpServers": {"prism": {"command": prism_mcp_bin,
                                            "args": ["--repo", mcp_repo]}}}, f)
    return IsolatedConfig(config_dir=cfg_dir, mcp_cfg=mcp_cfg)
