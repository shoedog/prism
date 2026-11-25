from pathlib import Path


def apply(pot_path: Path, text):
    if pot_path.exists():
        pot_path.write_text(text)
