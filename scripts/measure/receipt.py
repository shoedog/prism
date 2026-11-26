"""Command wire roles and admission of completed S1 receipts."""

from __future__ import annotations

from datetime import datetime
import re
import stat

from .corpora import COMMIT, DEFAULT_MANIFEST, KEY, load_manifest
from .inputs import SHA256, json_object, jsonl_objects, sha256_file

SCHEMA = "prism.measure.receipt/1"
HOST_ENV = ("PATH", "HOME", "TMPDIR", "TEMP", "TMP", "SYSTEMROOT")
EDGE_BYTES = "PRISM_EDGES_BYTES"
TRACE_NAME = re.compile(r"PRISM_[A-Z0-9_]+")
COMMIT_TOKEN = re.compile(r"[0-9a-fA-F]{7,40}")
STEP_ID = re.compile(r"[A-Za-z0-9_-]+")
VERSION = "version.stdout"
VERSION_STDERR = "version.stderr"
GUARD_EXITS = {"host_quiet": 75, "rss_unavailable": 76, "rss_limit": 137, "guard_error": 1}


def trace_settings(values):
    result = {}
    for setting in values:
        name, separator, value = setting.partition("=")
        if (not separator or not TRACE_NAME.fullmatch(name) or name == EDGE_BYTES
                or name in result or "\0" in value):
            raise ValueError(f"invalid or repeated trace setting {setting!r}")
        result[name] = value
    return result


def valid_version_pin(pin):
    return (pin is None or (len(pin) >= 2 and len(pin.split()) == 1 and pin == pin.strip()
            and (not re.fullmatch(r"[0-9a-fA-F]+", pin) or COMMIT_TOKEN.fullmatch(pin))))


def matches_version(version, pin):
    if pin is None:
        return True
    parts = version.split()
    if len(parts) == 1:
        token = commit = parts[0]
    elif len(parts) >= 2 and parts[0] in ("prism", "slicing") and "\n" not in version:
        token = parts[1]
        commit = ""
        # Current slicing emits "slicing VERSION (COMMIT)". Also support
        # explicitly labelled commit tokens in historical version banners.
        if parts.count("commit") == 1 and parts[-2] == "commit":
            commit = parts[-1]
        elif len(parts) == 3 and parts[-1].startswith("(") and parts[-1].endswith(")"):
            commit = parts[-1][1:-1]
    else:
        return False
    return token == pin or bool(COMMIT_TOKEN.fullmatch(commit) and COMMIT_TOKEN.fullmatch(pin)
                                and commit.lower().startswith(pin.lower()))


def shell_exit(code):
    return 128 - code if code < 0 else code


def guard_status(data, wrapper_exit):
    if not data.endswith(b"\n") or data.count(b"\n") != 1:
        raise ValueError("guard must emit one status object and LF")
    status = json_object(data)
    check_guard_status(status, wrapper_exit)
    return status


def check_guard_status(status, wrapper_exit):
    if (not isinstance(status, dict) or set(status) != {"kind", "launched", "child_exit"}
            or type(status["launched"]) is not bool or type(wrapper_exit) is not int):
        raise ValueError("invalid guard status")
    kind, launched, child = status["kind"], status["launched"], status["child_exit"]
    if launched:
        if type(child) is not int or not 0 <= child <= 255:
            raise ValueError("invalid reaped child exit")
    elif child is not None:
        raise ValueError("unlaunched guard status has a child exit")
    if kind == "child" and launched and wrapper_exit == child:
        return
    if (isinstance(kind, str) and kind in GUARD_EXITS and wrapper_exit == GUARD_EXITS[kind]
            and not (kind == "host_quiet" and launched)
            and not (kind == "rss_limit" and not launched)):
        return
    raise ValueError("inconsistent guard and wrapper exits")


