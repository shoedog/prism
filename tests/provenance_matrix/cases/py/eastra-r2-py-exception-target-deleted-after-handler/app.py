import os

def label():
    edition = os.getenv("EDITION")
    try:
        raise ValueError("preview")
    except ValueError as edition:
        pass
    try:
        return edition
    except UnboundLocalError:
        return "missing"
