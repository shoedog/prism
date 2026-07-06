"""CLI wiring for the Part-D `build-gold` and `run-partd` subcommands (spec P2/P5).
Fakes only — never calls a real LSP server / prism binary / live agent."""
from __future__ import annotations
import json

from tier_c.cli import main, _default_gold_root, _default_partd_run_store_root


def test_default_gold_root_is_under_tier_c():
    root = _default_gold_root()
    assert root.endswith("gold")
    assert "tier_c" in root


def test_default_partd_run_store_root_is_under_tier_c_runs():
    root = _default_partd_run_store_root()
    assert root.endswith("partd")
    assert "runs" in root


def test_run_partd_dry_run_without_live_prints_hint_and_returns_0(capsys):
    rc = main(["run-partd", "--task", "prometheus-matchstring", "--model", "opus-4.8"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "--live" in out


def test_run_partd_live_requires_run_id():
    import pytest
    with pytest.raises(SystemExit):
        main(["run-partd", "--task", "prometheus-matchstring", "--model", "opus-4.8", "--live"])


def test_build_gold_requires_task_or_all(tmp_path):
    import pytest
    toml = tmp_path / "structural.toml"
    toml.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\nreceiver = "R"\ndef_site = "a.go:1"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    with pytest.raises(SystemExit):
        main(["build-gold", "--structural-issues", str(toml)])


def test_build_gold_unknown_task_errors(tmp_path):
    import pytest
    toml = tmp_path / "structural.toml"
    toml.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\nreceiver = "R"\ndef_site = "a.go:1"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    with pytest.raises(SystemExit):
        main(["build-gold", "--task", "nonexistent", "--structural-issues", str(toml)])


def test_build_gold_cli_dispatches_to_real_build_gold_with_fakes(tmp_path, monkeypatch):
    """CLI wiring: --task selects the right task and threads bench-root/gold-root
    through to buildgold.build_gold (patched here to avoid touching real git/LSP/prism)."""
    import subprocess
    toml = tmp_path / "structural.toml"
    toml.write_text(
        '[[task]]\nid = "x"\nrepo = "r"\nlang = "go"\nsha = "abc"\n'
        'symbol = "S"\nreceiver = "R"\ndef_site = "a.go:1"\ndispatch = "d"\n'
        'prompt_change = "p"\ngrep_name_stats = "g"\n'
    )
    bench_root = tmp_path / "bench" / "r"
    bench_root.mkdir(parents=True)
    subprocess.run(["git", "init", "-q"], cwd=bench_root, check=True)

    calls = []

    class _FakeResult:
        sites = []
        oracle_health = {"lsp": "ok", "prism": "ok"}

    def fake_build_gold(task, co, *, out_root, oracle_factory=None, prism_runner=None):
        calls.append((task.id, out_root))
        return _FakeResult(), str(tmp_path / "candidates.json"), str(tmp_path / "adjudicate.md")

    class _FakeCheckout:
        def __init__(self, repo, sha):
            self.repo, self.sha = repo, sha

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

    import tier_c.cli as cli_mod
    monkeypatch.setattr(cli_mod, "Checkout", _FakeCheckout)
    monkeypatch.setattr("tier_c.buildgold.build_gold", fake_build_gold)

    rc = main(["build-gold", "--task", "x", "--structural-issues", str(toml),
              "--bench-root", str(tmp_path / "bench"), "--gold-root", str(tmp_path / "gold")])
    assert rc == 0
    assert calls == [("x", str(tmp_path / "gold"))]
