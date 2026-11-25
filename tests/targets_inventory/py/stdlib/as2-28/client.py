from .compat import importlib_metadata
def plugins():
    return importlib_metadata.entry_points()
