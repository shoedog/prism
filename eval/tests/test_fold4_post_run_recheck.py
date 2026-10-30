import copy
import tomllib

import pytest

from tests.helpers import ARGS, FakeSut, admitted_fake_run, anchored_repo
from tier_a import commands, runner


def test_oracle_startup_corpus_mutation_refuses_admission(tmp_path, monkeypatch):
    _lock, eval_dir, source_repo = anchored_repo(tmp_path)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    lock, repo, _overlay, identity = commands.preflight_quick(eval_dir, False)
    args = copy.copy(ARGS)
    config = tomllib.loads((eval_dir / "corpora.toml").read_text())

    monkeypatch.setattr(
        commands,
        "preflight_quick",
        lambda *_args: (lock, repo, {}, identity),
    )
    monkeypatch.setattr(commands, "observe_cargo", lambda *_args: ([], set()))

    def mutate_during_measurement(_name, cfg, *_args, **_kwargs):
        source = repo / "src/a.rs"
        source.write_text(source.read_text() + "// oracle startup side effect\n")
        run = admitted_fake_run()
        run["_sut_bin"] = source_repo / "target/release/prism"
        run["_sut_owner"] = FakeSut(str(source_repo))
        return run

    monkeypatch.setattr(runner, "run_corpus", mutate_during_measurement)

    with pytest.raises(commands.BoundaryViolation) as caught:
        commands.run_quick_admitted(
            "prism",
            config["corpus"]["prism"],
            config["defaults"],
            args,
            eval_dir,
        )
    assert caught.value.reasons == ["corpus_changed_during_run"]
