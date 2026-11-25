from pathlib import Path


def run(path, recursive):
    path_object = Path(path)
    files = []
    if path_object.is_file():
        files.append(path_object)
    elif path_object.exists() and not recursive:
        files.extend(path_object.glob("*.json"))
    return files
