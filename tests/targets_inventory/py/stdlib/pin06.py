from pathlib import Path

RELOAD_TRIGGER: Path = Path(__file__).resolve().parent / ".reload_trigger"


def trigger():
    RELOAD_TRIGGER.touch()
