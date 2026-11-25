from typing import cast
import os

def labels():
    claimed_type = input()
    from_env = os.getenv("EDITION")
    number = cast(claimed_type, 7)
    label = cast(str, from_env)
    return number, label
