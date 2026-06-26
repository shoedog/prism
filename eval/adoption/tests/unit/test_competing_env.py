# eval/adoption/tests/unit/test_competing_env.py
import json, os
from adoption.competing_env import build_competing_config

def _fake_real_claude(tmp_path, plugins=None):
    """Create a minimal fake ~/.claude directory for tests."""
    real = tmp_path / "realclaude"
    (real / "skills" / "prism-nav").mkdir(parents=True)
    (real / "skills" / "prism-nav" / "SKILL.md").write_text("---\nname: prism-nav\n---\nx")
    (real / "plugins").mkdir()
    settings = {
        "hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "echo hi"}]}]},
        "permissions": {"allow": ["Read"]},
    }
    if plugins is not None:
        settings["enabledPlugins"] = plugins
    (real / "settings.json").write_text(json.dumps(settings))
    (real / ".credentials.json").write_text('{"t":"x"}')
    return real


def test_competing_layout(tmp_path):
    # fake real ~/.claude with a competing skill + a SessionStart hook + creds
    real = _fake_real_claude(tmp_path)
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


# I1: Write and Edit must be in permissions.deny so the eval cannot modify the target repo.
def test_i1_write_edit_denied(tmp_path):
    real = _fake_real_claude(tmp_path)
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir()
    (tuned / "SKILL.md").write_text("y")

    cfg = build_competing_config(skill_src=str(tuned), mcp_repo="/repo", prism_mcp_bin="/bin/p",
                                 root=str(tmp_path / "iso"), real_home=str(real),
                                 credentials_src=str(real / ".credentials.json"))
    settings = json.load(open(os.path.join(cfg.config_dir, "settings.json")))
    deny = settings.get("permissions", {}).get("deny", [])
    assert "Write" in deny, f"I1: 'Write' missing from permissions.deny — got {deny}"
    assert "Edit" in deny,  f"I1: 'Edit' missing from permissions.deny — got {deny}"


# I3: prism@* entries in enabledPlugins must be filtered out; non-prism plugins survive.
def test_i3_prism_plugin_excluded(tmp_path):
    real = _fake_real_claude(tmp_path, plugins={
        "prism@prism-dev": True,
        "superpowers@claude-plugins-official": True,
        "rust-analyzer-lsp@claude-plugins-official": True,
    })
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir()
    (tuned / "SKILL.md").write_text("y")

    cfg = build_competing_config(skill_src=str(tuned), mcp_repo="/repo", prism_mcp_bin="/bin/p",
                                 root=str(tmp_path / "iso"), real_home=str(real),
                                 credentials_src=str(real / ".credentials.json"))
    settings = json.load(open(os.path.join(cfg.config_dir, "settings.json")))
    enabled = settings.get("enabledPlugins", {})
    # prism plugin must be absent — its prism-code-navigation would shadow the tuned skill
    assert "prism@prism-dev" not in enabled, \
        f"I3: prism plugin still present in enabledPlugins — plugin shadows tuned skill"
    # non-prism competitors must survive
    assert "superpowers@claude-plugins-official" in enabled, \
        "I3: superpowers plugin was wrongly removed"
    assert "rust-analyzer-lsp@claude-plugins-official" in enabled, \
        "I3: rust-analyzer plugin was wrongly removed"
