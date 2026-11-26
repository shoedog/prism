"""Snapshot a verified corpus and run the guarded analyses serially."""

from __future__ import annotations

from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import gzip
import json
import os
from pathlib import Path
import select
import shutil
import stat
import subprocess
import sys
from tempfile import TemporaryDirectory

from . import receipt
from .corpora import REPO, verify
from .inputs import (DIRECTORY, AliasedInput, PinError, UnboundLink, bind_tree,
                     copy_snapshot, git, remove_tree, sha256_file)

DEFAULT_GUARD = REPO / "scripts" / "memguard" / "memguard.sh"
KIT_PATHS = ("scripts/measure", "scripts/memguard", "scripts/hostquiet")
MAX_STATUS_BYTES = 4096


class OutputError(ValueError):
    pass


def now():
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def diagnostic(message, code):
    print(f"measure run: {message}", file=sys.stderr)
    return code


@contextmanager
def claim_output(path):
    try:
        path.mkdir(parents=True, exist_ok=True)
        fd = os.open(path, DIRECTORY)
    except OSError as error:
        raise OutputError(f"cannot allocate output {path}: {error}") from error
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            if os.listdir(fd):
                raise OutputError(f"output directory is not empty: {path}")
        except OSError as error:
            raise OutputError(f"cannot claim output {path}: {error}") from error
        yield
    finally:
        os.close(fd)


def read_status(read_fd, process):
    """Drain the pipe while the wrapper runs, with bounded status storage."""
    data = bytearray()
    os.set_blocking(read_fd, False)
    while True:
        ready, _, _ = select.select([read_fd], [], [], 0.1)
        if ready:
            try:
                chunk = os.read(read_fd, MAX_STATUS_BYTES)
            except BlockingIOError:
                continue
            if not chunk:
                break
            if len(data) <= MAX_STATUS_BYTES:
                data.extend(chunk)
        elif process.poll() is not None:
            break
    return bytes(data)


def guarded(guard, limit, argv, env, stdout, stderr):
    start = now()
    read_fd, write_fd = os.pipe()
    code = status = None
    error = None
    try:
        with stdout.open("wb") as out, stderr.open("wb") as err:
            try:
                process = subprocess.Popen([str(guard), "--status-fd", str(write_fd),
                                            str(limit), *argv], env=env, stdout=out,
                                           stderr=err, pass_fds=(write_fd,))
            finally:
                os.close(write_fd)
                write_fd = None
            with process:
                raw = read_status(read_fd, process)
                code = receipt.shell_exit(process.wait())
            if len(raw) > MAX_STATUS_BYTES:
                raise ValueError("guard status is too large")
            status = receipt.guard_status(raw, code)
    except (OSError, ValueError) as failure:
        error = str(failure)
    finally:
        os.close(read_fd)
        if write_fd is not None:
            os.close(write_fd)
    if error:
        diagnostic(error, 4)
    return {"argv": argv, "exit": code, "guard_status": status, "start": start, "end": now()}


def kit_dirty(repo=REPO):
    """True when tracked kit files differ from HEAD or kit files are untracked."""
    return bool(git(repo, "--literal-pathspecs", "status", "--porcelain=v1", "-z",
                    "--untracked-files=all", "--", *KIT_PATHS))


def executable_hash(path):
    info = path.stat()
    if (not stat.S_ISREG(info.st_mode) or not info.st_mode & 0o444
            or not info.st_mode & 0o111 or not os.access(path, os.R_OK | os.X_OK)):
        raise OSError(f"executable is not a readable executable file: {path}")
    return sha256_file(path)


def recheck_binary(path, expected):
    if executable_hash(path) != expected:
        raise OSError("executable changed during measurement")


def guard_env(host, settings, guard):
    env = {**host, **settings}
    if guard == DEFAULT_GUARD:
        # The stock wrapper strips these from its product child.
        for name in ("MEMGUARD_LOG", "XDG_STATE_HOME", "HOST_QUIET_DIR"):
            if name in os.environ:
                env[name] = os.environ[name]
    return env


def outcome(attempt):
    status = attempt["guard_status"]
    if status and status["kind"] == "host_quiet":
        return 75
    return 0 if status and status["kind"] == "child" and attempt["exit"] == 0 else 4