def analysis_commands(binary, root, options):
    common = [str(binary), "nav", "--no-cache"]
    repo = ["--repo", str(root)]
    edges_env = dict(options["trace_env"])
    if options["bytes"]:
        edges_env[EDGE_BYTES] = "1"
    commands = [
        ("dfg-stats", [*common, "dfg-stats", *repo], {}, "dfg-stats.json", "dfg-stats.stderr"),
        ("call-stats", [*common, "call-stats", *repo], {}, "call-stats.json", "call-stats.stderr"),
        ("edges", [*common, "dfg-stats", *repo, "--edges"], edges_env, "edges.jsonl.gz",
         "edges.stderr.gz" if options["trace_env"] else "edges.stderr"),
    ]
    if options["sites"]:
        commands.append(("sites", [*common, "call-stats", *repo, "--dump-sites"], {},
                         "sites.jsonl.gz", "sites.stderr"))
    return commands


def artifact(path):
    return {"sha256": sha256_file(path), "bytes": path.stat().st_size}


def is_digest(value):
    return isinstance(value, str) and SHA256.fullmatch(value) is not None


def gate_binding(gate_sha256, gate_step_id):
    if gate_sha256 is None and gate_step_id is None:
        return
    if (not is_digest(gate_sha256) or not isinstance(gate_step_id, str)
            or not STEP_ID.fullmatch(gate_step_id)):
        raise ValueError("invalid frozen gate digest or step ID")


def _check_times(record):
    def parse(value):
        if not isinstance(value, str) or "T" not in value or not value.endswith("Z"):
            raise ValueError("expected a UTC ISO 8601 timestamp")
        return datetime.fromisoformat(value[:-1] + "+00:00")

    if parse(record["end"]) < parse(record["start"]):
        raise ValueError("measurement ended before it started")


def _check_provenance(record):
    _check_times(record)
    if not COMMIT.fullmatch(record["kit_commit"]):
        raise ValueError("invalid kit commit")
    guard, host, prediction = (record[key] for key in ("guard", "host", "prediction"))
    env = host["env"]
    # Additive fields: validated when present.
    if "sha256" in guard and not is_digest(guard["sha256"]):
        raise ValueError("invalid guard digest")
    if "kit_dirty" in record and type(record["kit_dirty"]) is not bool:
        raise ValueError("invalid kit dirty flag")
    if (not isinstance(guard["path"], str) or not guard["path"]
            or type(guard["limit_gb"]) is not int or guard["limit_gb"] < 1
            or not isinstance(host["platform"], str) or not host["platform"]
            or (host["cpus"] is not None and (type(host["cpus"]) is not int or host["cpus"] < 1))
            or not isinstance(env, dict) or env.get("LC_ALL") != "C"
            or not env.keys() <= {*HOST_ENV, "LC_ALL"}
            or any(not isinstance(value, str) for value in env.values())):
        raise ValueError("invalid guard or host provenance")
    if prediction is None:
        if record["gate_sha256"] is not None:
            raise ValueError("gate receipt is missing its frozen prediction")
    elif (set(prediction) != {"path", "sha256"} or not isinstance(prediction["path"], str)
            or not prediction["path"] or not is_digest(prediction["sha256"])):
        raise ValueError("invalid prediction binding")


def _check_command(command, argv, env, stderr):
    _check_times(command)
    if any(command.get(key) != value for key, value in
           (("argv", argv), ("env", env), ("stderr", stderr), ("exit", 0))):
        raise ValueError("measurement command does not match its required role")
    status = command["guard_status"]
    check_guard_status(status, command["exit"])
    if status["kind"] != "child":
        raise ValueError("measurement command did not complete")


def _check_output(command, actual, sha_key, size_key):
    if (command.get(sha_key) != actual["sha256"] or type(command.get(size_key)) is not int
            or command[size_key] != actual["bytes"]):
        raise ValueError("command output digest or size mismatch")


def admit(directory, *, selected=(), entries=None):
    """Read one successful receipt and verify every artifact, plus selected data.

    Consumer-specific stream schemas and cross-arm decisions are applied by the
    consumer after this check. Selection is by analysis command ID. A
    manifest-bound receipt is checked against the effective corpus registry.
    """
    try:
        return _admit(directory, selected, entries)
    except (OSError, KeyError, TypeError, AttributeError) as error:
        raise ValueError(f"invalid measurement receipt: {error}") from error


