"""Estimate outbound HTTP call sites using source-text patterns."""

from collections import Counter
from pathlib import Path
import re


_GO_CALL = re.compile(
    r"\b(http\.(?:Get|Post|PostForm|Head|NewRequest|NewRequestWithContext)"
    r"|(?i:\w*client\w*)\.Do)\s*\("
)
_VERB = r"get|post|put|delete|patch|head|options"
_PYTHON_CALL = re.compile(
    rf"\b(requests\.(?:{_VERB}|request)|httpx\.(?:{_VERB}|Client|AsyncClient)"
    r"|urllib\.request\.urlopen|aiohttp\.ClientSession)\s*\("
)


def count(repo: Path) -> dict[str, int]:
    counts: Counter[str] = Counter()
    for path in repo.rglob("*.go"):
        if path.is_file() and not path.is_symlink() and not path.name.endswith("_test.go"):
            counts.update(_GO_CALL.findall(path.read_text(encoding="utf-8")))
    for path in repo.rglob("*.py"):
        if (
            path.is_file() and not path.is_symlink()
            and not path.name.startswith("test_") and not path.name.endswith("_test.py")
            and path.name != "test.py"
            and not {"test", "tests"}.intersection(path.relative_to(repo).parts[:-1])
        ):
            counts.update(_PYTHON_CALL.findall(path.read_text(encoding="utf-8")))
    return dict(sorted(counts.items()))
