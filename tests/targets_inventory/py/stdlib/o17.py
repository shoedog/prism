from pathlib import Path as P


def text(p):
    return P(p).read_text()
