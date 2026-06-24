from tier_c.cli import main

def test_cli_dry_run_lists_issues(tmp_path, capsys):
    p = tmp_path / "issues.toml"
    p.write_text('[[issue]]\nkey="k"\nlanguage="rust"\nrepo="r"\nsha="s"\n'
                 'url="u"\ntext="t"\nscoped_slice="s1"\nfiles_touched_hint=2\n')
    rc = main(["--issues", str(p), "--list"])
    assert rc == 0
    assert "k" in capsys.readouterr().out
