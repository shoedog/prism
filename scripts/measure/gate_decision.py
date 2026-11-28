"""Resolve and freeze the controller's decision and launch inputs."""

from __future__ import annotations

from dataclasses import dataclass, field
from hashlib import sha256
from importlib.machinery import PathFinder
from pathlib import Path
import sys
import sysconfig

from . import corpora, gate_file, oracle, oracle_inputs, population, receipt, run
from .diff_stats import patterns
from .edges import canonical
from .gate_expect import validate_scope_selection

MANIFEST_FIELDS = ("kind", "url", "commit", "root", "history", "manifest_sha256", "tree_sha256", "cloud_asset")
GUARDED = frozenset({"run", "review-diff", "oracle ts-edges", "oracle ts-calls"})
BUNDLE_RESERVED = frozenset({
    "snapshot", "receipt.json", "step-receipt.json", receipt.VERSION, receipt.VERSION_STDERR,
    "dfg-stats.json", "dfg-stats.stderr", "call-stats.json", "call-stats.stderr", "edges.jsonl.gz",
    "edges.stderr", "edges.stderr.gz", "sites.jsonl.gz", "sites.stderr", "__pycache__",
    "measure", "memguard", "hostquiet", "measure_gate_classifier",
})


def digest(data):
    return sha256(data).hexdigest()


def executable(path):
    # Keep the locator for rechecking retargeted links. The launchers resolve it.
    return run.executable_hash(path.resolve(strict=True))


@dataclass
class Decision:
    path: Path
    record: dict
    prediction_path: Path
    prediction: bytes
    projection: dict
    manifest: Path
    entries: dict
    by_id: dict
    binary_paths: dict
    paths: dict = field(default_factory=dict)
    watch: dict = field(default_factory=dict)
    limits: list = field(default_factory=list)

    @property
    def steps(self):
        return self.record["steps"]

    @property
    def binaries(self):
        return self.projection["binaries"]

    @property
    def files(self):
        return self.projection["files"]

    @property
    def encoded(self):
        return canonical(self.projection)

    @property
    def sha256(self):
        return digest(self.encoded)

    def bind(self, role, path, reader=gate_file.controller_bytes):
        key = (path, reader)
        if key not in self.watch:
            self.watch[key] = reader(path)
        measured = self.watch[key]
        self.paths[role] = path
        self.files[role] = digest(measured) if isinstance(measured, bytes) else measured
        return measured

    def recheck(self):
        try:
            current = gate_file.controller_bytes(self.prediction_path)
        except (OSError, ValueError) as error:
            raise ValueError(f"prediction-missing: {error}") from error
        if current != self.prediction:
            raise ValueError("prediction-changed")
        for (path, reader), frozen in self.watch.items():
            try:
                if reader(path) == frozen:
                    continue
            except (OSError, ValueError) as error:
                raise ValueError(f"bound input unavailable or changed: {path}: {error}") from error
            message = "executable changed" if reader is executable else "bound input changed"
            raise ValueError(f"{message}: {path}")

    def step_files(self, step):
        ident = step["id"]
        return {**{key: value for key, value in self.files.items() if key.startswith(f"{ident}/")},
                **{f"{ident}/binary/{name}": self.binaries[name]["sha256"]
                   for name in gate_file.used_binaries(step)}}


def binary_bindings(root, specs, supplied, used):
    overrides = {}
    for item in supplied:
        name, separator, path = item.partition("=")
        gate_file.require(separator and name in specs and name not in overrides and oracle_inputs.name(path),
                          f"invalid or repeated binary override {item!r}")
        overrides[name] = Path(path).absolute()
    paths = {name: overrides.get(name, root / specs[name]["path"]) for name in sorted(used)}
    binaries = {name: dict(sha256=executable(path), expect_version=specs[name]["expect_version"])
                for name, path in paths.items()}
    return paths, binaries


def prediction_binding(root, locator):
    try:
        path = gate_file.controller_path(root, locator)
        raw = gate_file.controller_bytes(path)
    except FileNotFoundError as error:
        raise ValueError("prediction-missing") from error
    raw.decode("utf-8")
    return path, raw


def quiet_helper(guard):
    return guard.resolve(strict=True).parent.parent / "hostquiet" / "host_quiet.py"


def helper_digest(guard):
    return digest(gate_file.regular(quiet_helper(guard)))


def stdlib_module(name):
    locations = [sysconfig.get_path("stdlib"), sysconfig.get_config_var("DESTSHARED")]
    return (name in sys.builtin_module_names or name in getattr(sys, "stdlib_module_names", ())
            or PathFinder.find_spec(name, [directory for directory in locations if directory]) is not None)


