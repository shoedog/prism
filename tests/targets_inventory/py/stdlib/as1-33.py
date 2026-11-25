from pathlib import Path
def read(root):
    return (Path(root) / "settings").read_text()
