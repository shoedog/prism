"""Publish and replay the S4 accounting and positional arm receipt."""

import os
from pathlib import Path
import random
import re

from . import population as populations
from . import receipt
from .corpora import text
from .edges import integer
from .review_findings import compare, decode, METRICS, review_json, review_object, structure_object

SCHEMA = "prism.measure.review-receipt/1"
TOTALS_SCHEMA = "prism.measure.review/1"
ROLES = ("base", "head")
IDENTITY = ("ordinal", "repo", "commit")
SAMPLE_FIELDS = (*IDENTITY, *METRICS)
COMPARISON_FIELDS = (*METRICS, "rule_transitions", "anchors")
ARM_FIELDS = {"status", "exit", "guard_status", "output", "output_sha256", "stderr", "stderr_sha256",
              "input_before_sha256", "input_after_sha256"}
DATA_FILES = ("per-diff.jsonl", "totals.json", "sample.jsonl")
RECEIPT_FILE = "review-receipt.json"
STATUS_MISSING = "review-guard-status-missing"
RESERVED = {"--", "--repo", "--diff", "--format", "--cache", "--cache-dir", "--no-cache"}
SHORT_RESERVED = re.compile(r"^-[rdf]")


def check_args(args):
    if not isinstance(args, list) or not all(isinstance(a, str) and "\0" not in a for a in args):
        raise ValueError("review arguments must be strings without NUL")
    for arg in args:
        if arg.partition("=")[0] in RESERVED or SHORT_RESERVED.match(arg):
            raise ValueError(f"review argument overrides a reserved option: {arg!r}")


def argv_for(arm, scratch, spec):
    return [arm["binary"]["path"], "--repo", scratch, "--diff", spec, *arm["shared_args"],
            *arm["arm_args"], "--format", "json", "--no-cache"]


def _summarize(registered, arms, rows, size, seed):
    ok = [row for row in rows if row["status"] == "ok"]
    exclusions = {status + "_ordinals": [r["ordinal"] for r in rows if r["status"] == status]
                  for status in ("failed", "refused", "excluded")}
    selected = len(rows) - len(exclusions["excluded_ordinals"])
    sampled = random.Random(seed).sample(ok, min(size, len(ok)))
    sampled.sort(key=lambda r: r["ordinal"])
    totals = {
        "schema": TOTALS_SCHEMA, "population_sha256": registered.sha256,
        "scope": "population" if selected == len(rows) else "subset",
        "population": len(rows), "selected": selected, "ok": len(ok), **exclusions,
        **{key: sum(row[key] for row in ok) for key in METRICS},
        "sample_size": len(sampled), "seed": seed, "arms": arms,
    }
    return totals, [{key: r[key] for key in SAMPLE_FIELDS} for r in sampled]


def _jsonl(rows):
    return b"".join(review_json(row) + b"\n" for row in rows)


def publish(out, stage, registered, arms, rows, size, seed, artifacts, gate_sha256, gate_step_id):
    totals, sample = _summarize(registered, arms, rows, size, seed)
    for name, data in zip(DATA_FILES, (_jsonl(rows), review_json(totals) + b"\n", _jsonl(sample))):
        (stage / name).write_bytes(data)
        artifacts[name] = receipt.artifact(stage / name)
    record = {
        "schema": SCHEMA, "population_sha256": registered.sha256, "arms": arms,
        "gate_sha256": gate_sha256, "gate_step_id": gate_step_id, "artifacts": artifacts,
    }
    (stage / RECEIPT_FILE).write_bytes(review_json(record) + b"\n")
    for name, artifact in artifacts.items():
        if receipt.artifact(stage / name) != artifact:
            raise ValueError(f"review artifact changed before publication: {name}")
    for name in (*artifacts, RECEIPT_FILE):
        target = out / name
        target.parent.mkdir(parents=True, exist_ok=True)
        os.link(stage / name, target)


def _require(condition, reason):
    if not condition:
        raise ValueError(reason)


def _closed(obj, fields, reason):
    _require(isinstance(obj, dict) and obj.keys() == fields, reason)


def _read(directory, artifacts, name):
    _require(name in artifacts, f"missing review artifact: {name}")
    path = directory / name
    _require(not any(parent.is_symlink() for parent in path.parents if directory in parent.parents),
             f"linked review artifact directory: {name}")
    return receipt.verified_bytes(path, artifacts[name])


