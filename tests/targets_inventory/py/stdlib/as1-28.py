from pathlib import Path
def cache_file(paths) -> Path:
    return next(p for p in paths)
class Cache:
    @classmethod
    def read(cls, paths):
        path = cache_file(paths)
        return path.exists()
