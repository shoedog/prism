import json
import os

import pytest

from tests.helpers import ADJ_RECORDS, base_ids, fake_run
from tier_a import commands
from tier_a.closure import raw_digest


CANDIDATE_DIGEST = "sha256:" + "b" * 64


@pytest.mark.parametrize("suffix", (".json", ".md"))
@pytest.mark.parametrize("target_name", ("base", "candidate", "adjudications"))
@pytest.mark.parametrize("link_kind", ("hard", "symbolic"))
def test_compare_output_refuses_every_link_alias_of_an_input(
    tmp_path, monkeypatch, suffix, target_name, link_kind
):
    adjudications = tmp_path / "adjudications.jsonl"
    adjudications.write_text("".join(json.dumps(row) + "\n" for row in ADJ_RECORDS))
    digest = raw_digest(adjudications)
    identities = base_ids()
    identities["data"] = {"eval/adjudications.jsonl": {"digest": digest}}
    candidate_identities = {
        **identities,
        "sut_sha_full": "b" * 40,
        "sut_digest": CANDIDATE_DIGEST,
    }
    base = tmp_path / "base.json"
    candidate = tmp_path / "candidate.json"
    base.write_text(json.dumps(fake_run(identities)))
    candidate.write_text(json.dumps(fake_run(candidate_identities)))
    targets = {"base": base, "candidate": candidate, "adjudications": adjudications}
    output = tmp_path / f"out{suffix}"
    if link_kind == "hard":
        os.link(targets[target_name], output)
    else:
        output.symlink_to(targets[target_name])
    before = targets[target_name].read_bytes()
    monkeypatch.setattr(commands, "_harness_sha", lambda _eval_dir: "a" * 40)

    with pytest.raises(ValueError, match="compare_output_aliases_input"):
        commands.compare(
            base,
            candidate,
            tmp_path / "out",
            adjudications=adjudications,
            repo_root=tmp_path,
        )

    assert targets[target_name].read_bytes() == before
