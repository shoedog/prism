import copy

import pytest

from tests.helpers import ARGS, anchored_repo
from tier_a import cli, commands
from tier_a.lock import Lock, resolve_corpus_identity
from tier_a.closure import manifest_digest, raw_digest


def _rust_checkout(tmp_path):
    checkout = tmp_path / "checkout"
    source = checkout / "src/a.rs"
    source.parent.mkdir(parents=True)
    source.write_text("fn a() {}\n")
    cargo = checkout / "Cargo.toml"
    cargo.write_text("[package]\nname='x'\nversion='0.1.0'\n")
    rows = [("Cargo.toml", raw_digest(cargo)), ("src/a.rs", raw_digest(source))]
    lock = Lock(
        prism={"sha_aliases": ["a" * 40], "closure": {"digest": manifest_digest(rows)}}
    )
    lock._corpus_rows = rows
    lock._policy_data = {
        "corpora": {"corpus": {"prism": {"lang": "rust", "excludes": []}}}
    }
    return lock, checkout


def test_nested_cargo_manifest_is_always_a_corpus_control_file(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    nested = checkout / "src/Cargo.toml"
    nested.write_text("[package]\nname='nested'\nversion='0.1.0'\nedition='2015'\n")

    added = resolve_corpus_identity(lock, checkout)
    nested.write_text(nested.read_text().replace("2015", "2024"))
    changed = resolve_corpus_identity(lock, checkout)

    assert added.dirty is True
    assert added.untracked == ["src/Cargo.toml"]
    assert changed.dirty is True
    assert changed.untracked == ["src/Cargo.toml"]


@pytest.mark.parametrize(
    ("sample", "expected"),
    [(None, "execute"), (3, "execute"), (1, "sample_override_mismatch")],
)
def test_lock_sample_refusal_runs_through_admitted_production_path(
    tmp_path, monkeypatch, sample, expected
):
    lock, eval_dir, repo = anchored_repo(tmp_path)
    args = copy.copy(ARGS)
    args.sample = sample
    reached = []

    class Executed(Exception):
        pass

    def execute(*_args, **_kwargs):
        reached.append(True)
        raise Executed

    monkeypatch.setattr(
        commands,
        "preflight_quick",
        lambda *_args: (lock, repo, {}, None),
    )
    monkeypatch.setattr(cli, "run_corpus", execute)

    if expected == "execute":
        with pytest.raises(Executed):
            commands.run_quick_admitted(
                "prism", {"path": str(repo)}, {}, args, eval_dir
            )
        assert reached == [True]
    else:
        with pytest.raises(commands.BoundaryViolation) as caught:
            commands.run_quick_admitted(
                "prism", {"path": str(repo)}, {}, args, eval_dir
            )
        assert caught.value.reasons == ["sample_override_mismatch: quick: 1 != 3"]
        assert reached == []
