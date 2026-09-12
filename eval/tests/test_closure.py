import hashlib
import re
import unicodedata

import pytest

from tier_a.model import FunctionDef, Location
from tier_a.closure import (
    canonical_path,
    corpus_manifest,
    manifest_digest,
    policy_digest,
    policy_manifest,
    population_manifest,
    raw_digest,
    sut_inputs_manifest,
)
from tests.helpers import git_repo, copy_eval_policy_files, edit


def test_raw_digest_is_over_disk_bytes(tmp_path):
    f = tmp_path / "a.rs"
    f.write_bytes(b"fn a() {}\r\n")
    crlf_digest = raw_digest(f)
    assert crlf_digest == "sha256:" + hashlib.sha256(b"fn a() {}\r\n").hexdigest()
    assert f.read_text() == "fn a() {}\n"

    f.write_bytes(b"fn a() {}\n")
    lf_digest = raw_digest(f)
    assert lf_digest == "sha256:" + hashlib.sha256(b"fn a() {}\n").hexdigest()
    assert f.read_text() == "fn a() {}\n"
    assert crlf_digest != lf_digest


def test_canonical_path_posix_nfc_relative(tmp_path):
    nfc = "mañana.rs"
    nfd = unicodedata.normalize("NFD", "mañana.rs")
    nfc_path = canonical_path(tmp_path, tmp_path / "src" / nfc)
    nfd_path = canonical_path(tmp_path, tmp_path / "src" / nfd)
    assert nfc_path == nfd_path == "src/mañana.rs"

    contents_digest = "sha256:" + hashlib.sha256(b"fn a() {}\n").hexdigest()
    nfc_digest = manifest_digest([(nfc_path, contents_digest, "blob")])
    nfd_digest = manifest_digest([(nfd_path, contents_digest, "blob")])
    expected = hashlib.sha256(
        f"src/mañana.rs\t{contents_digest}\n".encode()
    ).hexdigest()
    assert nfc_digest == nfd_digest == "sha256:" + expected

    with pytest.raises(ValueError): canonical_path(tmp_path, tmp_path / ".." / "x.rs")


def test_manifest_digest_moves_on_one_byte(tmp_path):
    repo = git_repo(tmp_path, {"src/a.rs": "fn a() {}\n", "Cargo.toml": "[package]\nname='x'\nversion='0.1.0'\n"})
    d1 = manifest_digest(corpus_manifest(repo, ["src/a.rs", "Cargo.toml"]))
    (repo / "src/a.rs").write_text("fn a() { }\n")
    assert manifest_digest(corpus_manifest(repo, ["src/a.rs", "Cargo.toml"])) != d1


def test_policy_digest_catches_floor_and_minimum_edits(tmp_path):        # T-a′
    ev = copy_eval_policy_files(tmp_path)
    before_floor = policy_digest(policy_manifest(ev))
    edit(ev / "corpora.toml", "rust = 0.10", "rust = 0.14")
    after_floor = policy_digest(policy_manifest(ev))
    assert after_floor != before_floor

    ev2 = copy_eval_policy_files(tmp_path / "b")
    before_minimum = policy_digest(policy_manifest(ev2))
    edit(ev2 / "tier_a/accounting.py", "min(6, stratum", "min(4, stratum")
    after_minimum = policy_digest(policy_manifest(ev2))
    assert before_minimum == before_floor
    assert after_minimum != before_minimum


def test_population_manifest_records_package_dirs_and_rule():           # T-l′
    fds = [FunctionDef("f", "function", None, Location("pkg/m.py", 1, 2), 1)]
    policy = {"lang": "python"}
    before = population_manifest(fds, policy, {"pkg"}, "python")
    assert before["symbols"]["pkg/m.py:1"] == {
        "stratum": "Q-scoped",
        "eligible": True,
        "rule": None,
    }
    assert before["package_dirs"] == ["pkg"]

    after = population_manifest(
        fds + [FunctionDef("g", "function", None, Location("pkg/m.py", 3, 4), 3)],
        policy,
        {"pkg"},
        "python",
    )
    before_digest = policy_digest(before)
    after_digest = policy_digest(after)
    assert "pkg/m.py:3" not in before["symbols"]
    assert "pkg/m.py:3" in after["symbols"]
    assert after_digest != before_digest


def test_digests_are_stable_for_unchanged_inputs(tmp_path):
    path = tmp_path / "stable.rs"
    path.write_bytes(b"fn stable() {}\n")
    first_raw = raw_digest(path)
    first_manifest = manifest_digest([("stable.rs", first_raw, "blob")])

    second_raw = raw_digest(path)
    second_manifest = manifest_digest([("stable.rs", second_raw, "blob")])

    assert second_raw == first_raw
    assert second_manifest == first_manifest


