# eval/adoption/tests/unit/test_competing_env.py
import json, os
from adoption.competing_env import build_competing_config

def test_competing_layout(tmp_path):
    # fake real ~/.claude with a competing skill + a SessionStart hook + creds
    real = tmp_path / "realclaude"; (real / "skills" / "prism-nav").mkdir(parents=True)
    (real / "skills" / "prism-nav" / "SKILL.md").write_text("---\nname: prism-nav\n---\nx")
    (real / "plugins").mkdir()
    (real / "settings.json").write_text(json.dumps({"hooks": {"SessionStart": [{"hooks": [{"type":"command","command":"echo hi"}]}]}, "permissions": {"allow": ["Read"]}}))
    (real / ".credentials.json").write_text('{"t":"x"}')
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir()
    (tuned / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\ny")

    cfg = build_competing_config(skill_src=str(tuned), mcp_repo="/repo", prism_mcp_bin="/bin/p",
                                 root=str(tmp_path/"iso"), real_home=str(real),
                                 credentials_src=str(real/".credentials.json"))
    sk = os.path.join(cfg.config_dir, "skills")
    assert os.path.exists(os.path.join(sk, "prism-nav", "SKILL.md"))            # real competitor present
    assert os.path.exists(os.path.join(sk, "prism-code-navigation", "SKILL.md"))# tuned skill present
    settings = json.load(open(os.path.join(cfg.config_dir, "settings.json")))
    assert settings.get("hooks", {}) == {}                                      # SessionStart STRIPPED
    assert "Read" in settings["permissions"]["allow"]                           # other settings kept
    assert json.load(open(os.path.join(cfg.config_dir, ".credentials.json")))["t"] == "x"
