from pathlib import Path
def f():
    with Path("pyproject.toml").open("rb") as fp:
        return fp
