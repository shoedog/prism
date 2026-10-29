import dataclasses
import json
import os
import subprocess
import sys
import types
from pathlib import Path

import pytest
from tests.helpers import (
    DEFAULTS,
    FakeOracle,
    FakeSut,
    admitted_fake_run,
    anchored_repo,
    copy_fixture,
    edit,
    git_head,
)
from tier_a import boundary, cli, commands, matrix
from tier_a.artifacts import ArtifactPaths
from tier_a.boundary import (
    BoundaryViolation,
    EnvRead,
    Observation,
    OpenAudit,
    admit,
    load_declared_inputs,
    observe_archive,
    observe_cargo,
    scrubbed_env,
)
from tier_a.closure import (
    manifest_digest,
    raw_digest,
    read_tsv,
    sut_inputs_manifest,
    write_tsv,
)
from tier_a.lock import Lock, integrity_check
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


def _bind_sut_provenance(lock, repo, rows):
    tracked = sorted(
        path
        for path, _digest in rows
        if not path.startswith((".git/", "target/")) and (repo / path).is_file()
    )
    if tracked:
        subprocess.run(["git", "add", "--", *tracked], cwd=repo, check=True)
    if subprocess.run(["git", "diff", "--cached", "--quiet"], cwd=repo).returncode:
        subprocess.run(
            ["git", "commit", "-m", "bind synthetic SUT inputs"],
            cwd=repo,
            check=True,
            capture_output=True,
        )
    lock.prism["sut"]["sha"] = git_head(repo)
    lock._sut_archive_inputs = {
        path: (repo / path).read_bytes()
        for path, _digest in rows
        if path.startswith("target/") and (repo / path).is_file()
    }


def test_scrubbed_env_keeps_only_allow_list(monkeypatch):
    for name in ALLOW:
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("MODE", "on")
    monkeypatch.setenv("PATH", "/usr/bin")
    lock = Lock(prism={"env": {"allow": ALLOW, "forbid_read_outside_allow": True}})
    assert scrubbed_env(lock) == {"PATH": "/usr/bin"}


def test_archive_observation_prefers_archived_env_declarations_and_requires_fallback(
    tmp_path,
):
    lock, _ev, repo = anchored_repo(tmp_path)
    artifacts = ArtifactPaths.from_lock(repo, lock)
    output_path = "target/release/build/x-fixture/output"
    archived = artifacts.sut_archive_input(output_path)
    archived.parent.mkdir(parents=True, exist_ok=True)
    archived.write_text("cargo:rerun-if-env-changed=MODE\n")
    manifest = repo / lock.prism["sut"]["inputs"]
    rows = read_tsv(manifest)
    rows.append((output_path, raw_digest(archived)))
    rebound = manifest.with_name(f"sut-inputs-{manifest_digest(rows)[7:19]}.tsv")
    write_tsv(rows, rebound)
    manifest.unlink()
    lock.prism["sut"]["inputs"] = rebound.relative_to(repo).as_posix()
    artifacts = ArtifactPaths.from_lock(repo, lock)

    reads, paths = observe_archive(lock, repo, {"MODE": "on"}, artifacts)
    assert reads == [EnvRead("MODE", True, "on")]
    assert paths == {row[0] for row in rows}

    fallback_rows = [row for row in rows if row[0] != output_path]
    fallback = rebound.with_name(
        f"sut-inputs-{manifest_digest(fallback_rows)[7:19]}.tsv"
    )
    write_tsv(fallback_rows, fallback)
    rebound.unlink()
    lock.prism["sut"]["inputs"] = fallback.relative_to(repo).as_posix()
    artifacts = ArtifactPaths.from_lock(repo, lock)
    lock.prism["env"].pop("allow")
    with pytest.raises(
        ValueError,
        match="sut_observation_unavailable: archive has no build outputs",
    ):
        observe_archive(lock, repo, {}, artifacts)


