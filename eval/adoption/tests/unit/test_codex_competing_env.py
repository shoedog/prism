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


# B1 regression: [mcp_servers.real] appearing AFTER [projects.*] must survive into the built config.
def test_b1_mcp_server_after_projects_survives(tmp_path):
    realcodex = tmp_path / "realcodex"; realcodex.mkdir()
    (realcodex / "auth.json").write_text('{"t":"x"}')
    # Construct a config.toml that mirrors the real ~/.codex/config.toml structure:
    # [projects.*] tables appear BEFORE [mcp_servers.*] (as in the real file).
    cfg_text = (
        "[projects.\"/some/path\"]\ntrust_level = \"trusted\"\n\n"
        "[mcp_servers.node_repl]\ncommand = \"/bin/node_repl\"\nargs = []\n"
    )
    (realcodex / "config.toml").write_text(cfg_text)
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir(); (tuned / "SKILL.md").write_text("y")
    home = build_competing_codex_home(skill_src=str(tuned), mcp_repo="/repo",
                                      prism_mcp_bin="/bin/p", root=str(tmp_path / "home"),
                                      real_home=str(realcodex))
    built_cfg = open(os.path.join(home, "config.toml")).read()
    # B1: the real [mcp_servers.node_repl] must be present (not truncated by break-at-[projects.])
    assert "[mcp_servers.node_repl]" in built_cfg, \
        "B1 REGRESSION: [mcp_servers.node_repl] was truncated because [projects.*] break fired"
    # prism section must also be appended
    assert "[mcp_servers.prism]" in built_cfg


# extra_skill_dirs: subdirs from each extra dir are merged into CODEX_HOME/skills/.
def test_extra_skill_dirs_copied(tmp_path):
    realcodex = tmp_path / "realcodex"; realcodex.mkdir()
    (realcodex / "auth.json").write_text('{"t":"x"}')
    (realcodex / "config.toml").write_text("[mcp_servers.node_repl]\ncommand='x'\n")
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir(); (tuned / "SKILL.md").write_text("tuned")
    # extra dir with prism-nav and lsp-nav skills (like ~/knowledge-ref/skills/)
    extra = tmp_path / "extra_skills"
    (extra / "prism-nav").mkdir(parents=True); (extra / "prism-nav" / "SKILL.md").write_text("pn")
    (extra / "lsp-nav").mkdir(); (extra / "lsp-nav" / "SKILL.md").write_text("ln")
    home = build_competing_codex_home(skill_src=str(tuned), mcp_repo="/repo",
                                      prism_mcp_bin="/bin/p", root=str(tmp_path / "home"),
                                      real_home=str(realcodex),
                                      extra_skill_dirs=[str(extra)])
    # extra skills must land in CODEX_HOME/skills/
    assert os.path.isfile(os.path.join(home, "skills", "prism-nav", "SKILL.md")), \
        "prism-nav from extra_skill_dirs not found in CODEX_HOME/skills/"
    assert os.path.isfile(os.path.join(home, "skills", "lsp-nav", "SKILL.md")), \
        "lsp-nav from extra_skill_dirs not found in CODEX_HOME/skills/"
    # tuned prism-code-navigation must NOT be overwritten by an extra-dir version
    assert open(os.path.join(home, "skills", "prism-code-navigation", "SKILL.md")).read() == "tuned"


# B2 regression: memories_1.sqlite in the real home must NOT be copied into the competing home.
def test_b2_memories_sqlite_not_copied(tmp_path):
    realcodex = tmp_path / "realcodex"; realcodex.mkdir()
    (realcodex / "auth.json").write_text('{"t":"x"}')
    (realcodex / "config.toml").write_text("[mcp_servers.node_repl]\ncommand = \"/bin/nr\"\n")
    # Place a fake memories sqlite in the real home (as exists at ~/.codex/memories_1.sqlite)
    (realcodex / "memories_1.sqlite").write_bytes(b"SQLITE")
    tuned = tmp_path / "prism-code-navigation"; tuned.mkdir(); (tuned / "SKILL.md").write_text("y")
    home = build_competing_codex_home(skill_src=str(tuned), mcp_repo="/repo",
                                      prism_mcp_bin="/bin/p", root=str(tmp_path / "home"),
                                      real_home=str(realcodex))
    # B2: no memories*.sqlite should appear in the competing home (no hint control)
    for fname in os.listdir(home):
        assert not fname.startswith("memories"), \
            f"B2: memories file '{fname}' leaked into competing home — no-hint control violated"
