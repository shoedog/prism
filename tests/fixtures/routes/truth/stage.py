"""Fixture staging and the isolated Python child's shared instrumentation."""
import ast
from collections import Counter
import importlib
import inspect
from io import StringIO
import json
from pathlib import Path
import re
import runpy
import shutil
import stat
import sys
import sysconfig
import tokenize

HERE = Path(__file__).resolve().parent
PYTHON_ENTRY = re.compile(r"([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)*):([a-zA-Z_]\w*)(\(\))?")
JS_EXTENSIONS = {".js", ".mjs", ".cjs", ".ts", ".mts", ".cts"}
PINNED_IMPORTS = set("""
    flask fastapi starlette pydantic werkzeug jinja2 click itsdangerous markupsafe
    blinker anyio typing_extensions
""".split())
PYTHON_REGISTRATIONS = set("""
    add_url_rule add_api_route add_api_websocket_route api_route route websocket
    get post put delete patch head options trace
""".split())


class Refusal(ValueError):
    """A fixture violates the executed-truth contract."""


class HandRequired(Refusal):
    """No supported way to derive truth for this shape."""


class EnvironmentError(Refusal):
    """The pinned runtime is unavailable or mismatched."""


def relative_path(value, label):
    if not isinstance(value, str) or not value or "\\" in value:
        raise Refusal(f"invalid {label}: {value!r}")
    if any(part in {"", ".", ".."} for part in value.split("/")):
        raise Refusal(f"invalid {label}: {value!r}")
    return Path(value)


def check_item(path):
    info = path.lstat()
    if stat.S_ISLNK(info.st_mode):
        raise Refusal(f"symlink in row: {path.name}")
    if stat.S_ISDIR(info.st_mode):
        return True
    if not stat.S_ISREG(info.st_mode):
        raise Refusal(f"non-regular file in row: {path.name}")
    if info.st_nlink != 1:
        raise Refusal(f"hardlink in row: {path.name}")
    return False


def validate_tree(folder):
    for path in sorted(folder.iterdir()):
        if check_item(path):
            if path.name in {"node_modules", "__pycache__"}:
                raise Refusal(f"reserved directory in row: {path.name}")
            validate_tree(path)
        elif path.suffix in {".pyc", ".pyo", ".pyd", ".so", ".node"}:
            raise Refusal(f"compiled module in row: {path.name}")


def stage(root, row, work, dependencies):
    relative = relative_path(row.get("dir"), "row directory")
    source = root
    for part in relative.parts:
        source = source / part
        if not check_item(source):
            raise Refusal(f"not a row directory: {relative}")
    if source.resolve() != source or not source.is_relative_to(root):
        raise Refusal(f"invalid row directory: {relative}")
    validate_tree(source)

    entry = row.get("entry")
    group = row.get("group", "")
    if group in {"w2-gin", "w2-method-value", "w2-nethttp"}:
        raise HandRequired("Go has no executed-truth adapter")
    if entry is None:
        if group.startswith("w3-"):
            raise HandRequired("framework group has no executed-truth adapter")
        raise Refusal("missing entry for executed-truth probe")
    if not isinstance(entry, str):
        raise Refusal("entry must be a string")
    if PYTHON_ENTRY.fullmatch(entry):
        runtime = "python"
    else:
        script = relative_path(entry, "JS entry")
        if script.suffix not in JS_EXTENSIONS or not (source / script).is_file():
            raise Refusal(f"invalid JS entry: {entry!r}")
        runtime = "node"

    destination = work / "row"
    shutil.copytree(source, destination, symlinks=True)
    validate_tree(destination)
    if runtime == "node":
        if not (dependencies / "express" / "package.json").is_file():
            raise EnvironmentError("ROUTES_TRUTH_NODE must point to pinned node_modules")
        try:
            shutil.copytree(dependencies, work / "node_modules", symlinks=True)
        except OSError as exc:
            raise EnvironmentError(f"cannot stage pinned node_modules: {exc}") from exc
    return runtime, destination, entry


def module_exists(directory, dotted):
    module = directory.joinpath(*dotted.split("."))
    return module.with_suffix(".py").is_file() or module.is_dir()


