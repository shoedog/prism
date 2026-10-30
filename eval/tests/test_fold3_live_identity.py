import sys

from tests.helpers import admitted_fake_run
from tier_a import cli
from tier_a.policy import is_live_corpus


def test_plain_prism_live_status_uses_canonical_name_before_oracle_suffix(
    monkeypatch, tmp_path
):
    seen = {}

    def run(name, cfg, defaults, args, **kwargs):
        seen.update(name=name, canonical_name=kwargs.get("canonical_name"))
        result = admitted_fake_run()
        result["meta"]["baseline_invalid"] = True
        result["meta"]["invalid_reasons"] = ["live_corpus"]
        return result

    monkeypatch.setattr(
        cli,
        "load_corpora",
        lambda: {
            "defaults": {},
            "corpus": {"prism": {"path": str(tmp_path), "oracle": "rust-analyzer"}},
        },
    )
    monkeypatch.setattr(cli, "select_corpora", lambda _cfg, _args: ["prism"])
    monkeypatch.setattr(cli, "run_corpus", run)
    monkeypatch.setattr(cli, "write_reports", lambda *_args: None)
    eval_dir = tmp_path / "eval"
    eval_dir.mkdir()
    monkeypatch.setattr(cli, "EVAL_DIR", eval_dir)
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--oracle", "rust-analyzer", "--date", "2099-01-01"],
    )

    assert cli.main() == 2
    assert seen == {"name": "prism-rust-analyzer", "canonical_name": "prism"}
    assert is_live_corpus(
        "prism-rust-analyzer", False, False, False, canonical_name="prism"
    )
