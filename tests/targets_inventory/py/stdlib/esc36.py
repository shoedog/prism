from pathlib import Path, WindowsPath as Concrete
setattr(Concrete, "read_text", lambda self: "replacement")
def read(path):
    p = Path(path)
    return p.read_text()
