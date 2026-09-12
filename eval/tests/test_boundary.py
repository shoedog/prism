import dataclasses
import json
import os
import subprocess
import sys
import types

import pytest
from tests.helpers import (
    DEFAULTS,
    FakeOracle,
    FakeSut,
    admitted_fake_run,
    anchored_repo,
    copy_fixture,
    edit,
)
from tier_a import boundary, cli, commands, matrix
from tier_a.boundary import (
    BoundaryViolation,
    EnvRead,
    Observation,
    OpenAudit,
    admit,
    load_declared_inputs,
    observe_cargo,
    scrubbed_env,
)
from tier_a.closure import raw_digest
from tier_a.lock import Lock
from tier_a.report import render_markdown
from tier_a.sut import PrismCli

ALLOW = ["PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "TMPDIR", "LANG", "TIER_A_ROOT"]
REFUSAL_DATE = "2099-01-04"


def _full_data_meta(lock, ev):
    declared = load_declared_inputs(lock, ev)
    data = {
        path: {"digest": digest}
        for path, digest in sorted(declared.digests.items())
    }
    data["eval/fixtures"] = {"digest": declared.fixtures_digest}
    return data


def _assert_no_run_or_report(ev):
    assert list((ev / "runs").glob("*.json")) == []
    report_dir = ev.parent / "docs/eval/tier-a"
    assert not (report_dir / f"{REFUSAL_DATE}-prism.json").exists()
    assert not (report_dir / f"{REFUSAL_DATE}-prism.md").exists()


def test_scrubbed_env_keeps_only_allow_list(monkeypatch):
    for name in ALLOW:
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("MODE", "on")
    monkeypatch.setenv("PATH", "/usr/bin")
    lock = Lock(prism={"env": {"allow": ALLOW, "forbid_read_outside_allow": True}})
    assert scrubbed_env(lock) == {"PATH": "/usr/bin"}


def test_env_read_unset_admits_and_set_refuses(tmp_path, monkeypatch):                     # T-q both legs (W2)
    lock, ev, repo = anchored_repo(tmp_path); corpus = copy_fixture("boundary/mode-corpus", tmp_path)
    env0 = {**scrubbed_env(lock), "PATH": os.environ["PATH"], "HOME": os.environ["HOME"]}
    subprocess.run(["cargo", "build", "--target-dir", str(tmp_path / "t0")], cwd=corpus, env=env0, check=True, capture_output=True)
    reads, deps = observe_cargo(tmp_path / "t0", corpus, env0)
    assert reads == [EnvRead("MODE", False, None)]
    declared = load_declared_inputs(lock, ev); closure = {"Cargo.toml", "build.rs", "src/lib.rs"}      # NON-EMPTY declared closure (astra W4)
    admit(lock, declared, Observation(reads, set(), set()), closure_paths=closure, eval_dir=ev)         # unset ⇒ admitted
    env1 = {**env0, "MODE": "on"}
    subprocess.run(["cargo", "build", "--target-dir", str(tmp_path / "t1")], cwd=corpus, env=env1, check=True, capture_output=True)
    reads1, _ = observe_cargo(tmp_path / "t1", corpus, env1); assert reads1 == [EnvRead("MODE", True, "on")]
    with pytest.raises(BoundaryViolation) as e: admit(lock, declared, Observation(reads1, set(), set()), closure, ev)
    assert e.value.reasons == ["env_read_outside_allow: MODE"]


def test_data_inputs_loaded_and_digested_before_admission(tmp_path):                       # T-r + W3
    lock, ev, repo = anchored_repo(tmp_path)
    with OpenAudit(ev) as audit: declared = load_declared_inputs(lock, ev)
    snapshot_path = lock.prism["snapshot"]["path"]
    assert declared.digests == {
        "eval/adjudications.jsonl": raw_digest(ev / "adjudications.jsonl"),
        snapshot_path: raw_digest(repo / snapshot_path),
        "eval/tier_a/pinned.py": raw_digest(ev / "tier_a/pinned.py"),
    }
    assert audit.opens == set(declared.digests)
    closure = {"src/a.rs", "Cargo.toml"}
    admit(lock, declared, Observation([], set(), audit.opens), closure, ev)                      # admitted, returns None
    edit(ev / "adjudications.jsonl", '"verdict": "oracle_miss"', '"verdict": "prism_fp"')          # a LEGAL same-direction mutation (R4-W2: `adjudication.py:10-18` — `confirmed_*` are spotcheck labels, `{"changed": true}` would raise in Adjudication(**…))
    assert declared.digests["eval/adjudications.jsonl"] == lock.prism["data"]["adjudications"]["digest"]   # the verdict uses the loaded copy
    with OpenAudit(ev) as audit2: declared2 = load_declared_inputs(lock, ev)
    with pytest.raises(BoundaryViolation) as e: admit(lock, declared2, Observation([], set(), audit2.opens), closure, ev)
    assert e.value.reasons == ["data_digest_moved: eval/adjudications.jsonl"]


@pytest.mark.parametrize(
    ("target", "reason"),
    [
        ("adjudications", "data_digest_moved: eval/adjudications.jsonl"),
        ("snapshot", "data_digest_moved: {snapshot}"),
        ("pinned", "data_digest_moved: eval/tier_a/pinned.py"),
        ("fixtures", "data_digest_moved: eval/fixtures"),
        ("sut", "data_digest_moved: src/a.rs"),
    ],
)
def test_every_declared_data_drift_uses_one_exact_vocabulary(tmp_path, target, reason):
    lock, ev, repo = anchored_repo(tmp_path)
    snapshot = repo / lock.prism["snapshot"]["path"]
    if target == "adjudications":
        edit(
            ev / "adjudications.jsonl",
            '"verdict": "oracle_miss"',
            '"verdict": "prism_fp"',
        )
    elif target == "snapshot":
        snapshot.write_text(snapshot.read_text() + "\n")
    elif target == "pinned":
        pinned = ev / "tier_a/pinned.py"
        pinned.write_text(pinned.read_text() + "\n")
    elif target == "fixtures":
        fixture = ev / "fixtures/new/expected.toml"
        fixture.parent.mkdir(parents=True)
        fixture.write_text("expected = true\n")
    else:
        (repo / "src/a.rs").write_text("fn a() { }\n")

    declared = load_declared_inputs(lock, ev)
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            declared,
            Observation([], set(), set()),
            {"src/a.rs", "Cargo.toml"},
            ev,
        )

    expected = reason.format(snapshot=lock.prism["snapshot"]["path"])
    assert exc.value.reasons == [expected]


