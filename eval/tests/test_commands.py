import json
import subprocess
import sys
from pathlib import Path

import pytest

from tests.helpers import FakeOracle, FakeSut, admitted_fake_run, anchored_repo, edit, git_head
from tier_a import boundary, cli, commands
from tier_a.closure import raw_digest, read_tsv, write_tsv
from tier_a.commands import preflight_quick
from tier_a.lock import corpus_check, integrity_check, save_lock
from tier_a.materialize import materialize
from tier_a.sut import PrismCli


REFUSAL_DATE = "2099-01-02"


def assert_no_run_or_report(ev: Path, date: str = REFUSAL_DATE) -> None:
    # A refusal writes NOTHING: no run record of any name (not only the dated one) and no report pair.
    runs = ev / "runs"
    assert not runs.exists() or sorted(runs.glob("*.json")) == [], sorted(runs.glob("*.json"))
    report_dir = ev.parent / "docs/eval/tier-a"
    assert not report_dir.exists() or sorted(report_dir.glob(f"{date}-prism.*")) == [], sorted(report_dir.glob("*"))
    report = report_dir / f"{date}-prism"
    assert not report.with_suffix(".json").exists()
    assert not report.with_suffix(".md").exists()


def test_checks_report_each_condition_one_at_a_time(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    co = materialize(lock, root, str(repo))
    tc = {
        "rustc": lock.prism["runtime"]["rustc"],
        "cargo": lock.prism["runtime"]["cargo"],
        "python": lock.prism["runtime"]["python"],
    }
    assert corpus_check(lock, co, lock.prism["oracle"]["version"], tc) == []
    assert integrity_check(lock, ev) == []
    (co / "src/a.rs").write_text("fn a() { }\n")
    assert corpus_check(lock, co, lock.prism["oracle"]["version"], tc) == [
        "checkout_closure_mismatch"
    ]
    assert corpus_check(lock, co, "rust-analyzer 0.0.0", tc)[0].startswith(
        "oracle_version_mismatch"
    )
    assert corpus_check(
        lock,
        co,
        lock.prism["oracle"]["version"],
        {**tc, "rustc": "rustc 0.0.0"},
    )[0].startswith("toolchain_mismatch: rustc")
    (ev / "adjudications.jsonl").write_text("{}\n")
    assert "data_digest_moved: eval/adjudications.jsonl" in integrity_check(lock, ev)
    lock2, ev2, _ = anchored_repo(tmp_path / "b")
    edit(ev2 / "corpora.toml", "rust = 0.10", "rust = 0.14")
    assert "policy_digest_moved" in integrity_check(lock2, ev2)
    lock3, ev3, _ = anchored_repo(tmp_path / "c")
    lock3.prism["snapshot"]["digest"] = "sha256:0"
    assert "snapshot_digest_moved" in integrity_check(lock3, ev3)


def test_preflights_refuse_with_exit_3_and_no_report(
    tmp_path, monkeypatch, capsys
):
    lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    (ev / "adjudications.jsonl").write_text("{}\n")
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(cli, "run_matrix", lambda *a, **k: [])
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--matrix-only", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )
    assert cli.main() == 3
    assert_no_run_or_report(ev)
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--quick", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )
    assert cli.main() == 3
    assert_no_run_or_report(ev)
    assert "FATAL boundary_violation: data_digest_moved" in capsys.readouterr().err


def test_missing_published_sut_manifest_preserves_matrix_preflight_boundary(
    tmp_path, monkeypatch, capsys
):
    lock, ev, repo = anchored_repo(tmp_path)
    sut_manifest = repo / lock.prism["sut"]["inputs"]
    assert integrity_check(lock, ev) == []
    assert hasattr(lock, "_sut_rows")
    sut_manifest.unlink()
    assert integrity_check(lock, ev) == []
    assert not hasattr(lock, "_sut_rows")

    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(cli, "run_matrix", lambda *a, **k: [])
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--matrix-only", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )
    assert cli.main() == 0
    assert capsys.readouterr().err == ""

    with pytest.raises(FileNotFoundError) as exc:
        boundary._sut_input_reasons(lock, ev)
    assert exc.value.filename == str(sut_manifest)


def test_preflight_refuses_half_published_anchor_without_a_lock(
    tmp_path, monkeypatch, capsys
):
    lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(cli, "run_matrix", lambda *a, **k: [])
    (ev / "tier-a.lock.toml").unlink()
    (ev / "closure/.staging-next").write_text("in progress\n")
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--matrix-only", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )
    assert cli.main() == 3
    assert_no_run_or_report(ev)
    assert "FATAL boundary_violation: half_published_anchor" in capsys.readouterr().err


def test_lockless_staging_only_is_not_half_published(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    (ev / "tier-a.lock.toml").unlink()
    for path in (ev / "closure").iterdir():
        if path.name != ".gitkeep":
            path.unlink()
    (ev / "closure/.staging-next").write_text("in progress\n")

    assert integrity_check(None, ev) == []


def test_quick_uses_pin_not_head_and_live_never_anchors(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    monkeypatch.setenv("TIER_A_ROOT", str(root))
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    seen = {}
    monkeypatch.setattr(
        cli,
        "run_corpus",
        lambda name, cfg, defaults, args, **kw: seen.update(path=cfg["path"])
        or admitted_fake_run(),
    )
    monkeypatch.setattr(sys, "argv", ["tier-a", "--quick", "--allow-stale-sut"])
    cli.main()
    assert Path(seen["path"]) == materialize(lock, root, str(repo))
    _, path, overlay, ident = preflight_quick(ev, live=True)
    assert path == ev.parent
    assert ident is None
    assert overlay["corpus_mode"] == "live"
    assert overlay["baseline_invalid"] is True
    assert "live_corpus" in overlay["invalid_reasons"]


def test_quick_run_id_and_sut_archive_name_both_output_pairs(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    seen = {}

    def from_archive(cls, prism_repo, path, sha_full, digest):
        seen.update(
            prism_repo=prism_repo,
            path=path,
            sha_full=sha_full,
            digest=digest,
        )
        return FakeSut(prism_repo, path)

    monkeypatch.setattr(PrismCli, "from_archive", classmethod(from_archive))
    monkeypatch.setattr(cli, "run_corpus", lambda *args, **kwargs: admitted_fake_run())
    archive = tmp_path / "archive-prism"
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "tier-a",
            "--quick",
            "--allow-stale-sut",
            "--date",
            REFUSAL_DATE,
            "--run-id",
            "base",
            "--sut-archive",
            str(archive),
        ],
    )

    assert cli.main() == 0
    assert seen == {
        "prism_repo": str(ev.parent),
        "path": str(archive),
        "sha_full": lock.prism["sut"]["sha"],
        "digest": lock.prism["sut"]["digest"],
    }
    assert (ev / f"runs/{REFUSAL_DATE}-prism-base.json").exists()
    report = ev.parent / f"docs/eval/tier-a/{REFUSAL_DATE}-prism-base"
    assert report.with_suffix(".json").exists()
    assert report.with_suffix(".md").exists()


def test_live_quick_drift_reports_only_live_corpus(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(
        cli, "make_oracle", lambda cfg, init_options=None: FakeOracle()
    )
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "tier-a",
            "--quick",
            "--live",
            "--allow-stale-sut",
            "--date",
            "2099-01-03",
        ],
    )

    assert cli.main() == 2
    run = json.loads((ev / "runs/2099-01-03-prism.json").read_text())
    assert run["probe_status"] == {
        "callers:src/a.rs:1": "inventory_miss",
        "callees:src/a.rs:1": "inventory_miss",
    }
    assert "live_corpus" in run["meta"]["invalid_reasons"]
    assert not any(
        reason.startswith("corpus_sha_drift")
        for reason in run["meta"]["invalid_reasons"]
    )


def test_published_anchor_at_descendant_head_admits_and_reports_execution_head(
    tmp_path, monkeypatch, capsys
):
    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    generated = repo / "target/generated.rs"
    generated.parent.mkdir(parents=True, exist_ok=True)
    generated.write_text("fn generated() {}\n")
    rows = read_tsv(manifest) + [
        (".git/HEAD", raw_digest(repo / ".git/HEAD")),
        ("target/generated.rs", raw_digest(generated)),
    ]
    write_tsv(rows, manifest)
    archived = Path(lock.prism["sut"]["archive"]).parent / "inputs/target/generated.rs"
    archived.parent.mkdir(parents=True, exist_ok=True)
    archived.write_bytes(generated.read_bytes())
    subprocess.run(["git", "add", "eval", "docs"], cwd=repo, check=True)
    subprocess.run(
        ["git", "commit", "-m", "publish anchor"], cwd=repo, check=True,
        capture_output=True,
    )
    (repo / "DESCENDANT").write_text("frozen harness\n")
    subprocess.run(["git", "add", "DESCENDANT"], cwd=repo, check=True)
    subprocess.run(
        ["git", "commit", "-m", "freeze harness"], cwd=repo, check=True,
        capture_output=True,
    )
    execution_head = git_head(repo)
    assert execution_head != lock.prism["harness_sha"]

    monkeypatch.setattr(commands, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(cli, "make_oracle", lambda *_a, **_k: FakeOracle())

    assert commands.dispatch(["lock-check", "--corpus", "prism"]) == 0
    assert capsys.readouterr().err == ""
    assert any((tmp_path / "root").rglob("src/a.rs"))
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--quick", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )

    assert cli.main() == 0
    run = json.loads((ev / f"runs/{REFUSAL_DATE}-prism.json").read_text())
    assert run["meta"]["harness_sha"] == execution_head
    assert run["meta"]["harness_sha"] != lock.prism["harness_sha"]
    assert run["meta"]["sut_input_provenance"]["repository_metadata"] == [
        ".git/HEAD"
    ]


def test_pinned_corpus_without_git_runs_to_a_verdict(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    root = tmp_path / "root"
    monkeypatch.setenv("TIER_A_ROOT", str(root))
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(
        cli, "make_oracle", lambda cfg, init_options=None: FakeOracle()
    )
    co = materialize(lock, root, str(repo))
    assert not (co / ".git").exists()
    _, _, overlay, _ = preflight_quick(ev, live=False)
    assert "harness_sha" not in overlay
    real_sha = cli.corpus_sha
    calls = []
    monkeypatch.setattr(
        cli,
        "corpus_sha",
        lambda root_: calls.append(Path(root_)) or real_sha(root_),
    )
    monkeypatch.setattr(
        cli,
        "corpus_dirty",
        lambda root_: calls.append(Path(root_)) or True,
    )
    monkeypatch.setattr(sys, "argv", ["tier-a", "--quick", "--allow-stale-sut"])
    rc = cli.main()
    run = json.loads(next((ev / "runs").glob("*-prism.json")).read_text())
    assert co not in calls and all(c == ev.parent for c in calls)
    assert run["meta"]["corpus_sha"] == lock.prism["sha_aliases"][0][:12]
    assert run["meta"]["corpus_sha_full"] == lock.prism["sha_aliases"][0]
    assert run["meta"]["corpus_dirty"] is False
    assert run["meta"]["untracked_sources"] == []
    assert run["meta"]["harness_sha"] == lock.prism["sha_aliases"][0]
