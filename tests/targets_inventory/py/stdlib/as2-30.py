from pathlib import Path
def info():
    return Path.home().stat()
