import sys
from pathlib import Path


def main():
    return sys.argv[1], Path("Dockerfile").read_text()