def bind_bundle(decision, step, entry):
    root, ident = decision.path.parent, step["id"]
    seen = {(entry.stat().st_dev, entry.stat().st_ino)}
    for group, items in step.get("classifier_files", {}).items():
        logical_paths = []
        for logical, spec in items.items():
            target = Path(logical)
            for part in target.parts:
                stem = part.split(".")[0].casefold()
                gate_file.require(part.casefold() not in BUNDLE_RESERVED and stem not in BUNDLE_RESERVED
                                  and not stdlib_module(stem), f"reserved classifier file {logical!r}")
            if group == "helpers":
                gate_file.require(target.parts[0].casefold() != entry.name.casefold()
                                  and target.parts[0].split(".")[0].casefold() != entry.stem.casefold(),
                                  f"classifier helper shadows the entry: {logical}")
            parts = tuple(part.casefold() for part in target.parts)
            gate_file.require(not any(other[:len(parts)] == parts or parts[:len(other)] == other
                                      for other in logical_paths), f"colliding classifier file {logical!r}")
            logical_paths.append(parts)
            source = gate_file.controller_path(root, spec["path"])
            info = source.stat()
            key = (info.st_dev, info.st_ino)
            gate_file.require(key not in seen, f"duplicate classifier input: {source}")
            seen.add(key)
            role = f"{ident}/classifier_files/{group}/{logical}"
            decision.bind(role, source)
            gate_file.require(decision.files[role] == spec["sha256"], f"classifier digest mismatch: {role}")


def bind_oracle(decision, step):
    root, ident = decision.path.parent, step["id"]
    question = decision.paths.get(f"{ident}/question")
    typescript = oracle_inputs.checker_path(root / step["typescript"] if "typescript" in step else None)
    prepared = oracle_inputs.prepare_inputs(typescript, question)
    validate_scope_selection(step, prepared)
    for key, component in (("question", "question_sha256"), ("typescript", "checker_tree_sha256")):
        decision.files[f"{ident}/{key}"] = prepared["components"][component]
    decision.paths[f"{ident}/typescript"] = typescript
    decision.paths[f"{ident}/oracle_inputs"] = prepared
    decision.watch[(typescript, oracle_inputs.checker_files)] = prepared["checker"]

    def current(path):
        return oracle_inputs.prepare_inputs(path, question)["components"]

    decision.watch[(typescript, current)] = prepared["components"]


def bind_step(decision, step, manifest_digest):
    ident, kind = step["id"], step["do"]
    root = decision.path.parent
    if kind in ("corpora-list", "corpora-verify", "run"):
        decision.files[f"{ident}/manifest"] = manifest_digest
        keys = [step["corpus"]] if kind == "run" else step.get("keys", [])
        gate_file.require(all(key in decision.entries for key in keys), f"{ident}: unknown corpus key")
    if kind in GUARDED:
        default = oracle.DEFAULT_GUARD if kind.startswith("oracle ") else run.DEFAULT_GUARD
        guard = root / step["memguard"] if "memguard" in step else default
        decision.bind(f"{ident}/memguard", guard, executable)
        decision.bind(f"{ident}/quiet_helper", guard, helper_digest)
    for key in ("classifier", "allow", "population", "question"):
        if key in step:
            role = f"{ident}/{key}"
            decision.bind(role, gate_file.controller_path(root, step[key]))
    if kind.startswith("oracle "):
        bind_oracle(decision, step)
    if kind == "diff-edges":
        bind_bundle(decision, step, decision.paths[f"{ident}/classifier"])
    if "allow" in step:
        patterns(decision.paths[f"{ident}/allow"])
    if kind == "review-diff":
        registered = population.load(decision.paths[f"{ident}/population"])
        available = {row["ordinal"] for row in registered.diffs}
        selected = set(step.get("only", available))
        gate_file.require(selected <= available, f"{ident}: unknown population ordinal")


def freeze(args, path, raw, record):
    root = path.parent
    prediction_path, prediction = prediction_binding(root, record["prediction"])
    used = {name for step in record["steps"] for name in gate_file.used_binaries(step)}
    binary_paths, binaries = binary_bindings(root, record["binaries"], args.binary, used)
    manifest = args.manifest.absolute()
    # The distribution's default registry is a kit input. Explicit registries
    # are controller files and are subject to the gate directory boundary.
    if manifest != corpora.DEFAULT_MANIFEST.absolute():
        gate_file.require(manifest.is_relative_to(root), "manifest is outside the gate directory")
    manifest_raw = gate_file.controller_bytes(manifest)
    entries = corpora.load_manifest(manifest)
    manifest_digest = digest(canonical(dict(
        schema=corpora.SCHEMA,
        corpora={key: {field: entry[field] for field in MANIFEST_FIELDS} for key, entry in entries.items()},
    )))
    steps = record["steps"]
    projection = dict(schema=record["schema"], lane=record["lane"], prediction_sha256=digest(prediction),
                      binaries=binaries, steps=[{k: v for k, v in step.items() if k not in gate_file.LOCATORS}
                                                for step in steps], files={})
    decision = Decision(path, record, prediction_path, prediction, projection, manifest, entries,
                        {step["id"]: step for step in steps}, binary_paths)
    decision.watch = {(path, gate_file.controller_bytes): raw,
                      (manifest, gate_file.controller_bytes): manifest_raw}
    decision.watch.update({(binary, executable): binaries[name]["sha256"] for name, binary in binary_paths.items()})
    for step in steps:
        bind_step(decision, step, manifest_digest)
        question = decision.paths.get(f"{step['id']}/question")
        if question is not None:
            gate_file.require(not args.out.resolve().is_relative_to(question.parent),
                              "gate output must be outside the oracle question directory")
    decision.recheck()
    return decision


def check_pin(args, decision):
    pin = args.decision_pin
    if pin is not None and (not receipt.is_digest(pin) or pin != decision.sha256):
        raise ValueError("decision-pin-mismatch")
    if (not args.retrospective and pin is None
            and any(step["do"] not in ("run", "corpora-list", "corpora-verify") for step in decision.steps)):
        raise ValueError("decision-pin-missing")
