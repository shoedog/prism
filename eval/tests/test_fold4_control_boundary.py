import json
import os
import re
from pathlib import Path

import pytest

from tests.helpers import ADJ_RECORDS, base_ids, fake_run
from tier_a import closure, commands
from tier_a.closure import manifest_digest, raw_digest
from tier_a.lock import Lock, resolve_corpus_identity


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


def _native_loader_manifest_names() -> set[str]:
    source = (Path(__file__).parents[2] / "src/repo_loader.rs").read_text()
    predicate = re.search(
        r"fn is_manifest_name\(name: &str\) -> bool \{(.*?)\n\}", source, re.S
    )
    assert predicate is not None
    names = set(re.findall(r'"([^"]+)"', predicate.group(1)))
    assert names
    return names


def test_recursive_corpus_controls_cover_native_loader_manifest_predicate(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    names = _native_loader_manifest_names()

    for name in names:
        control = checkout / "nested" / name
        control.parent.mkdir(exist_ok=True)
        control.write_text(f"control={name}\n")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == sorted(f"nested/{name}" for name in names)


def test_native_manifest_control_set_is_derived_exactly_from_loader_predicate():
    assert closure._NATIVE_MANIFEST_BASENAMES == _native_loader_manifest_names()


def test_symlinked_control_file_keeps_lexical_path_identity(tmp_path):
    lock, checkout = _rust_checkout(tmp_path)
    (checkout / "src/Cargo.toml").symlink_to("../Cargo.toml")

    identity = resolve_corpus_identity(lock, checkout)

    assert identity.dirty is True
    assert identity.untracked == ["src/Cargo.toml"]


def _compare_inputs(tmp_path):
    adjudications = tmp_path / "adjudications.jsonl"
    adjudications.write_text("".join(json.dumps(row) + "\n" for row in ADJ_RECORDS))
    identities = base_ids()
    identities["data"] = {
        "eval/adjudications.jsonl": {"digest": raw_digest(adjudications)}
    }
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": "sha256:" + "b" * 64,
    }
    base = tmp_path / "base.json"
    candidate = tmp_path / "candidate.json"
    base.write_text(json.dumps(fake_run(identities)))
    candidate.write_text(json.dumps(fake_run(candidate_identities)))
    return base, candidate, adjudications


@pytest.mark.parametrize("link_kind", ("hard", "symbolic"))
def test_compare_refuses_alias_between_output_pair(tmp_path, monkeypatch, link_kind):
    base, candidate, adjudications = _compare_inputs(tmp_path)
    output = tmp_path / "out"
    output_json = tmp_path / "out.json"
    output_md = tmp_path / "out.md"
    output_json.write_text("reserved\n")
    if link_kind == "hard":
        os.link(output_json, output_md)
    else:
        output_md.symlink_to(output_json.name)
    monkeypatch.setattr(commands, "_harness_sha", lambda _eval_dir: "a" * 40)

    with pytest.raises(ValueError, match="compare_outputs_alias_each_other"):
        commands.compare(
            base,
            candidate,
            output,
            adjudications=adjudications,
            repo_root=tmp_path,
        )