def _rows(raw, parse=structure_object):
    return [parse(line) for line in raw.split(b"\n") if line]


def _commands(arms):
    _closed(arms, set(ROLES), "invalid positional review arms")
    commands = {}
    for role in ROLES:
        arm = arms[role]
        _closed(arm, {"name", "binary", "shared_args", "arm_args", "commands"}, "invalid review arm")
        _require(text(arm["name"]), "invalid review arm name")
        for args in (arm["shared_args"], arm["arm_args"]):
            check_args(args)
        binary = arm["binary"]
        _closed(binary, {"path", "sha256", "version"}, "invalid review binary")
        _require(isinstance(binary["path"], str) and Path(binary["path"]).is_absolute()
                 and receipt.is_digest(binary["sha256"]) and (binary["version"] is None
                 or isinstance(binary["version"], str) and bool(binary["version"].strip())),
                 "invalid review binary identity")
        _require(isinstance(arm["commands"], list), "invalid review commands")
        by_ordinal = {}
        previous = 0
        for command in arm["commands"]:
            _closed(command, {"ordinal", "argv"}, "invalid review command")
            ordinal, argv = command["ordinal"], command["argv"]
            _require(integer(ordinal, previous + 1) and isinstance(argv, list) and len(argv) >= 7
                     and all(isinstance(arg, str) and "\0" not in arg for arg in argv),
                     "invalid review command ordinal or argv")
            root = Path(argv[2])
            _require(root.is_absolute() and root.name == "repo"
                     and root.parent.name.startswith(f"input-{ordinal}-")
                     and root.parent.parent.name.startswith(".stage-")
                     and argv == argv_for(arm, argv[2], str(root.parent / "spec.json"))
                     and binary["version"] is not None, "review argv disagrees with its positional arm")
            by_ordinal[ordinal] = argv
            previous = ordinal
        commands[role] = by_ordinal
    _require(arms["base"]["name"] != arms["head"]["name"]
             and arms["base"]["shared_args"] == arms["head"]["shared_args"],
             "invalid review arm names or shared arguments")
    return commands


def _streams(directory, artifacts, arm, role, ordinal, argv, expected):
    _closed(arm, ARM_FIELDS, "invalid per-ordinal review arm")
    guard = arm["guard_status"]
    if guard is not None:
        _require(isinstance(guard, dict), "invalid review guard status")
        wrapper = receipt.GUARD_EXITS.get(str(guard.get("kind")), guard.get("child_exit"))
        receipt.check_guard_status(guard, wrapper)
    launched = guard is not None and guard["launched"]
    # No guard status but kept streams: an indeterminate launch (review-guard-status-missing).
    indeterminate = guard is None and arm["output"] is not None
    _require((launched or indeterminate) == (argv is not None),
             "review command coverage differs from launch status")
    _require(arm["status"] in ("ok", "failed", "refused", "unlaunched"), "invalid review arm status")
    _require(guard is None or guard["kind"] != "host_quiet" or arm["status"] == "refused",
             "host-quiet review arm is not refused")
    before, after = arm["input_before_sha256"], arm["input_after_sha256"]
    _require(all(value is None or receipt.is_digest(value) for value in (before, after)),
             "invalid observed review input hash")
    if not launched and not indeterminate:
        _require(arm["status"] != "ok" and all(arm[key] is None for key in (
            "exit", "output", "output_sha256", "stderr", "stderr_sha256", "input_after_sha256",
        )), "unlaunched review arm has launch evidence")
        _require(arm["status"] != "unlaunched" or (before is None and guard is None),
                 "unattempted review arm has input or guard evidence")
        return None
    if indeterminate:
        _require(arm["status"] == "failed" and arm["exit"] is None and after is None
                 and receipt.is_digest(before), "invalid review child evidence")
    else:
        _require(arm["status"] in ("ok", "failed") and arm["exit"] == guard["child_exit"]
                 and type(arm["exit"]) is int and receipt.is_digest(before),
                 "invalid review child evidence")
    if arm["status"] == "ok":
        _require(guard["kind"] == "child" and arm["exit"] == 0 and before == after,
                 "successful review arm lacks matching input and exit evidence")
    data = {}
    for field, suffix in (("output", "stdout"), ("stderr", "stderr")):
        name = f"arms/{ordinal}/{role}.{suffix}"
        _require(arm[field] == name, f"invalid positional review stream: {name}")
        data[field] = _read(directory, artifacts, name)
        _require(arm[field + "_sha256"] == artifacts[name]["sha256"], f"review stream hash mismatch: {name}")
        expected.add(name)
    return data["output"]


