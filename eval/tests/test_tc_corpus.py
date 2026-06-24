import pytest
from tier_c.corpus import load_issues, CorpusError

VALID = b'''
[[issue]]
key = "ripgrep-1"
language = "rust"
repo = "ripgrep"
sha = "abc123abc123"
url = "https://github.com/BurntSushi/ripgrep/issues/1"
text = "globset ** matches hidden dirs"
scoped_slice = "fix the matcher in globset/src/glob.rs only"
files_touched_hint = 3
'''

def test_load_valid_issue(tmp_path):
    p = tmp_path / "issues.toml"; p.write_bytes(VALID)
    issues = load_issues(p)
    assert issues[0].key == "ripgrep-1"
    assert issues[0].language == "rust"

def test_rejects_one_liner(tmp_path):
    bad = VALID.replace(b"files_touched_hint = 3", b"files_touched_hint = 1")
    p = tmp_path / "i.toml"; p.write_bytes(bad)
    with pytest.raises(CorpusError, match="multi-file"):
        load_issues(p)

def test_rejects_missing_scoped_slice(tmp_path):
    bad = VALID.replace(b'scoped_slice = "fix the matcher in globset/src/glob.rs only"\n', b"")
    p = tmp_path / "i.toml"; p.write_bytes(bad)
    with pytest.raises(CorpusError, match="scoped_slice"):
        load_issues(p)
