# eval/adoption/tests/unit/test_codex_env.py
"""TDD tests for build_isolated_codex_home file-layout + isolation contract."""
import json, os, stat
import tomllib
from pathlib import Path
from adoption.codex_env import build_isolated_codex_home


def test_auth_copied_with_mode_600(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/x",
        prism_mcp_bin="/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    auth_dst = os.path.join(home, "auth.json")
    assert os.path.isfile(auth_dst)
    assert json.load(open(auth_dst))["apiKey"] == "sk-test"
    mode = stat.S_IMODE(os.stat(auth_dst).st_mode)
    assert mode == 0o600, f"Expected 0600, got {oct(mode)}"


def test_config_toml_has_only_prism_mcp(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/y",
        prism_mcp_bin="/usr/local/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    cfg_path = os.path.join(home, "config.toml")
    assert os.path.isfile(cfg_path), "config.toml missing"
    with open(cfg_path, "rb") as f:
        cfg = tomllib.load(f)
    # Must have prism MCP server configured
    assert "mcp_servers" in cfg
    assert "prism" in cfg["mcp_servers"]
    prism = cfg["mcp_servers"]["prism"]
    assert "/repo/y" in prism.get("args", [])
    # Must NOT have extra skill dirs or other top-level keys that could bleed in user config
    assert "skills_dirs" not in cfg


def test_skill_copied_into_home(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    (skill_src / "references").mkdir()
    (skill_src / "references" / "deep-dive.md").write_text("detail")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/x",
        prism_mcp_bin="/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    skill_dst = os.path.join(home, "skills", "prism-code-navigation", "SKILL.md")
    assert os.path.isfile(skill_dst), "SKILL.md not copied into CODEX_HOME/skills/"
    ref_dst = os.path.join(home, "skills", "prism-code-navigation", "references", "deep-dive.md")
    assert os.path.isfile(ref_dst), "references/ subdir not copied"


# ---------------------------------------------------------------------------
# F2/F3 harness hardening: shared --cache-dir + startup/tool timeouts
# ---------------------------------------------------------------------------

def test_config_toml_includes_cache_dir_in_args(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/y",
        prism_mcp_bin="/usr/local/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
        cache_dir="/tmp/shared-prism-cache",
    )
    with open(os.path.join(home, "config.toml"), "rb") as f:
        cfg = tomllib.load(f)
    args = cfg["mcp_servers"]["prism"]["args"]
    assert "--cache-dir" in args, f"expected --cache-dir in args; got {args}"
    assert "/tmp/shared-prism-cache" in args


def test_config_toml_omits_cache_dir_when_none(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/y",
        prism_mcp_bin="/usr/local/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    with open(os.path.join(home, "config.toml"), "rb") as f:
        cfg = tomllib.load(f)
    args = cfg["mcp_servers"]["prism"]["args"]
    assert "--cache-dir" not in args, f"cache_dir=None must not add --cache-dir; got {args}"


def test_config_toml_has_startup_and_tool_timeouts(tmp_path):
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{"apiKey": "sk-test"}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/y",
        prism_mcp_bin="/usr/local/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    with open(os.path.join(home, "config.toml"), "rb") as f:
        cfg = tomllib.load(f)
    prism = cfg["mcp_servers"]["prism"]
    assert prism.get("startup_timeout_sec") == 600, f"got {prism.get('startup_timeout_sec')!r}"
    assert prism.get("tool_timeout_sec") == 600, f"got {prism.get('tool_timeout_sec')!r}"


def test_off_arm_config_ignores_cache_dir(tmp_path):
    """The OFF arm writes no [mcp_servers.prism] section at all — cache_dir must be a no-op."""
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/x",
        prism_mcp_bin="/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
        include_skill_and_mcp=False,
        cache_dir="/tmp/shared-prism-cache",
    )
    content = (Path(home) / "config.toml").read_text()
    assert "mcp_servers" not in content
    assert "cache-dir" not in content
    assert "shared-prism-cache" not in content


def test_no_extra_skills_from_user_home(tmp_path):
    """The isolated home must NOT contain skills other than prism-code-navigation."""
    skill_src = tmp_path / "prism-code-navigation"
    skill_src.mkdir()
    (skill_src / "SKILL.md").write_text("---\nname: prism-code-navigation\n---\nbody")
    fake_auth = tmp_path / "auth.json"
    fake_auth.write_text('{}')
    home = build_isolated_codex_home(
        skill_src=str(skill_src),
        mcp_repo="/repo/x",
        prism_mcp_bin="/bin/prism-mcp",
        root=str(tmp_path / "codex_home"),
        auth_src=str(fake_auth),
    )
    skills_dir = os.path.join(home, "skills")
    if os.path.isdir(skills_dir):
        skill_dirs = [d for d in os.listdir(skills_dir) if os.path.isdir(os.path.join(skills_dir, d))]
        assert skill_dirs == ["prism-code-navigation"], f"Extra skills leaked in: {skill_dirs}"
