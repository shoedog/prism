"""Keep this directory out of the shared eval project's pytest run.

The prototype and its tests need packages that only this directory's own uv project installs
(`cd eval/schema_impact && uv run pytest`). When pytest runs from the shared eval project the
rootdir is `eval`, and nothing under this directory is collected.
"""
from pathlib import Path


def pytest_ignore_collect(collection_path, config):
    return Path(config.rootpath).resolve() != Path(__file__).resolve().parent
