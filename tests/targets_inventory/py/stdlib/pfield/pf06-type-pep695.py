from pathlib import Path
type Alias = int
type Pair[T] = tuple[T, T]

def f(path: Path):
    return path.exists()