def test_sut_inputs_manifest_includes_dep_info_targets(tmp_path):
    dep = tmp_path / "target/release/prism.d"; dep.parent.mkdir(parents=True)
    dep.write_text(f"{tmp_path}/target/release/prism: {tmp_path}/src/main.rs {tmp_path}/scripts/callable-observations/a.mjs\n")
    assert "scripts/callable-observations/a.mjs" in {p for p, _ in sut_inputs_manifest(tmp_path, tmp_path / "target/release/prism")}


def test_tsv_roundtrip_and_digest_use_sorted_lf_lines(tmp_path):
    from tier_a.closure import read_tsv, write_tsv

    rows = [("z.rs", "sha256:z", "blob-z"), ("a.rs", "sha256:a", "blob-a")]
    path = tmp_path / "closure/manifest.tsv"
    write_tsv(rows, path)

    assert path.read_bytes() == (
        b"a.rs\tsha256:a\tblob-a\nz.rs\tsha256:z\tblob-z\n"
    )
    assert read_tsv(path) == [rows[1], rows[0]]
    expected = hashlib.sha256(b"a.rs\tsha256:a\nz.rs\tsha256:z\n").hexdigest()
    assert manifest_digest(rows) == "sha256:" + expected


def test_sut_inputs_manifest_handles_escaped_spaces_and_missing_dep_info(tmp_path):
    source = tmp_path / "src/a b.rs"
    source.parent.mkdir()
    source.write_bytes(b"fn a() {}\n")
    dep = tmp_path / "target/release/prism.d"
    dep.parent.mkdir(parents=True)
    escaped = str(source).replace(" ", "\\ ")
    dep.write_text(f"{tmp_path}/target/release/prism: {escaped}\n")

    assert sut_inputs_manifest(tmp_path, dep.with_suffix("")) == [
        ("src/a b.rs", raw_digest(source))
    ]
    with pytest.raises(FileNotFoundError):
        sut_inputs_manifest(tmp_path / "missing", tmp_path / "missing/prism")


def test_anchored_repo_uses_mode_specific_sample_digests(tmp_path):
    from tests.helpers import anchored_repo

    lock, _ev, _repo = anchored_repo(tmp_path)
    quick_digest = lock.prism["sample"]["digest"]["quick"]
    full_digest = lock.prism["sample"]["digest"]["full"]

    assert re.fullmatch(r"sha256:[0-9a-f]{64}", quick_digest)
    assert re.fullmatch(r"sha256:[0-9a-f]{64}", full_digest)
    assert quick_digest != full_digest


def test_anchored_repo_publishes_real_manifests_and_can_remain_unpublished(
    tmp_path, monkeypatch
):
    import json

    from tier_a.closure import read_tsv
    from tier_a.lock import load_lock
    from tests.helpers import ADJ_DIGEST, ADJ_RECORDS, anchored_repo

    original_write_text = type(tmp_path).write_text

    def translated_write_text(path, data, *args, **kwargs):
        if path.name == "adjudications.jsonl":
            return path.write_bytes(data.replace("\n", "\r\n").encode())
        return original_write_text(path, data, *args, **kwargs)

    monkeypatch.setattr(type(tmp_path), "write_text", translated_write_text)

    lock, ev, repo = anchored_repo(tmp_path / "published")
    assert load_lock(ev / "tier-a.lock.toml") == lock
    assert read_tsv(repo / lock.prism["closure"]["manifest"])
    assert read_tsv(repo / lock.prism["sut"]["inputs"])
    assert raw_digest(repo / lock.prism["snapshot"]["path"]) == lock.prism["snapshot"]["digest"]
    adjudication_path = ev / "adjudications.jsonl"
    adjudication_bytes = "".join(
        json.dumps(record) + "\n" for record in ADJ_RECORDS
    ).encode()
    assert adjudication_path.read_bytes() == adjudication_bytes
    assert raw_digest(adjudication_path) == ADJ_DIGEST
    assert lock.prism["harness_sha"] == lock.prism["sha_aliases"][0]
    assert set(lock.prism["env"]["values"]) <= set(lock.prism["env"]["allow"])

    _lock, unpublished_ev, _repo = anchored_repo(tmp_path / "unpublished", publish=False)
    assert not (unpublished_ev / "closure").exists()
    assert not (unpublished_ev / "tier-a.lock.toml").exists()
