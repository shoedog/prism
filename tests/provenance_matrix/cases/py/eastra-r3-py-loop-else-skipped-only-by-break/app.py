import os
import sys

def selected():
    for candidate in sys.argv[1:]:
        if candidate:
            edition = candidate
            break
    else:
        edition = os.environ["EDITION"]
        return edition
    return edition