def _admit(directory, selected, entries):
    path = directory / "receipt.json"
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError("receipt must be a regular file")
    receipt = json_object(path.read_bytes())
    if receipt.get("schema") != SCHEMA or receipt.get("status") != "ok":
        raise ValueError("expected a successful supported measurement receipt")

    gate_binding(receipt["gate_sha256"], receipt["gate_step_id"])
    _check_provenance(receipt)
    binary, corpus, options = (receipt[key] for key in ("binary", "corpus", "options"))
    if (not is_digest(binary["sha256"]) or not isinstance(binary["path"], str)
            or not isinstance(corpus["root"], str) or not KEY.fullmatch(corpus["key"])
            or not is_digest(corpus["input_sha256"])
            or not is_digest(corpus["input_state_sha256"])
            or corpus["end_input_sha256"] != corpus["input_sha256"]
            or corpus["end_input_state_sha256"] != corpus["input_state_sha256"]
            or corpus["pin_check"] not in ("manifest", "git-clean")
            or (corpus["pin_check"] == "git-clean" or corpus["commit"] is not None)
            and not COMMIT.fullmatch(corpus["commit"] or "")
            or corpus["manifest_sha256"] is not None and not is_digest(corpus["manifest_sha256"])
            or (corpus["pin_check"] == "manifest" or corpus["tree_sha256"] is not None)
            and corpus["tree_sha256"] != corpus["input_sha256"]):
        raise ValueError("invalid binary or corpus binding")
    if corpus["pin_check"] == "manifest" and entries is None:
        entries = load_manifest(DEFAULT_MANIFEST)
    if entries is not None:
        entry = entries[corpus["key"]]
        if (corpus["pin_check"] == "git-clean" and entry["kind"] != "git"
                or any(corpus[field] != entry[field]
                       for field in ("commit", "tree_sha256", "manifest_sha256"))):
            raise ValueError("receipt differs from its corpus registration")
    if (set(options) != {"bytes", "sites", "cache", "trace_env"}
            or type(options["bytes"]) is not bool or type(options["sites"]) is not bool
            or options["cache"] != "no-cache" or not isinstance(options["trace_env"], dict)
            or any(not isinstance(value, str) for value in options["trace_env"].values())):
        raise ValueError("invalid measurement options")
    trace_settings(f"{name}={value}" for name, value in options["trace_env"].items())

    expected = analysis_commands(binary["path"], corpus["root"], options)
    artifacts = receipt["artifacts"]
    names = {VERSION, VERSION_STDERR}
    names.update(path for _, _, _, stdout, stderr in expected for path in (stdout, stderr))
    if not isinstance(artifacts, dict) or artifacts.keys() != names:
        raise ValueError("measurement artifacts do not match required output set")
    for name in sorted(names):
        path = directory / name
        record = artifacts[name]
        if (not stat.S_ISREG(path.lstat().st_mode) or not isinstance(record, dict)
                or type(record.get("bytes")) is not int or artifact(path) != record):
            raise ValueError(f"unbound measurement artifact: {name}")

    version = receipt["version_command"]
    _check_command(version, [binary["path"], "--version"], {}, VERSION_STDERR)
    _check_output(version, artifacts[VERSION], "stdout_sha256", "stdout_bytes")
    if binary["version"] != (directory / VERSION).read_bytes().decode("utf-8").strip():
        raise ValueError("version output differs from the recorded version")
    commands = receipt["commands"]
    if not isinstance(commands, list) or len(commands) != len(expected):
        raise ValueError("incomplete analysis commands")
    selected = set(selected)
    if selected - {entry[0] for entry in expected}:
        raise ValueError("selected measurement output was not requested")
    for command, (name, argv, env, stdout, stderr) in zip(commands, expected):
        _check_command(command, argv, env, stderr)
        if command.get("id") != name or command.get("output") != stdout:
            raise ValueError("analysis command has the wrong output role")
        _check_output(command, artifacts[stdout], "output_sha256", "bytes")
        if name in selected:
            path = directory / stdout
            if stdout.endswith(".json"):
                json_object(path.read_bytes())
            else:
                for _ in jsonl_objects(path):
                    pass
    return receipt
