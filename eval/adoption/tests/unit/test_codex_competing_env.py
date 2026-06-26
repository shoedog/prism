# eval/adoption/tests/unit/test_codex_competing_env.py
import os
from adoption.codex_competing_env import build_competing_codex_home

def test_codex_competing_layout(tmp_path):
    realcodex = tmp_path / "realcodex"; (realcodex).mkdir()
    (realcodex / "auth.json").write_text('{"t":"x"}')
    (realcodex / "config.toml").write_text("[mcp_servers.node_repl]\ncommand='x'\n")
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir(); (tuned/"SKILL.md").write_text("y")
    home = build_competing_codex_home(skill_src=str(tuned), mcp_repo="/repo",
                                      prism_mcp_bin="/bin/p", root=str(tmp_path/"home"),
                                      real_home=str(realcodex))
    assert os.path.isfile(os.path.join(home, "auth.json"))                       # auth carried
    cfg = open(os.path.join(home, "config.toml")).read()
    assert "[mcp_servers.prism]" in cfg                                          # prism added
    assert os.path.isfile(os.path.join(home, "skills", "prism-code-navigation", "SKILL.md"))
