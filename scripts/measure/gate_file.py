"""Parse the gate file and preflight its complete ordered step graph."""

from __future__ import annotations

import shlex
import stat
from pathlib import Path

from . import corpora, edges, inputs, receipt, review_diff
from .gate_credit import NEUTRAL_REVIEW_ARGS, without_review_options
from .gate_expect import validate_claims, validate_expect
from .oracle_inputs import name, safe_path
from .oracle_selection import TARGET_CONFIDENCES
from .review_receipt import check_args

SCHEMA = "prism.measure.gate/1"
STEPS = {
    "corpora-list": ("", ""),
    "corpora-verify": ("", "keys"),
    "run": ("binary corpus", "trace_env no_bytes sites memguard limit_gb reuse"),
    "diff-edges": ("base head classifier", "key line_reason list identity_control classifier_files"),
    "diff-stats": ("base head", "allow which identity_control"),
    "oracle ts-edges": ("run", "all changes typescript question relation side kind class quality absence"),
    "oracle ts-calls": ("", "census base head typescript question relation target_confidence "
                           "target_kind aggregate_only identity_control quality absence"),
    "review-diff": ("population clones arms", "review_args only sample seed identity_control"),
}
BOOL_FIELDS = frozenset({"allow_empty", "no_bytes", "sites", "list", "identity_control", "all", "aggregate_only"})
ARRAY_FIELDS = frozenset({"keys", "trace_env", "kind", "class", "target_confidence", "target_kind"})
SCALAR_FIELDS = frozenset({
    "binary", "corpus", "memguard", "base", "head", "classifier", "key", "line_reason", "allow",
    "which", "run", "changes", "typescript", "question", "relation", "side", "census", "population", "clones",
})
LOCATORS = frozenset({"clones", "memguard", "typescript", "reuse"})


def require(condition, message):
    if not condition:
        raise ValueError(message)


def regular(path):
    require(stat.S_ISREG(path.lstat().st_mode), f"expected a regular file: {path}")
    return path.read_bytes()


def controller_bytes(path):
    require(path == path.resolve(strict=True) and path.lstat().st_nlink == 1,
            f"linked controller input: {path}")
    return regular(path)


def controller_path(root, locator):
    require(isinstance(locator, str) and safe_path(locator),
            f"expected a relative controller input: {locator!r}")
    path = root / locator
    require(path == path.resolve(strict=True), f"linked controller input: {locator}")
    return path


def references(step):
    roles = {}
    for field in ("base", "head", "census", "run", "changes"):
        if field in step:
            roles[step[field]] = "diff-edges" if field == "changes" else "run"
    return roles


def used_binaries(step):
    if step["do"] == "run":
        return (step["binary"],)
    if step["do"] == "review-diff":
        return tuple(arm["binary"] for arm in step["arms"])
    return ()


def review_arguments(step):
    cache_options = {name: value for name, value in NEUTRAL_REVIEW_ARGS.items()
                     if name != "--review-min-severity"}

    def effective(args):
        without_review_options(args, NEUTRAL_REVIEW_ARGS)
        args = without_review_options(args, cache_options)
        check_args(args)
        return args

    return (effective(shlex.split(step.get("review_args", review_diff.DEFAULT_SHARED))),
            [effective(arm["args"]) for arm in step["arms"]])


def validate_review(step):
    arms = step["arms"]
    require(type(arms) is list and len(arms) == 2, "review requires two ordered arms")
    for arm in arms:
        receipt.closed(arm, ({"name", "binary", "args"}, set()), "review arm")
        require(corpora.text(arm["name"]) and name(arm["binary"]), "invalid review arm name or binary")
        require(type(arm["args"]) is list and all(isinstance(arg, str) and "\0" not in arg
                                                 for arg in arm["args"]), "invalid review arm args")
    require(arms[0]["name"] != arms[1]["name"], "duplicate review arm name")
    require(isinstance(step.get("review_args", ""), str), "review_args must be a string")
    require("\0" not in step.get("review_args", ""), "review_args contains NUL")
    if "only" in step:
        only = step["only"]
        require(type(only) is list and bool(only) and all(type(n) is int and n > 0 for n in only)
                and len(only) == len(set(only)), "review only requires distinct positive ordinals")
    require(type(step.get("seed", 0)) is int, "review seed must be an integer")
    require(type(step.get("sample", 0)) is int and step.get("sample", 0) >= 0,
            "review sample must be a nonnegative integer")
    review_arguments(step)


def validate_classifier_files(step):
    if "classifier_files" not in step:
        return
    bundle = step["classifier_files"]
    receipt.closed(bundle, ({"base", "head", "helpers"}, set()), "classifier_files")
    for entries in bundle.values():
        require(isinstance(entries, dict), "classifier_files groups must be objects")
        for logical, entry in entries.items():
            receipt.closed(entry, ({"path", "sha256"}, set()), "classifier_files member")
            require(safe_path(logical) and safe_path(entry["path"]) and receipt.is_digest(entry["sha256"]),
                    f"invalid classifier_files member {logical!r}")


