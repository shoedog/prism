import json, os
from adoption.env import build_isolated_config

def test_build_isolated_config_layout(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    (skill_src).mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    cfg = build_isolated_config(skill_src=str(skill_src), mcp_repo="/repo/x",
                                prism_mcp_bin="/bin/prism-mcp", root=str(tmp_path / "iso"))
    # the skill is present under the isolated config's skills dir
    assert os.path.isfile(os.path.join(cfg.config_dir, "skills", "prism-code-navigation", "SKILL.md"))
    # the MCP config points prism at the repo
    mcp = json.load(open(cfg.mcp_cfg))
    assert mcp["mcpServers"]["prism"]["args"] == ["--repo", "/repo/x"]
    # NO settings hooks leaked
    assert cfg.config_dir != os.path.expanduser("~/.claude")