def _check_row(directory, artifacts, row, written, pin, arms, commands, expected):
    _closed(row, {*IDENTITY, *COMPARISON_FIELDS, "status", "reason", "arms"}, "invalid review row")
    _require(integer(row["ordinal"], 1) and all(row[key] == pin[key] for key in IDENTITY),
             "review row disagrees with the population")
    _closed(row["arms"], {arm["name"] for arm in arms.values()}, "invalid review row arm names")
    argv = {role: commands[role].pop(pin["ordinal"], None) for role in ROLES}
    if argv["base"] is not None and argv["head"] is not None:
        _require(argv["base"][2] == argv["head"][2], "review arms disagree about the scratch checkout")
    observed = [row["arms"][arms[role]["name"]] for role in ROLES]
    streams = [_streams(directory, artifacts, arm, role, pin["ordinal"], argv[role], expected)
               for role, arm in zip(ROLES, observed)]
    status, reason = row["status"], row["reason"]
    if any(arm["guard_status"] is None and arm["output"] is not None for arm in observed):
        _require(reason == STATUS_MISSING, "indeterminate review launch has another reason")
    if status == "ok":
        _require(reason is None and all(arm["status"] == "ok" for arm in observed)
                 and observed[0]["input_before_sha256"] == observed[1]["input_before_sha256"],
                 "successful review row has incomplete arm evidence")
        actual = compare(*(decode(stream) for stream in streams))
    else:
        _require(status in ("failed", "refused", "excluded") and isinstance(reason, str) and bool(reason),
                 "invalid non-successful review row disposition")
        _require(observed[0]["status"] == "ok" or observed[1]["status"] == "unlaunched",
                 "head arm attempted after an unsuccessful base")
        last = next((arm["status"] for arm in reversed(observed) if arm["status"] != "unlaunched"), None)
        _require(status == last if last is not None else status in ("refused", "excluded"),
                 "review row disposition disagrees with the arm evidence")
        actual = {**dict.fromkeys(METRICS), "rule_transitions": [], "anchors": []}
    # Payload numbers keep their spelling (`-0` is not `0`), so compare the spelled row.
    _require(review_json(actual) == review_json({key: written[key] for key in COMPARISON_FIELDS}),
             "review row comparison does not match its evidence")


def admit(directory, *, population):
    """Check a finalized review against its population, streams and role-bound argv."""
    directory = Path(directory).resolve()
    registered = populations.load(Path(population))
    completion = directory / RECEIPT_FILE
    _require(completion.is_file() and not completion.is_symlink(), "missing or linked review receipt")
    record = structure_object(completion.read_bytes())
    _closed(record, {"schema", "population_sha256", "arms", "gate_sha256", "gate_step_id", "artifacts"},
            "invalid review receipt")
    _require(record["schema"] == SCHEMA and record["population_sha256"] == registered.sha256,
             "review receipt disagrees with the population")
    receipt.gate_binding(record["gate_sha256"], record["gate_step_id"])
    arms, artifacts = record["arms"], record["artifacts"]
    commands = _commands(arms)
    _require(isinstance(artifacts, dict), "invalid review artifact registry")
    expected = set(DATA_FILES)
    raw_rows = _read(directory, artifacts, DATA_FILES[0])
    rows, spelled = _rows(raw_rows), _rows(raw_rows, review_object)
    _require(len(rows) == len(registered.diffs), "incomplete review population accounting")
    for row, written, pin in zip(rows, spelled, registered.diffs):
        _check_row(directory, artifacts, row, written, pin, arms, commands, expected)
    _require(artifacts.keys() == expected and not any(commands.values()), "unexpected review evidence")
    totals = structure_object(_read(directory, artifacts, DATA_FILES[1]))
    sample = _rows(_read(directory, artifacts, DATA_FILES[2]))
    seed = totals.get("seed")
    _require(type(seed) is int, "invalid review sample seed")
    expected_totals, expected_sample = _summarize(registered, arms, rows, len(sample), seed)
    _require(review_json(totals) == review_json(expected_totals)
             and review_json(sample) == review_json(expected_sample), "invalid review totals or sample")
    return record
