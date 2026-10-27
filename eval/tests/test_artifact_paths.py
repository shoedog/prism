import re
from pathlib import Path


def test_declared_artifact_disk_joins_are_centralized():
    package = Path(__file__).parents[1] / "tier_a"
    forbidden = re.compile(
        r"(?:repo|eval_dir\.parent|cand\._repo)\s*/\s*"
        r"(?:lock|trial|cand\.lock)\.prism\["
    )
    archive = re.compile(r"Path\((?:lock|cand\.lock)\.prism\[.sut.\]\[.archive.\]\)")
    violations = []
    for source in sorted(package.glob("*.py")):
        if source.name == "artifacts.py":
            continue
        for line_number, line in enumerate(source.read_text().splitlines(), 1):
            if forbidden.search(line) or archive.search(line):
                violations.append(f"{source.name}:{line_number}: {line.strip()}")
    assert violations == []