def test_env_read_unset_admits_and_set_refuses(tmp_path, monkeypatch):                     # T-q both legs (W2)
    lock, ev, repo = anchored_repo(tmp_path); corpus = copy_fixture("boundary/mode-corpus", tmp_path)

    def complete_selected_binary(build_dir):
        binary = build_dir / "mode_corpus"
        binary.write_bytes(b"fixture binary\n")
        binary.with_suffix(".d").write_text(
            f"{binary}: {corpus / 'src/lib.rs'} {corpus / 'build.rs'}\n"
        )
        _write_binary_fingerprint(
            corpus,
            "mode_corpus",
            [],
            binary=binary,
        )

    env0 = {**scrubbed_env(lock), "PATH": os.environ["PATH"], "HOME": os.environ["HOME"]}
    subprocess.run(["cargo", "build", "--target-dir", str(tmp_path / "t0")], cwd=corpus, env=env0, check=True, capture_output=True)
    release_output = corpus / "target/release/build/mode_corpus-fixture/output"
    release_output.parent.mkdir(parents=True)
    release_output.write_text(next((tmp_path / "t0").rglob("output")).read_text())
    complete_selected_binary(tmp_path / "t0/debug")
    reads, deps = observe_cargo(tmp_path / "t0/debug", corpus, env0)
    assert reads == [EnvRead("MODE", False, None)]
    declared = load_declared_inputs(lock, ev); closure = {"Cargo.toml", "build.rs", "src/lib.rs"}      # NON-EMPTY declared closure (astra W4)
    admit(lock, declared, Observation(reads, set(), set()), closure_paths=closure, eval_dir=ev)         # unset ⇒ admitted
    env1 = {**env0, "MODE": "on"}
    subprocess.run(["cargo", "build", "--target-dir", str(tmp_path / "t1")], cwd=corpus, env=env1, check=True, capture_output=True)
    release_output.write_text(next((tmp_path / "t1").rglob("output")).read_text())
    complete_selected_binary(tmp_path / "t1/debug")
    reads1, _ = observe_cargo(tmp_path / "t1/debug", corpus, env1); assert reads1 == [EnvRead("MODE", True, "on")]
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
    else:
        fixture = ev / "fixtures/new/expected.toml"
        fixture.parent.mkdir(parents=True)
        fixture.write_text("expected = true\n")

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


def test_tracked_sut_input_is_verified_from_sut_commit_not_worktree(tmp_path):
    from tier_a.closure import read_tsv, write_tsv

    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    declared = load_declared_inputs(lock, ev)
    closure = {"src/a.rs", "Cargo.toml"}
    admit(lock, declared, Observation([], set(), set()), closure, ev)

    (repo / "src/a.rs").write_text("fn changed_only_in_worktree() {}\n")
    admit(lock, declared, Observation([], set(), set()), closure, ev)

    rows = [
        (path, "sha256:" + "0" * 64 if path == "src/a.rs" else digest)
        for path, digest in read_tsv(manifest)
    ]
    write_tsv(rows, manifest)
    assert integrity_check(lock, ev) == []
    with pytest.raises(BoundaryViolation) as exc:
        admit(lock, declared, Observation([], set(), set()), closure, ev)
    assert exc.value.reasons == ["data_digest_moved: src/a.rs"]


@pytest.mark.parametrize("bound", [False, True], ids=["unbound", "bound"])
@pytest.mark.parametrize(
    "path",
    [
        ".git/../src/a.rs",
        ".git/../Cargo.toml",
        "/src/a.rs",
        "src/./a.rs",
        r"src\a.rs",
        "src//a.rs",
    ],
)
def test_sut_input_noncanonical_paths_refuse_before_provenance_classification(
    tmp_path, bound, path
):
    from tier_a.closure import read_tsv, write_tsv

    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    metadata = [".git/HEAD", ".git/packed-refs", ".git/refs/heads/tier-a-reanchor"]
    rows = (
        read_tsv(manifest)
        + [(path, "sha256:" + "0" * 64)]
        + [(item, "sha256:" + "0" * 64) for item in metadata]
    )
    write_tsv(rows, manifest)
    if bound:
        assert integrity_check(lock, ev) == []
    else:
        lock.__dict__.pop("_sut_rows", None)

    reasons, _paths, provenance = boundary._sut_input_verification(lock, ev)

    assert reasons == [f"sut_input_path_noncanonical: {path}"]
    assert path not in {
        item for class_paths in provenance.values() for item in class_paths
    }
    assert provenance["repository_metadata"] == metadata