def python_sources(root):
    sources = {}
    allowed = PINNED_IMPORTS | sys.stdlib_module_names | set(sys.builtin_module_names)
    for path in sorted(root.rglob("*.py")):
        file = path.relative_to(root).as_posix()
        with tokenize.open(path) as stream:
            source = stream.read()
        tree = ast.parse(source, filename=file)
        sources[file] = source, tree
        package = path.parent.relative_to(root).parts
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom) and node.level:
                if node.level > len(package):
                    raise Refusal(f"{file}:{node.lineno}: relative import escapes row package")
                base = root.joinpath(*package[:len(package) - node.level + 1])
                if node.module and not module_exists(base, node.module):
                    raise Refusal(f"{file}:{node.lineno}: relative import absent from row: {node.module}")
                continue
            elif isinstance(node, ast.ImportFrom):
                names = [node.module]
            else:
                continue
            for name in names:
                if name.split(".")[0] not in allowed and not module_exists(root, name):
                    raise Refusal(f"{file}:{node.lineno}: undeclared Python import: {name}")
    return sources


def verify_module_origins(root):
    venv = Path(sys.prefix).resolve()
    standard_lib = Path(sysconfig.get_path("stdlib")).resolve()

    def allowed(origin):
        path = Path(origin).resolve()
        if path.is_relative_to(root) or path.is_relative_to(venv):
            return True
        if path.is_relative_to(standard_lib):
            parts = path.relative_to(standard_lib).parts
            return not {"site-packages", "dist-packages"}.intersection(parts)
        return False

    for name, module in tuple(sys.modules.items()):
        if module is None:
            continue
        if name in {"typing.io", "typing.re"}:
            # CPython 3.12 publishes deprecated class aliases in sys.modules.
            if getattr(sys.modules["typing"], name.split(".")[1], None) is module:
                continue
        spec = getattr(module, "__spec__", None)
        origin = getattr(spec, "origin", None)
        if origin in {"built-in", "frozen"}:
            continue
        if name == "__main__" and spec is None and not getattr(module, "__file__", None):
            continue
        locations = list(getattr(module, "__path__", ()))
        locations.extend(value for value in (origin, getattr(module, "__file__", None)) if value)
        if not locations or any(not allowed(value) for value in locations):
            raise Refusal(f"Python module outside row, venv or stdlib: {name}")


def methods(values):
    return None if values is None else sorted({value.upper() for value in values})


def method_arguments(values):
    if values is None or isinstance(values, str):
        return values
    if type(values) in {list, tuple}:
        return list(values)
    if type(values) in {set, frozenset}:
        return sorted(values)
    raise HandRequired("route method arguments are not a reusable built-in sequence")


