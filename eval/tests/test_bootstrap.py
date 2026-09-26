import hashlib
import json
import os
import subprocess
import tomllib
from pathlib import Path

import pytest

from tests.helpers import (
    FakeOracle,
    FakeSut,
    anchored_repo,
    fake_run,
    git_head,
    stub_binary,
)
from tier_a import cli, commands
from tier_a.lock import IDENTITY_KEYS, lock_identities, integrity_check
from tier_a.closure import policy_digest, policy_manifest, population_manifest, raw_digest
from tier_a.corpus import load_snapshot
from tier_a.model import FunctionDef, Location
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


def _binary_sha(repo):
    return subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD^"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def _distinct_same_shape(value, marker):
    if isinstance(value, dict):
        assert value
        key = next(iter(value))
        return {
            item_key: _distinct_same_shape(item_value, marker)
            if item_key == key
            else item_value
            for item_key, item_value in value.items()
        }
    if isinstance(value, list):
        assert value
        return [_distinct_same_shape(value[0], marker), *value[1:]]
    if isinstance(value, str):
        return f"sha256:{marker}" if value.startswith("sha256:") else marker
    if isinstance(value, bool):
        return not value
    if isinstance(value, int):
        return value + 1
    raise AssertionError(f"unhandled identity leaf: {type(value)}")