@pytest.mark.parametrize(
    ("mutation", "expected"),
    [
        ("tracked", "data_digest_moved: src/a.rs"),
        ("archive-corrupt", "data_digest_moved: target/generated.rs"),
        ("archive-missing", "sut_input_provenance_missing: target/generated.rs"),
    ],
)
@pytest.mark.parametrize("bound", [False, True], ids=["unbound", "bound"])
def test_published_sut_input_reasons_preserve_refusals(
    tmp_path, mutation, expected, bound
):
    from tier_a.closure import read_tsv, write_tsv

    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    rows = read_tsv(manifest)
    if mutation == "tracked":
        rows = [
            (path, "sha256:" + "0" * 64 if path == "src/a.rs" else digest)
            for path, digest in rows
        ]
    else:
        generated = repo / "target/generated.rs"
        generated.parent.mkdir(parents=True, exist_ok=True)
        generated.write_text("fn generated() {}\n")
        rows.append(("target/generated.rs", raw_digest(generated)))
        archived = (
            Path(lock.prism["sut"]["archive"]).parent
            / "inputs/target/generated.rs"
        )
        if mutation == "archive-corrupt":
            archived.parent.mkdir(parents=True, exist_ok=True)
            archived.write_text("corrupt\n")
    write_tsv(rows, manifest)
    if bound:
        assert integrity_check(lock, ev) == []

    reasons, paths = boundary._sut_input_reasons(lock, ev)

    assert reasons == [expected]
    assert expected.rsplit(": ", 1)[1] in paths


def test_sut_input_provenance_admits_other_checkouts_and_records_every_class(tmp_path):
    from tier_a.closure import read_tsv, write_tsv

    lock, ev, repo = anchored_repo(tmp_path)
    manifest = repo / lock.prism["sut"]["inputs"]
    generated = repo / "target/generated.rs"
    generated.parent.mkdir(parents=True, exist_ok=True)
    generated.write_text("fn generated() {}\n")
    write_tsv(
        read_tsv(manifest)
        + [
            (".git/HEAD", raw_digest(repo / ".git/HEAD")),
            ("target/generated.rs", raw_digest(generated)),
        ],
        manifest,
    )
    archived = Path(lock.prism["sut"]["archive"]).parent / "inputs/target/generated.rs"
    archived.parent.mkdir(parents=True, exist_ok=True)
    archived.write_bytes(generated.read_bytes())
    assert integrity_check(lock, ev) == []

    verification = admit(
        lock,
        load_declared_inputs(lock, ev),
        Observation([], set(), set()),
        {"src/a.rs", "Cargo.toml"},
        ev,
    )
    assert verification["sut_input_provenance"] == {
        "tracked_source": ["Cargo.toml", "src/a.rs"],
        "repository_metadata": [".git/HEAD"],
        "archived_generated": ["target/generated.rs"],
    }

    for name, revision in (("published", "HEAD"), ("archived-base", lock.prism["sut"]["sha"])):
        checkout = tmp_path / name
        subprocess.run(
            ["git", "clone", "--no-hardlinks", str(repo), str(checkout)],
            check=True,
            capture_output=True,
        )
        subprocess.run(
            ["git", "-C", str(checkout), "checkout", "--detach", revision],
            check=True,
            capture_output=True,
        )
        assert not (checkout / "target").exists()
        assert boundary._sut_input_reasons(lock, checkout / "eval")[0] == []