def test_undeclared_harness_read_has_exact_reason(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    (ev / "extra.json").write_text("{}")
    with OpenAudit(ev) as audit:
        declared = load_declared_inputs(lock, ev)
        (ev / "extra.json").read_text()
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            declared,
            Observation([], set(), audit.opens),
            {"src/a.rs", "Cargo.toml"},
            ev,
        )
    assert exc.value.reasons == ["undeclared_read: eval/extra.json"]


def test_oracle_is_not_a_boundary_subject(tmp_path, monkeypatch):                            # owner ruling 2026-09-12
    lock, ev, repo = anchored_repo(tmp_path); declared = load_declared_inputs(lock, ev); closure = {"src/a.rs", "Cargo.toml"}
    assert [f.name for f in dataclasses.fields(Observation)] == ["env_reads", "dep_paths", "harness_opens"]   # nothing about the oracle is observed
    admit(lock, declared, Observation([], set(), set()), closure, ev)                              # admitted with no oracle information at all
    run = admitted_fake_run(); assert "observation_coverage" not in run["meta"]
    md = render_markdown(run); assert "Observation coverage" not in md and "reads not observed" not in md
    launched = {}; monkeypatch.setattr(cli, "make_oracle", lambda cfg, init_options=None: launched.update(init=init_options) or FakeOracle())
    monkeypatch.setattr(cli, "PrismCli", FakeSut); monkeypatch.setattr(cli, "EVAL_DIR", ev); monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    monkeypatch.setattr(sys, "argv", ["tier-a", "--quick", "--allow-stale-sut"]); cli.main()
    assert launched == {"init": lock.prism["oracle"]["init"]}                                       # the oracle gets its profile and nothing else — no env, no read policy


def test_quick_and_matrix_version_launches_use_exact_scrubbed_env(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path)
    for name in ALLOW:
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("MODE", "on")
    monkeypatch.setenv("PATH", "/usr/bin")
    expected = {"PATH": "/usr/bin"}
    routed = []
    launched = []

    class VersionSeen(Exception):
        pass

    real_run = PrismCli._run

    def spy_run(self, args):
        routed.append((args, self.env))
        return real_run(self, args)

    def stop_at_version(argv, **kwargs):
        assert argv == ["/sut/prism", "--version"]
        launched.append(kwargs.get("env"))
        raise VersionSeen

    monkeypatch.setattr(PrismCli, "_run", spy_run)
    monkeypatch.setattr(subprocess, "run", stop_at_version)
    args = types.SimpleNamespace(sut_bin="/sut/prism", allow_stale_sut=True)

    with pytest.raises(VersionSeen):
        cli.run_corpus("prism", {}, DEFAULTS, args, env=expected)

    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(commands, "preflight_matrix", lambda _eval_dir: lock)
    monkeypatch.setattr(
        sys, "argv", ["tier-a", "--matrix-only", "--sut-bin", "/sut/prism"]
    )
    with pytest.raises(VersionSeen):
        cli.main()

    assert routed == [
        (["--version"], expected),
        (["--version"], expected),
    ]
    assert launched == [expected, expected]
    assert all("MODE" not in env for _, env in routed)


