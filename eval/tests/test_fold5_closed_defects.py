import copy
import tomllib

import pytest

from tests.helpers import ARGS, DECLARED_ADJ, FakeSut, admitted_fake_run, anchored_repo, base_ids, fake_run
from tier_a import commands, runner
from tier_a.closure import checkout_corpus_manifest, corpus_control_paths
from tier_a.compare_report import compare_runs


def test_matrix_ok_to_expected_gap_is_a_capability_regression():
    ids = base_ids()
    candidate_ids = {
        **ids,
        "sut_sha_full": "b" * 40,
        "sut_digest": "sha256:" + "b" * 64,
    }
    row = {
        "id": "fixture/dfg",
        "language": "rust",
        "capability": "dfg-stats",
        "outcome": "ok",
    }
    base = fake_run(ids)
    candidate = fake_run(candidate_ids)
    base["matrix"] = [row]
    candidate["matrix"] = [{**row, "outcome": "expected_gap"}]

    result = compare_runs(base, candidate, DECLARED_ADJ)

    assert result["criteria"]["matrix_no_regression"] is False
    assert result["verdict"] == "REGRESSION(matrix_no_regression)"


def test_checkout_identity_distinguishes_same_content_regular_and_symlink(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    manifest = repo / "Cargo.toml"
    manifest.write_text("[package]\nname='x'\n")
    before = checkout_corpus_manifest(repo, {"Cargo.toml"}, "rust", [])
    target = repo / "manifest-copy.txt"
    target.write_bytes(manifest.read_bytes())
    manifest.unlink()
    manifest.symlink_to(target.name)

    after = checkout_corpus_manifest(repo, {"Cargo.toml"}, "rust", [])

    assert after != before


def test_broken_manifest_symlink_is_not_dropped_from_control_inventory(tmp_path):
    repo = tmp_path / "repo"
    (repo / "nested").mkdir(parents=True)
    (repo / "nested/go.mod").symlink_to("missing.mod")

    assert corpus_control_paths(repo, tracked_only=False) == ["nested/go.mod"]


def _admission_fixture(tmp_path, monkeypatch):
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
    return eval_dir, source_repo, repo, args, config


def test_post_run_recheck_refuses_same_content_regular_to_symlink(
    tmp_path, monkeypatch
):
    eval_dir, source_repo, repo, args, config = _admission_fixture(
        tmp_path, monkeypatch
    )

    def swap_manifest_kind(_name, _cfg, *_args, **_kwargs):
        manifest = repo / "Cargo.toml"
        target = repo / "manifest-copy.txt"
        target.write_bytes(manifest.read_bytes())
        manifest.unlink()
        manifest.symlink_to(target.name)
        run = admitted_fake_run()
        run["_sut_bin"] = source_repo / "target/release/prism"
        run["_sut_owner"] = FakeSut(str(source_repo))
        return run

    monkeypatch.setattr(runner, "run_corpus", swap_manifest_kind)

    with pytest.raises(commands.BoundaryViolation) as caught:
        commands.run_quick_admitted(
            "prism",
            config["corpus"]["prism"],
            config["defaults"],
            args,
            eval_dir,
        )
    assert caught.value.reasons == ["corpus_changed_during_run"]


def test_transient_corpus_mutation_refuses_at_inventory_read_boundary(
    tmp_path, monkeypatch
):
    eval_dir, source_repo, repo, args, config = _admission_fixture(
        tmp_path, monkeypatch
    )

    def save_replace_restore(_name, _cfg, *_args, **kwargs):
        source = repo / "src/a.rs"
        original = source.read_bytes()
        source.write_bytes(original + b"// transient generator output\n")
        try:
            guard = kwargs.get("corpus_guard")
            if guard is not None:
                guard("oracle_inventory_read")
        finally:
            source.write_bytes(original)
        run = admitted_fake_run()
        run["_sut_bin"] = source_repo / "target/release/prism"
        run["_sut_owner"] = FakeSut(str(source_repo))
        return run

    monkeypatch.setattr(runner, "run_corpus", save_replace_restore)

    with pytest.raises(commands.BoundaryViolation) as caught:
        commands.run_quick_admitted(
            "prism",
            config["corpus"]["prism"],
            config["defaults"],
            args,
            eval_dir,
        )
    assert caught.value.reasons == ["corpus_changed_during_run"]