def produce(stage, guard, limit, host, definition, artifacts):
    name, argv, settings, stdout, stderr = definition
    raw = [stage / "stdout.raw", stage / "stderr.raw"]
    attempt = guarded(guard, limit, argv, guard_env(host, settings, guard), *raw)
    for source, target in zip(raw, (stdout, stderr)):
        stored = stage / target
        if target.endswith(".gz"):
            with source.open("rb") as stream, stored.open("wb") as output:
                with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0) as packed:
                    shutil.copyfileobj(stream, packed)
            source.unlink()
        else:
            source.rename(stored)
        artifacts[target] = receipt.artifact(stored)
    return {**attempt, "id": name, "env": settings, "output": stdout, "stderr": stderr,
            "output_sha256": artifacts[stdout]["sha256"], "bytes": artifacts[stdout]["bytes"]}


def prediction_binding(path, frozen):
    if path is None:
        if frozen is not None:
            raise ValueError("a frozen prediction requires its file path")
        return None
    path = path.resolve()
    if not stat.S_ISREG(path.stat().st_mode):
        raise ValueError("prediction is not a regular file")
    actual = sha256_file(path)
    if frozen is not None and frozen != actual:
        raise ValueError("prediction differs from its frozen digest")
    return {"path": str(path), "sha256": actual}


def run(args, entry, trace, *, gate_sha256=None, gate_step_id=None, prediction_sha256=None):
    start = now()
    try:
        receipt.gate_binding(gate_sha256, gate_step_id)
        if gate_sha256 is not None and prediction_sha256 is None:
            raise ValueError("a gate run requires a frozen prediction digest")
        prediction = prediction_binding(args.prediction, prediction_sha256)
        if args.out.is_symlink():
            raise OutputError(f"linked output directory: {args.out}")
        out = args.out.resolve()
        if out.exists() and (not out.is_dir() or any(out.iterdir())):
            raise OutputError(f"output is not an empty directory: {out}")
    except (OSError, ValueError, RuntimeError) as error:
        return diagnostic(error, 2)
    if not receipt.valid_version_pin(args.expect_version):
        return diagnostic("invalid version pin", 3)
    try:
        corpus = verify(args.corpus, entry, args.corpora_root)
    except PinError as error:
        return diagnostic(error, 3)
    try:
        if any(path.exists() and path.samefile(corpus.root) for path in (out, *out.parents)):
            return diagnostic("output directory is inside the bound query root", 4)
        binary = args.binary.resolve()
        binary_sha = executable_hash(binary)
        guard = args.memguard.resolve()
        host = {name: os.environ[name] for name in receipt.HOST_ENV if name in os.environ}
        host["LC_ALL"] = "C"
        record = {
            "schema": receipt.SCHEMA,
            "start": start,
            "kit_commit": git(REPO, "rev-parse", "HEAD").decode().strip(),
            "kit_dirty": kit_dirty(),
            "binary": {"path": str(binary), "sha256": binary_sha},
            "prediction": prediction,
            "gate_sha256": gate_sha256,
            "gate_step_id": gate_step_id,
            "options": {"bytes": not args.no_bytes, "sites": args.sites, "cache": "no-cache",
                        "trace_env": trace},
            "guard": {"path": str(guard), "limit_gb": args.limit_gb,
                      "sha256": sha256_file(guard)},
            "host": {"platform": sys.platform, "cpus": os.cpu_count(), "env": host},
            "commands": [],
            "artifacts": {},
        }
        with claim_output(out), TemporaryDirectory(prefix=".stage-", dir=out) as temp:
            stage = Path(temp)
            return measure(args.expect_version, corpus, out, stage, binary, guard, record)
    except OutputError as error:
        return diagnostic(error, 2)
    except (OSError, PinError, RuntimeError) as error:
        return diagnostic(error, 4)