def test_matrix_only_dfg_launcher_uses_exact_scrubbed_env(tmp_path, monkeypatch):
    lock, ev, repo = anchored_repo(tmp_path)
    for name in ALLOW:
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("MODE", "on")
    monkeypatch.setenv("PATH", "/usr/bin")
    seen = []
    completed = iter(
        [
            types.SimpleNamespace(returncode=0, stdout="{}\n", stderr=""),
            types.SimpleNamespace(
                returncode=0,
                stdout='{"dfg_label_loop_carried": 0}',
                stderr="",
            ),
        ]
    )
    case = types.SimpleNamespace(
        path=tmp_path,
        expect_dfg_edges=[{"from": "a", "to": "b"}],
        expect_dfg_stats={"dfg_label_loop_carried_min": 0},
        status=None,
        capability="dfg_x",
    )
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(cli, "PrismCli", FakeSut)
    monkeypatch.setattr(
        matrix.subprocess,
        "run",
        lambda cmd, **kw: seen.append(kw.get("env")) or next(completed),
    )
    monkeypatch.setattr(matrix, "_dfg_expectation_matches", lambda *args: True)
    monkeypatch.setattr(
        cli,
        "run_matrix",
        lambda fixtures, sut, languages: [matrix._run_dfg_case(case, "rust", sut)],
    )
    monkeypatch.setattr(sys, "argv", ["tier-a", "--matrix-only", "--allow-stale-sut"])

    assert cli.main() == 0
    assert seen == [{"PATH": "/usr/bin"}, {"PATH": "/usr/bin"}]


def test_report_only_replay_does_not_inherit_admission(tmp_path, monkeypatch):               # astra W9
    lock, ev, repo = anchored_repo(tmp_path); monkeypatch.setattr(cli, "EVAL_DIR", ev)
    run = admitted_fake_run(); run["meta"]["data"] = _full_data_meta(lock, ev)
    same = cli.recompute_metrics_from_stored(run, ev)
    assert same["meta"].get("derived") is None
    assert same["meta"]["admitted"] is True
    assert same["meta"].get("invalid_reasons", []) == []
    edit(ev / "adjudications.jsonl", '"verdict": "oracle_miss"', '"verdict": "prism_fp"')            # legal mutation (R4-W2)
    derived = cli.recompute_metrics_from_stored(run, ev)
    assert derived["meta"]["derived"] is True
    assert derived["meta"]["admitted"] is False
    assert derived["meta"]["invalid_reasons"] == [
        "derived_from_changed_inputs: eval/adjudications.jsonl"
    ]