class PythonTrace:
    def __init__(self, root, sources):
        self.root = root
        self.sources = sources
        self.calls = []
        self.ordinals = Counter()

    def filename(self, filename):
        if not filename or filename.startswith("<"):
            return None
        path = Path(filename)
        if path.is_relative_to(self.root):
            return path.relative_to(self.root).as_posix()
        return None

    def callback(self, view):
        code = getattr(view, "__code__", None)
        if code is None:
            code = getattr(getattr(type(view), "__call__", None), "__code__", None)
        if code is None:
            return {"kind": "unlocated", "name": getattr(view, "__qualname__", type(view).__qualname__)}
        file = self.filename(code.co_filename)
        if file is None:
            path = Path(code.co_filename)
            for prefix, label in ((Path(sys.prefix), "<venv>"), (Path(sys.base_prefix), "<stdlib>")):
                if path.is_relative_to(prefix):
                    file = f"{label}/{path.relative_to(prefix).as_posix()}"
                    break
            else:
                file = f"<external>/{path.name}"
        result = {"kind": "python", "file": file, "name": code.co_qualname, "first_line": code.co_firstlineno}
        if file in self.sources:
            _, tree = self.sources[file]
            for node in ast.walk(tree):
                if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    continue
                first = min([node.lineno, *(d.lineno for d in node.decorator_list)])
                if first == code.co_firstlineno and node.name == code.co_name:
                    result["end_line"] = node.end_lineno
                    break
        return result

    def path_argument(self, file, positions, value):
        source, tree = self.sources[file]
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            if (node.lineno, node.end_lineno, node.col_offset, node.end_col_offset) != positions:
                continue
            if not isinstance(node.func, ast.Attribute) or node.func.attr not in PYTHON_REGISTRATIONS:
                return None
            keyword = "rule" if node.func.attr == "add_url_rule" else "path"
            argument = node.args[0] if node.args else next(
                (item.value for item in node.keywords if item.arg == keyword), None,
            )
            if argument is None or isinstance(argument, ast.Starred):
                return None
            raw = ast.get_source_segment(source, argument)
            literal = None
            if isinstance(argument, ast.Constant) and isinstance(argument.value, str):
                tokens = tokenize.generate_tokens(StringIO(raw).readline)
                if "\\" not in raw and argument.value == value and sum(t.type == tokenize.STRING for t in tokens) == 1:
                    literal = value
            return {"raw": raw, "literal": literal}
        return None

    def record(self, kind, arguments, view, origin=None, mounted=False):
        frame = sys._getframe(1)
        while frame:
            file = self.filename(frame.f_code.co_filename)
            if file in self.sources:
                break
            frame = frame.f_back
        if frame is None:
            raise Refusal("registration has no fixture stack frame")
        line = frame.f_lineno
        self.ordinals[file, line] += 1
        site = {"file": file, "line": line, "ordinal": self.ordinals[file, line]}
        positions = inspect.getframeinfo(frame, context=0).positions
        if positions:
            site.update(column_byte=positions.col_offset, end_line=positions.end_lineno,
                        end_column_byte=positions.end_col_offset)
        call = {
            "id": len(self.calls) + 1, "kind": kind, "site": site, "args": arguments,
            "mounted": mounted, "callbacks": [] if view is None else [self.callback(view)],
        }
        if origin is not None:
            call["origin_call_id"] = origin["id"]
        if not mounted and positions is not None:
            path = self.path_argument(file, positions, arguments.get("rule", arguments.get("path")))
            if path is not None:
                call["path"] = path
        self.calls.append(call)
        return call


def python_child(root, entry):
    from contextlib import redirect_stdout
    from importlib.metadata import version

    root = Path(root)
    try:
        with redirect_stdout(sys.stderr):
            if sys.version_info[:2] != (3, 12) or sys.prefix == sys.base_prefix:
                raise EnvironmentError("ROUTES_TRUTH_VENV must be a CPython 3.12 virtual environment")
            for name, pin in {"flask": "3.1.3", "fastapi": "0.115.12", "starlette": "0.46.2"}.items():
                try:
                    installed = version(name)
                except importlib.metadata.PackageNotFoundError as exc:
                    raise EnvironmentError(f"missing pinned Python package: {name}") from exc
                if installed != pin:
                    raise EnvironmentError(f"expected {name} {pin}, found {installed}")
            sources = python_sources(root)
            module_name, attribute, factory = PYTHON_ENTRY.fullmatch(entry).groups()
            if not module_exists(root, module_name):
                raise Refusal(f"Python entry module absent from row: {module_name}")
            trace = PythonTrace(root, sources)
            adapters = [
                runpy.run_path(str(HERE / f"{name}_routes.py"))["install"](
                    trace, Refusal, HandRequired, methods, method_arguments,
                )
                for name in ("flask", "fastapi")
            ]
            sys.path.insert(0, str(root))
            module = importlib.import_module(module_name)
            if trace.filename(getattr(module, "__file__", None)) not in sources:
                raise Refusal(f"Python entry module resolved outside row: {module_name}")
            application = getattr(module, attribute)
            if factory:
                application = application()
            framework, served = None, []
            for name, application_type, extract in adapters:
                if isinstance(application, application_type):
                    framework, served = name, extract(application)
                    break
            if framework is None and (application is not None or trace.calls):
                raise Refusal("entry must export a Flask/FastAPI app, or None for a route-free row")
            if framework is not None and any(call["framework"] != framework for call in trace.calls):
                raise HandRequired("row registered routes in more than one framework")
            verify_module_origins(root)
        output = {"status": "executed", "framework": framework, "calls": trace.calls, "served": served}
    except Exception as exc:
        status = {EnvironmentError: "environment_error", HandRequired: "hand_required"}.get(type(exc), "error")
        output = {"status": status, "error": str(exc) or type(exc).__name__}
    print(json.dumps(output, allow_nan=False))