def measure(expect_version, corpus, out, stage, binary, guard, record):
    root = out / "snapshot"
    retain = False
    try:
        copy_snapshot(corpus.root, root, corpus.binding)
        snapshot = bind_tree(root)
        admitted = corpus.binding
        if (snapshot.sha256, snapshot.state_sha256) != (admitted.sha256, admitted.state_sha256):
            return diagnostic("private snapshot differs from the admitted input", 4)
        limit = record["guard"]["limit_gb"]
        host = record["host"]["env"]
        artifacts = record["artifacts"]
        expected_binary = record["binary"]["sha256"]
        recheck_binary(binary, expected_binary)
        version_args = [str(binary), "--version"]
        version = guarded(guard, limit, version_args, guard_env(host, {}, guard),
                          stage / receipt.VERSION, stage / receipt.VERSION_STDERR)
        code = outcome(version)
        if code:
            return diagnostic("version refused by host quiet" if code == 75 else "version failed", code)

        for name in (receipt.VERSION, receipt.VERSION_STDERR):
            artifacts[name] = receipt.artifact(stage / name)
        record["version_command"] = {**version, "env": {}, "stderr": receipt.VERSION_STDERR,
                                     "stdout_sha256": artifacts[receipt.VERSION]["sha256"],
                                     "stdout_bytes": artifacts[receipt.VERSION]["bytes"]}
        recheck_binary(binary, expected_binary)
        try:
            display = (stage / receipt.VERSION).read_bytes().decode("utf-8").strip()
        except UnicodeError as error:
            return diagnostic(f"version output is not UTF-8: {error}", 4)
        if not display:
            return diagnostic("version output was empty", 4)
        if not receipt.matches_version(display, expect_version):
            return diagnostic("version pin mismatch", 3)
        record["binary"]["version"] = display

        launched = False
        status = "ok"
        for definition in receipt.analysis_commands(binary, root, record["options"]):
            try:
                recheck_binary(binary, expected_binary)
            except OSError as error:
                code = diagnostic(error, 4)
                status = "failed"
                break
            attempt = produce(stage, guard, limit, host, definition, artifacts)
            record["commands"].append(attempt)
            observed = attempt["guard_status"]
            # An invalid status after a wrapper actually ran cannot certify
            # no launch. Retain diagnostics as a failed attempted analysis.
            launched |= observed["launched"] if observed else attempt["exit"] is not None
            code = outcome(attempt)
            if code:
                status = "failed"
                diagnostic(f"{attempt['id']} {'refused by host quiet' if code == 75 else 'failed'}", code)
                break

        if not launched:
            return code
        status, code, final_digest, final_state = check_completion(
            corpus, root, binary, expected_binary, status, code)
        status, code = check_artifacts(stage, artifacts, status, code)
        entry = corpus.entry
        record["corpus"] = {"key": corpus.key, "commit": entry["commit"], "root": str(root),
                            "pin_check": corpus.pin_check, "manifest_sha256": entry["manifest_sha256"],
                            "tree_sha256": entry["tree_sha256"], "input_sha256": admitted.sha256,
                            "end_input_sha256": final_digest, "input_state_sha256": admitted.state_sha256,
                            "end_input_state_sha256": final_state}
        record.update(status=status, end=now())
        publish(out, stage, record)
        retain = True
        return code
    finally:
        if not retain:
            if root.is_symlink():
                root.unlink()
            elif root.is_dir():
                remove_tree(root)


def check_artifacts(stage, artifacts, status, code):
    """A later command may rewrite an earlier artifact; publish what is on disk."""
    drifted = []
    for name in artifacts:
        actual = receipt.artifact(stage / name)
        if actual != artifacts[name]:
            drifted.append(name)
            artifacts[name] = actual
    if drifted:
        diagnostic(f"artifact changed after capture: {', '.join(drifted)}", 4)
        return ("failed" if status == "ok" else status), code or 4
    return status, code


def check_completion(corpus, root, binary, expected_binary, status, code):
    final = None
    try:
        recheck_binary(binary, expected_binary)
    except OSError as error:
        diagnostic(error, 4)
        status, code = "failed", 75 if code == 75 else 4
    try:
        final = bind_tree(root)
        original = bind_tree(corpus.root)
        admitted = corpus.binding
        if any((tree.sha256, tree.state_sha256) != (admitted.sha256, admitted.state_sha256)
               for tree in (final, original)):
            status = "input_changed"
    except (AliasedInput, UnboundLink) as error:
        diagnostic(error, 4)
        status = "input_changed"
    except (OSError, PinError) as error:
        diagnostic(error, 4)
        status = "failed"
    if status != "ok" and code != 75:
        code = 4
    return status, code, final.sha256 if final else None, final.state_sha256 if final else None


def publish(out, stage, record):
    completion = stage / "receipt.json"
    completion.write_text(json.dumps(record, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    for name in record["artifacts"]:
        os.link(stage / name, out / name)
    os.link(completion, out / completion.name)
