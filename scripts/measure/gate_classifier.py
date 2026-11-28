"""Build the private file views for a gate classifier and its helpers."""

from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
import shutil
import stat
import sys
import sysconfig
from tempfile import TemporaryDirectory
from types import ModuleType

from . import gate_file
from .inputs import sha256_file

PACKAGE = "measure_gate_classifier"


@contextmanager
def imports(directory, helpers):
    search = sys.path
    no_bytecode = sys.dont_write_bytecode
    roots = {PACKAGE, *(Path(name).parts[0].split(".")[0] for name in helpers)}
    saved = {name: module for name, module in sys.modules.items() if name.split(".")[0] in roots}
    package = ModuleType(PACKAGE)
    package.__path__ = [str(directory)]
    for name in saved:
        sys.modules.pop(name)
    sys.modules[PACKAGE] = package
    sys.path = [str(directory), str(Path(__file__).resolve().parents[1]), *filter(None, (
        sysconfig.get_path("stdlib"), sysconfig.get_config_var("DESTSHARED"),
    ))]
    sys.dont_write_bytecode = True
    try:
        yield
    finally:
        sys.path = search
        sys.dont_write_bytecode = no_bytecode
        for name in tuple(sys.modules):
            if name.split(".")[0] in roots:
                sys.modules.pop(name)
        sys.modules.update(saved)


def copy_bound(source, target, expected):
    target.parent.mkdir(parents=True, exist_ok=True)
    if not stat.S_ISREG(source.lstat().st_mode):
        raise ValueError(f"linked classifier snapshot source: {source}")
    shutil.copyfile(source, target)
    target.chmod(0o400)
    if sha256_file(target) != expected:
        raise ValueError(f"classifier snapshot digest changed: {source}")


@contextmanager
def snapshot(decision, step, runs, context, output_root):
    prefix = step["id"]
    closure = step.get("classifier_files", {})
    with TemporaryDirectory(prefix=".classifier-", dir=output_root) as scratch:
        root = Path(scratch)
        expected = {}

        def put(source, path, checksum):
            copy_bound(source, path, checksum)
            expected[path] = checksum

        classifier_role = f"{prefix}/classifier"
        entry = decision.paths[classifier_role]
        module = root / "module" / entry.name
        put(entry, module, decision.files[classifier_role])
        for role, bound in zip(("base", "head"), runs):
            directory, record, digest = bound
            view = root / role
            put(directory / "receipt.json", view / "receipt.json", digest)
            for name, artifact in record["artifacts"].items():
                put(directory / name, view / name, artifact["sha256"])
        for group, files in closure.items():
            for name in files:
                role = f"{prefix}/classifier_files/{group}/{name}"
                target = root / ("module" if group == "helpers" else group) / name
                put(decision.paths[role], target, decision.files[role])

        def recheck():
            actual = {}
            for path in root.rglob("*"):
                mode = path.lstat().st_mode
                if stat.S_ISREG(mode):
                    actual[path] = sha256_file(path)
                elif not stat.S_ISDIR(mode):
                    raise ValueError(f"classifier snapshot contains a linked or special entry: {path.name}")
            gate_file.require(actual == expected, "classifier snapshot changed")

        context.update(classifier_runs=(root / "base", root / "head"), recheck_classifier=recheck,
                       classifier_package=PACKAGE)
        with imports(module.parent, closure.get("helpers", {})):
            yield module
            recheck()
