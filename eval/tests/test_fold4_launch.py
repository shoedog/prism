import pytest

import tier_a_entrypoint
from tests.helpers import DECLARED_ADJ, base_ids, fake_run
from tier_a.compare_report import compare_runs


def test_console_execs_isolated_child_with_fresh_prefix_and_protocol(
    tmp_path, monkeypatch
):
    captured = {}

    def execve(executable, argv, env):
        captured.update(executable=executable, argv=argv, env=env)
        raise RuntimeError("captured")

    monkeypatch.setattr(tier_a_entrypoint.tempfile, "mkdtemp", lambda **_kw: str(tmp_path))
    monkeypatch.setattr(tier_a_entrypoint.os, "execve", execve)
    monkeypatch.setattr(tier_a_entrypoint, "_remove_private_cache", lambda _path: None)
    with pytest.raises(RuntimeError, match="captured"):
        tier_a_entrypoint._exec_private_child()

    assert captured["argv"][1:4] == [
        "-I",
        "-X",
        f"pycache_prefix={tmp_path}",
    ]
    assert captured["env"]["PYTHONDONTWRITEBYTECODE"] == "1"
    assert "PYTHONPATH" not in captured["env"]
    assert list(tmp_path.iterdir())


def test_child_rejects_prefix_without_launch_owned_nonce(tmp_path, monkeypatch):
    import tier_a_launch

    monkeypatch.setattr(tier_a_launch.sys, "flags", type("Flags", (), {"isolated": 1})())
    monkeypatch.setattr(tier_a_launch.sys, "pycache_prefix", str(tmp_path))
    with pytest.raises(RuntimeError, match="tier_a_launch_prefix_not_owned"):
        tier_a_launch.validate_launch(
            str(tmp_path), tier_a_launch.PROTOCOL, "wrong-nonce"
        )


def test_compare_refuses_record_without_valid_launch_provenance():
    ids = base_ids()
    base = fake_run(ids)
    candidate = fake_run(ids)
    base["meta"].pop("launch", None)
    result = compare_runs(base, candidate, DECLARED_ADJ)
    assert result["verdict"].startswith("NOT_COMPARABLE(launch_invalid: base")
