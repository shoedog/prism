"""Load a lane classifier and expose the standalone comparison views."""

from __future__ import annotations

from contextlib import contextmanager
from copy import deepcopy
from dataclasses import dataclass
import hashlib
import inspect
from pathlib import Path
import sys
from types import MappingProxyType, ModuleType

from .inputs import PinError, bind_tree

LINE_BREAKS = frozenset("\t\n\r\v\f\x1c\x1d\x1e\x85\u2028\u2029")


def line_text(value):
    if not isinstance(value, str) or not value or any(char in LINE_BREAKS for char in value):
        return False
    try:
        value.encode("utf-8")
    except UnicodeError:
        return False
    return True


def describe(error):
    return f"{type(error).__name__}: {error}".encode("utf-8", "backslashreplace").decode()


@dataclass(frozen=True)
class Classifier:
    classes: tuple
    classify: object
    parse_trace: object
    validate: object
    strays: str
    legacy_order: dict
    sha256: str


@contextmanager
def load(path, *, package=None):
    module = ModuleType("_measure_classifier")
    name = f"{module.__name__}_{id(module):x}"
    if package is not None:
        name = f"{package}.{name}"
        module.__package__ = package
    module.__name__ = name
    sys.modules[name] = module
    try:
        try:
            if not path.is_file():
                raise ValueError("classifier must be a regular Python file")
            code = path.read_bytes()
            module.__file__ = str(path.absolute())
            exec(compile(code, str(path), "exec"), module.__dict__)
            classes = getattr(module, "CLASSES", None)
            if (not isinstance(classes, list) or not all(map(line_text, classes))
                    or len(classes) != len(set(classes))):
                raise ValueError("CLASSES must list distinct, nonempty single-line names")

            def callback(name, arity, required=False):
                if not required and not hasattr(module, name):
                    return None
                function = getattr(module, name, None)
                if not callable(function):
                    raise ValueError(f"{name} must be callable")
                inspect.signature(function).bind(*([None] * arity))
                return function

            strays = getattr(module, "STRAYS", "fail")
            if strays not in ("fail", "allow"):
                raise ValueError("STRAYS must be fail or allow")
            order = getattr(module, "LEGACY_MECHANISM_ORDER", {})
            if not isinstance(order, dict) or any(
                    not line_text(key) or not isinstance(names, list)
                    or not all(map(line_text, names)) or len(names) != len(set(names))
                    for key, names in order.items()):
                raise ValueError("LEGACY_MECHANISM_ORDER must map keys to distinct tag lists")
            lane = Classifier(tuple(sorted(classes)), callback("classify", 2, True),
                              callback("parse_trace", 2), callback("validate", 1), strays,
                              deepcopy(order), hashlib.sha256(code).hexdigest())
        except (Exception, SystemExit) as error:
            raise ValueError(f"classifier interface: {describe(error)}") from error

        yield lane
    finally:
        sys.modules.pop(name, None)


class SourceBinding(ValueError):
    """A requested source view did not match its run's receipt."""


class Frozen:
    """Slotted base: no attribute can be assigned, replaced or deleted after setup."""

    __slots__ = ()

    def __setattr__(self, name, value):
        raise AttributeError(f"{type(self).__name__} is read-only: cannot assign {name!r}")

    def __delattr__(self, name):
        raise AttributeError(f"{type(self).__name__} is read-only: cannot delete {name!r}")


