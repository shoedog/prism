from pathlib import Path
REGISTRY: Path = directory / "registry.txt"
def f():
    return REGISTRY.exists()