def validate_oracle(step, previous):
    if step["do"] == "oracle ts-edges":
        require(("all" in step and step["all"] and "changes" not in step)
                or ("all" not in step and "changes" in step), "oracle ts-edges requires all:true or changes")
        if "all" in step:
            require(not {"side", "kind", "class"} & step.keys(), "side/kind/class require changes")
        else:
            comparison = previous[step["changes"]]
            pair = {arm: comparison[arm] for arm in ("base", "head")}
            if "side" in step:
                require(step["run"] == pair[step["side"]], "oracle run differs from selected change arm")
            else:
                require(list(pair.values()).count(step["run"]) == 1, "oracle run needs a unique change arm")
        require(all(kind in edges.KINDS for kind in step.get("kind", [])), "invalid change kind")
    else:
        census = "census" in step
        require((census and not {"base", "head", "identity_control"} & step.keys())
                or (not census and {"base", "head"} <= step.keys()), "use census or both call arms")
        confidence = step.get("target_confidence", ["exact"])
        require(bool(confidence) and all(value in TARGET_CONFIDENCES for value in confidence),
                "invalid target confidence")
        require("name_only" not in confidence or {"question", "relation"} <= step.keys(),
                "name_only requires question and relation")
        require(all(previous[ref].get("sites") for ref in references(step)), "call oracle needs sites:true")
    validate_claims(step)


def validate_step(step, previous, binaries, seen):
    require(isinstance(step, dict) and isinstance(step.get("do"), str) and step["do"] in STEPS, "unknown-step")
    kind = step["do"]
    required, optional = (set(fields.split()) for fields in STEPS[kind])
    receipt.closed(step, (required | {"id", "do"}, optional | {"expect", "allow_empty"}), kind)
    ident = step["id"]
    require(isinstance(ident, str) and receipt.STEP_ID.fullmatch(ident) and ident.casefold() not in seen,
            f"invalid or repeated step ID {ident!r}")
    for field in BOOL_FIELDS & step.keys():
        require(type(step[field]) is bool, f"{field} must be boolean")
    for field in SCALAR_FIELDS & step.keys():
        require(name(step[field]), f"invalid {field}")
    for field in ARRAY_FIELDS & step.keys():
        require(type(step[field]) is list and all(map(name, step[field])), f"{field} must be a string array")
    for field, allowed in (("key", ("bytes", "line")), ("which", ("dfg", "call", "both")),
                           ("side", ("base", "head"))):
        require(field not in step or step[field] in allowed, f"invalid {field}")
    for ref, expected in references(step).items():
        require(ref in previous and previous[ref]["do"] == expected,
                f"{ident}: reference {ref!r} must name an earlier {expected} step")
    # A duplicate reference must still meet the type of every reference field.
    for field in ("run", "base", "head", "census"):
        if field in step:
            require(previous[step[field]]["do"] == "run", f"{field} must reference a run")
    if kind == "run":
        receipt.trace_settings(step.get("trace_env", []))
        require(type(step.get("limit_gb", 8)) is int and step.get("limit_gb", 8) > 0,
                "limit_gb must be a positive integer")
        if "reuse" in step:
            reuse = step["reuse"]
            receipt.closed(reuse, ({"path"}, {"receipt_sha256"}), "reuse")
            require(name(reuse["path"]) and ("receipt_sha256" not in reuse
                    or receipt.is_digest(reuse["receipt_sha256"])), "invalid reuse locator or digest")
    if kind == "diff-edges":
        require("line_reason" not in step or step.get("key") == "line", "line_reason requires line mode")
        validate_classifier_files(step)
    if kind.startswith("oracle "):
        validate_oracle(step, previous)
    if kind == "review-diff":
        validate_review(step)
    require(all(binary in binaries for binary in used_binaries(step)), f"{ident}: unknown binary")
    validate_expect(step)
    seen.add(ident.casefold())


def load(path):
    path = path.absolute()
    raw = controller_bytes(path)
    record = inputs.json_object(raw)
    edges.canonical(record)  # Refuse strings that cannot be serialized as UTF-8.
    receipt.closed(record, ({"schema", "lane", "prediction", "binaries", "steps"}, set()), "gate file")
    require(record["schema"] == SCHEMA and name(record["lane"]) and name(record["prediction"]),
            "invalid gate schema, lane or prediction")
    require(isinstance(record["steps"], list) and bool(record["steps"]), "steps must be a nonempty array")
    return path, raw, record


def validate_graph(record):
    require(isinstance(record["binaries"], dict), "binaries must be an object")
    for key, binary in record["binaries"].items():
        receipt.closed(binary, ({"path", "expect_version"}, set()), "binary")
        require(name(key) and name(binary["path"]) and name(binary["expect_version"])
                and receipt.valid_version_pin(binary["expect_version"]), f"invalid binary declaration for {key!r}")
    previous, seen = {}, set()
    for step in record["steps"]:
        validate_step(step, previous, record["binaries"], seen)
        previous[step["id"]] = step