def _identity_shape(value):
    if isinstance(value, dict):
        return {key: _identity_shape(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_identity_shape(item) for item in value]
    return type(value)


def _identity_mismatch(key, original):
    marker = hashlib.sha256(f"identity-mismatch:{key}".encode()).hexdigest()
    if key in {"corpus_sha_full", "sut_sha_full", "harness_sha"}:
        return marker[:40]
    if key == "sha_aliases":
        return [marker[:40]]
    if isinstance(original, str) and original.startswith("sha256:"):
        return f"sha256:{marker}"
    if key == "sample_digest":
        return {
            "quick": f"sha256:{marker}",
            "full": f"sha256:{hashlib.sha256(marker.encode()).hexdigest()}",
        }
    if key == "oracle":
        return f"rust-analyzer mismatch {marker}"
    if key == "oracle_init":
        return {"cargo": {"features": marker}}
    if key in {"runtime", "data", "env_allow", "env_values"}:
        return _distinct_same_shape(original, marker)
    if key == "seed":
        return original + 1
    raise AssertionError(f"unhandled identity key: {key}")


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


@pytest.mark.parametrize("key", IDENTITY_KEYS)
def test_bootstrap_refuses_identity_inequality_and_publishes_nothing(tmp_path, key):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    ids = lock_identities(cand.lock)
    mismatch = _identity_mismatch(key, ids[key])
    if key in {"runtime", "data", "env_allow", "env_values"}:
        assert _identity_shape(mismatch) == _identity_shape(ids[key])
    run = fake_run({**ids, key: mismatch})
    with pytest.raises(ValueError) as exc:
        publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    assert str(exc.value) == f"identity mismatch: {key}"
    assert not (ev / "tier-a.lock.toml").exists() and not any(
        p for p in (ev / "closure").iterdir() if not p.name.startswith(".staging")
    )


def test_candidate_uses_verified_sut_commit_for_snapshot_alias_and_population(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    binary_sha = _binary_sha(repo)
    historical_pin = "20c8490591a3"
    policy_path = ev / "corpora.toml"
    policy_path.write_text(
        policy_path.read_text().replace(binary_sha[:12], historical_pin)
    )
    binary_snapshot = ev / "snapshots" / f"prism-{binary_sha[:12]}.json"
    historical_snapshot = ev / "snapshots" / f"prism-{historical_pin}.json"
    historical_snapshot.write_text(
        binary_snapshot.read_text().replace('"file": "src/a.rs"', '"file": "src/old.rs"')
    )
    subprocess.run(
        ["git", "-C", str(repo), "commit", "-qam", "restore historical policy pin"],
        check=True,
    )

    cand = STAGE(repo, ev, tmp_path)
    expected_population = population_manifest(
        load_snapshot(binary_snapshot), policy_manifest(ev), set(), "rust"
    )
    expected_population_path = repo / cand.lock.prism["population"]["manifest"]

    assert historical_pin != binary_sha[:12]
    assert cand.lock.prism["sha_aliases"] == [binary_sha]
    assert cand.lock.prism["snapshot"] == {
        "path": binary_snapshot.relative_to(repo).as_posix(),
        "digest": raw_digest(binary_snapshot),
    }
    assert cand.lock.prism["population"] == {
        "digest": policy_digest(expected_population),
        "manifest": expected_population_path.relative_to(repo).as_posix(),
    }
    assert json.loads(cand.staged[expected_population_path].read_text()) == expected_population


def test_candidate_refuses_when_verified_sut_snapshot_is_unavailable(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    binary_sha = _binary_sha(repo)
    (ev / "snapshots" / f"prism-{binary_sha[:12]}.json").unlink()

    with pytest.raises(ValueError) as exc:
        STAGE(repo, ev, tmp_path)
    assert str(exc.value) == f"snapshot_unavailable: {binary_sha}"


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


def test_publish_captures_binary_bytes_and_mode_from_one_file_descriptor(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    binary = ev.parent / "prism"
    binary.chmod(0o755)
    cand = STAGE(repo, ev, tmp_path)
    run = fake_run(lock_identities(cand.lock))
    original_bytes = binary.read_bytes()
    real_read_bytes = Path.read_bytes
    real_fdopen = os.fdopen
    swapped = False
    binary_path_reads = 0

    def swap_binary():
        nonlocal swapped
        if swapped:
            return
        replacement = binary.with_name("replacement-prism")
        replacement.write_bytes(b"replacement binary bytes\n")
        replacement.chmod(0o600)
        os.replace(replacement, binary)
        swapped = True

    def replace_binary_after_path_read(path):
        nonlocal binary_path_reads
        contents = real_read_bytes(path)
        if path == binary:
            binary_path_reads += 1
            if binary_path_reads == 2:
                swap_binary()
        return contents

    def replace_binary_after_fd_open(fd, *args, **kwargs):
        swap_binary()
        return real_fdopen(fd, *args, **kwargs)

    monkeypatch.setattr(Path, "read_bytes", replace_binary_after_path_read)
    monkeypatch.setattr(os, "fdopen", replace_binary_after_fd_open)
    published = publish_anchor_v1(cand, run, *OUT(ev, tmp_path))
    archive = Path(published.prism["sut"]["archive"])
    assert archive.read_bytes() == original_bytes
    assert raw_digest(archive) == published.prism["sut"]["digest"]
    assert archive.stat().st_mode & 0o777 == 0o755


def test_crash_before_publish_leaves_nothing_and_half_publish_is_refused(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    cand = STAGE(repo, ev, tmp_path)
    run = fake_run(lock_identities(cand.lock))
    real = os.replace

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
    staging = Path("closure") / f".staging-{os.getpid()}"
    assert {path.relative_to(ev) for path in cand.staged.values()} == {
        staging / Path(cand.lock.prism["closure"]["manifest"]).name,
        staging / Path(cand.lock.prism["sut"]["inputs"]).name,
        staging / Path(cand.lock.prism["policy"]["manifest"]).name,
        staging / Path(cand.lock.prism["population"]["manifest"]).name,
    }
    assert not (ev / "tier-a.lock.toml").exists()
    assert integrity_check(None, ev) == []
    (ev / "closure" / "prism-corpus-x.tsv").write_text("")
    assert (
        "half_published_anchor: closure manifests present without a lock"
        in integrity_check(None, ev)
    )


def test_bootstrap_sut_path_survives_the_policy_commit(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path, publish=False)
    binary_sha = git_head(repo)
    binary = stub_binary(ev.parent / "prism", binary_sha[:12])
    finalize_policy(ev)
    with pytest.raises(ValueError) as exc:
        STAGE(repo, ev, tmp_path)
    assert str(exc.value) == "policy_uncommitted"
    subprocess.run(
        ["git", "-C", str(repo), "commit", "-qam", "tier-a: finalize policy"],
        check=True,
    )
    head = git_head(repo)
    with pytest.raises(SutStale) as exc:
        PrismCli(str(repo), sut_bin=str(binary))
    assert str(exc.value) == (
        f"binary {binary_sha[:12]} != HEAD {head}; "
        "rebuild (cargo build --release) or pass allow_stale=True"
    )
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
    with pytest.raises(ValueError) as exc:
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
    assert str(exc.value) == f"not bindable: {','.join(IDENTITY_KEYS)}"


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
    with pytest.raises(ValueError) as exc:
        publish_anchor_v1(cand, bad1, *OUT(ev, tmp_path))
    assert str(exc.value) == "not publishable: baseline_invalid"
    bad2 = fake_run(ids)
    bad2["meta"]["admitted"] = False
    with pytest.raises(ValueError) as exc:
        publish_anchor_v1(cand, bad2, *OUT(ev, tmp_path))
    assert str(exc.value) == "not publishable: not admitted"
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


def _tree_bytes(root):
    if not root.exists():
        return None
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def _bootstrap_rc(argv):
    try:
        return commands._bootstrap(argv)
    except SystemExit as exc:
        return int(exc.code)


def test_prepare_snapshot_only_writes_binary_sha_snapshot_and_is_idempotent(
    tmp_path, monkeypatch, capsys
):
    _lock, ev, repo = anchored_repo(tmp_path)
    (ev / "tier-a.lock.toml").unlink()
    binary = repo / "prism"
    binary_sha = _binary_sha(repo)
    assert binary_sha != git_head(repo)
    snapshot = ev / "snapshots" / f"prism-{binary_sha[:12]}.json"
    snapshot.unlink()
    expected = FunctionDef(
        "prepared", "function", None, Location("src/a.rs", 1, 1), 1
    )
    seen = {}

    class InventoryOracle(FakeOracle):
        def document_symbols(self, path):
            seen.setdefault("files", []).append(path)
            return [expected]

    def make_oracle(cfg, init_options=None):
        seen["cfg"] = cfg
        seen["init"] = init_options
        return InventoryOracle()

    monkeypatch.setattr(commands, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "make_oracle", make_oracle)
    protected_before = {
        "corpora": (ev / "corpora.toml").read_bytes(),
        "closure": _tree_bytes(ev / "closure"),
        "docs": _tree_bytes(repo / "docs/eval/tier-a"),
        "runs": _tree_bytes(ev / "runs"),
    }

    assert _bootstrap_rc(
        ["--prepare-snapshot-only", "--sut-bin", str(binary)]
    ) == 0
    assert load_snapshot(snapshot) == [expected]
    assert seen["files"] == ["src/a.rs"]
    assert seen["cfg"]["path"] == str(repo)
    assert seen["init"] == {"cargo": {"features": "all"}}
    assert capsys.readouterr().out == (
        f"snapshot prepared: sha={binary_sha} digest={raw_digest(binary)}\n"
    )
    assert protected_before == {
        "corpora": (ev / "corpora.toml").read_bytes(),
        "closure": _tree_bytes(ev / "closure"),
        "docs": _tree_bytes(repo / "docs/eval/tier-a"),
        "runs": _tree_bytes(ev / "runs"),
    }
    assert not (ev / "tier-a.lock.toml").exists()

    original = snapshot.read_bytes()
    monkeypatch.setattr(
        cli,
        "make_oracle",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("existing snapshot must not be regenerated")
        ),
    )
    assert _bootstrap_rc(
        ["--prepare-snapshot-only", "--sut-bin", str(binary)]
    ) == 0
    assert snapshot.read_bytes() == original
    assert capsys.readouterr().out == "snapshot present\n"


def test_prepare_snapshot_only_refuses_dirty_binary(tmp_path, monkeypatch, capsys):
    _lock, ev, repo = anchored_repo(tmp_path)
    (ev / "tier-a.lock.toml").unlink()
    binary = repo / "prism-dirty"
    binary.write_text(
        "#!/bin/sh\nprintf '%s\\n' 'slicing 3.1.2 "
        f"({_binary_sha(repo)[:12]}-dirty)'\n"
    )
    binary.chmod(0o755)
    monkeypatch.setattr(commands, "EVAL_DIR", ev)

    assert _bootstrap_rc(
        ["--prepare-snapshot-only", "--sut-bin", str(binary)]
    ) == 3
    assert capsys.readouterr().err == (
        "FATAL bootstrap_refused: binary was built from a dirty tree\n"
    )


def test_prepare_snapshot_only_refuses_non_ancestor_binary(
    tmp_path, monkeypatch, capsys
):
    _lock, ev, repo = anchored_repo(tmp_path)
    (ev / "tier-a.lock.toml").unlink()
    tree = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD^{tree}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    unrelated = subprocess.run(
        ["git", "-C", str(repo), "commit-tree", tree],
        input="unrelated\n",
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    binary = stub_binary(repo / "prism-unrelated", unrelated[:12])
    monkeypatch.setattr(commands, "EVAL_DIR", ev)

    assert _bootstrap_rc(
        ["--prepare-snapshot-only", "--sut-bin", str(binary)]
    ) == 3
    assert capsys.readouterr().err == (
        "FATAL bootstrap_refused: binary commit is not an ancestor of HEAD: "
        f"{unrelated}\n"
    )


def test_prepare_snapshot_only_refuses_existing_lock(tmp_path, monkeypatch, capsys):
    _lock, ev, repo = anchored_repo(tmp_path)
    monkeypatch.setattr(commands, "EVAL_DIR", ev)

    assert _bootstrap_rc(
        ["--prepare-snapshot-only", "--sut-bin", str(repo / "prism")]
    ) == 3
    assert capsys.readouterr().err == (
        "FATAL bootstrap_refused: anchor_already_exists\n"
    )
