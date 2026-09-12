import os
import subprocess
import tomllib

import pytest

from tests.helpers import anchored_repo, fake_run, git_head, stub_binary, FakeSut
from tier_a.lock import lock_identities, integrity_check, Lock
from tier_a.closure import policy_digest, policy_manifest
from tier_a.bootstrap import (
    CandidateIdentity,
    finalize_policy,
    mark_history,
    publish_anchor_v1,
    stage_candidate,
    validate_staged,
)
from tier_a.sut import PrismCli, SutStale


STAGE = lambda repo, ev, tmp: stage_candidate(
    repo,
    ev,
    ev.parent / "prism",
    {"cargo": {"features": "all"}},
    tmp / "root",
    str(repo),
)
OUT = lambda ev, tmp: (ev, ev.parent / "docs/eval/tier-a", tmp / "root/sut")


def test_stage_then_validate_needs_no_published_lock(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    assert (
        not (ev / "tier-a.lock.toml").exists()
        and validate_staged(cand) == []
        and "baseline_report" not in cand.lock.prism
        and "blessed" not in cand.lock.prism
    )
    (ev / "adjudications.jsonl").write_text("{}\n")
    assert (
        "data_digest_moved: eval/adjudications.jsonl"
        in validate_staged(cand)
    )


def test_bootstrap_refuses_identity_inequality_and_publishes_nothing(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    ids = lock_identities(cand.lock)
    run = fake_run({**ids, "sut_digest": "sha256:other"})
    with pytest.raises(ValueError, match="identity mismatch: sut_digest"):
        publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    assert not (ev / "tier-a.lock.toml").exists() and not any(
        p for p in (ev / "closure").iterdir() if not p.name.startswith(".staging")
    )
    run2 = fake_run({**ids, "snapshot_digest": "sha256:other"})
    with pytest.raises(ValueError, match="identity mismatch: snapshot_digest"):
        publish_anchor_v1(cand, run2, *OUT(ev, tmp_path))


def test_bootstrap_publishes_atomically_and_lock_rederives(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    run = fake_run(lock_identities(cand.lock))
    published = publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    assert (
        integrity_check(published, ev) == []
        and published.prism["blessed"]["by"] == "bootstrap"
        and published.prism["baseline_report"]["digest"].startswith("sha256:")
    )
    assert (
        tmp_path
        / "root/sut"
        / published.prism["sut"]["digest"].split(":")[1]
        / "prism"
    ).exists()
    assert (
        not list(ev.glob("*.tmp"))
        and not list((ev / "closure").glob("*.tmp"))
        and not list((ev / "closure").glob(".staging-*"))
    )


def test_crash_before_publish_leaves_nothing_and_half_publish_is_refused(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    run = fake_run(lock_identities(cand.lock))
    real = os.replace
    calls = {"n": 0}

    def crash_on_lock(src, dst):
        if str(dst).endswith("tier-a.lock.toml"):
            raise OSError("simulated crash before the lock rename")
        return real(src, dst)

    monkeypatch.setattr(os, "replace", crash_on_lock)
    with pytest.raises(OSError):
        publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    monkeypatch.setattr(os, "replace", real)
    assert not (ev / "tier-a.lock.toml").exists() and any(
        (ev / "closure").glob("prism-*.tsv")
    )
    assert (
        "half_published_anchor: closure manifests present without a lock"
        in integrity_check(None, ev)
    )
    published = publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    assert integrity_check(published, ev) == []


def test_staged_candidate_files_never_trigger_half_publication(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    assert any((ev / "closure").glob(".staging-*/*.tsv")) and not (
        ev / "tier-a.lock.toml"
    ).exists()
    assert integrity_check(None, ev) == []
    (ev / "closure" / "prism-corpus-x.tsv").write_text("")
    assert (
        "half_published_anchor: closure manifests present without a lock"
        in integrity_check(None, ev)
    )


def test_bootstrap_sut_path_survives_the_policy_commit(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    binary = stub_binary(ev.parent / "prism", git_head(repo)[:12])
    finalize_policy(ev)
    with pytest.raises(ValueError, match="policy_uncommitted"):
        STAGE(repo, ev, tmp_path)
    subprocess.run(
        ["git", "-C", str(repo), "commit", "-qam", "tier-a: finalize policy"],
        check=True,
    )
    with pytest.raises(SutStale):
        PrismCli(str(repo), sut_bin=str(binary))
    cand = STAGE(repo, ev, tmp_path)
    sut = PrismCli.from_verified(
        str(repo),
        str(binary),
        cand.lock.prism["sut"]["sha"],
        cand.lock.prism["sut"]["digest"],
    )
    assert (
        sut.sha_full == cand.lock.prism["sut"]["sha"]
        and len(sut.sha_full) == 40
        and sut.sha == sut.sha_full[:12]
        and sut.dirty is False
    )


def test_bootstrap_refuses_report_without_identities(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    with pytest.raises(ValueError, match="not bindable: "):
        publish_anchor_v1(
            cand,
            {
                "meta": {
                    "corpus": "prism",
                    "admitted": True,
                    "baseline_invalid": False,
                }
            },
            *OUT(ev, tmp_path),
        )


def test_bootstrap_refuses_unadmitted_or_invalid_runs_with_matching_identities(
    tmp_path,
):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    ids = lock_identities(cand.lock)
    bad1 = fake_run(ids)
    bad1["meta"]["baseline_invalid"] = True
    bad1["meta"]["invalid_reasons"] = [
        "stratum C-method: 4/6 successful probes"
    ]
    with pytest.raises(ValueError, match="not publishable: baseline_invalid"):
        publish_anchor_v1(cand, bad1, *OUT(ev, tmp_path))
    bad2 = fake_run(ids)
    bad2["meta"]["admitted"] = False
    with pytest.raises(ValueError, match="not publishable: not admitted"):
        publish_anchor_v1(cand, bad2, *OUT(ev, tmp_path))
    assert not (ev / "tier-a.lock.toml").exists()


def test_policy_is_finalized_before_staging_and_integrity_holds_after_publish(
    tmp_path,
):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    assert "pinned_sha" in (ev / "corpora.toml").read_text()
    finalize_policy(ev)
    finalize_policy(ev)
    assert (
        "pinned_sha"
        not in tomllib.loads((ev / "corpora.toml").read_text())["corpus"]["prism"]
    )
    subprocess.run(
        ["git", "-C", str(repo), "commit", "-qam", "tier-a: finalize policy"],
        check=True,
    )
    cand = STAGE(repo, ev, tmp_path)
    published = publish_anchor_v1(
        cand, fake_run(lock_identities(cand.lock)), *OUT(ev, tmp_path)
    )
    assert (
        integrity_check(published, ev) == []
        and published.prism["policy"]["digest"]
        == policy_digest(policy_manifest(ev))
    )


def test_mark_history_prepends_once(tmp_path):
    md = tmp_path / "baseline.md"
    md.write_text("# Tier-A Baseline — 2026-07-04 (plan complete)\nbody\n")
    mark_history(md, "2026-09-12")
    mark_history(md, "2026-09-12")
    assert md.read_text().count("pre-boundary") == 1 and md.read_text().startswith(
        "# Tier-A Baseline — anchor v1 (2026-09-12)"
    )