class Context(Frozen):
    __slots__ = ("_runs", "_requested", "_verified", "_source_error", "_store", "_key_mode",
                 "_stats_deltas", "_counter_deltas", "_call_counter_deltas", "run_views")

    def __init__(self, runs, mode, store, tables, run_views=None):
        def put(name, value):
            object.__setattr__(self, name, value)

        put("_runs", runs)
        put("run_views", run_views)
        put("_requested", set())
        put("_verified", set())
        put("_source_error", None)
        put("_store", store)
        put("_key_mode", mode)
        # Hooks get read-only views: a classifier cannot consume, rewrite or replace a
        # delta that a later hook, the validator included, will read.
        put("_stats_deltas", tuple(MappingProxyType(dict(row)) for table in tables.values()
                                   for row in table["changed"]))
        put("_counter_deltas", MappingProxyType(
            {row["key"]: row["delta"] for row in tables["dfg"]["changed"]}))
        put("_call_counter_deltas", MappingProxyType(
            {row["key"]: row["delta"] for row in tables["call"]["changed"]}))

    @property
    def key_mode(self):
        return self._key_mode

    @property
    def stats_deltas(self):
        return self._stats_deltas

    @property
    def counter_deltas(self):
        return self._counter_deltas

    @property
    def call_counter_deltas(self):
        return self._call_counter_deltas

    def _verify(self, index):
        self._requested.add(index)
        run = self._runs[index]
        corpus = run.receipt["corpus"]
        path = Path(corpus["root"])
        try:
            if not path.is_absolute():
                raise ValueError("expected an absolute snapshot path")
            observed = bind_tree(path)
            if (observed.sha256, observed.state_sha256) != (
                    corpus["input_sha256"], corpus["input_state_sha256"]):
                raise ValueError("snapshot differs from the receipt")
        except (OSError, PinError, ValueError) as error:
            arm = "base" if index == 0 else "head"
            failure = SourceBinding(f"source binding: {arm}: {error}")
            object.__setattr__(self, "_source_error", failure)
            raise failure from error
        self._verified.add(index)
        return path

    def _source(self, index):
        if index in self._verified:
            return Path(self._runs[index].receipt["corpus"]["root"])
        return self._verify(index)

    def recheck_sources(self):
        if self._source_error is not None:
            raise self._source_error
        for index in sorted(self._requested):
            self._verify(index)

    @property
    def corpus_root(self):
        return self._source(1)

    @property
    def base_run(self):
        self._source(0)
        return self.run_views[0] if self.run_views is not None else self._runs[0].directory

    @property
    def head_run(self):
        self._source(1)
        return self.run_views[1] if self.run_views is not None else self._runs[1].directory

    @property
    def base_receipt(self):
        self._source(0)
        return deepcopy(self._runs[0].receipt)

    @property
    def head_receipt(self):
        self._source(1)
        return deepcopy(self._runs[1].receipt)

    @property
    def unchanged_keys(self):
        return self._store.unchanged

    @property
    def all_records(self):
        return self._store.records

    def records(self, change):
        return self._store.attached(change["id"])


class Bound(Frozen):
    __slots__ = ("_context", "_store")

    def __init__(self, context):
        object.__setattr__(self, "_context", context)
        object.__setattr__(self, "_store", context._store)

    def __getattr__(self, name):
        return getattr(self._context, name)

    @property
    def records(self):
        return self._store.records

    @property
    def changes(self):
        return self._store.changes

    @property
    def joins(self):
        return self._store.joins

    @property
    def unjoined(self):
        return self._store.unjoined

    def nodes(self, arm, file, start_byte, end_byte, access, *, owner=None):
        return self._store.nodes(arm, file, start_byte, end_byte, access, owner=owner)


def classify(classifier, change, context):
    try:
        result = classifier.classify(deepcopy(change), context)
        if result is None or isinstance(result, str) and result in classifier.classes:
            return result, None
        return None, f"undeclared classifier result {result!r}"
    except SourceBinding:
        raise
    except (Exception, SystemExit) as error:
        return None, describe(error)


def validate(classifier, context):
    if classifier.validate is None:
        return []
    try:
        result = classifier.validate(context)
        if not isinstance(result, list):
            raise ValueError("validate must return a list of nonempty single-line strings")
        result = list(result)
        if not all(map(line_text, result)):
            raise ValueError("validate must return a list of nonempty single-line strings")
        return result
    except SourceBinding:
        raise
    except (Exception, SystemExit) as error:
        return [f"validation: {describe(error)}"]
