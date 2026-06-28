import hashlib
from tier_c.cli import main, _prism_build_id

def test_cli_dry_run_lists_issues(tmp_path, capsys):
    p = tmp_path / "issues.toml"
    p.write_text('[[issue]]\nkey="k"\nlanguage="rust"\nrepo="r"\nsha="s"\n'
                 'url="u"\ntext="t"\nscoped_slice="s1"\nfiles_touched_hint=2\n')
    rc = main(["--issues", str(p), "--list"])
    assert rc == 0
    assert "k" in capsys.readouterr().out


def test_prism_build_id_hashes_binary(tmp_path):
    f = tmp_path / "prism-mcp"
    f.write_bytes(b"hello prism")
    # Full 64-hex sha256 (not truncated)
    expected = "sha256:" + hashlib.sha256(b"hello prism").hexdigest()
    assert _prism_build_id(str(f)) == expected
    assert len(expected) == len("sha256:") + 64
    assert _prism_build_id("/nonexistent/prism-mcp").startswith("error:")
