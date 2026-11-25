import os
def label():
    edition = os.getenv("EDITION")
    snapshots = [(edition := input()) for _ in (0,)]
    return edition
