from pathlib import Path


def build(isolated_filesystem):
    frontend_dist: Path = isolated_filesystem / "frontend" / "dist"
    frontend_dist.mkdir()