def test_observe_cargo_equals_selected_binary_manifest_and_keeps_refusal(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    dep_info = binary.with_suffix(".d")
    dep_info.write_text(
        f"{binary}: {repo / 'src/a.rs'} {repo / 'Cargo.toml'}\n\n"
        f"{repo / 'src/a.rs'}:\n{repo / 'Cargo.toml'}:\n"
    )
    other = repo / "src/other.rs"
    other.write_text("fn other() {}\n")
    unrelated = binary.parent / "deps/other-crate.d"
    unrelated.parent.mkdir(parents=True, exist_ok=True)
    unrelated.write_text(f"other: {other}\n\n{other}:\n")

    _env_reads, observed = observe_cargo(
        binary.parent, repo, {}, "prism"
    )
    declared = {path for path, _digest in sut_inputs_manifest(repo, binary)}

    assert observed == declared
    assert "src/other.rs" not in observed
    assert "src/a.rs:" not in observed

    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation([], observed | {"src/other.rs"}, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == ["undeclared_read: src/other.rs"]


def test_observe_cargo_separates_renamed_binary_from_package_outputs(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    original = repo / "target/release/prism"
    binary = repo / "bin/renamed-prism"
    binary.parent.mkdir()
    original.rename(binary)
    original.with_suffix(".d").rename(binary.with_suffix(".d"))
    _write_binary_fingerprint(
        repo,
        binary.stem,
        [],
        binary=binary,
    )
    (repo / "build.rs").write_text("fn main() {}\n")
    package_output = repo / "target/release/build/x-fixture/output"
    package_output.parent.mkdir(parents=True)
    package_output.write_text(
        "cargo:rerun-if-changed=build.rs\n"
        "cargo:rerun-if-env-changed=MODE\n"
    )
    (repo / "src/debug-only.rs").write_text("fn debug_only() {}\n")
    debug_output = repo / "target/debug/build/x-debug/output"
    debug_output.parent.mkdir(parents=True)
    debug_output.write_text(
        "cargo:rerun-if-changed=src/debug-only.rs\n"
        "cargo:rerun-if-env-changed=DEBUG_ONLY\n"
    )

    env_reads, observed = observe_cargo(
        binary.parent,
        repo,
        {"MODE": "on", "DEBUG_ONLY": "on"},
        binary.stem,
    )
    declared = {path for path, _digest in sut_inputs_manifest(repo, binary)}

    assert observed == declared
    assert env_reads == [EnvRead("MODE", True, "on")]
    assert "src/debug-only.rs" not in observed
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed, set()),
            {"build.rs"},
            ev,
        )
    assert exc.value.reasons == ["env_read_outside_allow: MODE"]


def test_selected_dependency_generated_input_is_observed_and_declared(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    (repo / "Cargo.toml").write_text(
        "[package]\nname='prism'\nversion='0.1.0'\nedition='2021'\n"
        "[dependencies]\ndep_fixture={path='dep_fixture'}\n"
    )
    (repo / "src/main.rs").write_text(
        "fn main() { println!(\"{}\", dep_fixture::generated()); }\n"
    )
    dependency = repo / "dep_fixture"
    (dependency / "src").mkdir(parents=True)
    (dependency / "Cargo.toml").write_text(
        "[package]\nname='dep_fixture'\nversion='0.1.0'\nedition='2021'\n"
        "build='build.rs'\n"
    )
    (dependency / "build.rs").write_text(
        "use std::{env, fs, path::Path};\n"
        "fn main() {\n"
        "  let out = env::var(\"OUT_DIR\").unwrap();\n"
        "  fs::write(Path::new(&out).join(\"generated.rs\"), "
        "\"pub fn generated() -> u32 { 7 }\\n\").unwrap();\n"
        "}\n"
    )
    (dependency / "src/lib.rs").write_text(
        "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\n"
    )
    subprocess.run(
        ["cargo", "build", "--release", "--offline"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )

    binary = repo / "target/release/prism"
    generated = next(
        (repo / "target/release/build").glob("dep_fixture-*/out/generated.rs")
    )
    generated_path = generated.relative_to(repo).as_posix()
    dependency_dep_info = next(
        (repo / "target/release/deps").glob("dep_fixture-*.d")
    )
    assert str(generated) in dependency_dep_info.read_text()
    binary.with_suffix(".d").write_text(
        f"{binary}: {repo / 'src/main.rs'}\n"
    )
    env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    full_rows = sut_inputs_manifest(repo, binary)
    manifest = repo / lock.prism["sut"]["inputs"]
    reduced_rows = [row for row in full_rows if row[0] != generated_path]
    _bind_sut_provenance(lock, repo, reduced_rows)
    write_tsv(reduced_rows, manifest)

    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == [f"undeclared_read: {generated_path}"]

    write_tsv(full_rows, manifest)
    _bind_sut_provenance(lock, repo, full_rows)
    assert observed == {path for path, _digest in full_rows}
    admit(
        lock,
        load_declared_inputs(lock, ev),
        Observation(env_reads, observed, set()),
        set(),
        ev,
    )

def _cargo_fingerprint_value(marker: str) -> int:
    return int.from_bytes(bytes.fromhex(marker), "little")


def _write_binary_fingerprint(
    repo,
    target,
    dependencies,
    *,
    discriminator="0000000000000001",
    binary=None,
    artifact_bytes=None,
):
    binary = binary or repo / "target/release" / target
    cargo_target = target.replace("-", "_")
    fingerprint = (
        binary.parent
        / ".fingerprint"
        / f"prism-{discriminator}"
        / f"bin-{cargo_target}.json"
    )
    fingerprint.parent.mkdir(parents=True, exist_ok=True)
    fingerprint.write_text(json.dumps({"deps": dependencies}))
    (fingerprint.parent / f"bin-{cargo_target}").write_text(
        "0000000000000000"
    )
    artifact = binary.parent / "deps" / f"{cargo_target}-{discriminator}"
    artifact.parent.mkdir(parents=True, exist_ok=True)
    artifact.write_bytes(
        binary.read_bytes() if artifact_bytes is None else artifact_bytes
    )
    source_dep_info = binary.with_suffix(".d")
    artifact.with_suffix(".d").write_bytes(source_dep_info.read_bytes())
    return fingerprint


def _write_library_fingerprint(
    repo,
    package,
    target,
    discriminator,
    marker,
    prerequisites,
    dependencies=(),
):
    fingerprint = (
        repo
        / "target/release/.fingerprint"
        / f"{package}-{discriminator}"
    )
    fingerprint.mkdir(parents=True, exist_ok=True)
    (fingerprint / f"lib-{target}").write_text(marker)
    (fingerprint / f"lib-{target}.json").write_text(
        json.dumps({"deps": list(dependencies)})
    )
    dep_info = repo / "target/release/deps" / f"{target}-{discriminator}.d"
    dep_info.parent.mkdir(parents=True, exist_ok=True)
    artifact = dep_info.with_suffix(".rlib")
    dep_info.write_text(
        f"{artifact}: {' '.join(str(path) for path in prerequisites)}\n"
    )
    return dep_info


def _write_build_script_fingerprint(
    repo,
    package,
    discriminator,
    marker_name,
    marker,
    dependencies,
    prerequisites=(),
):
    fingerprint = (
        repo
        / "target/release/.fingerprint"
        / f"{package}-{discriminator}"
    )
    fingerprint.mkdir(parents=True, exist_ok=True)
    (fingerprint / marker_name).write_text(marker)
    (fingerprint / f"{marker_name}.json").write_text(
        json.dumps({"deps": list(dependencies)})
    )
    if marker_name.startswith("build-script-"):
        dep_info = (
            repo
            / "target/release/build"
            / fingerprint.name
            / f"build_script_build-{discriminator}.d"
        )
        dep_info.parent.mkdir(parents=True, exist_ok=True)
        artifact = dep_info.with_suffix("")
        dep_info.write_text(
            f"{artifact}: {' '.join(str(path) for path in prerequisites)}\n"
        )
        return dep_info
    return None


def test_dependency_graph_excludes_stale_same_name_build_variant(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    selected = repo / "target/release/build/dep_fixture-selected/out/generated.rs"
    stale = repo / "target/release/build/dep_fixture-stale/out/unused.rs"
    selected.parent.mkdir(parents=True)
    stale.parent.mkdir(parents=True)
    selected.write_text("pub fn selected() {}\n")
    stale.write_text("pub fn stale() {}\n")
    selected_marker = "0102030405060708"
    stale_marker = "1112131415161718"
    _write_binary_fingerprint(
        repo,
        "prism",
        [[1, "dep_fixture", False, _cargo_fingerprint_value(selected_marker)]],
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "1111111111111111",
        selected_marker,
        [selected],
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "2222222222222222",
        stale_marker,
        [stale],
    )

    env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    full_rows = sut_inputs_manifest(repo, binary)
    manifest = repo / lock.prism["sut"]["inputs"]
    _bind_sut_provenance(lock, repo, full_rows)
    write_tsv(full_rows, manifest)
    stale_path = stale.relative_to(repo).as_posix()
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed | {stale_path}, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == [f"undeclared_read: {stale_path}"]

    selected_path = selected.relative_to(repo).as_posix()
    assert observed == {path for path, _digest in full_rows}
    assert selected_path in observed
    assert stale_path not in observed
    admit(
        lock,
        load_declared_inputs(lock, ev),
        Observation(env_reads, observed, set()),
        set(),
        ev,
    )

    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "4444444444444444",
        selected_marker,
        [selected],
    )
    with pytest.raises(
        ValueError,
        match=f"dependency_unit_ambiguous: dep_fixture:{selected_marker}",
    ):
        sut_inputs_manifest(repo, binary)


def test_hyphenated_dependency_uses_target_name_for_dep_info(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    generated = repo / "target/release/build/dep-fixture-selected/out/generated.rs"
    generated.parent.mkdir(parents=True)
    generated.write_text("pub fn generated() {}\n")
    marker = "2122232425262728"
    _write_binary_fingerprint(
        repo,
        "prism",
        [[1, "dep_fixture", False, _cargo_fingerprint_value(marker)]],
    )
    dep_info = _write_library_fingerprint(
        repo,
        "dep-fixture",
        "dep_fixture",
        "3333333333333333",
        marker,
        [generated],
    )

    env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    full_rows = sut_inputs_manifest(repo, binary)
    generated_path = generated.relative_to(repo).as_posix()
    manifest = repo / lock.prism["sut"]["inputs"]
    reduced_rows = [row for row in full_rows if row[0] != generated_path]
    _bind_sut_provenance(lock, repo, reduced_rows)
    write_tsv(reduced_rows, manifest)
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == [f"undeclared_read: {generated_path}"]

    write_tsv(full_rows, manifest)
    _bind_sut_provenance(lock, repo, full_rows)
    assert observed == {path for path, _digest in full_rows}
    assert generated_path in observed
    admit(
        lock,
        load_declared_inputs(lock, ev),
        Observation(env_reads, observed, set()),
        set(),
        ev,
    )

    dep_info.unlink()
    with pytest.raises(
        ValueError,
        match="dependency_dep_info_unavailable: dep_fixture-3333333333333333",
    ):
        sut_inputs_manifest(repo, binary)

    missing_marker = "3132333435363738"
    _write_binary_fingerprint(
        repo,
        "prism",
        [[1, "dep_fixture", False, _cargo_fingerprint_value(missing_marker)]],
    )
    with pytest.raises(
        ValueError,
        match=f"dependency_unit_unavailable: dep_fixture:{missing_marker}",
    ):
        sut_inputs_manifest(repo, binary)


def test_sut_fingerprint_directory_unavailable_refuses(tmp_path):
    lock, _ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    _write_binary_fingerprint(repo, "prism", [])
    manifest = repo / lock.prism["sut"]["inputs"]
    manifest_before = manifest.read_bytes()
    fingerprint_root = binary.parent / ".fingerprint"
    hidden = binary.parent / ".fingerprint-hidden"
    fingerprint_root.rename(hidden)

    with pytest.raises(ValueError, match="sut_fingerprint_unavailable: prism"):
        sut_inputs_manifest(repo, binary)
    assert manifest.read_bytes() == manifest_before

    hidden.rename(fingerprint_root)
    _write_binary_fingerprint(
        repo,
        "prism",
        [],
        discriminator="0000000000000002",
    )
    with pytest.raises(ValueError, match="sut_fingerprint_ambiguous: prism"):
        sut_inputs_manifest(repo, binary)


def test_sut_fingerprint_populations_absent_refuses_without_manifest_mutation(
    tmp_path,
):
    lock, _ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    _write_binary_fingerprint(repo, "prism", [])
    manifest = repo / lock.prism["sut"]["inputs"]
    manifest_before = manifest.read_bytes()

    fingerprint_root = binary.parent / ".fingerprint"
    artifact_root = binary.parent / "deps"
    fingerprint_hidden = binary.parent / ".fingerprint-hidden"
    artifact_hidden = binary.parent / "deps-hidden"
    fingerprint_root.rename(fingerprint_hidden)
    artifact_root.rename(artifact_hidden)

    with pytest.raises(ValueError, match="sut_fingerprint_unavailable: prism"):
        sut_inputs_manifest(repo, binary)
    assert manifest.read_bytes() == manifest_before

    fingerprint_hidden.rename(fingerprint_root)
    artifact_hidden.rename(artifact_root)
    _env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    declared = {path for path, _digest in sut_inputs_manifest(repo, binary)}
    assert observed == declared


def test_sut_fingerprint_does_not_fall_through_to_stale_root(tmp_path):
    lock, _ev, repo = anchored_repo(tmp_path)
    binary = repo / "target/release/prism"
    default_discriminator = "0000000000000001"
    default_fingerprint = (
        binary.parent / ".fingerprint" / f"prism-{default_discriminator}"
    )
    for path in default_fingerprint.iterdir():
        path.unlink()
    default_fingerprint.rmdir()
    (binary.parent / "deps" / f"prism-{default_discriminator}").unlink()
    (binary.parent / "deps" / f"prism-{default_discriminator}.d").unlink()
    selected = repo / "target/release/build/selected/out/generated.rs"
    stale = repo / "target/release/build/stale/out/unused.rs"
    selected.parent.mkdir(parents=True)
    stale.parent.mkdir(parents=True)
    selected.write_text("pub fn selected() {}\n")
    stale.write_text("pub fn stale() {}\n")
    selected_marker = "4142434445464748"
    stale_marker = "5152535455565758"
    selected_root = _write_binary_fingerprint(
        repo,
        "prism",
        [[1, "dep_fixture", False, _cargo_fingerprint_value(selected_marker)]],
        discriminator="1111111111111111",
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "3333333333333333",
        selected_marker,
        [selected],
    )
    _write_binary_fingerprint(
        repo,
        "prism",
        [[1, "dep_fixture", False, _cargo_fingerprint_value(stale_marker)]],
        discriminator="2222222222222222",
        artifact_bytes=b"stale binary",
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "4444444444444444",
        stale_marker,
        [stale],
    )
    selected_root.unlink()
    manifest = repo / lock.prism["sut"]["inputs"]
    manifest_before = manifest.read_bytes()

    with pytest.raises(ValueError, match="sut_fingerprint_unavailable: prism"):
        sut_inputs_manifest(repo, binary)
    assert manifest.read_bytes() == manifest_before
    assert stale.relative_to(repo).as_posix() not in manifest.read_text()


def _selected_build_script_chain_fixture(repo, *, include_ordering_edge=False):
    binary = repo / "target/release/prism"
    dependency = repo / "dep_fixture"
    (dependency / "src").mkdir(parents=True)
    library_source = dependency / "src/lib.rs"
    build_source = dependency / "build.rs"
    library_source.write_text("pub fn selected() {}\n")
    build_source.write_text("fn main() {}\n")
    host_generated = (
        repo / "target/release/build/dep_fixture-host/out/host_generated.rs"
    )
    unrelated_generated = (
        repo / "target/release/build/dep_fixture-unrelated/out/unused.rs"
    )
    host_generated.parent.mkdir(parents=True)
    unrelated_generated.parent.mkdir(parents=True)
    host_generated.write_text("pub fn host_generated() {}\n")
    unrelated_generated.write_text("pub fn unrelated() {}\n")

    target_marker = "6162636465666768"
    run_marker = "7172737475767778"
    compile_marker = "8182838485868788"
    host_marker = "9192939495969798"
    unrelated_marker = "a1a2a3a4a5a6a7a8"
    ordering_marker = "b1b2b3b4b5b6b7b8"
    _write_binary_fingerprint(
        repo,
        "prism",
        [
            [1, "dep_fixture", False, _cargo_fingerprint_value(target_marker)],
            [2, "build_script_build", False, _cargo_fingerprint_value(run_marker)],
        ],
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "1111111111111111",
        target_marker,
        [library_source],
    )
    _write_build_script_fingerprint(
        repo,
        "dep_fixture",
        "2222222222222222",
        "run-build-script-build-script-build",
        run_marker,
        [
            [3, "build_script_build", False, _cargo_fingerprint_value(compile_marker)],
            *(
                [
                    [
                        5,
                        "build_script_build",
                        False,
                        _cargo_fingerprint_value(ordering_marker),
                    ]
                ]
                if include_ordering_edge
                else []
            ),
        ],
    )
    compile_dep_info = _write_build_script_fingerprint(
        repo,
        "dep_fixture",
        "3333333333333333",
        "build-script-build-script-build",
        compile_marker,
        [[4, "dep_fixture", False, _cargo_fingerprint_value(host_marker)]],
        [build_source],
    )
    if include_ordering_edge:
        _write_build_script_fingerprint(
            repo,
            "ordering_fixture",
            "6666666666666666",
            "run-build-script-build-script-build",
            ordering_marker,
            [],
        )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "4444444444444444",
        host_marker,
        [host_generated],
    )
    _write_library_fingerprint(
        repo,
        "dep_fixture",
        "dep_fixture",
        "5555555555555555",
        unrelated_marker,
        [unrelated_generated],
    )

    return binary, host_generated, unrelated_generated, compile_dep_info


def test_selected_build_script_chain_includes_host_dependency_inputs(tmp_path):
    lock, ev, repo = anchored_repo(tmp_path)
    binary, host_generated, unrelated_generated, _compile_dep_info = (
        _selected_build_script_chain_fixture(repo)
    )

    env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    full_rows = sut_inputs_manifest(repo, binary)
    host_path = host_generated.relative_to(repo).as_posix()
    unrelated_path = unrelated_generated.relative_to(repo).as_posix()
    manifest = repo / lock.prism["sut"]["inputs"]
    reduced_rows = [row for row in full_rows if row[0] != host_path]
    _bind_sut_provenance(lock, repo, reduced_rows)
    write_tsv(reduced_rows, manifest)
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == [f"undeclared_read: {host_path}"]

    write_tsv(full_rows, manifest)
    _bind_sut_provenance(lock, repo, full_rows)
    assert observed == {path for path, _digest in full_rows}
    assert host_path in observed
    assert unrelated_path not in observed
    admit(
        lock,
        load_declared_inputs(lock, ev),
        Observation(env_reads, observed, set()),
        set(),
        ev,
    )
    with pytest.raises(BoundaryViolation) as exc:
        admit(
            lock,
            load_declared_inputs(lock, ev),
            Observation(env_reads, observed | {unrelated_path}, set()),
            set(),
            ev,
        )
    assert exc.value.reasons == [f"undeclared_read: {unrelated_path}"]


def test_selected_build_script_chain_refuses_missing_compiler_unit_but_skips_ordering_edge(
    tmp_path,
):
    _lock, _ev, repo = anchored_repo(tmp_path)
    binary, host_generated, _unrelated_generated, compile_dep_info = (
        _selected_build_script_chain_fixture(repo, include_ordering_edge=True)
    )

    _env_reads, observed = observe_cargo(binary.parent, repo, {}, binary.stem)
    declared = {path for path, _digest in sut_inputs_manifest(repo, binary)}
    assert observed == declared
    assert host_generated.relative_to(repo).as_posix() in observed

    compile_fingerprint = (
        repo
        / "target/release/.fingerprint/dep_fixture-3333333333333333"
        / "build-script-build-script-build.json"
    )
    compile_fingerprint.unlink()
    manifest = repo / _lock.prism["sut"]["inputs"]
    manifest_before = manifest.read_bytes()
    with pytest.raises(
        ValueError,
        match=(
            "build_script_compile_unit_unavailable: "
            "build_script_build:8182838485868788"
        ),
    ):
        sut_inputs_manifest(repo, binary)
    assert manifest.read_bytes() == manifest_before
    assert compile_dep_info.is_file()
