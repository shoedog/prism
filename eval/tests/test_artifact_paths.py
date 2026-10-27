import ast
from pathlib import Path

import pytest


ARTIFACT_KEYS = {"archive", "inputs", "manifest", "path"}


def _declared_artifact_subscript(expression):
    keys = []
    while isinstance(expression, ast.Subscript):
        if isinstance(expression.slice, ast.Constant):
            keys.append(expression.slice.value)
        expression = expression.value
    return (
        isinstance(expression, ast.Attribute)
        and expression.attr == "prism"
        and any(key in ARTIFACT_KEYS for key in keys)
    )


def _direct_artifact_join_violations(sources):
    violations = []
    for source in sorted(sources):
        if source.name == "artifacts.py":
            continue
        tree = ast.parse(source.read_text(), filename=str(source))
        for node in ast.walk(tree):
            if (
                isinstance(node, ast.BinOp)
                and isinstance(node.op, ast.Div)
                and _declared_artifact_subscript(node.right)
            ):
                violations.append(f"{source.name}:{node.lineno}")
    return violations


def test_declared_artifact_disk_joins_are_centralized():
    package = Path(__file__).parents[1] / "tier_a"
    assert _direct_artifact_join_violations(package.glob("*.py")) == []


@pytest.mark.parametrize(
    "join",
    [
        'repo / lock.prism["snapshot"]["path"]',
        'repo / (\n    lock.prism["snapshot"]["path"]\n)',
    ],
)
def test_declared_artifact_join_guard_rejects_formatting_mutations(tmp_path, join):
    source = tmp_path / "reader.py"
    source.write_text(f"resolved = {join}\n")

    assert _direct_artifact_join_violations([source]) == ["reader.py:1"]


def test_declared_artifact_join_guard_exempts_resolver_and_identity_reads(tmp_path):
    resolver = tmp_path / "artifacts.py"
    resolver.write_text('resolved = repo / lock.prism["snapshot"]["path"]\n')
    identity = tmp_path / "identity.py"
    identity.write_text('matches = lock.prism["snapshot"]["path"] == expected\n')

    assert _direct_artifact_join_violations([resolver, identity]) == []
