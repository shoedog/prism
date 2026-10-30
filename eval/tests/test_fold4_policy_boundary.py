import ast
import re
import shutil
from pathlib import Path

from tests.helpers import edit
from tier_a.closure import _POLICY_MODULES, policy_digest, policy_manifest


EVAL_DIR = Path(__file__).resolve().parents[1]
PACKAGE = EVAL_DIR / "tier_a"
ENTRY_MODULES = {"runner", "admission", "compare_report"}
PLUMBING_MODULES = {"cli", "commands"}


def _copy_eval(tmp_path: Path) -> Path:
    destination = tmp_path / "eval"
    shutil.copytree(PACKAGE, destination / "tier_a")
    shutil.copy2(EVAL_DIR / "corpora.toml", destination / "corpora.toml")
    return destination


def _imports(path: Path) -> set[str]:
    tree = ast.parse(path.read_text())
    found = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.level and node.module:
            found.add(node.module.split(".")[0])
        elif isinstance(node, ast.ImportFrom) and node.level:
            found.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
            if (
                isinstance(node.func.value, ast.Name)
                and node.func.value.id == "importlib"
                and node.func.attr == "import_module"
                and node.args
                and isinstance(node.args[0], ast.Constant)
                and isinstance(node.args[0].value, str)
            ):
                match = re.match(r"(?:tier_a|\.)(?:\.([A-Za-z_]\w*))?", node.args[0].value)
                if match and match.group(1):
                    found.add(match.group(1))
    return {name for name in found if (PACKAGE / f"{name}.py").exists()}


def _closure(entries: set[str]) -> set[str]:
    pending = list(entries)
    seen = set()
    while pending:
        module = pending.pop()
        if module in seen:
            continue
        seen.add(module)
        pending.extend(_imports(PACKAGE / f"{module}.py") - seen)
    return seen


def test_measurement_import_closure_equals_policy_modules():
    assert {Path(name).stem for name in _POLICY_MODULES} == _closure(ENTRY_MODULES)


def test_hashed_modules_never_import_plumbing_or_unhashed_modules():
    hashed = {Path(name).stem for name in _POLICY_MODULES}
    for module in sorted(hashed):
        imports = _imports(PACKAGE / f"{module}.py")
        assert not imports & PLUMBING_MODULES, module
        assert imports <= hashed, (module, sorted(imports - hashed))


def test_cli_is_absolute_plumbing_only():
    tree = ast.parse((PACKAGE / "cli.py").read_text())
    forbidden = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.Compare, ast.BinOp, ast.AugAssign)):
            forbidden.append((node.lineno, type(node).__name__))
        if (
            isinstance(node, ast.Constant)
            and isinstance(node.value, (int, float))
            and not isinstance(node.value, bool)
            and node.value not in {0, 1, 2, 3}
        ):
            forbidden.append((node.lineno, f"numeric:{node.value}"))
    assert forbidden == []


def test_duplicate_name_threshold_mutations_move_policy_digest(tmp_path):
    eval_dir = _copy_eval(tmp_path)
    source_path = next(
        path
        for path in (eval_dir / "tier_a").glob("*.py")
        if "oracle_named_lines" in path.read_text() and "> 1" in path.read_text()
    )
    before = policy_digest(policy_manifest(eval_dir))
    edit(source_path, "> 1", "> 0")
    lower = policy_digest(policy_manifest(eval_dir))
    assert lower != before
    edit(source_path, "> 0", "> 2")
    assert policy_digest(policy_manifest(eval_dir)) not in {before, lower}


def test_cli_argparse_help_edit_does_not_move_policy_digest(tmp_path):
    eval_dir = _copy_eval(tmp_path)
    cli = eval_dir / "tier_a/cli.py"
    before = policy_digest(policy_manifest(eval_dir))
    edit(cli, "symbols per stratum", "sampled symbols per stratum")
    assert policy_digest(policy_manifest(eval_dir)) == before
