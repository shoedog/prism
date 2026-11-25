import os
import sys

def labels():
    values = sys.argv[1:]
    shown = (word for word in values)
    values = [os.getenv("EDITION")]
    for word in shown:
        return word
