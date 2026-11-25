from pathlib import Path
def available(seed: Path):
    path = seed.home()
    return path.exists()
