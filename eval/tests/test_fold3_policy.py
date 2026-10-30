import ast
from pathlib import Path

import pytest

from tests.helpers import copy_eval_policy_files, edit
from tier_a.closure import _POLICY_MODULES, policy_digest, policy_manifest


EVAL_DIR = Path(__file__).resolve().parents[1]
SENSITIVE_META = {
    "baseline_invalid",
    "invalid_reasons",
    "oracle_error_rate",
    "sut_error_rate",
    "sampling_inventory",
    "sample_per_stratum",
    "stale_adjudications",
}
SENSITIVE_SECTIONS = {
    "m1",
    "m2",
    "m3",
    "pinned",
    "summary",
    "frame",
    "member_sites",
    "probe_status",
    "_boundary_state",
}


def _subscript_path(node):
    path = []
    while isinstance(node, ast.Subscript):
        if not isinstance(node.slice, ast.Constant) or not isinstance(node.slice.value, str):
            path.append("*")
        else:
            path.append(node.slice.value)
        node = node.value
    if isinstance(node, ast.Name):
        path.append(node.id)
    return list(reversed(path))


def _policy_imports(tree):
    modules = {Path(name).stem for name in _POLICY_MODULES}
    functions = set()
    aliases = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.ImportFrom):
            continue
        module = (node.module or "").split(".")[-1]
        for imported in node.names:
            local = imported.asname or imported.name
            if module in modules:
                functions.add(local)
            elif module == "" and imported.name in modules:
                aliases.add(local)
    return functions, aliases


def _is_policy_call(node, functions, aliases):
    if not isinstance(node, ast.Call):
        return False
    if isinstance(node.func, ast.Name):
        return node.func.id in functions
    return (
        isinstance(node.func, ast.Attribute)
        and isinstance(node.func.value, ast.Name)
        and node.func.value.id in aliases
    )


def _sensitive_target(node):
    path = _subscript_path(node)
    return bool(
        path
        and (
            (path[0] == "run" and len(path) > 1 and path[1] in SENSITIVE_SECTIONS)
            or (
                path[:2] == ["run", "meta"]
                and len(path) > 2
                and path[2] in SENSITIVE_META
            )
            or "_strata" in path
            or path[0] == "strata_counts"
        )
    )


def inline_policy_violations(source):
    tree = ast.parse(source)
    functions, aliases = _policy_imports(tree)
    violations = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.Assign, ast.AnnAssign)):
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            if any(_sensitive_target(target) for target in targets) and not _is_policy_call(
                node.value, functions, aliases
            ):
                violations.append(node.lineno)
        elif isinstance(node, ast.AugAssign) and _sensitive_target(node.target):
            violations.append(node.lineno)
        elif isinstance(node, ast.Dict):
            for key, value in zip(node.keys, node.values):
                if (
                    isinstance(key, ast.Constant)
                    and key.value in SENSITIVE_META | SENSITIVE_SECTIONS
                    and not _is_policy_call(value, functions, aliases)
                ):
                    violations.append(node.lineno)
        elif isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
            path = _subscript_path(node.func.value)
            if path[:2] == ["run", "meta"]:
                for keyword in node.keywords:
                    if keyword.arg in SENSITIVE_META:
                        violations.append(node.lineno)
                if (
                    node.func.attr == "setdefault"
                    and node.args
                    and isinstance(node.args[0], ast.Constant)
                    and node.args[0].value in SENSITIVE_META
                ):
                    violations.append(node.lineno)
    return sorted(set(violations))


def test_cli_contains_no_inline_run_record_policy():
    source = (EVAL_DIR / "tier_a/cli.py").read_text()
    assert inline_policy_violations(source) == []


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ("oracle_version != lock_oracle_version", "oracle_version == lock_oracle_version"),
        ("len(fds) * 2", "len(fds) * 4"),
        ('["successful"] += 1', '["successful"] += 0'),
        ("if capable:", "if not capable:"),
    ],
    ids=(
        "oracle-version-condition",
        "eligibility-count",
        "successful-probe-credit",
        "unsupported-oracle-rule",
    ),
)
def test_every_r2_policy_mutation_moves_policy_digest(tmp_path, old, new):
    eval_dir = copy_eval_policy_files(tmp_path)
    before = policy_digest(policy_manifest(eval_dir))
    edit(eval_dir / "tier_a/policy.py", old, new)
    assert policy_digest(policy_manifest(eval_dir)) != before