def test_report_only_missing_declared_adjudication_digest_refuses(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    run = admitted_fake_run()
    run["meta"]["data"] = _full_data_meta(lock, ev)
    del run["meta"]["data"]["eval/adjudications.jsonl"]
    edit(ev / "adjudications.jsonl", '"verdict": "oracle_miss"', '"verdict": "prism_fp"')

    derived = cli.recompute_metrics_from_stored(run, ev)

    assert derived["meta"]["derived"] is True
    assert derived["meta"]["admitted"] is False
    assert derived["meta"]["invalid_reasons"] == [
        "missing_declared_digest: eval/adjudications.jsonl"
    ]


@pytest.mark.parametrize(
    "reason",
    [
        "env_read_outside_allow: MODE",
        "data_digest_moved: eval/adjudications.jsonl",
        "sut_input_absent_present_transition: generated.rs",
        "undeclared_read: eval/extra.json",
    ],
)
def test_each_refusal_writes_no_artifacts_and_exact_stderr(
    tmp_path, monkeypatch, capsys, reason
):
    lock, ev, repo = anchored_repo(tmp_path); monkeypatch.setattr(cli, "EVAL_DIR", ev); monkeypatch.setenv("TIER_A_ROOT", str(tmp_path / "root"))
    monkeypatch.setattr(cli, "PrismCli", FakeSut); monkeypatch.setattr(cli, "make_oracle", lambda cfg, init_options=None: FakeOracle())
    monkeypatch.setattr(commands, "admit", lambda *a, **k: (_ for _ in ()).throw(BoundaryViolation([reason])))
    monkeypatch.setattr(sys, "argv", ["tier-a", "--quick", "--allow-stale-sut", "--date", REFUSAL_DATE])
    assert cli.main() == 3
    assert capsys.readouterr().err == f"FATAL boundary_violation: {reason}\n"
    _assert_no_run_or_report(ev)


def test_lockless_quick_stays_inside_boundary_and_is_not_admitted(
    tmp_path, monkeypatch
):
    lock, ev, repo = anchored_repo(tmp_path)
    (ev / "tier-a.lock.toml").unlink()
    for path in (ev / "closure").iterdir():
        if path.name != ".gitkeep":
            path.unlink()
    for name in ALLOW:
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("MODE", "on")
    monkeypatch.setenv("PATH", "/usr/bin")
    seen = {}

    def fake_run_corpus(*args, **kwargs):
        sut = FakeSut()
        sut.env = kwargs.get("env")
        seen["sut_env"] = sut.env
        seen["compute_metrics"] = kwargs.get("compute_metrics")
        seen["declared"] = kwargs.get("data_inputs")
        run = admitted_fake_run()
        run["meta"]["invalid_reasons"] = []
        return run

    cfg = {
        "lang": "rust",
        "path": str(repo),
        "oracle": "rust-analyzer",
        "pinned_sha": lock.prism["sha_aliases"][0][:12],
        "excludes": [],
    }
    monkeypatch.setattr(cli, "EVAL_DIR", ev)
    monkeypatch.setattr(
        cli,
        "load_corpora",
        lambda: {"defaults": DEFAULTS, "corpus": {"prism": cfg}},
    )
    monkeypatch.setattr(cli, "run_corpus", fake_run_corpus)
    monkeypatch.setattr(commands, "observe_cargo", lambda *args: ([], set()))
    monkeypatch.setattr(
        sys,
        "argv",
        ["tier-a", "--quick", "--allow-stale-sut", "--date", REFUSAL_DATE],
    )

    assert cli.main() == 2
    run = json.loads((ev / f"runs/{REFUSAL_DATE}-prism.json").read_text())

    assert seen["sut_env"] == {"PATH": "/usr/bin"}
    assert seen["compute_metrics"] is False
    assert seen["declared"].digests == {
        "eval/adjudications.jsonl": raw_digest(ev / "adjudications.jsonl"),
        lock.prism["snapshot"]["path"]: raw_digest(
            repo / lock.prism["snapshot"]["path"]
        ),
        "eval/tier_a/pinned.py": raw_digest(ev / "tier_a/pinned.py"),
    }
    assert run["meta"]["admitted"] is False
    assert run["meta"]["baseline_invalid"] is True
    assert run["meta"]["invalid_reasons"] == ["no_lock"]


def test_open_audit_installs_one_dispatcher_and_scopes_collectors(tmp_path, monkeypatch):
    hooks = []
    monkeypatch.setattr(boundary.sys, "addaudithook", hooks.append)
    monkeypatch.setattr(boundary, "_AUDIT_DISPATCHER_INSTALLED", False, raising=False)
    monkeypatch.setattr(boundary, "_ACTIVE_OPEN_AUDITS", [], raising=False)
    eval_dir = tmp_path / "eval"
    eval_dir.mkdir()
    first_path = eval_dir / "first.json"
    second_path = eval_dir / "second.json"

    with OpenAudit(eval_dir) as first:
        assert len(hooks) == 1
        hooks[0]("open", (str(first_path), "r", 0))
    with OpenAudit(eval_dir) as second:
        assert len(hooks) == 1
        hooks[0]("open", (str(second_path), "r", 0))

    assert first.opens == {"eval/first.json"}
    assert second.opens == {"eval/second.json"}


def test_sut_input_absent_present_transition_refuses(tmp_path):
    from tier_a.closure import read_tsv, write_tsv

    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    write_tsv(read_tsv(manifest) + [("generated.rs", "absent")], manifest)
    declared = load_declared_inputs(lock, ev)
    closure = {"src/a.rs", "Cargo.toml"}
    admit(lock, declared, Observation([], set(), set()), closure, ev)

    (repo / "generated.rs").write_text("fn generated() {}\n")
    with pytest.raises(BoundaryViolation) as exc:
        admit(lock, declared, Observation([], set(), set()), closure, ev)
    assert exc.value.reasons == [
        "sut_input_absent_present_transition: generated.rs"
    ]
