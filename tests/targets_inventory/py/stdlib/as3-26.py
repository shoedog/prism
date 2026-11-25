from pathlib import Path
def exists():
    return Path.cwd().exists()
